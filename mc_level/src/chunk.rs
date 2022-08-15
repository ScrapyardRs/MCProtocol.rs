use crate::Either;
use mc_serializer::ext::BitSet::{SimpleStorage, ZeroStorage};
use mc_serializer::ext::{BitSet, BitSetValidationError, BitSetVisitor};
use mc_serializer::primitive::VarInt;
use mc_serializer::serde::{Contextual, Deserialize, ProtocolVersion, Serialize};
use mc_serializer::{contextual, wrap_indexed_struct_context, wrap_struct_context};
use serde::de::{EnumAccess, Error, MapAccess, SeqAccess, Visitor};
use serde::{Deserializer, Serializer};
use std::fmt::Formatter;
use std::io::{Cursor, Read, Write};

#[derive(Debug)]
pub enum Strategy {
    Section,
    Biome,
}

impl Strategy {
    const BIOME_SIZE_BITS: i32 = 2;
    const SECTION_SIZE_BITS: i32 = 4;
    const BIOME_DIRECT_ENTRY_SIZE: i32 = 6;
    const SECTION_DIRECT_ENTRY_SIZE: i32 = 15;

    pub const fn bit_size(&self, given: u8) -> i32 {
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

    pub const fn locked_entry_count(&self) -> i32 {
        match self {
            Strategy::Section => 4096,
            Strategy::Biome => 64,
        }
    }

    pub const fn entry_size(&self) -> i32 {
        match self {
            Strategy::Section => Strategy::SECTION_DIRECT_ENTRY_SIZE,
            Strategy::Biome => Strategy::BIOME_DIRECT_ENTRY_SIZE,
        }
    }

    pub const fn size(&self) -> i32 {
        match self {
            Strategy::Section => 1 << (Strategy::SECTION_SIZE_BITS * 3),
            Strategy::Biome => 1 << (Strategy::BIOME_SIZE_BITS * 3),
        }
    }

    pub const fn retrieve_index(&self, x: i32, y: i32, z: i32) -> i32 {
        let bits = match self {
            Strategy::Section => Strategy::SECTION_SIZE_BITS,
            Strategy::Biome => Strategy::BIOME_SIZE_BITS,
        };
        (((y << bits) | z) << bits) | x
    }
}

#[derive(Debug)]
pub enum Palette {
    SingleValue {
        block_type_id: VarInt,
    },
    Indirect {
        palette_length: VarInt,
        palette: Vec<VarInt>,
    },
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

    fn id_for(&self, block_id: VarInt) -> Either<i32, u8> {
        match self {
            Palette::SingleValue { block_type_id } => {
                if *block_type_id == *block_id {
                    Either::Left(0)
                } else {
                    Either::Right(2)
                }
            }
            Palette::Indirect { palette, .. } => {
                for (index, x) in palette.iter().enumerate() {
                    if *x == *block_id {
                        return Either::Left(index as i32);
                    }
                }
                return Either::Right(palette.len() as u8 + 1);
            }
            Palette::Direct => Either::Left(block_id.into()),
        }
    }

    fn size(&self, protocol_version: ProtocolVersion) -> mc_serializer::serde::Result<i32> {
        match self {
            Palette::SingleValue { block_type_id } => {
                wrap_struct_context!("block_type_id", block_type_id.size(protocol_version))
            }
            Palette::Indirect {
                palette,
                palette_length,
                ..
            } => {
                let i =
                    wrap_struct_context!("palette_length", palette_length.size(protocol_version))?;
                wrap_struct_context!("palette", palette.size(protocol_version)).map(|x| x + i)
            }
            Palette::Direct => Ok(0),
        }
    }
}

#[derive(Debug)]
pub struct PaletteContainer {
    bits_per_entry: u8,
    palette: Palette,
    storage: BitSet,
}

impl PaletteContainer {
    pub fn get(&self, index: i32) -> Result<VarInt, BitSetValidationError> {
        let out = self.storage.get(index)?;
        Ok(self.palette.get(out))
    }

    pub fn copy_to_new_linear(&self, new_value: VarInt) -> Palette {
        match &self.palette {
            Palette::SingleValue { block_type_id } => Palette::Indirect {
                palette_length: 2.into(),
                palette: vec![*block_type_id, new_value],
            },
            Palette::Indirect { palette, .. } => {
                let mut new_palette = palette.clone();
                new_palette.push(new_value);
                return Palette::Indirect {
                    palette_length: (new_palette.len() as i32).into(),
                    palette: new_palette,
                };
            }
            Palette::Direct => unreachable!(),
        }
    }

