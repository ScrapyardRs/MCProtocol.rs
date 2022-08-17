use crate::shared_types::play::*;
use crate::shared_types::sound::*;
use crate::shared_types::{GameProfile, MCIdentifiedKey, Nothing, Signature};
use mc_chat::Chat;
use mc_commands::Command;
use mc_serializer::primitive::{VarInt, VarLong};
use mc_serializer::serde::Contextual;
use std::collections::HashMap;

crate::easy_mappings! {
    AddEntity {
        def 0x0;
        id: VarInt,
        uuid: uuid::Uuid,
        entity_type: VarInt,
        x: f64,
        y: f64,
        z: f64,
        x_rot: u8,
        y_rot: u8,
        data: VarInt,
        xa: u16,
        ya: u16,
        za: u16,
    }
    AddExperienceOrb {
        def 0x1;
        id: VarInt,
        x: f64,
        y: f64,
        z: f64,
        value: u16,
    }
    AddPlayer {
        def 0x2;
        entity_id: VarInt,
        player_id: uuid::Uuid,
        x: f64,
        y: f64,
        z: f64,
        y_rot: u8,
        x_rot: u8,
    }
    Animate {
        def 0x3;
        id: VarInt,
        action: AnimateAction,
    }
    AwardStats {
        def 0x4;
        stats: HashMap<AwardStatsType, VarInt>,
    }
    BlockChangeAck {
        def 0x5;
        sequence: VarInt,
    }
    BlockDestruction {
        def 0x6;
        id: VarInt,
        pos: BlockPos,
        progress: u8,
    }
    BlockEntityData {
        def 0x7;
        pos: BlockPos,
        block_entity_type: VarInt,
        #[nbt]
        tag: nbt::Blob,
    }
    BlockEvent {
        def 0x8;
        pos: BlockPos,
        b0: u8,
        b1: u8,
        block_id: VarInt,
    }
    BlockUpdate {
        def 0x9;
        pos: BlockPos,
        block_state: VarInt,
    }
    BossEvent {
        def 0xA;
        id: uuid::Uuid,
        operation: BossEventOperation,
    }
    ChangeDifficulty {
        def 0xB;
        difficulty: Difficulty,
        locked: bool,
    }
    ChatPreview {
        def 0xC;
        query_id: i32,
        preview_data: MaybeComponent,
    }
    ClearTitles {
        def 0xD;
        reset_times: bool,
    }
    CommandSuggestions {
        def 0xE;
        id: VarInt,
        start: VarInt,
        end: VarInt,
        suggestions: (VarInt, Vec<CommandSuggestion>),
    }
    DeclareCommands {
        def 0xF;
        commands: (VarInt, Vec<Command>),
        root_index: VarInt,
    }
    ContainerClose {
        def 0x10;
        container_id: u8,
    }
    ContainerSetContent {
        def 0x11;
        container_id: u8,
        state_id: VarInt,
        items: (VarInt, Vec<ItemStackContainer>),
        carried_item: ItemStackContainer,
    }
    ContainerSetData {
        def 0x12;
        container_id: u8,
        id: u16,
        value: u16,
    }
    ContainerSetSlot {
        def 0x13;
        container_id: u8,
        state_id: VarInt,
        slot: u16,
        item: ItemStackContainer,
    }
    Cooldown {
        def 0x14;
        item: VarInt,
        duration: VarInt,
    }
    CustomChatCompletions {
        def 0x15;
        action: CustomChatCompletionsAction,
        entries: (VarInt, Vec<String>),
    }
    PluginMessage {
        def 0x16;
        identifier: ResourceLocation,
        data: Vec<u8>,
    }
    CustomSound {
        def 0x17;
        name: ResourceLocation,
        source: SoundSource,
        x: i32,
        y: i32,
        z: i32,
        volume: f32,
        pitch: f32,
        seed: u64,
    }
    DeleteChat {
        def 0x18;
        signature: (VarInt, Vec<u8>),
    }
    Disconnect {
        def 0x19;
        #[json(32767)]
        reason: Chat,
    }
    EntityEvent {
        def 0x1A;
        entity_id: i32,
        event_id: u8,
    }
    Explode {
        def 0x1B;
        x: f32,
        y: f32,
        z: f32,
        power: f32,
        to_blow: (VarInt, Vec<ExplodeOffset>),
        knockback_x: f32,
        knockback_y: f32,
        knockback_z: f32,
    }
    ForgetLevelChunk {
        def 0x1C;
        x: i32,
        z: i32,
    }
    GameEvent {
        def 0x1D;
        event_type: GameEventType,
        param: f32,
    }
    HorseScreenOpen {
        def 0x1E;
        container_id: u8,
        size: VarInt,
        entity_id: VarInt,
    }
    WorldBorder {
        def 0x1F;
        new_center_x: f64,
        new_center_z: f64,
        old_size: f64,
        new_size: f64,
        lerp_time: VarLong,
        new_absolute_max_size: VarInt,
        warning_blocks: VarInt,
        warning_time: VarInt,
    }
    KeepAlive {
        def 0x20;
        id: u64,
    }
    LevelChunkWithLight {
        def 0x21;
        chunk_data: LevelChunkData,
        data: LightUpdateData,
    }
    LevelEvent {
        def 0x22;
        event_type: i32,
        pos: BlockPos,
        data: i32,
        global_event: bool,
    }
    LevelParticles {
        def 0x23;
        particle: Particle,
    }
    LightUpdate {
        def 0x24;
        x: VarInt,
        z: VarInt,
        data: LightUpdateData,
    }
    JoinGame {
        def 0x25;
        player_id: i32,
        hardcore: bool,
        game_type: GameType,
        previous_game_type: GameType,
        levels: (VarInt, Vec<ResourceLocation>),
        #[nbt]
        codec: mc_level::codec::Codec,
        dimension_type: ResourceLocation,
        dimension: ResourceLocation,
        seed: u64,
        max_players: VarInt,
        chunk_radius: VarInt,
        simulation_distance: VarInt,
        reduced_debug_info: bool,
        show_death_screen: bool,
        is_debug: bool,
        is_flat: bool,
        last_death_location: (bool, Option<BlockPos>),
    }
    MapItemData {
        def 0x26;
        map_id: VarInt,
        scale: u8,
        locked: bool,
        decorations: (bool, Option<(VarInt, Vec<MapDecoration>)>),
        patch: MapPatch,
    }
    MerchantOffers {
        def 0x27;
        container_id: VarInt,
        offers: (VarInt, Vec<MerchantOffer>),
        villager_level: VarInt,
        villager_xp: VarInt,
        show_progress: bool,
        can_restock: bool,
    }
    MoveEntityPos {
        def 0x28;
        entity_id: VarInt,
        x: u16,
        y: u16,
        z: u16,
        on_ground: bool,
    }
    MoveEntityPosRot {
        def 0x29;
        entity_id: VarInt,
        x: u16,
        y: u16,
        z: u16,
        y_rot: u8,
        x_rot: u8,
        on_ground: bool,
    }
    MoveEntityRot {
        def 0x2A;
        entity_id: VarInt,
        y_rot: u8,
        x_rot: u8,
        on_ground: bool,
    }
    MoveVehicle {
        def 0x2B;
        x: f64,
        y: f64,
        z: f64,
        y_rot: f32,
        x_rot: f32,
    }
    OpenBook {
        def 0x2C;
        interaction_hand: InteractionHand,
    }
    OpenScreen {
        def 0x2D;
        container_id: VarInt,
        menu_type: MenuType,
        #[json(32767)]
        title: Chat,
    }
    OpenSignEditor {
        def 0x2E;
        pos: BlockPos,
    }
    Ping {
        def 0x2F;
        id: i32,
    }
    PlaceGhostRecipe {
        def 0x30;
        container_id: u8,
        recipe: ResourceLocation,
    }
    PlayerAbilities {
        def 0x31;
        player_abilities_bits: PlayerAbilitiesBitMap,
        flying_speed: f32,
        walking_speed: f32,
    }
    PlayerChatHeader {
        def 0x32;
        header: ChatHeader,
        header_signature: Signature,
        body_digest: Signature,
    }
    PlayerChat {
        def 0x33;
        message: ChatMessage,
        chat_type: ChatType,
    }
    PlayerCombatEnd {
        def 0x34;
        duration: VarInt,
        killer_id: i32,
    }
    PlayerCombatEnter {
        def 0x35;
        nothing: Nothing,
    }
    PlayerCombatKill {
        def 0x36;
        player_id: VarInt,
        killer_id: i32,
        #[json(32767)]
        message: Chat,
    }
    PlayerInfo {
        def 0x37;
        entry: PlayerInfoEntry,
    }
    PlayerLookAt {
        def 0x38;
        from_anchor: PositionAnchor,
        x: f64,
        y: f64,
        z: f64,
        to_entity: (bool, Option<PlayerLookAtEntity>),
    }
    PlayerPosition {
        def 0x39;
        x: f64,
        y: f64,
        z: f64,
        y_rot: f32,
        x_rot: f32,
        relative_arguments: RelativeArgument,
        id: VarInt,
        dismount_vehicle: bool,
    }
    DeclareRecipes {
        def 0x3A;
        data: DeclareRecipesData,
    }
    RemoveEntities {
        def 0x3B;
        entities_to_remove: (VarInt, Vec<VarInt>),
    }
    RemoveMobEffects {
        def 0x3C;
        entity_id: VarInt,
        mob_effect: MobEffect,
    }
    ResourcePack {
        def 0x3D;
        url: String,
        hash: ResourcePackHash,
        required: bool,
        prompt: MaybeComponent,
    }
    Respawn {
        def 0x3E;
        dimension_type: ResourceLocation,
        dimension: ResourceLocation,
        seed: u64,
        game_type: GameType,
        previous_game_type: GameType,
        is_debug: bool,
        is_flat: bool,
        keep_all_player_data: bool,
        last_death_location: (bool, Option<GlobalPos>),
    }
    RotateHead {
        def 0x3F;
        entity_id: VarInt,
        y_head_rot: u8,
    }
    SectionBlocksUpdate {
        def 0x40;
        pos: SectionPos,
        suppress_light_updates: bool,
        positions: (VarInt, Vec<VarLong>),
    }
    SelectAdvancementsTab {
        def 0x41;
        tab: (bool, Option<ResourceLocation>),
    }
    ServerData {
        def 0x42;
        info: ServerDataInfo,
    }
    SetActionBarText {
        def 0x43;
        #[json(32767)]
        text: Chat,
    }
    SetBorderCenter {
        def 0x44;
        new_center_x: f64,
        new_center_z: f64,
    }
    SetBorderLerpSize {
        def 0x45;
        new_center_x: f64,
        new_center_z: f64,
        lerp_time: VarLong,
    }
    SetBorderSize {
        def 0x46;
        size: f64,
    }
    SetBorderWarningDelay {
        def 0x47;
        warning_delay: VarInt,
    }
    SetBorderWarningDistance {
        def 0x48;
        warning_blocks: VarInt,
    }
    SetCamera {
        def 0x49;
        camera_id: VarInt,
    }
    SetCarriedItem {
        def 0x4A;
        slot: u8,
    }
    SetChunkCacheCenter {
        def 0x4B;
        x: VarInt,
        z: VarInt,
    }
    SetChunkCacheRadius {
        def 0x4C;
        radius: VarInt,
    }
    SetDefaultSpawnPosition {
        def 0x4D;
        pos: BlockPos,
        angle: f32,
    }
    SetDisplayChatPreview {
        def 0x4E;
        enabled: bool,
    }
    SetDisplayObjective {
        def 0x4F;
        slot: u8,
        objective_name: String,
    }
    SetEntityData {
        def 0x50;
        entity_id: VarInt,
        entity_data_info: EntityDataInfo,
    }
    SetEntityLink {
        def 0x51;
        source_id: i32,
        dest_id: i32,
    }
    SetEntityMotion {
        def 0x52;
        entity_id: VarInt,
        xa: u16,
        ya: u16,
        za: u16,
    }
    SetEquipment {
        def 0x53;
        entity_id: VarInt,
        items: SetEquipmentSlotItems,
    }
    SetExperience {
        def 0x54;
        progress: f32,
        level: VarInt,
        total: VarInt,
    }
    SetHealth {
        def 0x55;
        health: f32,
        foot: VarInt,
        saturation: f32,
    }
    SetObjective {
        def 0x56;
        objective_name: String,
        method: SetObjectiveMethod,
    }
    SetPassengers {
        def 0x57;
        vehicle: VarInt,
        passengers: (VarInt, Vec<VarInt>),
    }
    SetPlayerTeam {
        def 0x58;
        name: String,
        method: SetPlayerTeamMethod,
    }
    SetScore {
        def 0x59;
        owner: String,
        method: SetScoreMethod,
    }
    SetSimulationDistance {
        def 0x5A;
        distance: VarInt,
    }
    SetSubtitle {
        def 0x5B;
        #[json(32767)]
        text: Chat,
    }
    SetTime {
        def 0x5C;
        game_time: u64,
        day_time: u64,
    }
    SetTitleText {
        def 0x5D;
        #[json(32767)]
        text: Chat,
    }
    SetTitlesAnimation {
        def 0x5E;
        fade_in: i32,
        stay: i32,
        fade_out: i32,
    }
    SoundEntity {
        def 0x5F;
        sound_event: SoundEvent,
        source: SoundSource,
        id: VarInt,
        volume: f32,
        pitch: f32,
        seed: u64,
    }
    Sound {
        def 0x60;
        sound_event: SoundEvent,
        source: SoundSource,
        x: i32,
        y: i32,
        z: i32,
        volume: f32,
        pitch: f32,
        seed: u64,
    }
    StopSound {
        def 0x61;
        method: StopSoundMethod,
    }
    SystemChat {
        def 0x62;
        #[json(32767)]
        content: Chat,
        overlay: bool,
    }
    TabList {
        def 0x63;
        #[json(32767)]
        header: Chat,
        #[json(32767)]
        footer: Chat,
    }
    TagQuery {
        def 0x64;
        transaction_id: VarInt,
        #[nbt]
        nbt: nbt::Blob,
    }
    TakeItemEntity {
        def 0x65;
        item_id: VarInt,
        player_id: VarInt,
        amount: VarInt,
    }
    TeleportEntity {
        def 0x66;
        entity_id: VarInt,
        x: f64,
        y: f64,
        z: f64,
        y_rot: u8,
        x_rot: u8,
        on_ground: bool,
    }
    UpdateAdvancements {
        def 0x67;
        reset: bool,
        added: HashMap<ResourceLocation, AdvancementAddEntry>,
        remove: (VarInt, Vec<ResourceLocation>),
        progress: HashMap<ResourceLocation, AdvancementProgressEntry>,
    }
    UpdateAttributes {
        def 0x68;
        entity_id: VarInt,
        attributes: (VarInt, Vec<Attribute>),
    }
    UpdateMobEffect {
        def 0x69;
        entity_id: VarInt,
        mob_effect: MobEffect,
        effect_emplifier: i8,
        effect_duration_ticks: VarInt,
        flags: u8,
        factor_data: MaybeNbt,
    }
    UpdateRecipes {
        def 0x6A;
        recipes: (VarInt, Vec<Recipe>),
    }
    UpdateTags {
        def 0x6B;
        tags: HashMap<
            ResourceLocation,
            HashMap<ResourceLocation, (VarInt, Vec<VarInt>)>
        >,
    }
}
