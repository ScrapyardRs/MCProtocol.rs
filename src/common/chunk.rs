use drax::nbt::{EnsuredCompoundTag, Tag};
use drax::prelude::{
    AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, DraxReadExt, DraxWriteExt, PacketComponent,
    Result, Size, TransportError,
};
use drax::transport::buffer::var_num::size_var_int;
use drax::{err_explain, throw_explain, PinnedLivelyResult};

use crate::common::bit_storage::{BitSetValidationError, BitStorage};
use crate::common::play::ceil_log_2;

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
#[derive(Debug, Clone)]
pub enum Palette {
    SingleValue { block_type_id: i32 },
    Indirect { palette: Vec<i32> },
    Direct,
}

impl Palette {
    fn get(&self, id_index: i32) -> i32 {
        match self {
            Palette::SingleValue { block_type_id } => *block_type_id,
            Palette::Indirect { palette, .. } => palette[id_index as usize].clone(),
            Palette::Direct => id_index.into(),
        }
    }

    fn id_for(&self, block_id: i32) -> Index {
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

    fn size(&self) -> Result<usize> {
        match self {
            Palette::SingleValue { block_type_id } => Ok(size_var_int(*block_type_id)),
            Palette::Indirect { palette, .. } => {
                let mut size = size_var_int(palette.len() as i32);
                for t in palette {
                    size += size_var_int(*t);
                }
                Ok(size)
            }
            Palette::Direct => Ok(0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PaletteContainer {
    bits_per_entry: u8,
    palette: Palette,
    storage: BitStorage,
}

impl PaletteContainer {
    pub fn get(&self, index: i32) -> std::result::Result<i32, BitSetValidationError> {
        let out = self.storage.get(index)?;
        Ok(self.palette.get(out))
    }

    pub fn copy_to_new_linear(&self, new_value: i32) -> Palette {
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

    pub fn reassert_storage(
        &mut self,
        strategy: Strategy,
        block_id: i32,
    ) -> std::result::Result<i32, BitSetValidationError> {
        match self.palette.id_for(block_id) {
            Index::NewSize(new_size) => {
                let bits_per_entry = strategy.bit_size(new_size);
                let new_palette = match new_size {
                    0 | 1 => unreachable!(),
                    2 | 3 => self.copy_to_new_linear(block_id),
                    4 if matches!(strategy, Strategy::Section) => self.copy_to_new_linear(block_id),
                    x if x <= 8 && matches!(strategy, Strategy::Section) => {
                        self.copy_to_new_linear(block_id)
                    }
                    _ => Palette::Direct,
                };
                let mut new_bitset = BitStorage::new(strategy.locked_entry_count(), bits_per_entry);
                for idx in 0..self.storage.size() {
                    let out = self.storage.get(idx)?;
                    new_bitset.set(idx, out)?;
                }
                self.bits_per_entry = bits_per_entry as u8;
                self.palette = new_palette;
                self.storage = new_bitset;
                Ok(self.palette.id_for(block_id).current())
            }
            Index::CurrentIndex(map) => Ok(map),
        }
    }

    pub fn set_all(
        &mut self,
        strategy: Strategy,
        indexes: Vec<i32>,
        block_id: i32,
    ) -> std::result::Result<(), BitSetValidationError> {
        let block_mapping = self.reassert_storage(strategy, block_id)?;
        for index in indexes {
            self.storage.set(index, block_mapping)?;
        }
        Ok(())
    }

    pub fn set(
        &mut self,
        strategy: Strategy,
        index: i32,
        block_id: i32,
    ) -> std::result::Result<i32, BitSetValidationError> {
        let block_mapping = self.reassert_storage(strategy, block_id)?;
        self.storage.get_and_set(index, block_mapping)
    }

    pub async fn serialize<W: AsyncWrite + Unpin + Send + Sync + ?Sized>(
        &self,
        write: &mut W,
    ) -> Result<()> {
        write.write_u8(self.bits_per_entry).await?;
        match &self.palette {
            Palette::SingleValue { block_type_id } => {
                write.write_var_int(*block_type_id).await?;
            }
            Palette::Indirect { palette, .. } => {
                write.write_var_int(palette.len() as i32).await?;
                for item in palette {
                    write.write_var_int(*item).await?;
                }
            }
            _ => (),
        }
        BitStorage::to_writer(&self.storage, write).await
    }

    pub fn size(&self) -> Result<usize> {
        let mut size = 1;
        size += self.palette.size()?;
        self.storage.check_size().map(|x| x + size)
    }

    pub async fn deserialize_with_strategy<R: AsyncRead + Unpin + Send + Sync + ?Sized>(
        strategy: Strategy,
        read: &mut R,
    ) -> Result<Self> {
        let bits_per_entry = read.read_u8().await?;

        Ok(match bits_per_entry {
            0 => {
                let block_type_id = read.read_var_int().await?;
                PaletteContainer {
                    bits_per_entry,
                    palette: Palette::SingleValue { block_type_id },
                    storage: BitStorage::from_reader(read, 0, strategy.locked_entry_count())
                        .await?,
                }
            }
            1 | 2 | 3 => {
                let palette_len = read.read_var_int().await?;
                let mut palette = Vec::with_capacity(palette_len as usize);
                for _ in 0..palette_len {
                    palette.push(read.read_var_int().await?);
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
                    )
                    .await?,
                }
            }
            4 if matches!(strategy, Strategy::Section) => {
                let palette_len = read.read_var_int().await?;
                let mut palette = Vec::with_capacity(palette_len as usize);
                for _ in 0..palette_len {
                    palette.push(read.read_var_int().await?);
                }
                PaletteContainer {
                    bits_per_entry,
                    palette: Palette::Indirect { palette },
                    storage: BitStorage::from_reader(read, 4, strategy.locked_entry_count())
                        .await?,
                }
            }
            x if x <= 8 && matches!(strategy, Strategy::Section) => {
                let palette_len = read.read_var_int().await?;
                let mut palette = Vec::with_capacity(palette_len as usize);
                for _ in 0..palette_len {
                    palette.push(read.read_var_int().await?);
                }
                PaletteContainer {
                    bits_per_entry,
                    palette: Palette::Indirect { palette },
                    storage: BitStorage::from_reader(read, x, strategy.locked_entry_count())
                        .await?,
                }
            }
            _ => PaletteContainer {
                bits_per_entry,
                palette: Palette::Direct,
                storage: BitStorage::from_reader(
                    read,
                    match strategy {
                        Strategy::Section => Strategy::SECTION_DIRECT_ENTRY_SIZE,
                        Strategy::Biome => Strategy::BIOME_DIRECT_ENTRY_SIZE,
                    } as u8,
                    strategy.locked_entry_count(),
                )
                .await?,
            },
        })
    }
}

#[derive(Debug, Clone)]
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
    ) -> std::result::Result<i32, BitSetValidationError> {
        self.states.get(Strategy::Section.retrieve_index(x, y, z))
    }

    fn set_block_id(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        block_id: i32,
    ) -> std::result::Result<i32, BitSetValidationError> {
        let state_index = Strategy::Section.retrieve_index(x, y, z);
        self.states.set(Strategy::Section, state_index, block_id)
    }

    fn rewrite_plane(
        &mut self,
        y: i32,
        block_id: i32,
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

impl<C: Send + Sync> PacketComponent<C> for ChunkSection {
    type ComponentType = ChunkSection;

    fn decode<'a, A: AsyncRead + Unpin + Send + Sync + ?Sized>(
        context: &'a mut C,
        read: &'a mut A,
    ) -> PinnedLivelyResult<'a, Self::ComponentType> {
        Box::pin(async move {
            let block_count = u16::decode(context, read).await?;
            let states =
                PaletteContainer::deserialize_with_strategy(Strategy::Section, read).await?;
            let biomes = PaletteContainer::deserialize_with_strategy(Strategy::Biome, read).await?;
            Ok(Self {
                block_count,
                states,
                biomes,
            })
        })
    }

    fn encode<'a, A: AsyncWrite + Unpin + Send + Sync + ?Sized>(
        component_ref: &'a Self::ComponentType,
        context: &'a mut C,
        write: &'a mut A,
    ) -> PinnedLivelyResult<'a, ()> {
        Box::pin(async move {
            u16::encode(&component_ref.block_count, context, write).await?;
            component_ref.states.serialize(write).await?;
            component_ref.biomes.serialize(write).await
        })
    }

    fn size(input: &Self::ComponentType, context: &mut C) -> Result<Size> {
        let mut size = u16::size(&input.block_count, context)?;
        size = size + PaletteContainer::size(&input.states)?;
        size = size + PaletteContainer::size(&input.biomes)?;
        Ok(size)
    }
}

#[derive(Debug, Clone)]
pub struct HeightMaps {
    world_surface: BitStorage,
    motion_blocking: BitStorage,
    cached_compound_tag: Option<Tag>,
}

impl HeightMaps {
    async fn encode<A: AsyncWrite + Unpin + Send + Sync + ?Sized>(
        &self,
        writer: &mut A,
    ) -> Result<()> {
        EnsuredCompoundTag::<0>::encode(&self.cached_compound_tag, &mut (), writer).await?;
        Ok(())
    }

