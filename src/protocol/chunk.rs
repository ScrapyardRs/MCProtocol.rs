use std::io::{Cursor, Read};

use drax::extension::*;
use drax::nbt::{CompoundTag, Tag};
use drax::transport::TransportProcessorContext;
use drax::transport::{DraxTransport, Result};
use drax::VarInt;

use crate::protocol::bit_storage::{BitSetValidationError, BitStorage};
use crate::protocol::play::ceil_log_2;

pub enum Index {
    NewSize(u8),
    CurrentIndex(i32),
}

impl Index {
    pub fn current(self) -> i32 {
        match self {
            Index::NewSize(_) => panic!("Unexpected resize."),
            Index::CurrentIndex(idx) => idx,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Strategy {
    Section,
    Biome,
}

impl Strategy {
    const BIOME_SIZE_BITS: i32 = 2;
    const SECTION_SIZE_BITS: i32 = 4;
    const BIOME_DIRECT_ENTRY_SIZE: i32 = 6;
    const SECTION_DIRECT_ENTRY_SIZE: i32 = 15;

    pub const fn bit_size(self, given: u8) -> i32 {
        match self {
            Strategy::Section => match given {
                0 => 0,
                1 | 2 | 3 | 4 => 4,
                x if x <= 8 => given as i32,
                _ => self.entry_size(),
            },
            Strategy::Biome => given as i32,
        }
    }

    pub const fn locked_entry_count(self) -> i32 {
        match self {
            Strategy::Section => 4096,
            Strategy::Biome => 64,
        }
    }

    pub const fn entry_size(self) -> i32 {
        match self {
            Strategy::Section => Strategy::SECTION_DIRECT_ENTRY_SIZE,
            Strategy::Biome => Strategy::BIOME_DIRECT_ENTRY_SIZE,
        }
    }

    pub const fn size(self) -> i32 {
        match self {
            Strategy::Section => 1 << (Strategy::SECTION_SIZE_BITS * 3),
            Strategy::Biome => 1 << (Strategy::BIOME_SIZE_BITS * 3),
        }
    }

    pub const fn retrieve_index(self, x: i32, y: i32, z: i32) -> i32 {
        let bits = match self {
            Strategy::Section => Strategy::SECTION_SIZE_BITS,
            Strategy::Biome => Strategy::BIOME_SIZE_BITS,
        };
        (((y << bits) | z) << bits) | x
    }
}

// todo: update palette to take a "state"?
#[derive(Debug)]
pub enum Palette {
    SingleValue { block_type_id: VarInt },
    Indirect { palette: Vec<VarInt> },
    Direct,
}

impl Palette {
    fn get(&self, id_index: i32) -> VarInt {
        match self {
            Palette::SingleValue { block_type_id } => *block_type_id,
            Palette::Indirect { palette, .. } => palette[id_index as usize].clone(),
            Palette::Direct => id_index.into(),
        }
    }

    fn id_for(&self, block_id: VarInt) -> Index {
        match self {
            Palette::SingleValue { block_type_id } => {
                if *block_type_id == block_id {
                    Index::CurrentIndex(0)
                } else {
                    Index::NewSize(2)
                }
            }
            Palette::Indirect { palette, .. } => {
                for (index, x) in palette.iter().enumerate() {
                    if *x == block_id {
                        return Index::CurrentIndex(index as i32);
                    }
                }
                return Index::NewSize(palette.len() as u8 + 1);
            }
            Palette::Direct => Index::CurrentIndex(block_id.into()),
        }
    }

    fn size(&self, context: &mut TransportProcessorContext) -> Result<usize> {
        match self {
            Palette::SingleValue { block_type_id } => size_var_int(*block_type_id, context),
            Palette::Indirect { palette, .. } => {
                let mut size = size_var_int(palette.len() as i32, context)?;
                for t in palette {
                    size += size_var_int(*t, context)?;
                }
                Ok(size)
            }
            Palette::Direct => Ok(0),
        }
    }
}

#[derive(Debug)]
pub struct PaletteContainer {
    bits_per_entry: u8,
    palette: Palette,
    storage: BitStorage,
}

impl PaletteContainer {
    pub fn get(&self, index: i32) -> std::result::Result<VarInt, BitSetValidationError> {
        let out = self.storage.get(index)?;
        Ok(self.palette.get(out))
    }

    pub fn copy_to_new_linear(&self, new_value: VarInt) -> Palette {
        match &self.palette {
            Palette::SingleValue { block_type_id } => Palette::Indirect {
                palette: vec![*block_type_id, new_value],
            },
            Palette::Indirect { palette, .. } => {
                let mut new_palette = palette.clone();
                new_palette.push(new_value);
                return Palette::Indirect {
                    palette: new_palette,
                };
            }
            Palette::Direct => unreachable!(),
        }
    }

    pub fn set_all(
        &mut self,
        strategy: Strategy,
        indexes: Vec<i32>,
        block_id: VarInt,
    ) -> std::result::Result<(), BitSetValidationError> {
        match self.palette.id_for(block_id) {
            Index::CurrentIndex(id) => {
                for index in indexes {
                    self.storage.set(index, id)?;
                }
                Ok(())
            }
            Index::NewSize(new_size) => {
                let bits_per_entry = strategy.bit_size(new_size);
                let new_palette = match new_size {
                    0 | 1 => unreachable!(),
                    2 | 3 => self.copy_to_new_linear(block_id),
                    4 if matches!(strategy, Strategy::Section) => self.copy_to_new_linear(block_id),
                    x if x <= 8 && matches!(strategy, Strategy::Section) => {
                        self.copy_to_new_linear(block_id)
                    }
                    x => Palette::Direct,
                };
                let mut new_bitset = BitStorage::new(strategy.locked_entry_count(), bits_per_entry);
                for idx in 0..self.storage.size() {
                    let out = self.storage.get(idx)?;
                    let new = new_palette.id_for(out.into()).current();
                    println!("Translating: {} from idx {} to state {}", out, idx, new);
                    new_bitset.set(idx, new)?;
                }
                let raw_id = new_palette.id_for(block_id).current();
                for index in indexes {
                    new_bitset.set(index, raw_id)?;
                }
                self.bits_per_entry = bits_per_entry as u8;
                self.palette = new_palette;
                self.storage = new_bitset;
                Ok(())
            }
        }
    }

    pub fn set(
        &mut self,
        strategy: Strategy,
        index: i32,
        block_id: VarInt,
    ) -> std::result::Result<VarInt, BitSetValidationError> {
        match self.palette.id_for(block_id) {
            Index::CurrentIndex(id) => self.storage.get_and_set(index, id).map(Into::into),
            Index::NewSize(new_size) => {
                let bits_per_entry = strategy.bit_size(new_size);
                let new_palette = match new_size {
                    0 | 1 => unreachable!(),
                    2 | 3 => self.copy_to_new_linear(block_id),
                    4 if matches!(strategy, Strategy::Section) => self.copy_to_new_linear(block_id),
                    x if x <= 8 && matches!(strategy, Strategy::Section) => {
                        self.copy_to_new_linear(block_id)
                    }
                    x => Palette::Direct,
                };
                let mut new_bitset = BitStorage::new(strategy.locked_entry_count(), bits_per_entry);

                for idx in 0..self.storage.size() {
                    let out = self.storage.get(idx)?;
                    new_bitset.set(idx, new_palette.id_for(out.into()).current())?;
                }
                let index_index = new_palette.id_for(block_id).current();
                new_bitset.set(index, index_index)?;

                self.palette = new_palette;
                self.storage = new_bitset;
                Ok(VarInt::from(-1))
            }
        }
    }

    pub fn serialize(
        &self,
        write: &mut Cursor<Vec<u8>>,
        context: &mut TransportProcessorContext,
    ) -> Result<()> {
        self.bits_per_entry.write_to_transport(context, write)?;
        match &self.palette {
            Palette::SingleValue { block_type_id } => {
                write_var_int_sync(*block_type_id, context, write)?;
            }
            Palette::Indirect { palette, .. } => {
                write_var_int_sync(palette.len() as i32, context, write)?;
                for item in palette {
                    write_var_int_sync(*item, context, write)?;
                }
            }
            _ => (),
        }
        BitStorage::to_writer(&self.storage, write, context)
    }

    pub fn size(&self, context: &mut TransportProcessorContext) -> Result<usize> {
        let mut size = self.bits_per_entry.precondition_size(context)?;
        size += self.palette.size(context)?;
        self.storage.check_size(context).map(|x| x + size)
    }

    pub fn deserialize_with_strategy<R: Read>(
        strategy: Strategy,
        read: &mut R,
        context: &mut TransportProcessorContext,
    ) -> Result<Self> {
        let bits_per_entry = u8::read_from_transport(context, read)?;

        Ok(match bits_per_entry {
            0 => {
                let block_type_id = read_var_int_sync(context, read)?;
                PaletteContainer {
                    bits_per_entry,
                    palette: Palette::SingleValue { block_type_id },
                    storage: BitStorage::from_reader(
                        read,
                        0,
                        strategy.locked_entry_count(),
                        context,
                    )?,
                }
            }
            1 | 2 | 3 => {
                let mut palette = Vec::with_capacity(read_var_int_sync(context, read)? as usize);
                for _ in 0..palette.len() {
                    palette.push(read_var_int_sync(context, read)?);
                }
                PaletteContainer {
                    bits_per_entry,
                    palette: Palette::Indirect { palette },
                    storage: BitStorage::from_reader(
                        read,
                        if matches!(strategy, Strategy::Biome) {
                            bits_per_entry
                        } else {
                            4
                        },
                        strategy.locked_entry_count(),
                        context,
                    )?,
                }
            }
            4 if matches!(strategy, Strategy::Section) => {
                let mut palette = Vec::with_capacity(read_var_int_sync(context, read)? as usize);
                for _ in 0..palette.len() {
                    palette.push(read_var_int_sync(context, read)?);
                }
                PaletteContainer {
                    bits_per_entry,
                    palette: Palette::Indirect { palette },
                    storage: BitStorage::from_reader(
                        read,
                        4,
                        strategy.locked_entry_count(),
                        context,
                    )?,
                }
            }
            x if x <= 8 && matches!(strategy, Strategy::Section) => {
                let mut palette = Vec::with_capacity(read_var_int_sync(context, read)? as usize);
                for _ in 0..palette.len() {
                    palette.push(read_var_int_sync(context, read)?);
                }
                PaletteContainer {
                    bits_per_entry,
                    palette: Palette::Indirect { palette },
                    storage: BitStorage::from_reader(
                        read,
                        x,
                        strategy.locked_entry_count(),
                        context,
                    )?,
                }
            }
            x => PaletteContainer {
                bits_per_entry,
                palette: Palette::Direct,
                storage: BitStorage::from_reader(
                    read,
                    match strategy {
                        Strategy::Section => Strategy::SECTION_DIRECT_ENTRY_SIZE,
                        Strategy::Biome => Strategy::BIOME_DIRECT_ENTRY_SIZE,
                    } as u8,
                    strategy.locked_entry_count(),
                    context,
                )?,
            },
        })
    }
}

#[derive(Debug)]
pub struct ChunkSection {
    block_count: u16,
    states: PaletteContainer,
    biomes: PaletteContainer,
}

impl ChunkSection {
    fn increment_non_empty_block_count(&mut self) {
        self.block_count += 1;
    }

    fn decrement_non_empty_block_count(&mut self) {
        self.block_count -= 1;
    }

    fn get_block_id(
        &self,
        x: i32,
        y: i32,
        z: i32,
    ) -> std::result::Result<VarInt, BitSetValidationError> {
        self.states.get(Strategy::Section.retrieve_index(x, y, z))
    }

    fn set_block_id(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        block_id: VarInt,
    ) -> std::result::Result<VarInt, BitSetValidationError> {
        let state_index = Strategy::Section.retrieve_index(x, y, z);
        self.states.set(Strategy::Section, state_index, block_id)
    }

    fn rewrite_plane(
        &mut self,
        y: i32,
        block_id: VarInt,
    ) -> std::result::Result<(), BitSetValidationError> {
        let mut indexes = Vec::with_capacity(16 * 16);
        let p1 = y << Strategy::SECTION_SIZE_BITS;
        for z in 0..16 {
            let p2 = (p1 | z) << Strategy::SECTION_SIZE_BITS;
            for x in 0..16 {
                let idx = p2 | x;
                indexes.push(idx);
            }
        }
        self.states.set_all(Strategy::Section, indexes, block_id)
    }
}

impl DraxTransport for ChunkSection {
    fn write_to_transport(
        &self,
        context: &mut TransportProcessorContext,
        writer: &mut Cursor<Vec<u8>>,
    ) -> Result<()> {
        self.block_count.write_to_transport(context, writer)?;
        self.states.serialize(writer, context)?;
        self.biomes.serialize(writer, context)
    }

    fn read_from_transport<R: Read>(
        context: &mut TransportProcessorContext,
        read: &mut R,
    ) -> Result<Self>
    where
        Self: Sized,
    {
        let block_count = u16::read_from_transport(context, read)?;
        let states = PaletteContainer::deserialize_with_strategy(Strategy::Section, read, context)?;
        let biomes = PaletteContainer::deserialize_with_strategy(Strategy::Biome, read, context)?;
        Ok(Self {
            block_count,
            states,
            biomes,
        })
    }

    fn precondition_size(&self, context: &mut TransportProcessorContext) -> Result<usize> {
        let mut size = self.block_count.precondition_size(context)?;
        size += self.states.size(context)?;
        self.biomes.size(context).map(|x| x + size)
    }
}

#[derive(Debug)]
pub struct HeightMaps {
    world_surface: BitStorage,
    motion_blocking: BitStorage,
    cached_compound_tag: CompoundTag,
}

impl HeightMaps {
    fn write_to_transport(
        &self,
        _: &mut TransportProcessorContext,
        writer: &mut Cursor<Vec<u8>>,
    ) -> Result<()> {
        drax::nbt::write_nbt(&self.cached_compound_tag, writer)
    }

    fn read_from_transport<R: Read>(
        _: &mut TransportProcessorContext,
        read: &mut R,
        height: i32,
    ) -> Result<Self>
    where
        Self: Sized,
    {
        let tag = drax::nbt::read_nbt(read, 0x200000u64)?;
        match tag {
            None => return drax::transport::Error::cause("Failed to load tag, none was found."),
            Some(tag) => {
                match (
                    tag.get_tag(&"WORLD_SURFACE".to_string()),
                    tag.get_tag(&"MOTION_BLOCKING".to_string()),
                ) {
                    (
                        Some(Tag::LongArrayTag(world_surface)),
                        Some(Tag::LongArrayTag(motion_blocking)),
                    ) => Ok(HeightMaps {
                        world_surface: BitStorage::with_seeded_raw(
                            256,
                            ceil_log_2(height + 1),
                            world_surface.clone(),
                        )
                        .map_err(|err| drax::transport::Error::Unknown(Some(err.0)))?,
                        motion_blocking: BitStorage::with_seeded_raw(
                            256,
                            ceil_log_2(height + 1),
                            motion_blocking.clone(),
                        )
                        .map_err(|err| drax::transport::Error::Unknown(Some(err.0)))?,
                        cached_compound_tag: tag,
                    }),
                    (_, _) => {
                        return drax::transport::Error::cause(
                            "Could not find WORLD_SURFACE and MOTION_BLOCKING as larrs.",
                        );
                    }
                }
            }
        }
    }

    fn precondition_size(&self, _: &mut TransportProcessorContext) -> Result<usize> {
        Ok(drax::nbt::size_nbt(&self.cached_compound_tag))
    }
}

impl HeightMaps {
    pub(crate) fn cache_compound_tag(&mut self) {
        let mut tag = CompoundTag::new();
        tag.put_tag(
            "WORLD_SURFACE".to_string(),
            Tag::LongArrayTag(self.world_surface.get_raw().clone()),
        );
        tag.put_tag(
            "MOTION_BLOCKING".to_string(),
            Tag::LongArrayTag(self.motion_blocking.get_raw().clone()),
        );
        self.cached_compound_tag = tag;
    }

    fn get_index(x: i32, z: i32) -> i32 {
        x + (z * 16)
    }

    fn get_first_available(
        bit_set: &BitStorage,
        index: i32,
    ) -> std::result::Result<i32, BitSetValidationError> {
        bit_set.get(index).map(|x| x + DEFAULT_WORLD_MIN)
    }

    fn update_inner(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        new_block: VarInt,
    ) -> std::result::Result<(), BitSetValidationError> {
        let index = Self::get_index(x, z);
        let opaque = new_block != 0;
        let mut changed = false;

        for bit_set in vec![&mut self.world_surface, &mut self.motion_blocking] {
            let first_available = Self::get_first_available(&bit_set, index)?;
            if y > first_available + 1 && opaque {
                changed = true;
                bit_set.set(index, (y + 1) - DEFAULT_WORLD_MIN)?;
            }
        }

        self.cache_compound_tag();

        Ok(())
    }
}

#[derive(Debug)]
pub struct Chunk {
    min_height: i32,
    max_height: i32,
    chunk_x: i32,
    chunk_z: i32,
    height_maps: HeightMaps,
    chunk_sections: Vec<ChunkSection>,
}

const BLOCKS_PER_SECTION: i32 = 16;
const DEFAULT_WORLD_MIN: i32 = -64;
const DEFAULT_WORLD_HEIGHT: i32 = 320;
const DEFAULT_TOTAL_WORLD_HEIGHT: i32 = DEFAULT_WORLD_HEIGHT - DEFAULT_WORLD_MIN;

impl Chunk {
    pub fn new(x: i32, z: i32) -> Self {
        Self::using_world_height(x, z, DEFAULT_WORLD_MIN, DEFAULT_WORLD_HEIGHT)
    }

    pub fn using_world_height(x: i32, z: i32, min_height: i32, max_height: i32) -> Chunk {
        let sections = (max_height - min_height) / BLOCKS_PER_SECTION;
        let mut section_vec = Vec::with_capacity(sections as usize);
        for _ in 0..sections {
            section_vec.push(ChunkSection {
                block_count: 0,
                states: PaletteContainer {
                    bits_per_entry: 0,
                    palette: Palette::SingleValue {
                        block_type_id: VarInt::from(0),
                    },
                    storage: BitStorage::ZeroStorage {
                        size: Strategy::SECTION_DIRECT_ENTRY_SIZE,
                        raw: vec![],
                    },
                },
                biomes: PaletteContainer {
                    bits_per_entry: 0,
                    palette: Palette::SingleValue {
                        block_type_id: VarInt::from(0),
                    },
                    storage: BitStorage::ZeroStorage {
                        size: Strategy::BIOME_DIRECT_ENTRY_SIZE,
                        raw: vec![],
                    },
                },
            })
        }
        let height = max_height - min_height;
        let mut height_maps = HeightMaps {
            world_surface: BitStorage::new(256, ceil_log_2(height + 1)),
            motion_blocking: BitStorage::new(256, ceil_log_2(height + 1)),
            cached_compound_tag: Default::default(),
        };
        height_maps.cache_compound_tag();
        Self {
            chunk_x: x,
            chunk_z: z,
            min_height,
            max_height,
            height_maps,
            chunk_sections: section_vec,
        }
    }

    pub const fn section_coord_from(i: i32) -> i32 {
        i >> 4
    }

    pub const fn position_coord_from(i: i32) -> i32 {
        i << 4
    }

    const fn get_min_section(&self) -> i32 {
        Self::section_coord_from(self.min_height)
    }

    const fn get_section_index(&self, y: i32) -> i32 {
        Self::section_coord_from(y) - self.get_min_section()
    }

    pub fn get_block_id(
        &self,
        x: i32,
        y: i32,
        z: i32,
    ) -> std::result::Result<VarInt, BitSetValidationError> {
        let section_index = self.get_section_index(y);
        if section_index < 0 || self.chunk_sections.len() <= section_index as usize {
            return Ok(0.into());
        }
        let section = &self.chunk_sections[section_index as usize];
        section.get_block_id(x & 0xF, y & 0xF, z & 0xF)
    }

    pub fn rewrite_plane(
        &mut self,
        y: i32,
        block_id: VarInt,
    ) -> std::result::Result<(), BitSetValidationError> {
        let section_index = self.get_section_index(y);
        if section_index < 0 || self.chunk_sections.len() <= section_index as usize {
            return Err(BitSetValidationError(format!("Out of range.")));
        }
        let section = &mut self.chunk_sections[section_index as usize];
        section.rewrite_plane(y & 0xF, block_id)?;
        // todo: recalculate section block count better here
        section.block_count += 16 * 16;
        for x in 0..15 {
            for z in 0..15 {
                self.height_maps.update_inner(x, y, z, block_id)?;
            }
        }
        Ok(())
    }

    pub fn set_block_id(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        block_id: VarInt,
    ) -> std::result::Result<(), BitSetValidationError> {
        let section_index = self.get_section_index(y);
        if section_index < 0 || self.chunk_sections.len() <= section_index as usize {
            return Err(BitSetValidationError(format!("Out of range.")));
        }
        let section = &mut self.chunk_sections[section_index as usize];
        let mutated = section.set_block_id(x & 0xF, y & 0xF, z & 0xF, block_id)?;
        if mutated != block_id || mutated == VarInt::from(-1) {
            let x_and = x & 0xF;
            let z_and = z & 0xF;
            self.height_maps.update_inner(x_and, y, z_and, block_id)?;
            if block_id == 0 {
                section.decrement_non_empty_block_count();
            } else {
                section.increment_non_empty_block_count();
            }
            Ok(())
        } else {
            // we didn't mutate the thing at all
            Ok(())
        }
    }

    pub fn height(&self) -> i32 {
        self.max_height - self.min_height
    }
}

impl DraxTransport for Chunk {
    fn write_to_transport(
        &self,
        context: &mut TransportProcessorContext,
        writer: &mut Cursor<Vec<u8>>,
    ) -> Result<()> {
        self.chunk_x.write_to_transport(context, writer)?;
        self.chunk_z.write_to_transport(context, writer)?;
        self.height_maps.write_to_transport(context, writer)?;

        let mut buffer = Cursor::new(Vec::new());
        for chunk_section in &self.chunk_sections {
            chunk_section.write_to_transport(context, &mut buffer)?;
        }
        let buffer = buffer.into_inner();
        write_var_int_sync(buffer.len() as i32, context, writer)?;
        std::io::Write::write_all(writer, &buffer)?;
        Ok(())
    }

    fn read_from_transport<R: Read>(
        context: &mut TransportProcessorContext,
        read: &mut R,
    ) -> Result<Self>
    where
        Self: Sized,
    {
        let chunk_x = i32::read_from_transport(context, read)?;
        let chunk_z = i32::read_from_transport(context, read)?;

        let height_maps = HeightMaps::read_from_transport(context, read, DEFAULT_WORLD_HEIGHT)?;

        let data_size = read_var_int_sync(context, read)?;

        let mut frame = vec![0; data_size as usize];

        read.read_exact(&mut frame)?;
        let mut frame_cursor = Cursor::new(frame);
        let mut chunk_data = Vec::with_capacity(24);
        for _ in 0..24 {
            chunk_data.push(ChunkSection::read_from_transport(
                context,
                &mut frame_cursor,
            )?);
        }
        Ok(Self {
            chunk_x,
            chunk_z,
            min_height: DEFAULT_WORLD_MIN,
            max_height: DEFAULT_WORLD_HEIGHT,
            height_maps,
            chunk_sections: chunk_data,
        })
    }

    fn precondition_size(&self, context: &mut TransportProcessorContext) -> Result<usize> {
        let mut size = self.chunk_x.precondition_size(context)?;
        size += self.chunk_z.precondition_size(context)?;
        size += self.height_maps.precondition_size(context)?;

        let mut intermediate = 0;
        for section in &self.chunk_sections {
            intermediate += section.precondition_size(context)?;
        }
        size += size_var_int(intermediate as i32, context)?;
        size += intermediate;
        Ok(size)
    }
}
