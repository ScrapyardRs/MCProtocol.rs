use mc_serializer::primitive::VarInt;
use mc_serializer::serde::{Deserialize, ProtocolVersion, Serialize};
use mc_serializer::{contextual, serde::Contextual, wrap_struct_context};
use mc_serializer_derive::{Contextual, Serial};

use std::io::{Read, Write};
use std::ops::Index;

#[derive(Debug)]
pub enum Palette {
    SingleValue { block_type_id: VarInt },
    Linear { block_ids: (VarInt, Vec<VarInt>) },
    HashMap { block_ids: (VarInt, Vec<VarInt>) },
    Global,
}

#[derive(Debug)]
pub enum Strategy {
    Section,
    Biome,
}

impl Strategy {
    const BIOME_SIZE_BITS: i32 = 2;
    const SECTION_SIZE_BITS: i32 = 4;

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
pub struct PaletteContainer {
    strategy: Strategy,
    by: u8,
    palette: Palette,
    raw: (VarInt, Vec<u64>),
}

contextual!(PaletteContainer);

impl Serialize for PaletteContainer {
    fn serialize<W: Write>(
        &self,
        writer: &mut W,
        protocol_version: ProtocolVersion,
    ) -> mc_serializer::serde::Result<()> {
        wrap_struct_context!("data_byte", self.by.serialize(writer, protocol_version))?;
        match &self.palette {
            Palette::SingleValue { block_type_id } => wrap_struct_context!(
                "Palette::Single::block_type_id",
                block_type_id.serialize(writer, protocol_version)
            )?,
            Palette::Linear { block_ids } => wrap_struct_context!(
                "Palette::Linear::block_ids",
                block_ids.serialize(writer, protocol_version)
            )?,
            Palette::HashMap { block_ids } => wrap_struct_context!(
                "Palette::HashMap::block_ids",
                block_ids.serialize(writer, protocol_version)
            )?,
            _ => (),
        }
        wrap_struct_context!("raw", self.raw.serialize(writer, protocol_version))
    }

    fn size(&self, protocol_version: ProtocolVersion) -> mc_serializer::serde::Result<i32> {
        let mut size = 0;

        size += wrap_struct_context!("data_byte", self.by.size(protocol_version))?;
        size += match &self.palette {
            Palette::SingleValue { block_type_id } => wrap_struct_context!(
                "Palette::Single::block_type_id",
                block_type_id.size(protocol_version)
            )?,
            Palette::Linear { block_ids } => wrap_struct_context!(
                "Palette::Linear::block_ids",
                block_ids.size(protocol_version)
            )?,
            Palette::HashMap { block_ids } => wrap_struct_context!(
                "Palette::HashMap::block_ids",
                block_ids.size(protocol_version)
            )?,
            _ => 0,
        };
        wrap_struct_context!("raw", self.raw.size(protocol_version)).map(move |r| r + size)
    }
}

impl PaletteContainer {
    fn deserialize<R: Read>(
        biome_strategy: bool,
        reader: &mut R,
        protocol_version: ProtocolVersion,
    ) -> mc_serializer::serde::Result<Self> {
        let data_byte =
            wrap_struct_context!("data_byte", u8::deserialize(reader, protocol_version))?;
        let palette = match data_byte {
            0 => {
                let block_type_id = wrap_struct_context!(
                    "block_type_id",
                    VarInt::deserialize(reader, protocol_version)
                )?;
                Palette::SingleValue { block_type_id }
            }
            1 | 2 | 3 => {
                let block_ids: (VarInt, Vec<VarInt>) = wrap_struct_context!(
                    "block_ids",
                    Deserialize::deserialize(reader, protocol_version)
                )?;
                Palette::Linear { block_ids }
            }
            4 if !biome_strategy => {
                let block_ids: (VarInt, Vec<VarInt>) = wrap_struct_context!(
                    "block_ids",
                    Deserialize::deserialize(reader, protocol_version)
                )?;
                Palette::Linear { block_ids }
            }
            5 | 6 | 7 | 8 if !biome_strategy => {
                let block_ids: (VarInt, Vec<VarInt>) = wrap_struct_context!(
                    "block_ids",
                    Deserialize::deserialize(reader, protocol_version)
                )?;
                Palette::HashMap { block_ids }
            }
            _ => Palette::Global,
        };

        Ok(Self {
            strategy: if biome_strategy {
                Strategy::Biome
            } else {
                Strategy::Section
            },
            by: data_byte,
            palette,
            raw: wrap_struct_context!("raw", Deserialize::deserialize(reader, protocol_version))?,
        })
    }
}

#[derive(Debug, mc_serializer_derive::MCSerialize)]
pub struct ChunkSection {
    non_empty_block_count: u16,
    states: PaletteContainer,
    biomes: PaletteContainer,
}

contextual!(ChunkSection);

impl Deserialize for ChunkSection {
    fn deserialize<R: Read>(
        reader: &mut R,
        protocol_version: ProtocolVersion,
    ) -> mc_serializer::serde::Result<Self> {
        let non_empty_block_count = wrap_struct_context!(
            "non_empty_block_count",
            u16::deserialize(reader, protocol_version)
        )?;
        let states = wrap_struct_context!(
            "states",
            PaletteContainer::deserialize(false, reader, protocol_version)
        )?;
        let biomes = wrap_struct_context!(
            "biomes",
            PaletteContainer::deserialize(true, reader, protocol_version)
        )?;
        Ok(Self {
            non_empty_block_count,
            states,
            biomes,
        })
    }
}

#[derive(Serial, Debug)]
pub struct BlockEntityInfo {
    packed_xz: u8,
    y: u16,
    block_type: VarInt,
    #[nbt]
    tag: nbt::Blob,
}

#[derive(Serial, Debug)]
pub struct Chunk {
    #[nbt]
    pub height_maps: nbt::Blob,
    pub chunk_sections: (VarInt, Vec<ChunkSection>),
    pub block_entities: (VarInt, Vec<BlockEntityInfo>),
}