    async fn decode<R: AsyncRead + Unpin + Send + Sync + ?Sized>(
        read: &mut R,
        height: i32,
    ) -> Result<Self>
    where
        Self: Sized,
    {
        let tag = EnsuredCompoundTag::<0>::decode(&mut (), read).await?;
        match tag {
            Some(Tag::CompoundTag(tag)) => {
                let mut world_surface = None;
                let mut motion_blocking = None;
                macro_rules! assign_inner {
                    ($bind_out:ident, $bind_v:ident, $bind_height:ident) => {
                        if let Tag::TagLongArray($bind_v) = $bind_v {
                            $bind_out = Some(
                                BitStorage::with_seeded_raw(
                                    256,
                                    ceil_log_2($bind_height + 1),
                                    $bind_v.clone(),
                                )
                                .map_err(|err| err_explain!(err.0))?,
                            );
                        }
                    };
                }
                for (k, v) in &tag {
                    if format!("WORLD_SURFACE").eq(k) {
                        assign_inner!(world_surface, v, height);
                    } else if format!("MOTION_BLOCKING").eq(k) {
                        assign_inner!(motion_blocking, v, height);
                    }
                }

                match (world_surface, motion_blocking) {
                    (Some(world_surface), Some(motion_blocking)) => Ok(HeightMaps {
                        world_surface,
                        motion_blocking,
                        cached_compound_tag: Some(Tag::CompoundTag(tag)),
                    }),
                    (_, _) => {
                        throw_explain!("Could not find WORLD_SURFACE and MOTION_BLOCKING as larrs.")
                    }
                }
            }
            None | Some(_) => {
                throw_explain!("Failed to load tag, none was found.")
            }
        }
    }

