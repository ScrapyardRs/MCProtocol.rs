use crate::shared_types::{GameProfile, Identifier, MCIdentifiedKey, Signature};
use mc_serializer::primitive::VarInt;
use mc_serializer::serde::{
    Contextual, Deserialize, ProtocolVersion, Serialize, SerializerContext,
};
use mc_serializer::wrap_indexed_struct_context;
use mc_serializer::wrap_struct_context;
use mc_serializer::{auto_string, contextual};
use std::collections::HashMap;

use crate::shared_types::sound::SoundSource;
use mc_chat::Chat;
use std::io::{Read, Write};
use uuid::Uuid;

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct Ingredient {
    pub ingredients: (VarInt, Vec<ItemStackContainer>),
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct ItemStack {
    pub item_id: VarInt,
    pub count: u8,
    #[nbt]
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
        wrap_struct_context!(
            "what_is_this",
            ResourceLocation::deserialize(reader, protocol_version)
        )?;
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
    identifier: Identifier,
    pub group: Identifier,
    pub ingredient: Ingredient,
    pub item_stack: ItemStackContainer,
    pub f: f32,
    pub n: VarInt,
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(ResourceLocation)]
pub enum CraftingRecipe {
    #[key(ResourceLocation::from("minecraft:crafting_shaped"))]
    ShapedRecipe(ShapedRecipeSerializer),
    #[key(ResourceLocation::from("minecraft:crafting_shapeless"))]
    ShapelessRecipe {
        group: Identifier,
        ingredients: (VarInt, Vec<Ingredient>),
        result: ItemStackContainer,
    },
    #[key(ResourceLocation::from("minecraft:crafting_special_armordye"))]
    ArmorDye,
    #[key(ResourceLocation::from("minecraft:crafting_special_bookcloning"))]
    BookCloning,
    #[key(ResourceLocation::from("minecraft:crafting_special_mapcloning"))]
    MapCloning,
    #[key(ResourceLocation::from("minecraft:crafting_special_mapextending"))]
    MapExtending,
    #[key(ResourceLocation::from("minecraft:crafting_special_firework_rocket"))]
    FireworkRocket,
    #[key(ResourceLocation::from("minecraft:crafting_special_firework_star"))]
    FireworkStar,
    #[key(ResourceLocation::from("minecraft:crafting_special_firework_star_fade"))]
    FireworkStarFade,
    #[key(ResourceLocation::from("minecraft:crafting_special_tippedarrow"))]
    TippedArrow,
    #[key(ResourceLocation::from("minecraft:crafting_special_bannerduplicate"))]
    BannerDuplicate,
    #[key(ResourceLocation::from("minecraft:crafting_special_shielddecoration"))]
    ShieldDecoration,
    #[key(ResourceLocation::from("minecraft:crafting_special_shulkerboxcoloring"))]
    ShulkerBoxColoring,
    #[key(ResourceLocation::from("minecraft:crafting_special_suspiciousstew"))]
    SuspiciousStew,
    #[key(ResourceLocation::from("minecraft:crafting_special_repairitem"))]
    RepairItem,
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(ResourceLocation)]
pub enum SmeltingRecipe {
    #[key(ResourceLocation::from("minecraft:smelting"))]
    SmeltingRecipe(SimpleCookingRecipeBase),
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(ResourceLocation)]
pub enum BlastingRecipe {
    #[key(ResourceLocation::from("minecraft:blasting"))]
    BlastingRecipe(SimpleCookingRecipeBase),
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(ResourceLocation)]
pub enum SmokingRecipe {
    #[key(ResourceLocation::from("minecraft:smoking"))]
    SmokingRecipe(SimpleCookingRecipeBase),
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(ResourceLocation)]
pub enum CampfireCookingRecipe {
    #[key(ResourceLocation::from("minecraft:campfire_cooking"))]
    CampfireCookingRecipe(SimpleCookingRecipeBase),
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(ResourceLocation)]
pub enum StonecutterRecipe {
    #[key(ResourceLocation::from("minecraft:stonecutting"))]
    StoneCutterRecipe {
        identifier: Identifier,
        group: Identifier,
        ingredient: Ingredient,
        result: ItemStackContainer,
    },
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(ResourceLocation)]
pub enum UpgradeRecipe {
    #[key(ResourceLocation::from("minecraft:smithing"))]
    SmithingRecipe {
        identifier: Identifier,
        ingredient_1: Ingredient,
        ingredient_2: Ingredient,
        result: ItemStackContainer,
    },
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(ResourceLocation)]
pub enum Recipe {
    #[key(ResourceLocation::from("minecraft:crafting_shaped"))]
    ShapedRecipe(ShapedRecipeSerializer),
    #[key(ResourceLocation::from("minecraft:crafting_shapeless"))]
    ShapelessRecipe {
        identifier: Identifier,
        group: Identifier,
        ingredients: (VarInt, Vec<Ingredient>),
        result: ItemStackContainer,
    },
    #[key(ResourceLocation::from("minecraft:crafting_special_armordye"))]
    ArmorDye,
    #[key(ResourceLocation::from("minecraft:crafting_special_bookcloning"))]
    BookCloning,
    #[key(ResourceLocation::from("minecraft:crafting_special_mapcloning"))]
    MapCloning,
    #[key(ResourceLocation::from("minecraft:crafting_special_mapextending"))]
    MapExtending,
    #[key(ResourceLocation::from("minecraft:crafting_special_firework_rocket"))]
    FireworkRocket,
    #[key(ResourceLocation::from("minecraft:crafting_special_firework_star"))]
    FireworkStar,
    #[key(ResourceLocation::from("minecraft:crafting_special_firework_star_fade"))]
    FireworkStarFade,
    #[key(ResourceLocation::from("minecraft:crafting_special_tippedarrow"))]
    TippedArrow,
    #[key(ResourceLocation::from("minecraft:crafting_special_bannerduplicate"))]
    BannerDuplicate,
    #[key(ResourceLocation::from("minecraft:crafting_special_shielddecoration"))]
    ShieldDecoration,
    #[key(ResourceLocation::from("minecraft:crafting_special_shulkerboxcoloring"))]
    ShulkerBoxColoring,
    #[key(ResourceLocation::from("minecraft:crafting_special_suspiciousstew"))]
    SuspiciousStew,
    #[key(ResourceLocation::from("minecraft:crafting_special_repairitem"))]
    RepairItem,
    #[key(ResourceLocation::from("minecraft:smelting"))]
    SmeltingRecipe(SimpleCookingRecipeBase),
    #[key(ResourceLocation::from("minecraft:blasting"))]
    BlastingRecipe(SimpleCookingRecipeBase),
    #[key(ResourceLocation::from("minecraft:smoking"))]
    SmokingRecipe(SimpleCookingRecipeBase),
    #[key(ResourceLocation::from("minecraft:campfire_cooking"))]
    CampfireCookingRecipe(SimpleCookingRecipeBase),
    #[key(ResourceLocation::from("minecraft:stonecutting"))]
    StoneCutterRecipe {
        identifier: Identifier,
        group: Identifier,
        ingredient: Ingredient,
        result: ItemStackContainer,
    },
    #[key(ResourceLocation::from("minecraft:smithing"))]
    SmithingRecipe {
        identifier: Identifier,
        ingredient_1: Ingredient,
        ingredient_2: Ingredient,
        result: ItemStackContainer,
    },
    #[default]
    #[key(ResourceLocation::from("unknown"))]
    Custom(Box<Recipe>),
}

// #[derive(mc_serializer_derive::Serial, Debug)]
// #[key(ResourceLocation)]
// pub enum Recipe {
//     CraftingRecipe(CraftingRecipe),
//     SmeltingRecipe(SmeltingRecipe),
//     BlastingRecipe(BlastingRecipe),
//     SmokingRecipe(SmokingRecipe),
//     CampfireCookingRecipe(CampfireCookingRecipe),
//     StonecutterRecipe(StonecutterRecipe),
//     UpgradeRecipe(UpgradeRecipe),
// }

auto_string!(ResourceLocation, 32767);

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(u8)]
pub enum Difficulty {
    #[key(0u8)]
    Peaceful,
    #[key(1u8)]
    Easy,
    #[key(2u8)]
    Normal,
    #[key(3u8)]
    Hard,
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(u8)]
pub enum GameType {
    #[key(255u8)]
    None,
    #[key(0u8)]
    Survival,
    #[key(1u8)]
    Creative,
    #[key(2u8)]
    Adventure,
    #[key(3u8)]
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
pub struct SectionPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Contextual for SectionPos {
    fn context() -> String {
        "SectionPos".to_string()
    }
}

impl Serialize for SectionPos {
    fn serialize<W: Write>(
        &self,
        writer: &mut W,
        protocol_version: ProtocolVersion,
    ) -> mc_serializer::serde::Result<()> {
        let mut l = 0;
        l |= (self.x as u64 & 4194303) << 42;
        l |= (self.y as u64 & 1048575) << 0;
        l |= (self.z as u64 & 4194303) << 20;
        u64::serialize(&l, writer, protocol_version)
    }

