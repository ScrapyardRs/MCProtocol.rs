use drax::{nbt::CompoundTag, Maybe, SizedVec, VarInt};
use uuid::Uuid;

use crate::chat::Chat;

use super::{chunk::Chunk, GameProfile};

const MULTIPLY_DE_BRUIJN_BIT_POSITION: [i32; 32] = [
    0, 1, 28, 2, 29, 14, 24, 3, 30, 22, 20, 15, 25, 17, 4, 8, 31, 27, 13, 23, 21, 19, 16, 7, 26,
    12, 18, 6, 11, 5, 10, 9,
];

pub const fn ceil_log_2(n: i32) -> i32 {
    let n = if is_power_of_2(n) { n } else { shift_2(n) };
    MULTIPLY_DE_BRUIJN_BIT_POSITION
        [((((n as u64 * 125613361u64) >> 27u64) as i32) & 0x1Fi32) as usize]
}

const fn is_power_of_2(n: i32) -> bool {
    n != 0i32 && (n & (n - 1i32)) == 0i32
}

const fn shift_2(n: i32) -> i32 {
    let mut n2 = n - 1i32;
    n2 |= n2 >> 1i32;
    n2 |= n2 >> 2i32;
    n2 |= n2 >> 4i32;
    n2 |= n2 >> 8i32;
    n2 |= n2 >> 16i32;
    n2 + 1i32
}

const fn log2(n: i32) -> i32 {
    ceil_log_2(n) - if is_power_of_2(n) { 0i32 } else { 1i32 }
}

#[derive(Debug)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    const PACKED_Z_LENGTH: i32 = (1i32 + log2(shift_2(30000000)));
    const PACKED_X_LENGTH: i32 = (Self::PACKED_Z_LENGTH) as i32;
    const PACKED_Y_LENGTH: i32 =
        (64u64 - Self::PACKED_X_LENGTH as u64 - Self::PACKED_Z_LENGTH as u64) as i32;
    const PACKED_X_MASK: i64 = ((1u64 << Self::PACKED_X_LENGTH) - 1u64) as i64;
    const PACKED_Y_MASK: i64 = ((1u64 << Self::PACKED_Y_LENGTH) - 1u64) as i64;
    const PACKED_Z_MASK: i64 = ((1u64 << Self::PACKED_Z_LENGTH) - 1u64) as i64;
    const Z_OFFSET: i32 = (Self::PACKED_Y_LENGTH) as i32;
    const X_OFFSET: i32 = (Self::PACKED_Y_LENGTH + Self::PACKED_Z_LENGTH) as i32;
}

impl drax::transport::DraxTransport for BlockPos {
    fn write_to_transport(
        &self,
        context: &mut drax::transport::TransportProcessorContext,
        writer: &mut std::io::Cursor<Vec<u8>>,
    ) -> drax::transport::Result<()> {
        let mut value: i64 = 0;
        value |= (self.x as i64 & Self::PACKED_X_MASK) << Self::X_OFFSET;
        value |= self.y as i64 & Self::PACKED_Y_MASK;
        value |= (self.z as i64 & Self::PACKED_Z_MASK) << Self::Z_OFFSET;
        value.write_to_transport(context, writer)
    }

    fn read_from_transport<R: std::io::Read>(
        context: &mut drax::transport::TransportProcessorContext,
        read: &mut R,
    ) -> drax::transport::Result<Self>
    where
        Self: Sized,
    {
        let long = i64::read_from_transport(context, read)?;
        let x = (long << (64 - Self::X_OFFSET - Self::PACKED_X_LENGTH)
            >> (64 - Self::PACKED_X_LENGTH)) as i32;
        let y = (long << (64 - Self::PACKED_Y_LENGTH) >> (64 - Self::PACKED_Y_LENGTH)) as i32;
        let z = (long << (64 - Self::Z_OFFSET - Self::PACKED_Z_LENGTH)
            >> (64 - Self::PACKED_Z_LENGTH)) as i32;
        Ok(BlockPos { x, y, z })
    }

