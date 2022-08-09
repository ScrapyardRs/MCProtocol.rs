use crate::shared_types::Identifier;
use mc_serializer::auto_string;
use mc_serializer::primitive::VarInt;
use mc_serializer::serde::{
    Contextual, Deserialize, ProtocolVersion, Serialize, SerializerContext,
};
use mc_serializer::wrap_indexed_struct_context;
use mc_serializer::wrap_struct_context;
use std::io::{Read, Write};

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct Ingredient {
    ingredients: Vec<ItemStackContainer>,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct ItemStack {
    pub item_id: VarInt,
    pub count: u8,
    pub item_tag: nbt::Blob,
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(bool)]
pub enum ItemStackContainer {
    #[key(false)]
    Empty,
    #[key(true)]
    Item(ItemStack),
}

#[derive(Debug)]
pub struct ShapedRecipeSerializer {
    pub width: VarInt,
    pub height: VarInt,
    pub group: Identifier,
    pub ingredients: Vec<Ingredient>,
    pub result: ItemStackContainer,
}

impl Contextual for ShapedRecipeSerializer {
    fn context() -> String {
        "ShapedRecipeSerializer".to_string()
    }
}

impl Serialize for ShapedRecipeSerializer {
    fn serialize<W: Write>(
        &self,
        writer: &mut W,
        protocol_version: ProtocolVersion,
    ) -> mc_serializer::serde::Result<()> {
        wrap_struct_context!("width", self.width.serialize(writer, protocol_version))?;
        wrap_struct_context!("height", self.height.serialize(writer, protocol_version))?;
        wrap_struct_context!("group", self.group.serialize(writer, protocol_version))?;
        wrap_struct_context!(
            "ingredients",
            self.ingredients.serialize(writer, protocol_version)
        )?;
        wrap_struct_context!("result", self.result.serialize(writer, protocol_version))
    }

    fn size(&self, protocol_version: ProtocolVersion) -> mc_serializer::serde::Result<i32> {
        let mut size = 0;
        size += wrap_struct_context!("width", self.width.size(protocol_version))?;
        size += wrap_struct_context!("height", self.height.size(protocol_version))?;
        size += wrap_struct_context!("group", self.group.size(protocol_version))?;
        size += wrap_struct_context!("ingredients", self.ingredients.size(protocol_version))?;
        wrap_struct_context!(
            "result",
            self.result.size(protocol_version).map(move |x| x + size)
        )
    }
}

impl Deserialize for ShapedRecipeSerializer {
    fn deserialize<R: Read>(
        reader: &mut R,
        protocol_version: ProtocolVersion,
    ) -> mc_serializer::serde::Result<Self> {
        let width = wrap_struct_context!("width", VarInt::deserialize(reader, protocol_version))?;
        let height =
            wrap_struct_context!("height", Deserialize::deserialize(reader, protocol_version))?;
        let group =
            wrap_struct_context!("group", Deserialize::deserialize(reader, protocol_version))?;
        let mut ingredients =
            Vec::with_capacity(TryInto::<usize>::try_into(width * height).map_err(|err| {
                mc_serializer::serde::Error::TryFromIntError(
                    err,
                    SerializerContext::new(
                        Self::context(),
                        format!(
                            "Failed to create a usize from (width/height) ({}/{})",
                            width, height
                        ),
                    ),
                )
                .update_context(|ctx| {
                    ctx.current_field("ingredients".to_string())
                        .current_struct(Self::context());
                })
            })?);
        for x in 0..Into::<i32>::into(width * height) {
            ingredients.push(wrap_indexed_struct_context!(
                "ingredients",
                x,
                Deserialize::deserialize(reader, protocol_version)
            )?);
        }
        let result =
            wrap_struct_context!("result", Deserialize::deserialize(reader, protocol_version))?;
        Ok(Self {
            width,
            height,
            group,
            ingredients,
            result,
        })
    }
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct SimpleCookingRecipeBase {
    pub group: Identifier,
    pub ingredient: Ingredient,
    pub item_stack: ItemStackContainer,
    pub f: f32,
    pub n: VarInt,
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(ResourceLocation)]
pub enum CraftingRecipe {
    #[key(ResourceLocation::from("crafting_shaped"))]
    ShapedRecipe(ShapedRecipeSerializer),
    #[key(ResourceLocation::from("crafting_shapeless"))]
    ShapelessRecipe {
        group: Identifier,
        ingredients: (VarInt, Vec<Ingredient>),
        result: ItemStackContainer,
    },
    #[key(ResourceLocation::from("crafting_special_armordye"))]
    ArmorDye,
    #[key(ResourceLocation::from("crafting_special_bookcloning"))]
    BookCloning,
    #[key(ResourceLocation::from("crafting_special_mapcloning"))]
    MapCloning,
    #[key(ResourceLocation::from("crafting_special_mapextending"))]
    MapExtending,
    #[key(ResourceLocation::from("crafting_special_firework_rocket"))]
    FireworkRocket,
    #[key(ResourceLocation::from("crafting_special_firework_star"))]
    FireworkStar,
    #[key(ResourceLocation::from("crafting_special_firework_star_fade"))]
    FireworkStarFade,
    #[key(ResourceLocation::from("crafting_special_tippedarrow"))]
    TippedArrow,
    #[key(ResourceLocation::from("crafting_special_bannerduplicate"))]
    BannerDuplicate,
    #[key(ResourceLocation::from("crafting_special_shielddecoration"))]
    ShieldDecoration,
    #[key(ResourceLocation::from("crafting_special_shulkerboxcoloring"))]
    ShulkerBoxColoring,
    #[key(ResourceLocation::from("crafting_special_suspiciousstew"))]
    SuspiciousStew,
    #[key(ResourceLocation::from("crafting_special_repairitem"))]
    RepairItem,
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(ResourceLocation)]
pub enum SmeltingRecipe {
    #[key(ResourceLocation::from("smelting"))]
    SmeltingRecipe(SimpleCookingRecipeBase),
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(ResourceLocation)]
pub enum BlastingRecipe {
    #[key(ResourceLocation::from("blasting"))]
    BlastingRecipe(SimpleCookingRecipeBase),
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(ResourceLocation)]
pub enum SmokingRecipe {
    #[key(ResourceLocation::from("smoking"))]
    SmokingRecipe(SimpleCookingRecipeBase),
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(ResourceLocation)]
pub enum CampfireCookingRecipe {
    #[key(ResourceLocation::from("campfire_cooking"))]
    CampfireCookingRecipe(SimpleCookingRecipeBase),
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(ResourceLocation)]
pub enum StonecutterRecipe {
    #[key(ResourceLocation::from("stonecutting"))]
    StoneCutterRecipe {
        group: Identifier,
        ingredient: Ingredient,
        result: ItemStackContainer,
    },
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(ResourceLocation)]
pub enum UpgradeRecipe {
    #[key(ResourceLocation::from("smithing"))]
    SmithingRecipe {
        ingredient_1: Ingredient,
        ingredient_2: Ingredient,
        result: ItemStackContainer,
    },
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(ResourceLocation)]
pub enum Recipe {
    #[key(ResourceLocation::from("crafting"))]
    CraftingRecipe(CraftingRecipe),
    #[key(ResourceLocation::from("smelting"))]
    SmeltingRecipe(SmeltingRecipe),
    #[key(ResourceLocation::from("blasting"))]
    BlastingRecipe(BlastingRecipe),
    #[key(ResourceLocation::from("smoking"))]
    SmokingRecipe(SmokingRecipe),
    #[key(ResourceLocation::from("campfire_cooking"))]
    CampfireCookingRecipe(CampfireCookingRecipe),
    #[key(ResourceLocation::from("stonecutting"))]
    StonecutterRecipe(StonecutterRecipe),
    #[key(ResourceLocation::from("smithing"))]
    UpgradeRecipe(UpgradeRecipe),
}

auto_string!(ResourceLocation, 32767);

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(u8)]
pub enum Difficulty {
    #[key(0)]
    Peaceful,
    #[key(1)]
    Easy,
    #[key(2)]
    Normal,
    #[key(3)]
    Hard,
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(u8)]
pub enum GameType {
    #[key(255)]
    None,
    #[key(0)]
    Survival,
    #[key(1)]
    Creative,
    #[key(2)]
    Adventure,
    #[key(3)]
    Spectator,
}

#[derive(Debug)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Contextual for BlockPos {
    fn context() -> String {
        "BlockPos".to_string()
    }
}