    fn precondition_size(&self) -> Result<usize> {
        Ok(
            match EnsuredCompoundTag::<0>::size(&self.cached_compound_tag, &mut ())? {
                Size::Dynamic(x) | Size::Constant(x) => x,
            },
        )
    }
}

impl HeightMaps {
    pub(crate) fn cache_compound_tag(&mut self) {
        let mut tag = Vec::new();
        tag.push((
            "WORLD_SURFACE".to_string(),
            Tag::TagLongArray(self.world_surface.get_raw().clone()),
        ));
        tag.push((
            "MOTION_BLOCKING".to_string(),
            Tag::TagLongArray(self.motion_blocking.get_raw().clone()),
        ));
        self.cached_compound_tag = Some(Tag::CompoundTag(tag));
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
        new_block: i32,
    ) -> std::result::Result<(), BitSetValidationError> {
        let index = Self::get_index(x, z);
        let opaque = new_block != 0;

        for bit_set in vec![&mut self.world_surface, &mut self.motion_blocking] {
            let first_available = Self::get_first_available(&bit_set, index)?;
            if y > first_available + 1 && opaque {
                bit_set.set(index, (y + 1) - DEFAULT_WORLD_MIN)?;
            }
        }

        self.cache_compound_tag();

        Ok(())
    }
}

#[derive(Debug, Clone)]
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

impl Chunk {
    pub fn new(x: i32, z: i32) -> Self {
        Self::using_world_height(x, z, DEFAULT_WORLD_MIN, DEFAULT_WORLD_HEIGHT)
    }

    pub fn clone_for(&self, x: i32, z: i32) -> Self {
        Self {
            min_height: self.min_height,
            max_height: self.max_height,
            chunk_x: x,
            chunk_z: z,
            height_maps: self.height_maps.clone(),
            chunk_sections: self.chunk_sections.clone(),
        }
    }