    fn precondition_size(
        &self,
        _: &mut drax::transport::TransportProcessorContext,
    ) -> drax::transport::Result<usize> {
        Ok(8)
    }
}

#[derive(drax_derive::DraxTransport, Debug, Default)]
pub struct RelativeArgument {
    current_mask: u8,
}

impl RelativeArgument {
    pub const X: u8 = Self::get_mask(0);
    pub const Y: u8 = Self::get_mask(1);
    pub const Z: u8 = Self::get_mask(2);
    pub const Y_ROT: u8 = Self::get_mask(3);
    pub const X_ROT: u8 = Self::get_mask(4);

    pub fn from_mask(current_mask: u8) -> Self {
        Self { current_mask }
    }

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

#[derive(drax_derive::DraxTransport, Debug)]
pub struct Identifier {
    pub data: String,
}

impl Identifier {
    pub fn new(data: String) -> Identifier {
        Identifier { data: String::from("minecraft:") + &data }
    }
}

#[derive(Debug)]
pub struct ItemStack {
    pub id: Maybe<VarInt>,
    pub count: Maybe<u8>,
    pub tag: Maybe<CompoundTag>,
}

impl drax::transport::DraxTransport for ItemStack {
    fn write_to_transport(
        &self,
        context: &mut drax::transport::TransportProcessorContext,
        writer: &mut std::io::Cursor<Vec<u8>>,
    ) -> drax::transport::Result<()> {
        if let (Some(id), Some(count)) = (self.id, self.count) {
            if count == 0 {
                false.write_to_transport(context, writer)
            } else {
                true.write_to_transport(context, writer)?;
                id.write_to_transport(context, writer)?;
                count.write_to_transport(context, writer)?;
                match &self.tag {
                    Some(n) => drax::nbt::write_nbt(&n, writer),
                    None => (0 as u8).write_to_transport(context, writer)
                }
            }
        } else {
            false.write_to_transport(context, writer)
        }
    }
    fn read_from_transport<R: std::io::Read>(
        context: &mut drax::transport::TransportProcessorContext,
        read: &mut R,
    ) -> drax::transport::Result<Self>
    where
        Self: Sized,
    {
        let present = bool::read_from_transport(context, read)?;
        if present {
            let id = VarInt::read_from_transport(context, read)?;
            let count = u8::read_from_transport(context, read)?;
            let tag = drax::nbt::read_nbt(read, 32767)?;
            Ok(ItemStack { id: Some(id), count: Some(count), tag })
        } else {
            Ok(ItemStack {
                id: None,
                count: None,
                tag: None,
            })
        }
    }
    fn precondition_size(
        &self,
        context: &mut drax::transport::TransportProcessorContext,
    ) -> drax::transport::Result<usize> {
        let mut size: usize = 0;
        if let (Some(id), Some(count)) = (self.id, self.count) {
            if count == 0 {
                return Ok(1 as usize);
            } else {
                size += drax::transport::DraxTransport::precondition_size(&id, context)?;
                size += 1;
                if let Some(tag) = &self.tag {
                    size += drax::nbt::size_nbt(&tag);
                }
                Ok(size)
            }
        } else {
            Ok(1 as usize)
        }
    }
}

#[derive(drax_derive::DraxTransport, Debug)]
pub struct BlockEntityInfo {
    pub packed_xz: u8,
    pub y: i16,
    pub block_type: VarInt,
    pub tag: CompoundTag,
}

#[derive(drax_derive::DraxTransport, Debug)]
pub struct LevelChunkData {
    pub chunk: Chunk,
    pub block_entities: SizedVec<BlockEntityInfo>,
}

#[derive(drax_derive::DraxTransport, Debug)]
pub struct LightUpdateData {
    pub trust_edges: bool,
    pub sky_y_mask: SizedVec<u64>,
    pub block_y_mask: SizedVec<u64>,
    pub empty_sky_y_mask: SizedVec<u64>,
    pub empty_block_y_mask: SizedVec<u64>,
    pub sky_updates: SizedVec<SizedVec<u8>>,
    pub block_updates: SizedVec<SizedVec<u8>>,
}

#[derive(drax_derive::DraxTransport, Debug)]
#[drax(key = {match u8})]
pub enum GameType {
    Survival,
    Creative,
    Adventure,
    Spectator,
    #[drax(key = {255u8})]
    None,
}

#[derive(drax_derive::DraxTransport, Debug)]
pub struct AddPlayerEntry {
    pub profile: GameProfile,
    pub game_type: GameType,
    pub latency: VarInt,
    #[drax(json = 262144)]
    pub display_name: Maybe<Chat>,
    pub key_data: Maybe<crate::protocol::login::MojangIdentifiedKey>,
}

#[derive(drax_derive::DraxTransport, Debug)]
pub struct UpdateGameModeEntry {
    pub uuid: Uuid,
    pub game_type: GameType,
}

#[derive(drax_derive::DraxTransport, Debug)]
pub struct UpdateLatencyEntry {
    pub uuid: Uuid,
    pub latency: VarInt,
}

#[derive(drax_derive::DraxTransport, Debug)]
pub struct UpdateDisplayNameEntry {
    pub uuid: Uuid,
    #[drax(json = 32767)]
    pub display_name: Maybe<Chat>,
}

#[derive(drax_derive::DraxTransport, Debug)]
pub struct RemovePlayerEntry {
    pub uuid: Uuid,
}

#[derive(drax_derive::BitMapTransport, Debug)]
pub struct PlayerAbilitiesBitMap {
    pub invulnerable: bool,
    pub flying: bool,
    pub can_fly: bool,
    pub instant_build: bool,
}

pub mod sb {
    use drax::VarLong;
    use drax::{nbt::CompoundTag, Maybe, SizedVec, VarInt};
    use uuid::Uuid;