const MULTIPLY_DE_BRUIJN_BIT_POSITION: [u64; 32] = [
    0, 1, 28, 2, 29, 14, 24, 3, 30, 22, 20, 15, 25, 17, 4, 8, 31, 27, 13, 23, 21, 19, 16, 7, 26,
    12, 18, 6, 11, 5, 10, 9,
];

const fn ceil_log_2(n: u64) -> u64 {
    let n = if is_power_of_2(n) { n } else { shift_2(n) };
    MULTIPLY_DE_BRUIJN_BIT_POSITION[(((n * 125613361u64) >> 27u64) & 0x1Fu64) as usize]
}

const fn is_power_of_2(n: u64) -> bool {
    n != 0u64 && (n & (n - 1u64)) == 0u64
}

const fn shift_2(n: u64) -> u64 {
    let mut n2 = n - 1u64;
    n2 |= n2 >> 1u64;
    n2 |= n2 >> 2u64;
    n2 |= n2 >> 4u64;
    n2 |= n2 >> 8u64;
    n2 |= n2 >> 16u64;
    n2 + 1u64
}

const fn log2(n: u64) -> u64 {
    ceil_log_2(n) - if is_power_of_2(n) { 0u64 } else { 1u64 }
}

const PACKED_Z_LENGTH: i32 = (1u64 + log2(shift_2(30000000u64))) as i32;
const PACKED_X_LENGTH: i32 = (PACKED_Z_LENGTH) as i32;
const PACKED_Y_LENGTH: i32 = (64u64 - PACKED_X_LENGTH as u64 - PACKED_Z_LENGTH as u64) as i32;
const PACKED_X_MASK: i64 = ((1u64 << PACKED_X_LENGTH) - 1u64) as i64;
const PACKED_Y_MASK: i64 = ((1u64 << PACKED_Y_LENGTH) - 1u64) as i64;
const PACKED_Z_MASK: i64 = ((1u64 << PACKED_Z_LENGTH) - 1u64) as i64;
const Z_OFFSET: i32 = (PACKED_Y_LENGTH) as i32;
const X_OFFSET: i32 = (PACKED_Y_LENGTH + PACKED_Z_LENGTH) as i32;

