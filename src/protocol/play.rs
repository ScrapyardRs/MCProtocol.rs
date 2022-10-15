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

pub mod sb {}

pub mod cb {
    use drax::{nbt::CompoundTag, Maybe, SizedVec, VarInt};
    use uuid::Uuid;

    use crate::protocol::bit_storage::BitStorage;
    use crate::protocol::chunk::Chunk;
    use crate::{chat::Chat, commands::Command, protocol::GameProfile};

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct DeclareCommands {
        pub commands: SizedVec<Command>,
        pub root_index: VarInt,
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
    pub struct LevelChunkWithLight {
        pub chunk_data: LevelChunkData,
        pub light_data: LightUpdateData,
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
    pub struct JoinGame {
        pub player_id: i32,
        pub hardcore: bool,
        pub game_type: GameType,
        pub previous_game_type: GameType,
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

    #[derive(drax_derive::BitMapTransport, Debug)]
    pub struct PlayerAbilitiesBitMap {
        pub invulnerable: bool,
        pub flying: bool,
        pub can_fly: bool,
        pub instant_build: bool,
    }

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct PlayerAbilities {
        pub player_abilities_map: PlayerAbilitiesBitMap,
        pub flying_speed: f32,
        pub fov_modifier: f32,
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

    #[derive(drax_derive::DraxTransport, Debug)]
    #[drax(key = {match VarInt})]
    pub enum PlayerInfo {
        AddPlayer(SizedVec<AddPlayerEntry>),
        UpdateGameMode(SizedVec<UpdateGameModeEntry>),
        UpdateLatency(SizedVec<UpdateLatencyEntry>),
        UpdateDisplayName(SizedVec<UpdateDisplayNameEntry>),
        RemovePlayer(SizedVec<RemovePlayerEntry>),
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

    use super::super::CURRENT_VERSION_IMPL;

    crate::import_registrations! {
        DeclareCommands {
            CURRENT_VERSION_IMPL -> 0xF,
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
    }
}