    use crate::protocol::chunk::Chunk;
    use crate::{chat::Chat, commands::Command, protocol::GameProfile};
    use crate::protocol::bit_storage::BitStorage;

    use super::{BlockPos, Identifier, ItemStack};

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct AcceptTeleportation {
        pub id: VarInt,
    }
    
    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct BlockEntityTagQuery {
        pub id: VarInt,
        pub location: BlockPos,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct ChangeDifficulty {
        pub difficulty: u8,
    }

    // TODO: chat packets

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct ContainerButtonClick {
        pub window_id: u8,
        pub button_id: u8,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct ContainerClick {
        pub window_id: u8,
        pub state_id: VarInt,
        pub slot: i16,
        pub button: u8,
        pub mode: VarInt,
        pub slots_array: SizedVec<ItemStack>,
        pub carried_item: ItemStack
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct ContainerClose {
        pub window_id: u8,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct PluginMessage {
        pub channel: Identifier,
        pub data: Vec<u8>,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct EditBook {
        pub slot: VarInt,
        pub count: VarInt,
        pub entries: Vec<String>,
        pub titled: bool,
        pub title: Maybe<String>,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct EntityTagQuery {
        pub transaction_id: VarInt,
        pub entity_id: VarInt,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct Interact {
        pub id: VarInt,
        pub interact_type: VarInt,
        pub target_x: Maybe<f32>,
        pub target_y: Maybe<f32>,
        pub target_z: Maybe<f32>,
        pub hand: Maybe<VarInt>,
        pub sneaking: bool,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct JigsawGenerate {
        pub location: BlockPos,
        pub levels: VarInt,
        pub keep_jigsaws: bool,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct KeepAlive {
        pub id: i64,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct LockDifficulty {
        pub locked: bool,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct MovePlayer {
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub on_ground: bool,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct MoveRotatePlayer {
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub yaw: f32,
        pub pitch: f32,
        pub on_ground: bool,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct RotatePlayer {
        pub yaw: f32,
        pub pitch: f32,
        pub on_ground: bool,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct GroundPlayer {
        pub on_ground: bool,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct MoveVehicle {
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub yaw: f32,
        pub pitch: f32,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct PaddleBoat {
        pub left: bool,
        pub right: bool,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct PickItem {
        pub slot: VarInt,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct PlaceRecipe {
        pub id: u8,
        pub recipe: Identifier,
        pub shift_down: bool,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct PlayerAbilities {
        pub is_flying: u8,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct PlayerAction {
        pub action: VarInt,
        pub location: BlockPos,
        pub direction: u8,
        pub sequence: VarInt,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct PlayerCommand {
        pub id: VarInt,
        pub action: VarInt,
        pub jump_boost: VarInt,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct PlayerInput {
        pub sideways: f32,
        pub forward: f32,
        pub flags: u8,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct Pong {
        pub id: i32,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct ChangeRecipeBookSettings {
        pub id: VarInt,
        pub is_open: bool,
        pub is_filtering: bool,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct SetSeenRecipe {
        pub recipe_id: Identifier,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct RenameItem {
        #[drax(limit = 32767)]
        pub name: String,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct ResourcePack {
        pub result: VarInt,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct SeenAdvancements {
        pub action: VarInt,
        pub tab: Maybe<Identifier>,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct SelectTrade {
        pub slot: VarInt,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct SetBeaconEffect {
        pub has_primary_effect: bool,
        pub primary_effect: VarInt,
        pub has_secondary_effect: bool,
        pub secondary_effect: VarInt,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct SetHeldItem {
        pub slot: i16,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct ProgramCommandBlock {
        pub location: BlockPos,
        #[drax(limit = 32767)]
        pub command: String,
        pub mode: VarInt,
        pub flags: u8,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct ProgramCommandBlockMinecart {
        pub id: VarInt,
        #[drax(limit = 32767)]
        pub command: String,
        pub track_output: bool,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct SetCreativeModeSlot {
        pub slot: i16,
        pub clicked_item: ItemStack,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct ProgramJigsawBlock {
        pub location: BlockPos,
        pub name: Identifier,
        pub target: Identifier,
        pub pool: Identifier,
        #[drax(limit = 32767)]
        pub final_state: String,
        pub joint_type: String,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct ProgramStructureBlock {
        pub location: BlockPos,
        pub action: VarInt,
        pub mode: VarInt,
        #[drax(limit = 32767)]
        pub name: String,
        pub offset_x: i8,
        pub offset_y: i8,
        pub offset_z: i8,
        pub size_x: i8,
        pub size_y: i8,
        pub size_z: i8,
        pub mirror: VarInt,
        pub rotation: VarInt,
        #[drax(limit = 128)]
        pub metadata: String,
        pub integrity: f32,
        pub seed: VarLong,
        pub flags: u8,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct UpdateSign {
        pub location: BlockPos,
        #[drax(limit = 384)]
        pub line1: String,
        #[drax(limit = 384)]
        pub line2: String,
        #[drax(limit = 384)]
        pub line3: String,
        #[drax(limit = 384)]
        pub line4: String,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct SwingArm {
        pub hand: VarInt,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct TeleportToEntity {
        pub player: Uuid,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct UseItemOn {
        pub hand: VarInt,
        pub location: BlockPos,
        pub face: VarInt,
        pub cursor_pos_x: f32,
        pub cursor_pos_y: f32,
        pub cursor_pos_z: f32,
        pub inside_block: bool,
        pub sequence: VarInt,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct UseItem {
        pub hand: VarInt,
        pub sequence: VarInt,
    }

    use super::super::CURRENT_VERSION_IMPL;
    
    crate::import_registrations! {
        AcceptTeleportation {
            CURRENT_VERSION_IMPL -> 0x00,
        }
        BlockEntityTagQuery {
            CURRENT_VERSION_IMPL -> 0x01,
        }
        ChangeDifficulty {
            CURRENT_VERSION_IMPL -> 0x02,
        }
        // TODO: Chat nonsense
        ContainerButtonClick {
            CURRENT_VERSION_IMPL -> 0x0A,
        }
        ContainerClick {
            CURRENT_VERSION_IMPL -> 0x0B,
        }
        ContainerClose {
            CURRENT_VERSION_IMPL -> 0x0C,
        }
        PluginMessage {
            CURRENT_VERSION_IMPL -> 0x0D,
        }
        EditBook {
            CURRENT_VERSION_IMPL -> 0x0E,
        }
        EntityTagQuery {
            CURRENT_VERSION_IMPL -> 0x0F,
        }
        Interact {
            CURRENT_VERSION_IMPL -> 0x10,
        }
        JigsawGenerate {
            CURRENT_VERSION_IMPL -> 0x11,
        }
        KeepAlive {
            CURRENT_VERSION_IMPL -> 0x12,
        }
        LockDifficulty {
            CURRENT_VERSION_IMPL -> 0x13,
        }
        MovePlayer {
            CURRENT_VERSION_IMPL -> 0x14,
        }
        MoveRotatePlayer {
            CURRENT_VERSION_IMPL -> 0x15,
        }
        RotatePlayer {
            CURRENT_VERSION_IMPL -> 0x16,
        }
        GroundPlayer {
            CURRENT_VERSION_IMPL -> 0x17,
        }
        MoveVehicle {
            CURRENT_VERSION_IMPL -> 0x18,
        }
        PaddleBoat {
            CURRENT_VERSION_IMPL -> 0x19,
        }
        PickItem {
            CURRENT_VERSION_IMPL -> 0x1A,
        }
        PlaceRecipe{
            CURRENT_VERSION_IMPL -> 0x1B,
        }
        PlayerAbilities {
            CURRENT_VERSION_IMPL -> 0x1C,
        }
        PlayerAction {
            CURRENT_VERSION_IMPL -> 0x1D,
        }
        PlayerCommand {
            CURRENT_VERSION_IMPL -> 0x1E,
        }
        PlayerInput {
            CURRENT_VERSION_IMPL -> 0x1F,
        }
        Pong {
            CURRENT_VERSION_IMPL -> 0x20,
        }
        ChangeRecipeBookSettings {
            CURRENT_VERSION_IMPL -> 0x21,
        }
        SetSeenRecipe {
            CURRENT_VERSION_IMPL -> 0x22,
        }
        RenameItem {
            CURRENT_VERSION_IMPL -> 0x23,
        }
        ResourcePack {
            CURRENT_VERSION_IMPL -> 0x24,
        }
        SeenAdvancements {
            CURRENT_VERSION_IMPL -> 0x25,
        }
        SelectTrade {
            CURRENT_VERSION_IMPL -> 0x26,
        }
        SetBeaconEffect {
            CURRENT_VERSION_IMPL -> 0x27,
        }
        SetHeldItem {
            CURRENT_VERSION_IMPL -> 0x28,
        }
        ProgramCommandBlock {
            CURRENT_VERSION_IMPL -> 0x29,
        }
        ProgramCommandBlockMinecart {
            CURRENT_VERSION_IMPL -> 0x2A,
        }
        SetCreativeModeSlot {
            CURRENT_VERSION_IMPL -> 0x2B,
        }
        ProgramJigsawBlock {
            CURRENT_VERSION_IMPL -> 0x2C,
        }
        ProgramStructureBlock {
            CURRENT_VERSION_IMPL -> 0x2D,
        }
        UpdateSign {
            CURRENT_VERSION_IMPL -> 0x2E,
        }
        SwingArm {
            CURRENT_VERSION_IMPL -> 0x2F,
        }
        TeleportToEntity {
            CURRENT_VERSION_IMPL -> 0x30,
        }
        UseItemOn {
            CURRENT_VERSION_IMPL -> 0x31,
        }
        UseItem {
            CURRENT_VERSION_IMPL -> 0x32,
        }
    }
}

pub mod cb {
    use drax::{nbt::CompoundTag, Maybe, SizedVec, VarInt};

    use crate::{chat::Chat, commands::Command};

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct DeclareCommands {
        pub commands: SizedVec<Command>,
        pub root_index: VarInt,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct PluginMessage {
        pub identifier: String,
        pub data: Vec<u8>,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct Disconnect {
        #[drax(json = 32767)]
        pub reason: Chat,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct KeepAlive {
        pub id: u64,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct LevelChunkWithLight {
        pub chunk_data: super::LevelChunkData,
        pub light_data: super::LightUpdateData,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct JoinGame {
        pub player_id: i32,
        pub hardcore: bool,
        pub game_type: super::GameType,
        pub previous_game_type: super::GameType,
        pub levels: SizedVec<String>,
        pub codec: CompoundTag,
        pub dimension_type: String,
        pub dimension: String,
        pub seed: u64,
        pub max_players: VarInt,
        pub chunk_radius: VarInt,
        pub simulation_distance: VarInt,
        pub reduced_debug_info: bool,
        pub show_death_screen: bool,
        pub is_debug: bool,
        pub is_flat: bool,
        pub last_death_location: Maybe<super::BlockPos>,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct PlayerAbilities {
        pub player_abilities_map: super::PlayerAbilitiesBitMap,
        pub flying_speed: f32,
        pub fov_modifier: f32,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    #[drax(key = {match VarInt})]
    pub enum PlayerInfo {
        AddPlayer(SizedVec<super::AddPlayerEntry>),
        UpdateGameMode(SizedVec<super::UpdateGameModeEntry>),
        UpdateLatency(SizedVec<super::UpdateLatencyEntry>),
        UpdateDisplayName(SizedVec<super::UpdateDisplayNameEntry>),
        RemovePlayer(SizedVec<super::RemovePlayerEntry>),
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct PlayerPosition {
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub y_rot: f32,
        pub x_rot: f32,
        pub relative_arguments: super::RelativeArgument,
        pub id: VarInt,
        pub dismount_vehicle: bool,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct SetSubtitle {
        #[drax(json = 32767)]
        pub text: Chat,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct SetTitle {
        #[drax(json = 32767)]
        pub text: Chat,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct SetTitleAnimationTimes {
        pub fade_in: i32,
        pub stay: i32,
        pub fade_out: i32,
    }

    use super::super::CURRENT_VERSION_IMPL;

    crate::import_registrations! {
        DeclareCommands {
            CURRENT_VERSION_IMPL -> 0xF,
        }

        PluginMessage {
            CURRENT_VERSION_IMPL -> 0x16,
        }

        Disconnect {
            CURRENT_VERSION_IMPL -> 0x19,
        }

        KeepAlive {
            CURRENT_VERSION_IMPL -> 0x20,
        }

        LevelChunkWithLight {
            CURRENT_VERSION_IMPL -> 0x21,
        }

        JoinGame {
            CURRENT_VERSION_IMPL -> 0x25,
        }

        PlayerAbilities {
            CURRENT_VERSION_IMPL -> 0x31,
        }

        PlayerInfo {
            CURRENT_VERSION_IMPL -> 0x37,
        }

        PlayerPosition {
            CURRENT_VERSION_IMPL -> 0x39,
        }

        SetSubtitle {
            CURRENT_VERSION_IMPL -> 0x5B,
        }

        SetTitle {
            CURRENT_VERSION_IMPL -> 0x5D,
        }

        SetTitleAnimationTimes {
            CURRENT_VERSION_IMPL -> 0x5E,
        }
    }
}