    pub fn set(
        &mut self,
        strategy: Strategy,
        index: i32,
        block_id: VarInt,
    ) -> Result<VarInt, BitSetValidationError> {
        match self.palette.id_for(block_id) {
            Either::Left(id) => self.storage.get_and_set(index, id).map(Into::into),
            Either::Right(new_size) => {
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
                let mut new_bitset = BitSet::new(strategy.locked_entry_count(), bits_per_entry);
                for x in 0..(new_size as i32 - 1) {
                    let out = self.storage.get(x)?;
                    new_bitset.set(x, new_palette.id_for(out.into()).assert_left())?;
                }
                self.palette = new_palette;
                self.storage = new_bitset;
                Ok(VarInt::from(-1))
            }
        }
    }

    pub fn serialize<W: Write>(
        &self,
        write: &mut W,
        protocol_version: ProtocolVersion,
    ) -> mc_serializer::serde::Result<()> {
        wrap_struct_context!(
            "bits_per_entry",
            self.bits_per_entry.serialize(write, protocol_version)
        )?;
        match &self.palette {
            Palette::SingleValue { block_type_id } => {
                wrap_struct_context!(
                    "block_type_id",
                    block_type_id.serialize(write, protocol_version)
                )?;
            }
            Palette::Indirect {
                palette_length,
                palette,
                ..
            } => {
                wrap_struct_context!(
                    "palette_length",
                    palette_length.serialize(write, protocol_version)
                )?;
                wrap_struct_context!("palette", palette.serialize(write, protocol_version))?;
            }
            _ => (),
        }
        wrap_struct_context!(
            "storage",
            BitSet::to_writer(&self.storage, write, protocol_version)
        )
    }

    pub fn size(&self, protocol_version: ProtocolVersion) -> mc_serializer::serde::Result<i32> {
        let mut size =
            wrap_struct_context!("bits_per_entry", self.bits_per_entry.size(protocol_version))?;
        size += wrap_struct_context!("palette", self.palette.size(protocol_version))?;
        wrap_struct_context!("storage", self.storage.check_size(protocol_version)).map(|x| x + size)
    }

    pub fn deserialize_with_strategy<R: Read>(
        strategy: Strategy,
        read: &mut R,
        protocol_version: ProtocolVersion,
    ) -> mc_serializer::serde::Result<Self> {
        let bits_per_entry =
            wrap_struct_context!("bits_per_entry", u8::deserialize(read, protocol_version))?;

        Ok(match bits_per_entry {
            0 => {
                let block_type_id: VarInt = wrap_struct_context!(
                    "palette_block_type_id",
                    Deserialize::deserialize(read, protocol_version)
                )?;
                PaletteContainer {
                    bits_per_entry,
                    palette: Palette::SingleValue { block_type_id },
                    storage: BitSet::from_reader(
                        read,
                        0,
                        strategy.locked_entry_count(),
                        protocol_version,
                    )?,
                }
            }
            1 | 2 | 3 => {
                let (palette_length, palette): (VarInt, Vec<VarInt>) = wrap_struct_context!(
                    "palette_data",
                    Deserialize::deserialize(read, protocol_version)
                )?;
                PaletteContainer {
                    bits_per_entry,
                    palette: Palette::Indirect {
                        palette_length,
                        palette,
                    },
                    storage: BitSet::from_reader(
                        read,
                        if matches!(strategy, Strategy::Biome) {
                            bits_per_entry
                        } else {
                            4
                        },
                        strategy.locked_entry_count(),
                        protocol_version,
                    )?,
                }
            }
            4 if matches!(strategy, Strategy::Section) => {
                let (palette_length, palette): (VarInt, Vec<VarInt>) = wrap_struct_context!(
                    "palette",
                    Deserialize::deserialize(read, protocol_version)
                )?;
                PaletteContainer {
                    bits_per_entry,
                    palette: Palette::Indirect {
                        palette_length,
                        palette,
                    },
                    storage: BitSet::from_reader(
                        read,
                        4,
                        strategy.locked_entry_count(),
                        protocol_version,
                    )?,
                }
            }
            x if x <= 8 && matches!(strategy, Strategy::Section) => {
                let (palette_length, palette): (VarInt, Vec<VarInt>) = wrap_struct_context!(
                    "palette",
                    Deserialize::deserialize(read, protocol_version)
                )?;
                PaletteContainer {
                    bits_per_entry,
                    palette: Palette::Indirect {
                        palette_length,
                        palette,
                    },
                    storage: BitSet::from_reader(
                        read,
                        x,
                        strategy.locked_entry_count(),
                        protocol_version,
                    )?,
                }
            }
            x => PaletteContainer {
                bits_per_entry,
                palette: Palette::Direct,
                storage: BitSet::from_reader(
                    read,
                    match strategy {
                        Strategy::Section => Strategy::SECTION_DIRECT_ENTRY_SIZE,
                        Strategy::Biome => Strategy::BIOME_DIRECT_ENTRY_SIZE,
                    } as u8,
                    strategy.locked_entry_count(),
                    protocol_version,
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
}

impl Serialize for ChunkSection {
    fn serialize<W: Write>(
        &self,
        writer: &mut W,
        protocol_version: ProtocolVersion,
    ) -> mc_serializer::serde::Result<()> {
        wrap_struct_context!(
            "block_count",
            self.block_count.serialize(writer, protocol_version)
        )?;
        wrap_struct_context!("states", self.states.serialize(writer, protocol_version))?;
        wrap_struct_context!("biomes", self.biomes.serialize(writer, protocol_version))
    }

    fn size(&self, protocol_version: ProtocolVersion) -> mc_serializer::serde::Result<i32> {
        let mut size =
            wrap_struct_context!("block_count", self.block_count.size(protocol_version))?;
        size += wrap_struct_context!("states", self.states.size(protocol_version))?;
        wrap_struct_context!("biomes", self.biomes.size(protocol_version)).map(|x| x + size)
    }
}

impl Deserialize for ChunkSection {
    fn deserialize<R: Read>(
        reader: &mut R,
        protocol_version: ProtocolVersion,
    ) -> mc_serializer::serde::Result<Self> {
        let block_count =
            wrap_struct_context!("block_count", u16::deserialize(reader, protocol_version))?;
        let states = wrap_struct_context!(
            "states",
            PaletteContainer::deserialize_with_strategy(
                Strategy::Section,
                reader,
                protocol_version
            )
        )?;
        let biomes = wrap_struct_context!(
            "biomes",
            PaletteContainer::deserialize_with_strategy(Strategy::Biome, reader, protocol_version)
        )?;
        Ok(Self {
            block_count,
            states,
            biomes,
        })
    }
}

fn deserialize_heightmap_bitset<'de, D>(deserializer: D) -> Result<BitSet, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(BitSetVisitor::new(BitSet::new(256, 9)))
}

fn serialize_heightmap_bitset<S>(item: &BitSet, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    nbt::i64_array(item.get_raw(), serializer)
}

#[derive(serde_derive::Serialize, serde_derive::Deserialize, Debug)]
pub struct HeightMaps {
    #[serde(rename = "WORLD_SURFACE")]
    #[serde(
        deserialize_with = "deserialize_heightmap_bitset",
        serialize_with = "serialize_heightmap_bitset"
    )]
    world_surface: BitSet,
    #[serde(rename = "MOTION_BLOCKING")]
    #[serde(
        deserialize_with = "deserialize_heightmap_bitset",
        serialize_with = "serialize_heightmap_bitset"
    )]
    motion_blocking: BitSet,
}

impl HeightMaps {
    fn get_index(x: i32, z: i32) -> i32 {
        x + (z * 16)
    }