    pub fn using_world_height(x: i32, z: i32, min_height: i32, max_height: i32) -> Chunk {
        let sections = (max_height - min_height) / BLOCKS_PER_SECTION;
        let mut section_vec = Vec::with_capacity(sections as usize);
        for _ in 0..sections {
            section_vec.push(ChunkSection {
                block_count: 0,
                states: PaletteContainer {
                    bits_per_entry: 0,
                    palette: Palette::SingleValue { block_type_id: 0 },
                    storage: BitStorage::ZeroStorage {
                        size: Strategy::SECTION_DIRECT_ENTRY_SIZE,
                        raw: vec![],
                    },
                },
                biomes: PaletteContainer {
                    bits_per_entry: 0,
                    palette: Palette::SingleValue { block_type_id: 0 },
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
            cached_compound_tag: Some(Tag::CompoundTag(Vec::new())),
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
    ) -> std::result::Result<i32, BitSetValidationError> {
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
        block_id: i32,
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
        block_id: i32,
    ) -> std::result::Result<(), BitSetValidationError> {
        let section_index = self.get_section_index(y);
        if section_index < 0 || self.chunk_sections.len() <= section_index as usize {
            return Err(BitSetValidationError(format!("Out of range.")));
        }
        let section = &mut self.chunk_sections[section_index as usize];
        let mutated = section.set_block_id(x & 0xF, y & 0xF, z & 0xF, block_id)?;
        if mutated != block_id || mutated == -1 {
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

impl<C: Send + Sync> PacketComponent<C> for Chunk {
    type ComponentType = Chunk;

    fn decode<'a, A: AsyncRead + Unpin + Send + Sync + ?Sized>(
        context: &'a mut C,
        read: &'a mut A,
    ) -> PinnedLivelyResult<'a, Self::ComponentType> {
        Box::pin(async move {
            let chunk_x = i32::decode(context, read).await?;
            let chunk_z = i32::decode(context, read).await?;

            let height_maps = HeightMaps::decode(read, DEFAULT_WORLD_HEIGHT).await?;

            let data_size = read.read_var_int().await?;

            let mut frame = read.take(data_size as u64);

            let mut chunk_data = Vec::with_capacity(24);
            for _ in 0..24 {
                chunk_data.push(ChunkSection::decode(context, &mut frame).await?);
            }
            Ok(Self {
                chunk_x,
                chunk_z,
                min_height: DEFAULT_WORLD_MIN,
                max_height: DEFAULT_WORLD_HEIGHT,
                height_maps,
                chunk_sections: chunk_data,
            })
        })
    }

    fn encode<'a, A: AsyncWrite + Unpin + Send + Sync + ?Sized>(
        component_ref: &'a Self::ComponentType,
        context: &'a mut C,
        write: &'a mut A,
    ) -> PinnedLivelyResult<'a, ()> {
        Box::pin(async move {
            i32::encode(&component_ref.chunk_x, context, write).await?;
            i32::encode(&component_ref.chunk_z, context, write).await?;
            HeightMaps::encode(&component_ref.height_maps, write).await?;
            write
                .write_var_int(
                    component_ref
                        .chunk_sections
                        .iter()
                        .map(|x| ChunkSection::size(x, context))
                        .try_fold(Size::Dynamic(0), |acc, x| {
                            Ok::<Size, TransportError>(acc + x?)
                        })
                        .map(|x| match x {
                            Size::Dynamic(x) | Size::Constant(x) => x as i32,
                        })?,
                )
                .await?;
            for chunk_section in &component_ref.chunk_sections {
                ChunkSection::encode(chunk_section, context, write).await?;
            }
            Ok(())
        })
    }

    fn size(input: &Self::ComponentType, context: &mut C) -> Result<Size> {
        let mut size = Size::Constant(8);
        size = size + input.height_maps.precondition_size()?;
        let mut v_size = Size::Dynamic(0);
        for section in &input.chunk_sections {
            v_size = v_size + ChunkSection::size(section, context)?;
        }
        size = size + v_size;
        size = size
            + size_var_int(match v_size {
                Size::Dynamic(x) | Size::Constant(x) => x,
            } as i32);
        Ok(size)
    }
}