impl Serialize for BlockPos {
    fn serialize<W: Write>(
        &self,
        writer: &mut W,
        protocol_version: ProtocolVersion,
    ) -> mc_serializer::serde::Result<()> {
        let mut value: i64 = 0;
        value |= (self.x as i64 & PACKED_X_MASK) << X_OFFSET;
        value |= self.y as i64 & PACKED_Y_MASK;
        value |= (self.z as i64 & PACKED_Z_MASK) << Z_OFFSET;
        i64::serialize(&value, writer, protocol_version)
    }

    fn size(&self, protocol_version: ProtocolVersion) -> mc_serializer::serde::Result<i32> {
        i64::size(&0, protocol_version) // always the same size
    }
}

impl Deserialize for BlockPos {
    fn deserialize<R: Read>(
        reader: &mut R,
        protocol_version: ProtocolVersion,
    ) -> mc_serializer::serde::Result<Self> {
        let long = i64::deserialize(reader, protocol_version)?;
        let x = (long << (64 - X_OFFSET - PACKED_X_LENGTH) >> (64 - PACKED_X_LENGTH)) as i32;
        let y = (long << (64 - PACKED_Y_LENGTH) >> (64 - PACKED_Y_LENGTH)) as i32;
        let z = (long << (64 - Z_OFFSET - PACKED_Z_LENGTH) >> (64 - PACKED_Z_LENGTH)) as i32;
        Ok(BlockPos { x, y, z })
    }
}

#[derive(Debug)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

impl Contextual for ChunkPos {
    fn context() -> String {
        "ChunkPos".to_string()
    }
}

impl Serialize for ChunkPos {
    fn serialize<W: Write>(
        &self,
        writer: &mut W,
        protocol_version: ProtocolVersion,
    ) -> mc_serializer::serde::Result<()> {
        let long = self.x as i64 & 0xFFFFFFFF | (self.z as i64 & 0xFFFFFFFF) << 32;
        i64::serialize(&long, writer, protocol_version)
    }

    fn size(&self, protocol_version: ProtocolVersion) -> mc_serializer::serde::Result<i32> {
        i64::size(&0, protocol_version)
    }
}

impl Deserialize for ChunkPos {
    fn deserialize<R: Read>(
        reader: &mut R,
        protocol_version: ProtocolVersion,
    ) -> mc_serializer::serde::Result<Self> {
        let long = i64::deserialize(reader, protocol_version)?;
        let x = (long & 0xFFFFFFFF) as i32;
        let z = (long as u64 >> 32 & 0xFFFFFFFF) as i32;
        Ok(ChunkPos { x, z })
    }
}