    fn get_first_available(bit_set: &BitSet, index: i32) -> Result<i32, BitSetValidationError> {
        bit_set.get(index).map(|x| x + DEFAULT_WORLD_MIN)
    }

    fn update_inner(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        new_block: VarInt,
    ) -> Result<(), BitSetValidationError> {
        let index = Self::get_index(x, z);
        let opaque = new_block != 0;

        for bit_set in vec![&mut self.world_surface, &mut self.motion_blocking] {
            let first_available = Self::get_first_available(&bit_set, index)?;
            if y > first_available + 1 && opaque {
                bit_set.set(index, (y + 1) - DEFAULT_WORLD_MIN)?;
            }
        }

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
                    storage: ZeroStorage {
                        size: Strategy::SECTION_DIRECT_ENTRY_SIZE,
                        raw: vec![],
                    },
                },
                biomes: PaletteContainer {
                    bits_per_entry: 0,
                    palette: Palette::SingleValue {
                        block_type_id: VarInt::from(0),
                    },
                    storage: ZeroStorage {
                        size: Strategy::BIOME_DIRECT_ENTRY_SIZE,
                        raw: vec![],
                    },
                },
            })
        }
        Self {
            chunk_x: x,
            chunk_z: z,
            min_height,
            max_height,
            height_maps: HeightMaps {
                world_surface: BitSet::new(256, 9),
                motion_blocking: BitSet::new(256, 9),
            },
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

    const fn state_index_from(x: i32, y: i32, z: i32) -> i32 {
        (((y & 15) << 8) | ((z & 15) << 4)) | (x & 15)
    }

    const fn biome_index_from(x: i32, y: i32, z: i32) -> i32 {
        (((y & 15) << 8) | ((z & 15) << 4)) | (x & 15)
    }

    pub fn get_block_id(&self, x: i32, y: i32, z: i32) -> Result<VarInt, BitSetValidationError> {
        let section_index = self.get_section_index(y);
        if section_index < 0 || self.chunk_sections.len() <= section_index as usize {
            return Ok(0.into());
        }
        let section = &self.chunk_sections[section_index as usize];
        section.states.get(Self::state_index_from(x, y, z))
    }

    pub fn rewrite_plane(&mut self, y: i32, block_id: VarInt) -> Result<(), BitSetValidationError> {
        let section_index = self.get_section_index(y);
        if section_index < 0 || self.chunk_sections.len() <= section_index as usize {
            return Err(BitSetValidationError(format!("Out of range.")));
        }
        let section = &mut self.chunk_sections[section_index as usize];
        section.states = PaletteContainer {
            bits_per_entry: 0,
            palette: Palette::SingleValue {
                block_type_id: block_id,
            },
            storage: BitSet::new(0, 0),
        };
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
    ) -> Result<(), BitSetValidationError> {
        let section_index = self.get_section_index(y);
        if section_index < 0 || self.chunk_sections.len() <= section_index as usize {
            return Err(BitSetValidationError(format!("Out of range.")));
        }
        let section = &mut self.chunk_sections[section_index as usize];
        let state_index = Self::state_index_from(x, y, z);
        let mutated = section
            .states
            .set(Strategy::Section, state_index, block_id)?;
        if mutated != block_id || mutated == VarInt::from(-1) {
            let x_and = x & 15;
            let z_and = z & 15;
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
}

contextual!(Chunk);

impl Serialize for Chunk {
    fn serialize<W: Write>(
        &self,
        writer: &mut W,
        protocol_version: ProtocolVersion,
    ) -> mc_serializer::serde::Result<()> {
        wrap_struct_context!("chunk_x", self.chunk_x.serialize(writer, protocol_version))?;
        wrap_struct_context!("chunk_z", self.chunk_z.serialize(writer, protocol_version))?;
        wrap_struct_context!(
            "height_maps",
            mc_serializer::ext::write_nbt(&self.height_maps, writer, protocol_version)
        )?;

        let data_size =
            wrap_struct_context!("chunk_data", self.chunk_sections.size(protocol_version))?;
        wrap_struct_context!(
            "chunk_data_size",
            VarInt::from(data_size).serialize(writer, protocol_version)
        )?;
        let mut buffer = Vec::new();
        for chunk_section in &self.chunk_sections {
            let initial = buffer.len();
            wrap_struct_context!(
                "chunk_data",
                chunk_section.serialize(&mut buffer, protocol_version)
            )?;
            let expected = chunk_section.size(protocol_version)?;
            let actual = buffer.len() - initial;
        }
        wrap_struct_context!("chunk_data", buffer.serialize(writer, protocol_version))
    }

    fn size(&self, protocol_version: ProtocolVersion) -> mc_serializer::serde::Result<i32> {
        let mut size = wrap_struct_context!("chunk_x", self.chunk_x.size(protocol_version))?;
        size += wrap_struct_context!("chunk_z", self.chunk_z.size(protocol_version))?;
        size += wrap_struct_context!(
            "height_maps",
            mc_serializer::ext::size_nbt(&self.height_maps, protocol_version)
        )?;
        let data_size =
            wrap_struct_context!("chunk_data", self.chunk_sections.size(protocol_version))?;
        size += data_size;
        size += wrap_struct_context!(
            "chunk_data_size",
            VarInt::from(data_size).size(protocol_version)
        )?;
        Ok(size)
    }
}

impl Deserialize for Chunk {
    fn deserialize<R: Read>(
        reader: &mut R,
        protocol_version: ProtocolVersion,
    ) -> mc_serializer::serde::Result<Self> {
        let chunk_x = wrap_struct_context!("chunk_x", i32::deserialize(reader, protocol_version))?;
        let chunk_z = wrap_struct_context!("chunk_z", i32::deserialize(reader, protocol_version))?;

        let height_maps = wrap_struct_context!(
            "height_maps",
            mc_serializer::ext::read_nbt(reader, protocol_version)
        )?;

        let data_size =
            wrap_struct_context!("data_size", VarInt::deserialize(reader, protocol_version))?;

        let mut frame = vec![0; Into::<i32>::into(data_size) as usize];
        wrap_struct_context!(
            "raw_chunk_data",
            reader
                .read_exact(&mut frame)
                .map_err(|err| mc_serializer::serde::Error::IoError(err, Self::base_context()))
        )?;
        let mut frame_cursor = Cursor::new(frame);
        let mut chunk_data = Vec::with_capacity(24);
        for index in 0..24 {
            chunk_data.push(wrap_indexed_struct_context!(
                "chunk_data",
                index,
                ChunkSection::deserialize(&mut frame_cursor, protocol_version)
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
}

contextual!(HeightMaps);
contextual!(ChunkSection);
contextual!(Palette);
contextual!(PaletteContainer);
