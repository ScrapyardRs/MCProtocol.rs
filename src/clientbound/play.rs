use crate::common::chat::Chat;
use crate::common::play::{
    BlockPos, GlobalPos, InteractionHand, ItemStack, Location, SimpleLocation,
};
use drax::nbt::CompoundTag;
use drax::prelude::Uuid;
use drax::transport::packet::option::Maybe;
use drax::transport::packet::primitive::{VarInt, VarLong};
use drax::transport::packet::serde_json::JsonDelegate;
use drax::transport::packet::vec::ByteDrain;

registry! {
    components {
        struct StatsEntry {
            stat_id: VarInt,
            stat_cap: VarInt,
            stat_value: VarInt
        },

        enum BossBarColor<key: VarInt> {
            Pink {},
            Blue {},
            Red {},
            Green {},
            Yellow {},
            Purple {},
            White {}
        },

        enum BossBarOverlay<key: VarInt> {
            Progress {},
            Notched6 {},
            Notched10 {},
            Notched20 {}
        },

        enum BossEventOperationType<key: VarInt> {
            Add {
                name: JsonDelegate<Chat>,
                progress: f32,
                color: BossBarColor,
                overlay: BossBarOverlay,
                mask: u8
            },
            Remove {},
            UpdateProgress {
                progress: f32
            },
            UpdateName {
                name: JsonDelegate<Chat>
            },
            UpdateStyle {
                color: BossBarColor,
                overlay: BossBarOverlay
            },
            UpdateProperties {
                mask: u8
            }
        },

        enum Difficulty<key: u8> {
            Peaceful {},
            Easy {},
            Normal {},
            Hard {}
        }
    }

    registry ClientboundPlayRegistry {
        struct AddEntity {
            id: VarInt,
            uuid: Uuid,
            entity_type: VarInt,
            location: SimpleLocation,
            x_rot: u8,
            y_rot: u8,
            data: VarInt,
            xa: u16,
            ya: u16,
            za: u16
        },

        struct AddExperienceOrb {
            entity_id: VarInt,
            location: SimpleLocation,
            value: u16
        },

        struct AddPlayer {
            entity_id: VarInt,
            player_id: Uuid,
            location: SimpleLocation,
            y_rot: u8,
            x_rot: u8
        },

        struct Animate {
            id: VarInt,
            action: u8
        },

        struct AwardStats {
            stat_entries: Vec<StatsEntry>
        },

        struct BlockChangedAck {
            sequence_id: VarInt
        },

        struct BlockDestruction {
            id: VarInt,
            pos: BlockPos,
            progress: u8
        },

        struct BlockEntityData {
            pos: BlockPos,
            block_entity_type: VarInt,
            tag: Option<CompoundTag>
        },

        struct BlockEvent {
            pos: BlockPos,
            b0: u8,
            b1: u8,
            block: VarInt
        },

        struct BlockUpdate {
            pos: BlockPos,
            state: VarInt
        },

        struct BossEvent {
            id: Uuid,
            operation: BossEventOperationType
        },

        struct ChangeDifficulty {
            difficulty: Difficulty,
            locked: bool
        },

        struct ClearTitles {
            reset_times: bool
        },

        struct CommandSuggestions {
            // todo
        },

        struct Commands {
            // todo
        },

        struct ContainerClose {
            container_id: u8
        },

        struct ContainerSetContent {
            container_id: u8,
            state_id: VarInt,
            items: Vec<Maybe<ItemStack>>,
            carried_item: Maybe<ItemStack>
        },

        struct ContainerSetData {
            container_id: u8,
            id: u16,
            value: u16
        },

        struct ContainerSetSlot {
            container_id: u8,
            state_id: VarInt,
            slot: u16,
            item: Maybe<ItemStack>
        },

        struct Cooldown {
            item_id: VarInt,
            duration: VarInt
        },

        struct CustomChatCompletions {
            // todo
        },

        struct CustomPayload {
            identifier: String,
            data: ByteDrain
        },

        struct DeleteChat {
            // todo
        },

        struct Disconnect {
            reason: JsonDelegate<Chat>
        },

        struct DisguisedChat {
            // todo
        },

        struct EntityEvent {
            entity_id: i32,
            event_id: u8
        },

        struct Explode {
            location: SimpleLocation,
            power: f32,
            offsets: Vec<[u8; 3]>,
            knockback_offsets: [f32; 3]
        },

        struct ForgetLevelChunk {
            x: i32,
            z: i32
        },

        struct GameEvent {
            event: u8,
            param: f32
        },

        struct HorseScreenOpen {
            container_id: u8,
            size: VarInt,
            entity_id: i32
        },

        struct InitializeBorder {
            new_center_x: f64,
            new_center_z: f64,
            old_size: f64,
            new_size: f64,
            lerp_time: VarLong,
            new_absolute_max_size: VarInt,
            warning_blocks: VarInt,
            warning_time: VarInt
        },

        struct KeepAlive {
            id: u64
        },

        struct LevelChunkWithLight {
            // todo
        },

        struct LevelEvent {
            event_type: i32,
            pos: BlockPos,
            data: i32,
            global_event: bool
        },

        struct LevelParticles {
            // todo
        },

        struct LightUpdate {
            // todo
        },

        struct ClientLogin {
            player_id: i32,
            hardcore: bool,
            game_type: u8,
            previous_game_type: u8,
            levels: Vec<String>,
            codec: Option<CompoundTag>,
            dimension_type: String,
            dimension: String,
            seed: u64,
            max_players: VarInt,
            chunk_radius: VarInt,
            simulation_distance: VarInt,
            reduced_debug_info: bool,
            is_debug: bool,
            is_flat: bool,
            last_death_location: Maybe<GlobalPos>
        },

        struct MapItemData {
            // todo
        },

        struct MerchantOffers {
            // todo
        },

        struct MoveEntityPos {
            // todo
        },

        struct MoveEntityPosRot {
            // todo
        },

        struct MoveEntityRot {
            // todo
        },

        struct MoveVehicle {
            location: Location
        },

        struct OpenBook {
            interaction_hand: InteractionHand
        },

        struct OpenScreen {
            container_id: VarInt,
            container_type: VarInt,
            title: JsonDelegate<Chat>
        },

        struct OpenSignEditor {
            pos: BlockPos
        },

        struct Ping {
            ping_id: VarInt
        },

        struct PlaceGhostRecipe {
            container_id: u8,
            recipe: String
        },

        struct PlayerAbilities {
            // todo
        },

        struct PlayerChat {
            // todo
        },

        struct PlayerCombatEnd {
            duration: VarInt,
            killer_id: i32
        },

        struct PlayerCombatEnter {
        },

        struct PlayerCombatKill {
            player_id: VarInt,
            killer_id: i32,
            message: JsonDelegate<Chat>
        },

        struct PlayerInfoRemove {
            // todo
        },

        struct PlayerInfoUpdate {
            // todo
        },

        struct PlayerLookAt {
            // todo
        },

        struct PlayerPosition {
            // todo
        },

        struct Recipe {
            // todo
        },

        struct RemoveEntities {
            entity_ids: Vec<VarInt>
        },

        struct RemoveMobEffect {
            entity_id: VarInt,
            effect_id: VarInt
        },

        struct ResourcePack {
            url: String,
            hash: String,
            required: bool,
            prompt: Maybe<JsonDelegate<Chat>>
        },

        struct Respawn {
            dimension_type: String,
            dimension: String,
            seed: u64,
            game_type: u8,
            previous_game_type: u8,
            is_debug: bool,
            is_flat: bool,
            data_to_keep: u8,
            last_death_location: Maybe<GlobalPos>
        },

        struct RotateHead {
            entity_id: VarInt,
            y_head_rot: u8
        },

        struct SectionBlocksUpdate {
            // todo
        },

        struct SelectAdvancementsTab {
            tab: String
        },

        struct ServerData {
            motd: Maybe<JsonDelegate<Chat>>,
            icon_base_64: Maybe<String>,
            enforces_secure_chat: bool
        },

        struct SetActionBarText {
            text: JsonDelegate<Chat>
        },

        struct SetBorderCenter {
            new_center_x: f64,
            new_center_z: f64
        },

        struct SetBorderLerpSize {
            old_size: f64,
            new_size: f64,
            lerp_time: VarLong
        },

        struct SetBorderSize {
            size: f64
        },

        struct SetBorderWarningDelay {
            warning_time: VarInt
        },

        struct SetBorderWarningDistance {
            warning_blocks: VarInt
        },

        struct SetCamera {
            camera_id: VarInt
        },

        struct SetCarriedItem {
            slot: u8
        },

        struct SetChunkCacheCenter {
            x: i32,
            z: i32
        },

        struct SetChunkCacheRadius {
            radius: VarInt
        },

        struct SetDefaultSpawnPosition {
            pos: BlockPos,
            angle: f32
        },

        struct SetDisplayObjective {
            slot: u8,
            objective: String
        },

        struct SetEntityData {
            // todo
        },

        struct SetEntityLink {
            source_id: i32,
            dest_id: i32
        },

        struct SetEntityMotion {
            entity_id: VarInt,
            xa: u16,
            ya: u16,
            za: u16
        },

        struct SetEquipment {
            // todo
        },

        struct SetExperience {
            experience_progress: f32,
            experience_level: VarInt,
            total_experience: VarInt
        },

        struct SetHealth {
            health: f32,
            food: VarInt,
            saturation: f32
        },

        struct SetObjective {
            // todo
        },

        struct SetPassengers {
            vehicle: VarInt,
            passengers: Vec<VarInt>
        },

        struct SetPlayerTeam {
            // todo
        },

        struct SetScore {
            // todo
        },

        struct SetSimulationDistance {
            simulation_distance: VarInt
        },

        struct SetSubtitleText {
            text: JsonDelegate<Chat>
        },

        struct SetTime {
            game_time: VarLong,
            day_time: VarLong
        },

        struct SetTitleText {
            text: JsonDelegate<Chat>
        },

        struct SetTitlesAnimation {
            fade_in: VarInt,
            stay: VarInt,
            fade_out: VarInt
        },

        struct SoundEntity {
            sound: VarInt,
            source: VarInt,
            id: VarInt,
            volume: f32,
            seed: u64
        },

        struct Sound {
            // todo
        },

        struct StopSound {
            // todo
        },

        struct SystemChat {
            content: JsonDelegate<Chat>,
            overlay: bool
        },

        struct TabList {
            header: JsonDelegate<Chat>,
            footer: JsonDelegate<Chat>
        },

        struct TagQuery {
            transaction_id: VarInt,
            tag: Option<CompoundTag>
        },

        struct TakeItemEntity {
            item_id: VarInt,
            player_id: VarInt,
            amount: VarInt
        },

        struct TeleportEntity {
            entity_id: VarInt,
            location: SimpleLocation,
            y_rot: u8,
            x_rot: u8,
            on_ground: bool
        },

        struct UpdateAdvancements {
            // todo
        },

        struct UpdateAttributes {
            // todo
        },

        struct UpdateEnabledFeatures {
            features: Vec<String>
        },

        struct UpdateMobEffect {
            entity_id: VarInt,
            effect_id: VarInt,
            amplifier: u8,
            duration: VarInt,
            flags: u8,
            factor_data: Maybe<Option<CompoundTag>>
        },

        struct UpdateRecipes {
            // todo
        },

        struct UpdateTags {
            // todo
        }
    }
}
