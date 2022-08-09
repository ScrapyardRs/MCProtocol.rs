use std::collections::HashMap;
use mc_chat::Chat;
use mc_commands::Command;
use mc_serializer::primitive::{VarInt, VarLong};
use crate::shared_types::{GameProfile, MCIdentifiedKey};
use crate::shared_types::play::{BlockPos, Difficulty, GameType, Recipe, ResourceLocation};
use mc_serializer::serde::Contextual;

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
pub struct PlayerPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub y_rot: f32,
    pub x_rot: f32,
    pub relative_arguments: RelativeArgument,
    pub id: VarInt,
    pub dismount_vehicle: bool,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct SetDefaultSpawnPosition {
    pub pos: BlockPos,
    pub angle: f32,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct WorldBorder {
    pub new_center_x: f64,
    pub new_center_z: f64,
    pub old_size: f64,
    pub new_size: f64,
    pub lerp_time: VarLong,
    pub new_absolute_max_size: VarInt,
    pub warning_blocks: VarInt,
    pub warning_time: VarInt,
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
    pub heightmaps: nbt::Blob,
    pub buffer: (VarInt, Vec<u8>),
    pub block_entities: Vec<BlockEntityInfo>,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct LevelChunkWithLight {
    pub x: VarInt,
    pub z: VarInt,
    pub data: LightUpdateData,
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

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct LightUpdate {
    pub x: VarInt,
    pub z: VarInt,
    pub data: LightUpdateData,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct SetChunkCacheCenter {
    pub x: VarInt,
    pub z: VarInt,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct AddPlayerEntry {
    pub profile: GameProfile,
    pub game_type: GameType,
    pub latency: VarInt,
    #[json(262144)]
    pub display_name: Chat,
    pub key_data: MCIdentifiedKey,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct UpdateGameModeEntry {
    pub uuid: uuid::Uuid,
    pub game_type: GameType,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct UpdateLatencyEntry {
    pub uuid: uuid::Uuid,
    pub latency: VarInt,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct UpdateDisplayNameEntry {
    pub uuid: uuid::Uuid,
    #[json(262144)]
    pub display_name: Chat,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct RemovePlayerEntry {
    pub uuid: uuid::Uuid,
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(VarInt)]
pub enum PlayerInfo {
    #[key(VarInt::from(0))]
    AddPlayer(Vec<AddPlayerEntry>),
    #[key(VarInt::from(1))]
    UpdateGameMode(Vec<UpdateGameModeEntry>),
    #[key(VarInt::from(2))]
    UpdateLatency(Vec<UpdateLatencyEntry>),
    #[key(VarInt::from(3))]
    UpdateDisplayName(Vec<UpdateDisplayNameEntry>),
    #[key(VarInt::from(4))]
    RemovePlayer(Vec<RemovePlayerEntry>),
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
pub struct RecipeBookSettings {
    crafting_open: bool,
    crafting_filtering: bool,
    furnace_open: bool,
    furnace_filtering: bool,
    blast_furnace_open: bool,
    blast_furnace_filtering: bool,
    smoker_open: bool,
    smoker_filtering: bool,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct DeclareRecipes {
    pub state: RecipeState,
    pub settings: RecipeBookSettings,
    pub recipes: (VarInt, Vec<ResourceLocation>),
    #[serial_if(let RecipeState::INIT = __serde_state)]
    #[default((VarInt::from(0), Vec::new()))]
    pub to_highlight: (VarInt, Vec<ResourceLocation>),
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct DeclareCommands {
    pub commands: Vec<Command>,
    pub root_index: VarInt,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct EntityEvent {
    pub entity_id: i32,
    pub event_id: u8,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct UpdateTags {
    pub tags: HashMap<ResourceLocation, (VarInt, Vec<VarInt>)>,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct UpdateRecipes {
    pub recipes: Vec<Recipe>,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct SetCarriedItem {
    pub slot: u8,
}

#[derive(mc_serializer_derive::SerialBitMap, Debug)]
pub struct PlayerAbilitiesBitMap {
    pub invulnerable: bool,
    pub flying: bool,
    pub can_fly: bool,
    pub instant_build: bool,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct PlayerAbilities {
    pub player_abilities_bits: PlayerAbilitiesBitMap,
    pub flying_speed: f32,
    pub walking_speed: f32,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct ChangeDifficulty {
    pub difficulty: Difficulty,
    pub locked: bool,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct PluginMessage {
    pub identifier: ResourceLocation,
    pub data: Vec<u8>,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct JoinGame {
    pub player_id: i32,
    pub hardcore: bool,
    pub game_type: GameType,
    pub previous_game_type: GameType,
    pub levels: Vec<ResourceLocation>,
    pub codec: nbt::Blob,
    pub dimension_type: ResourceLocation,
    pub dimension: ResourceLocation,
    pub seed: u64,
    pub max_players: i32,
    pub chunk_radius: i32,
    pub simulation_distance: i32,
    pub reduced_debug_info: bool,
    pub show_death_screen: bool,
    pub is_debug: bool,
    pub is_flat: bool,
    pub last_death_location: (bool, Option<BlockPos>),
}
