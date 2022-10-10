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

#[derive(Debug)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    const PACKED_Z_LENGTH: i32 = (1u64 + log2(shift_2(30000000u64))) as i32;
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

    use crate::{chat::Chat, commands::Command, protocol::GameProfile};

    #[derive(drax_derive::DraxTransport, Debug)]
    pub struct DeclareCommands {
        commands: SizedVec<Command>,
        root_index: VarInt,
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
        player_id: i32,
        hardcore: bool,
        game_type: GameType,
        previous_game_type: GameType,
        levels: SizedVec<String>,
        codec: CompoundTag,
        dimension_type: String,
        dimension: String,
        seed: u64,
        max_players: VarInt,
        chunk_radius: VarInt,
        simulation_distance: VarInt,
        reduced_debug_info: bool,
        show_death_screen: bool,
        is_debug: bool,
        is_flat: bool,
        last_death_location: Maybe<super::BlockPos>,
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
        pub walking_speed: f32,
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
        y: f64,
        z: f64,
        y_rot: f32,
        x_rot: f32,
        relative_arguments: super::RelativeArgument,
        id: VarInt,
        dismount_vehicle: bool,
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
