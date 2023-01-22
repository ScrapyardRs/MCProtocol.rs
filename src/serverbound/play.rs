use crate::common::bit_set::FixedBitSet;
use crate::common::play::{
    BlockHitResult, BlockPos, Difficulty, InteractionHand, ItemStack, Location, MessageSignature,
};
use crate::common::play::{RecipeBookType, RemoteChatSession};
use drax::prelude::Uuid;
use drax::transport::packet::option::Maybe;
use drax::transport::packet::primitive::{VarInt, VarLong};
use drax::transport::packet::string::LimitedString;
use drax::transport::packet::vec::{ByteDrain, LimitedVec};

registry! {
    components {
        #[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
        enum ClientCommandAction<key: VarInt> {
            PerformRespawn {},
            RequestStats {}
        },


        #[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
        enum PlayerAbilitiesMask<key: u8> {
            NonFlying {},
            Flying {
                @key(2);
            }
        },

        struct ArgumentSignature {
            name: LimitedString<16>,
            signature: MessageSignature
        },

        struct ContainerSlot {
            index: u16,
            item: Maybe<ItemStack>
        },

        #[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
        enum ClickType<key: VarInt> {
            Pickup {},
            QuickMove {},
            Swap {},
            Clone {},
            Throw {},
            QuickCraft {},
            PickupAll {}
        },

        enum InteractAction<key: VarInt> {
            Generic {
                hand: InteractionHand
            },
            Attack {
            },
            InteractAt {
                xa: f32,
                ya: f32,
                za: f32,
                hand: InteractionHand
            }
        },

        #[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
        enum PlayerActionType<key: VarInt> {
            StartDestroyBlock {},
            AbortDestroyBlock {},
            StopDestroyBlock {},
            DropAllItems {},
            DropItem {},
            ReleaseUseItem {},
            SwapItemWithOffhand {}
        },

        #[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
        enum PlayerCommandType<key: VarInt> {
            PressShiftKey {},
            ReleaseShiftKey {},
            StopSleeping {},
            StartSprinting {},
            StopSprinting {},
            StartRidingJump {},
            StopRidingJump {},
            OpenInventory {},
            StartFallFlying {}
        },

        #[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
        enum InputFlags<key: u8> {
            Neiether {},
            Jumping {},
            ShiftKeyDown {},
            ShiftKeyDownAndJumping {}
        },

        #[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
        enum ResourcePackAction<key: VarInt> {
            SuccessfullyLoaded {},
            Declined {},
            FailedDownload {},
            Accepted {}
        },

        #[derive(Clone, PartialEq)]
        enum SeenAdvancementsAction<key: VarInt> {
            OpenedTab {
                tab: String
            },
            ClosedScreen {}
        },

        #[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
        enum SetCommandBlockMode<key: VarInt> {
            Sequence {},
            Auto {},
            Redstone {}
        },

        #[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
        enum StructureBlockUpdateType<key: VarInt> {
            UpdateData {},
            SaveArea {},
            LoadArea {},
            ScanArea {}
        },

        #[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
        enum StructureMode<key: VarInt> {
            Save {},
            Load {},
            Corner {},
            Data {}
        },

        #[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
        enum StructureMirror<key: VarInt> {
            None {},
            LeftRight {},
            FrontBack {}
        },

        #[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
        enum Rotation<key: VarInt> {
            None {},
            Clockwise90 {},
            Clockwise180 {},
            CounterClockwise90 {}
        }
    }

    registry ServerboundPlayRegistry {
        struct AcceptTeleportation {
            teleportation_id: VarInt
        },

        struct BlockEntityTagQuery {
            transaction_id: VarInt,
            location: BlockPos
        },

        struct ChangeDifficulty {
            difficulty: Difficulty
        },

        struct ChatAck {
            offset: VarInt
        },

        struct ChatCommand {
            command: LimitedString<256>,
            timestamp: u64,
            salt: u64,
            signatures: Vec<ArgumentSignature>,
            last_seen_offset: VarInt,
            last_seen_set: FixedBitSet<20>
        },

        struct Chat {
            message: LimitedString<256>,
            timestamp: u64,
            salt: u64,
            signature: Maybe<MessageSignature>,
            last_seen_offset: VarInt,
            last_seen_set: FixedBitSet<20>
        },

        struct ClientCommand {
            action: ClientCommandAction
        },

        struct ClientInformation {
            locale: String,
            view_distance: u8,
            chat_visibility: VarInt,
            chat_colors: bool,
            displayed_skin_parts: u8,
            main_hand: VarInt,
            text_filtering: bool,
            allows_listing: bool
        },

        struct CommandSuggestion {
            transaction_id: VarInt,
            command: LimitedString<32500>
        },

        struct ContainerButtonClick {
            container_id: u8,
            button_id: u8
        },

        struct ContainerClick {
            container_id: u8,
            state_id: VarInt,
            slot: u16,
            button: u8,
            action: ClickType,
            changed_slots: LimitedVec<ContainerSlot, 128>,
            carried_item: Maybe<ItemStack>
        },

        struct ContainerClose {
            container_id: u8
        },

        struct CustomPayload {
            channel_identifier: String,
            data: ByteDrain
        },

        struct EditBook {
            slot: VarInt,
            pages: Vec<LimitedString<8192>>,
            title: Maybe<LimitedString<128>>
        },

        struct EntityTagQuery {
            transaction_id: VarInt,
            entity_id: VarInt
        },

        struct Interact {
            entity_id: VarInt,
            action: InteractAction,
            using_secondary_action: bool
        },

        struct JigsawGenerate {
            location: BlockPos,
            levels: VarInt,
            keep_jigsaws: bool
        },

        struct KeepAlive {
            keep_alive_id: u64
        },

        struct LockDifficulty {
            locked: bool
        },

        struct MovePlayerPos {
            x: f64,
            y: f64,
            z: f64,
            on_ground: bool
        },

        struct MovePlayerPosRot {
            x: f64,
            y: f64,
            z: f64,
            y_rot: f32,
            x_rot: f32,
            on_ground: bool
        },

        struct MovePlayerRot {
            y_rot: f32,
            x_rot: f32,
            on_ground: bool
        },

        struct MovePlayerStatusOnly {
            status: u8
        },

        struct MoveVehicle {
            location: Location
        },

        struct PaddleBoat {
            left_paddle: bool,
            right_paddle: bool
        },

        struct PickItem {
            slot: VarInt
        },

        struct PlaceRecipe {
            container_id: u8,
            recipe_identifier: String,
            shift: bool
        },

        struct PlayerAbilities {
            mask: PlayerAbilitiesMask
        },

        struct PlayerAction {
            action_type: PlayerActionType,
            at_pos: BlockPos,
            direction: u8,
            sequence: VarInt
        },

        struct PlayerCommand {
            id: VarInt,
            action_type: PlayerCommandType,
            data: VarInt
        },

        struct PlayerInput {
            xxa: f32,
            zza: f32,
            flags: InputFlags
        },

        struct Pong {
            transaction_id: i32
        },

        struct ChatSessionUpdate {
            chat_session: RemoteChatSession
        },

        struct RecipeBookChangeSettings {
            book_type: RecipeBookType,
            is_open: bool,
            is_filtering: bool
        },

        struct RecipeBookSeenRecipe {
            recipe_identifier: String
        },

        struct RenameItem {
            name: String
        },

        struct ResourcePack {
            action: ResourcePackAction
        },

        struct SeenAdvancements {
            action: SeenAdvancementsAction
        },

        struct SelectTrade {
            item: VarInt
        },

        struct SetBeacon {
            primary_effect: Maybe<VarInt>,
            secondary_effect: Maybe<VarInt>
        },

        struct SetCarriedItem {
            slot: u16
        },

        struct SetCommandBlock {
            location: BlockPos,
            command: String,
            mode: SetCommandBlockMode,
            flags: u8
        },

        struct SetCommandMinecart {
            entity_id: VarInt,
            command: String,
            track_output: bool
        },

        struct SetCreativeModeSlot {
            slot_num: u16,
            item: Maybe<ItemStack>
        },

        struct SetJigsawBlock {
            location: BlockPos,
            name: String,
            target: String,
            pool: String,
            final_state: String,
            joint: String
        },

        struct SetStructureBlock {
            pos: BlockPos,
            update_type: StructureBlockUpdateType,
            mode: StructureMode,
            name: String,
            off_x: u8,
            off_y: u8,
            off_z: u8,
            size_x: u8,
            size_y: u8,
            size_z: u8,
            mirror: StructureMirror,
            rotation: Rotation,
            data: LimitedString<128>,
            integrity: f32,
            seed: VarLong,
            flags: u8
        },

        struct SignUpdate {
            pos: BlockPos,
            lines: [LimitedString<384>; 4]
        },

        struct Swing {
            hand: InteractionHand
        },

        struct TeleportToEntity {
            uuid: Uuid
        },

        struct UseItemOn {
            hand: InteractionHand,
            result: BlockHitResult,
            sequence: VarInt
        },

        struct UseItem {
            hand: InteractionHand,
            sequence: VarInt
        }
    }
}