    fn size(&self, protocol_version: ProtocolVersion) -> mc_serializer::serde::Result<i32> {
        u64::size(&0, protocol_version)
    }
}

impl Deserialize for SectionPos {
    fn deserialize<R: Read>(
        reader: &mut R,
        protocol_version: ProtocolVersion,
    ) -> mc_serializer::serde::Result<Self> {
        let l = u64::deserialize(reader, protocol_version)?;
        let x = (l << 0 >> 42) as i32;
        let y = (l << 44 >> 44) as i32;
        let z = (l << 22 >> 42) as i32;
        Ok(Self { x, y, z })
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

#[derive(mc_serializer_derive::SerialBitMap, Debug)]
pub struct PlayerAbilitiesBitMap {
    pub invulnerable: bool,
    pub flying: bool,
    pub can_fly: bool,
    pub instant_build: bool,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct RecipeBookSettings {
    pub crafting_open: bool,
    pub crafting_filtering: bool,
    pub furnace_open: bool,
    pub furnace_filtering: bool,
    pub blast_furnace_open: bool,
    pub blast_furnace_filtering: bool,
    pub smoker_open: bool,
    pub smoker_filtering: bool,
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(VarInt)]
pub enum RecipeState {
    #[key(VarInt::from(0))]
    INIT,
    #[key(VarInt::from(1))]
    ADD,
    #[key(VarInt::from(2))]
    REMOVE,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct AddPlayerEntry {
    pub profile: GameProfile,
    pub game_type: GameType,
    pub latency: VarInt,
    pub has_display_name: bool,
    #[serialize_if(*__serde_has_display_name)]
    #[deserialize_if(__serde_has_display_name)]
    #[json(262144)]
    pub display_name: Option<Chat>,
    pub key_data: (bool, Option<MCIdentifiedKey>),
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct UpdateGameModeEntry {
    pub uuid: Uuid,
    pub game_type: GameType,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct UpdateLatencyEntry {
    pub uuid: Uuid,
    pub latency: VarInt,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct UpdateDisplayNameEntry {
    pub uuid: Uuid,
    pub display_name: MaybeComponent,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct RemovePlayerEntry {
    pub uuid: Uuid,
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(VarInt)]
pub enum PlayerInfoEntry {
    AddPlayer((VarInt, Vec<AddPlayerEntry>)),
    UpdateGameMode((VarInt, Vec<UpdateGameModeEntry>)),
    UpdateLatency((VarInt, Vec<UpdateLatencyEntry>)),
    UpdateDisplayName((VarInt, Vec<UpdateDisplayNameEntry>)),
    RemovePlayer((VarInt, Vec<RemovePlayerEntry>)),
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct BlockEntityInfo {
    pub packed_xz: u8,
    pub y: i16,
    pub block_type: VarInt,
    pub tag: nbt::Blob,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct LevelChunkData {
    pub chunk: mc_level::chunk::Chunk,
    pub block_entities: (VarInt, Vec<BlockEntityInfo>),
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct LightUpdateData {
    pub trust_edges: bool,
    pub sky_y_mask: (VarInt, Vec<i64>),
    pub block_y_mask: (VarInt, Vec<i64>),
    pub empty_sky_y_mask: (VarInt, Vec<i64>),
    pub empty_block_y_mask: (VarInt, Vec<i64>),
    pub sky_updates: (VarInt, Vec<(VarInt, Vec<u8>)>),
    pub block_updates: (VarInt, Vec<(VarInt, Vec<u8>)>),
}

#[derive(mc_serializer_derive::Serial, Debug, Default)]
pub struct RelativeArgument {
    current_mask: u8,
}

impl RelativeArgument {
    pub const X: u8 = Self::get_mask(0);
    pub const Y: u8 = Self::get_mask(1);
    pub const Z: u8 = Self::get_mask(2);
    pub const Y_ROT: u8 = Self::get_mask(3);
    pub const X_ROT: u8 = Self::get_mask(4);

    const fn get_mask(value: u8) -> u8 {
        1 << value
    }

    pub fn set(&mut self, value: u8) {
        self.current_mask |= value;
    }

    pub fn is_set(&self, value: u8) -> bool {
        (self.current_mask & value) == value
    }
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(u8)]
pub enum AnimateAction {
    #[key(0u8)]
    SwingMainHand,
    #[key(1u8)]
    Hurt,
    #[key(2u8)]
    WakeUp,
    #[key(3u8)]
    SwingOffHand,
    #[key(4u8)]
    CriticalHit,
    #[key(5u8)]
    MagicCriticalHit,
}

#[derive(mc_serializer_derive::Serial, Debug, PartialEq, PartialOrd, Hash, Eq)]
pub struct AwardStatsType {
    pub stat_type: VarInt,
    pub value_type: VarInt,
}

#[derive(mc_serializer_derive::SerialBitMap)]
pub struct BossEventProperties {
    pub darken_screen: bool,
    pub play_music: bool,
    pub create_world_fog: bool,
}

#[derive(mc_serializer_derive::Serial)]
#[key(VarInt)]
pub enum BossBarColor {
    Pink,
    Blue,
    Red,
    Green,
    Yellow,
    Purple,
    White,
}

#[derive(mc_serializer_derive::Serial)]
#[key(VarInt)]
pub enum BossBarOverlay {
    Progress,
    Notched6,
    Notched10,
    Notched12,
    Notched20,
}

#[derive(mc_serializer_derive::Serial)]
#[key(VarInt)]
pub enum BossEventOperation {
    Add {
        #[json(32767)]
        name: Chat,
        progress: f32,
        color: BossBarColor,
        overlay: BossBarOverlay,
        properties: BossEventProperties,
    },
    Remove,
    UpdateProgress {
        progress: f32,
    },
    UpdateName {
        #[json(32767)]
        name: Chat,
    },
    UpdateStyle {
        color: BossBarColor,
        overlay: BossBarOverlay,
    },
    UpdateProperties {
        properties: BossEventProperties,
    },
}

#[derive(mc_serializer_derive::Serial)]
pub struct DeclareRecipesData {
    pub state: RecipeState,
    pub settings: RecipeBookSettings,
    pub recipes: (VarInt, Vec<ResourceLocation>),
    #[serial_if(let RecipeState::INIT = __serde_state)]
    #[default((VarInt::from(0), Vec::new()))]
    pub to_highlight: (VarInt, Vec<ResourceLocation>),
}

#[derive(mc_serializer_derive::Serial)]
pub struct CommandSuggestion {
    pub text: String,
    pub has_tooltip: bool,
    #[serialize_if(* __serde_has_tooltip)]
    #[deserialize_if(__serde_has_tooltip)]
    #[default(Chat::text(""))]
    #[json(32767)]
    pub tooltip: Chat,
}

#[derive(mc_serializer_derive::Serial)]
#[key(VarInt)]
pub enum CustomChatCompletionsAction {
    Add,
    Remove,
    Set,
}

#[derive(mc_serializer_derive::Serial)]
pub struct ExplodeOffset {
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

#[derive(mc_serializer_derive::Serial)]
#[key(u8)]
pub enum GameEventType {
    #[key(0u8)]
    NoRespawnBlockAvailable,
    #[key(1u8)]
    StartRaining,
    #[key(2u8)]
    StopRaining,
    #[key(3u8)]
    ChangeGameMode,
    #[key(4u8)]
    WinGame,
    #[key(5u8)]
    DemoEvent,
    #[key(6u8)]
    ArrowHitPlayer,
    #[key(7u8)]
    RainLevelChange,
    #[key(8u8)]
    ThunderLevelChange,
    #[key(9u8)]
    PufferFishSting,
    #[key(10u8)]
    GuardianElderEffect,
    #[key(11u8)]
    ImmediateRespawn,
}

#[derive(mc_serializer_derive::Serial)]
pub struct ParticleBase {
    pub override_limiter: bool,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub x_dist: f32,
    pub y_dist: f32,
    pub z_dist: f32,
    pub max_speed: f32,
    pub count: i32,
}

#[derive(mc_serializer_derive::Serial)]
#[key(VarInt)]
pub enum RawParticle {
    AmbientEntityEffect,
    AngryVillager,
    Block {
        block_id: VarInt,
    },
    BlockMarker {
        block_id: VarInt,
    },
    Bubble,
    Cloud,
    Crit,
    DamageIndicator,
    DragonBreath,
    DrippingLava,
    FallingLava,
    LandingLava,
    DrippingWater,
    FallingWater,
    Dust {
        color_x: f32,
        color_y: f32,
        color_z: f32,
        scale: f32,
    },
    DustColorTransition {
        color_x: f32,
        color_y: f32,
        color_z: f32,
        scale: f32,
        to_color_x: f32,
        to_color_y: f32,
        to_color_z: f32,
    },
    Effect,
    ElderGuardian,
    EnchantedHit,
    Enchant,
    EndRod,
    EntityEffect,
    ExplosionEmitter,
    Explosion,
    SonicBoom,
    FallingDust {
        block_id: VarInt,
    },
    Firework,
    Fishing,
    Flame,
    SculkSoul,
    SculkCharge {
        roll: f32,
    },
    SculkChargePop,
    SoulFireFlame,
    Soul,
    Flash,
    HappyVillager,
    Composter,
    Heart,
    InstantEffect,
    Item {
        item: ItemStackContainer,
    },
    Vibration {
        source: ResourceLocation,
        arrival_in_ticks: VarInt,
    },
    ItemSlime,
    ItemSnowball,
    LargeSmoke,
    Lava,
    Mycelium,
    Note,
    Poof,
    Portal,
    Rain,
    Smoke,
    Sneeze,
    Spit,
    SquidInk,
    SweepAttack,
    TotemOfUndying,
    Underwater,
    Splash,
    Witch,
    BubblePop,
    CurrentDown,
    BubbleColumnUp,
    Nautilus,
    Dolphin,
    CampfireCosySmoke,
    CampfireSignalSmoke,
    DrippingHoney,
    FallingHoney,
    LandingHoney,
    FallingNectar,
    FallingSporeBlossom,
    Ash,
    CrimsonSpore,
    WarpedSpore,
    SporeBlossomAir,
    DrippingObsidianTear,
    FallingObsidianTear,
    LandingObsidianTear,
    ReversePortal,
    WhiteAsh,
    SmallFlame,
    Snowflake,
    DrippingDripStoneLava,
    FallingDripStoneLava,
    GlowSquidInk,
    Glow,
    WaxOn,
    WaxOff,
    ElectricSpark,
    Scrape,
    Shriek {
        delay: i32,
    },
}

#[derive(mc_serializer_derive::Serial)]
#[key(VarInt)]
pub enum Particle {
    AmbientEntityEffect(ParticleBase),
    AngryVillager(ParticleBase),
    Block {
        base: ParticleBase,
        block_id: VarInt,
    },
    BlockMarker {
        base: ParticleBase,
        block_id: VarInt,
    },
    Bubble(ParticleBase),
    Cloud(ParticleBase),
    Crit(ParticleBase),
    DamageIndicator(ParticleBase),
    DragonBreath(ParticleBase),
    DrippingLava(ParticleBase),
    FallingLava(ParticleBase),
    LandingLava(ParticleBase),
    DrippingWater(ParticleBase),
    FallingWater(ParticleBase),
    Dust {
        base: ParticleBase,
        color_x: f32,
        color_y: f32,
        color_z: f32,
        scale: f32,
    },
    DustColorTransition {
        base: ParticleBase,
        color_x: f32,
        color_y: f32,
        color_z: f32,
        scale: f32,
        to_color_x: f32,
        to_color_y: f32,
        to_color_z: f32,
    },
    Effect(ParticleBase),
    ElderGuardian(ParticleBase),
    EnchantedHit(ParticleBase),
    Enchant(ParticleBase),
    EndRod(ParticleBase),
    EntityEffect(ParticleBase),
    ExplosionEmitter(ParticleBase),
    Explosion(ParticleBase),
    SonicBoom(ParticleBase),
    FallingDust {
        base: ParticleBase,
        block_id: VarInt,
    },
    Firework(ParticleBase),
    Fishing(ParticleBase),
    Flame(ParticleBase),
    SculkSoul(ParticleBase),
    SculkCharge {
        base: ParticleBase,
        roll: f32,
    },
    SculkChargePop(ParticleBase),
    SoulFireFlame(ParticleBase),
    Soul(ParticleBase),
    Flash(ParticleBase),
    HappyVillager(ParticleBase),
    Composter(ParticleBase),
    Heart(ParticleBase),
    InstantEffect(ParticleBase),
    Item {
        base: ParticleBase,
        item: ItemStackContainer,
    },
    Vibration {
        base: ParticleBase,
        source: ResourceLocation,
        arrival_in_ticks: VarInt,
    },
    ItemSlime(ParticleBase),
    ItemSnowball(ParticleBase),
    LargeSmoke(ParticleBase),
    Lava(ParticleBase),
    Mycelium(ParticleBase),
    Note(ParticleBase),
    Poof(ParticleBase),
    Portal(ParticleBase),
    Rain(ParticleBase),
    Smoke(ParticleBase),
    Sneeze(ParticleBase),
    Spit(ParticleBase),
    SquidInk(ParticleBase),
    SweepAttack(ParticleBase),
    TotemOfUndying(ParticleBase),
    Underwater(ParticleBase),
    Splash(ParticleBase),
    Witch(ParticleBase),
    BubblePop(ParticleBase),
    CurrentDown(ParticleBase),
    BubbleColumnUp(ParticleBase),
    Nautilus(ParticleBase),
    Dolphin(ParticleBase),
    CampfireCosySmoke(ParticleBase),
    CampfireSignalSmoke(ParticleBase),
    DrippingHoney(ParticleBase),
    FallingHoney(ParticleBase),
    LandingHoney(ParticleBase),
    FallingNectar(ParticleBase),
    FallingSporeBlossom(ParticleBase),
    Ash(ParticleBase),
    CrimsonSpore(ParticleBase),
    WarpedSpore(ParticleBase),
    SporeBlossomAir(ParticleBase),
    DrippingObsidianTear(ParticleBase),
    FallingObsidianTear(ParticleBase),
    LandingObsidianTear(ParticleBase),
    ReversePortal(ParticleBase),
    WhiteAsh(ParticleBase),
    SmallFlame(ParticleBase),
    Snowflake(ParticleBase),
    DrippingDripStoneLava(ParticleBase),
    FallingDripStoneLava(ParticleBase),
    GlowSquidInk(ParticleBase),
    Glow(ParticleBase),
    WaxOn(ParticleBase),
    WaxOff(ParticleBase),
    ElectricSpark(ParticleBase),
    Scrape(ParticleBase),
    Shriek {
        base: ParticleBase,
        delay: i32,
    },
}

#[derive(mc_serializer_derive::Serial)]
#[key(VarInt)]
pub enum MapDecorationType {
    Player,
    Frame,
    RedMarker,
    BlueMarker,
    TargetX,
    TargetPoint,
    PlayerOffMap,
    PlayerOffLimits,
    Mansion,
    Monument,
    BannerWhite,
    BannerOrange,
    BannerMagenta,
    BannerYellow,
    BannerLime,
    BannerPink,
    BannerGray,
    BannerLightGray,
    BannerCyan,
    BannerPurple,
    BannerBlue,
    BannerBrown,
    BannerGreen,
    BannerRed,
    BannerBlack,
    RedX,
}

#[derive(mc_serializer_derive::Serial)]
pub struct MapDecoration {
    pub decoration_type: MapDecorationType,
    pub x: u8,
    pub y: u8,
    pub rot: u8,
    pub name: MaybeComponent,
}

#[derive(mc_serializer_derive::Serial)]
pub struct MapPatch {
    pub width: u8,
    #[serialize_if(*__serde_width > 0)]
    #[deserialize_if(__serde_width > 0)]
    #[default(0)]
    pub height: u8,
    #[serialize_if(*__serde_width > 0)]
    #[deserialize_if(__serde_width > 0)]
    #[default(0)]
    pub start_x: u8,
    #[serialize_if(*__serde_width > 0)]
    #[deserialize_if(__serde_width > 0)]
    #[default(0)]
    pub start_y: u8,
    #[serialize_if(*__serde_width > 0)]
    #[deserialize_if(__serde_width > 0)]
    #[default((VarInt::from(0), vec![]))]
    pub colors: (VarInt, Vec<u8>),
}

#[derive(mc_serializer_derive::Serial)]
pub struct MerchantOffer {
    pub base_cost_a: ItemStackContainer,
    pub cost_b: ItemStackContainer,
    pub result: ItemStackContainer,
    pub out_of_stock: bool,
    pub uses: i32,
    pub max_uses: i32,
    pub xp: i32,
    pub special_price_diff: i32,
    pub price_multiplier: f32,
    pub demand: i32,
}

#[derive(mc_serializer_derive::Serial)]
#[key(VarInt)]
pub enum InteractionHand {
    MainHand,
    OffHand,
}

#[derive(mc_serializer_derive::Serial)]
#[key(VarInt)]
pub enum MenuType {
    Generic9x1,
    Generic9x2,
    Generic9x3,
    Generic9x4,
    Generic9x5,
    Generic9x6,
    Generic3x3,
    Anvil,
    Beacon,
    BlastFurnace,
    BrewingStand,
    Crafting,
    Enchantment,
    Furnace,
    Grindstone,
    Hopper,
    Lectern,
    Loom,
    Merchant,
    ShulkerBox,
    Smithing,
    Smoker,
    CartographyTable,
    StoneCutter,
}

#[derive(mc_serializer_derive::Serial)]
pub struct ChatHeader {
    pub previous_signature: (bool, Option<Signature>),
    pub sender: Uuid,
}

auto_string!(MessageContent, 256);

#[derive(mc_serializer_derive::Serial)]
pub struct LastSeenEntry {
    pub uuid: Uuid,
    pub signature: Signature,
}

#[derive(mc_serializer_derive::Serial)]
pub struct SignedMessageBody {
    pub content: MessageContent,
    pub decoration: MaybeComponent,
    pub timestamp: u64,
    pub salt: u64,
    pub last_seen: (VarInt, Vec<LastSeenEntry>),
}

#[derive(mc_serializer_derive::Serial)]
#[key(VarInt)]
pub enum FilterMask {
    PassThrough,
    FullyFiltered,
    PartiallyFiltered { mask: (VarInt, Vec<u64>) },
}

#[derive(mc_serializer_derive::Serial)]
pub struct ChatMessage {
    pub header: ChatHeader,
    pub header_signature: Signature,
    pub signed_body: SignedMessageBody,
    pub unsigned_content: MaybeComponent,
    pub filter_mask: FilterMask,
}

#[derive(mc_serializer_derive::Serial)]
pub struct ChatType {
    pub chat_type: VarInt,
    #[json(32767)]
    pub name: Chat,
    pub target_name: MaybeComponent,
}

#[derive(mc_serializer_derive::Serial)]
#[key(VarInt)]
pub enum PositionAnchor {
    Feet,
    Eyes,
}

#[derive(mc_serializer_derive::Serial)]
pub struct PlayerLookAtEntity {
    pub entity: VarInt,
    pub anchor: PositionAnchor,
}

#[derive(mc_serializer_derive::Serial)]
#[key(VarInt)]
pub enum MobEffect {
    Nil,
    MovementSpeed,
    MovementSlowdown,
    DigSpeed,
    DigSlowdown,
    DamageBoost,
    Heal,
    Harm,
    Jump,
    Confusion,
    Regeneration,
    DamageResistance,
    FireResistance,
    WaterBreathing,
    Invisibility,
    Blindness,
    NightVision,
    Hunger,
    Weakness,
    Poison,
    Wither,
    HealthBoost,
    Absorption,
    Saturation,
    Glowing,
    Levitation,
    Luck,
    Unluck,
    SlowFalling,
    ConduitPower,
    DolphinsGrace,
    BadOmen,
}

auto_string!(ResourcePackHash, 40);

#[derive(mc_serializer_derive::Serial)]
pub struct GlobalPos {
    pub dimension: ResourceLocation,
    pub pos: BlockPos,
}

#[derive(mc_serializer_derive::Serial)]
pub struct ServerDataInfo {
    pub motd: MaybeComponent,
    pub icon: (bool, Option<String>),
    pub previews_chat: bool,
    pub enforces_secure_chat: bool,
}

#[derive(mc_serializer_derive::Serial)]
#[key(VarInt)]
pub enum Pose {
    Standing,
    FallFlying,
    Sleeping,
    Swimming,
    SpinAttack,
    Crouching,
    LongJumping,
    Dying,
    Croaking,
    UsingTongue,
    Roaring,
    Sniffing,
    Emerging,
    Digging,
}

#[derive(mc_serializer_derive::Serial)]
#[key(VarInt)]
pub enum Direction {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

#[derive(mc_serializer_derive::Serial)]
#[key(VarInt)]
pub enum EntityData {
    Byte(u8),
    Int(VarInt),
    Float(f32),
    String(String),
    Component(#[json(32767)] Chat),
    OptionalComponent(MaybeComponent),
    ItemStack(ItemStackContainer),
    Boolean(bool),
    Rotations(f32, f32, f32),
    BlockPos(BlockPos),
    OptionalBlockPos((bool, Option<BlockPos>)),
    Direction(Direction),
    OptionalUuid((bool, Option<Uuid>)),
    BlockState(VarInt),
    CompoundTag(#[nbt] nbt::Blob),
    Particle(RawParticle),
    VillagerData {
        villager_type: VarInt,
        villager_profession: VarInt,
        level: VarInt,
    },
    OptionalUnsignedInt((bool, Option<VarInt>)),
    Pose(Pose),
    CatVariant(VarInt),
    FrogVariant(VarInt),
    OptionalGlobalPos((bool, Option<GlobalPos>)),
    PaintingVariant(VarInt),
}

pub struct EntityDataInfo {
    pub packed_items: Vec<(u8, EntityData)>,
}

contextual!(EntityDataInfo);

impl Serialize for EntityDataInfo {
    fn serialize<W: Write>(
        &self,
        writer: &mut W,
        protocol_version: ProtocolVersion,
    ) -> mc_serializer::serde::Result<()> {
        for packed_item in &self.packed_items {
            packed_item.0.serialize(writer, protocol_version)?;
            packed_item.1.serialize(writer, protocol_version)?;
        }
        255u8.serialize(writer, protocol_version)
    }

    fn size(&self, protocol_version: ProtocolVersion) -> mc_serializer::serde::Result<i32> {
        let mut size = 1;
        for packed_item in &self.packed_items {
            size += packed_item.0.size(protocol_version)?;
            size += packed_item.1.size(protocol_version)?;
        }
        Ok(size)
    }
}

impl Deserialize for EntityDataInfo {
    fn deserialize<R: Read>(
        reader: &mut R,
        protocol_version: ProtocolVersion,
    ) -> mc_serializer::serde::Result<Self> {
        let mut packed_items = Vec::<(u8, EntityData)>::new();
        let mut next_id;
        while {
            next_id = u8::deserialize(reader, protocol_version)?;
            next_id != 255
        } {
            packed_items.push((next_id, EntityData::deserialize(reader, protocol_version)?));
        }
        Ok(Self { packed_items })
    }
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(bool)]
pub enum MaybeComponent {
    #[key(false)]
    Empty,
    #[key(true)]
    Filled(#[json(32767)] Chat),
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(bool)]
pub enum MaybeNbt {
    #[key(false)]
    Empty,
    #[key(true)]
    Filled(#[nbt] nbt::Blob),
}

#[derive(mc_serializer_derive::MCSerialize)]
#[key(pass)]
pub enum SetEquipmentSlot {
    MainHand(ItemStackContainer),
    OffHand(ItemStackContainer),
    Feet(ItemStackContainer),
    Legs(ItemStackContainer),
    Chest(ItemStackContainer),
    Head(ItemStackContainer),
}

contextual!(SetEquipmentSlot);

pub struct SetEquipmentSlotItems {
    pub items: Vec<SetEquipmentSlot>,
}

contextual!(SetEquipmentSlotItems);

impl Serialize for SetEquipmentSlotItems {
    fn serialize<W: Write>(
        &self,
        writer: &mut W,
        protocol_version: ProtocolVersion,
    ) -> mc_serializer::serde::Result<()> {
        let mut iter = self.items.iter().peekable();
        while iter.peek().is_some() {
            let next = iter.next().unwrap();
            let self_mask = iter.peek().is_none();
            let (bit_write, item) = match next {
                SetEquipmentSlot::MainHand(item) => (0, item),
                SetEquipmentSlot::OffHand(item) => (1, item),
                SetEquipmentSlot::Feet(item) => (2, item),
                SetEquipmentSlot::Legs(item) => (3, item),
                SetEquipmentSlot::Chest(item) => (4, item),
                SetEquipmentSlot::Head(item) => (5, item),
            };
            let bit_write = if self_mask {
                bit_write | -128
            } else {
                bit_write
            };
            i8::serialize(&bit_write, writer, protocol_version)?;
            ItemStackContainer::serialize(item, writer, protocol_version)?;
        }
        Ok(())
    }

    fn size(&self, protocol_version: ProtocolVersion) -> mc_serializer::serde::Result<i32> {
        Ok(self.items.size(protocol_version)? + self.items.len() as i32)
    }
}

impl Deserialize for SetEquipmentSlotItems {
    fn deserialize<R: Read>(
        reader: &mut R,
        protocol_version: ProtocolVersion,
    ) -> mc_serializer::serde::Result<Self> {
        let mut items = Vec::new();
        loop {
            let next_byte = i8::deserialize(reader, protocol_version)?;
            let item_stack = ItemStackContainer::deserialize(reader, protocol_version)?;
            let equipment_slot = match next_byte & 127 {
                0 => SetEquipmentSlot::MainHand(item_stack),
                1 => SetEquipmentSlot::OffHand(item_stack),
                2 => SetEquipmentSlot::Feet(item_stack),
                3 => SetEquipmentSlot::Legs(item_stack),
                4 => SetEquipmentSlot::Chest(item_stack),
                5 => SetEquipmentSlot::Head(item_stack),
                _ => {
                    return Err(mc_serializer::serde::Error::Generic(
                        SerializerContext::new(
                            Self::context(),
                            format!("Failed to recognize equipment slot {}", next_byte & 127),
                        ),
                    ))
                }
            };
            items.push(equipment_slot);

            if next_byte & -128 == 0 {
                break;
            }
        }
        Ok(Self { items })
    }
}

#[derive(mc_serializer_derive::Serial)]
#[key(VarInt)]
pub enum SetObjectiveRenderType {
    Integer,
    Hearts,
}

#[derive(mc_serializer_derive::Serial)]
#[key(i8)]
pub enum SetObjectiveMethod {
    #[key(0i8)]
    Method0 {
        #[json(32767)]
        display_name: Chat,
        render_type: SetObjectiveRenderType,
    },
    #[key(1i8)]
    Method1,
    #[key(2i8)]
    Method2 {
        #[json(32767)]
        display_name: Chat,
        render_type: SetObjectiveRenderType,
    },
}

auto_string!(ParameterString, 40);

#[derive(mc_serializer_derive::Serial)]
#[key(VarInt)]
pub enum ChatFormatting {
    Black,
    DarkBlue,
    DarkGreen,
    DarkAqua,
    DarkRed,
    DarkPurple,
    Gold,
    Gray,
    DarkGray,
    Blue,
    Green,
    Aqua,
    Red,
    LightPurple,
    Yellow,
    White,
    Obfuscated,
    Bold,
    Strikethrough,
    Underline,
    Italic,
    Reset,
}

#[derive(mc_serializer_derive::Serial)]
pub struct PlayerTeamParameters {
    #[json(32767)]
    pub display_name: Chat,
    pub options: u8,
    pub name_tag_visibility: ParameterString,
    pub collision_rule: ParameterString,
    pub color: ChatFormatting,
    #[json(32767)]
    pub player_prefix: Chat,
    #[json(32767)]
    pub player_suffix: Chat,
}

#[derive(mc_serializer_derive::Serial)]
pub struct PlayerTeamPlayerList {
    players: (VarInt, Vec<String>),
}

#[derive(mc_serializer_derive::Serial)]
#[key(i8)]
pub enum SetPlayerTeamMethod {
    #[key(0i8)]
    Add(PlayerTeamParameters, PlayerTeamPlayerList),
    #[key(1i8)]
    Remove,
    #[key(2i8)]
    Change(PlayerTeamParameters),
    #[key(3i8)]
    Join(PlayerTeamPlayerList),
    #[key(4i8)]
    Leave(PlayerTeamPlayerList),
}

#[derive(mc_serializer_derive::Serial)]
#[key(VarInt)]
pub enum SetScoreMethod {
    Change {
        objective_name: String,
        score: VarInt,
    },
    Remove {
        objective_name: String,
    },
}

#[derive(mc_serializer_derive::Serial)]
#[key(i8)]
pub enum StopSoundMethod {
    #[key(1i8)]
    Unnamed(SoundSource),
    #[key(2i8)]
    Named(ResourceLocation),
}

#[derive(mc_serializer_derive::Serial)]
#[key(VarInt)]
pub enum FrameType {
    Task,
    Challenge,
    Goal,
}

#[derive(mc_serializer_derive::Serial)]
pub struct DisplayInfo {
    #[json(32767)]
    pub title: Chat,
    #[json(32767)]
    pub description: Chat,
    pub icon: ItemStackContainer,
    pub frame: FrameType,
    pub opts: i32,
    #[serial_if(__serde_opts & 1 != 0)]
    pub background: Option<ResourceLocation>,
    pub x: f32,
    pub y: f32,
}

#[derive(mc_serializer_derive::Serial)]
pub struct Criteria;

#[derive(mc_serializer_derive::Serial)]
pub struct AdvancementAddEntry {
    pub parent_id: (bool, Option<ResourceLocation>),
    pub display_info: (bool, Option<DisplayInfo>),
    pub criteria: HashMap<String, Criteria>,
}

#[derive(mc_serializer_derive::Serial)]
pub struct CriteriaProgress {
    pub obtained: (bool, Option<u64>),
}

#[derive(mc_serializer_derive::Serial)]
pub struct AdvancementProgressEntry {
    pub criteria: HashMap<String, CriteriaProgress>,
}

#[derive(mc_serializer_derive::Serial)]
#[key(i8)]
pub enum Operation {
    #[key(0i8)]
    Addition,
    #[key(1i8)]
    MultiplyBase,
    #[key(2i8)]
    MultiplyTotal,
}

#[derive(mc_serializer_derive::Serial)]
pub struct AttributeModifier {
    modifier_id: Uuid,
    amount: f64,
    operation: Operation,
}

#[derive(mc_serializer_derive::Serial)]
pub struct Attribute {
    attribute_name: ResourceLocation,
    base: f64,
    modifiers: (VarInt, Vec<Operation>),
}
