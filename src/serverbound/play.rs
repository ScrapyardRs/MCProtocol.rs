use crate::clientbound::play::Difficulty;
use crate::common::play::{BlockPos, InteractionHand, ItemStack, Location};
use drax::prelude::Uuid;
use drax::transport::packet::option::Maybe;
use drax::transport::packet::primitive::VarInt;
use drax::transport::packet::string::LimitedString;
use drax::transport::packet::vec::ByteDrain;

registry! {
    components {
        enum ClientCommandAction<key: VarInt> {
            PerformRespawn {},
            RequestStats {}
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
            // todo
        },
        struct Chat {
            // todo
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
            // todo
        },
        struct ContainerClose {
            container_id: u8
        },
        struct CustomPayload {
            channel_identifier: String,
            data: ByteDrain
        },
        struct EditBook {
            // todo
        },
        struct EntityTagQuery {
            transaction_id: VarInt,
            entity_id: VarInt
        },
        struct Interact {
            // todo
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
            // todo
        },
        struct MovePlayerPosRot {
            // todo
        },
        struct MovePlayerRot {
            // todo
        },
        struct MovePlayerStatusOnly {
            // todo
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
            // todo
        },
        struct PlayerAction {
            // todo
        },
        struct PlayerCommand {
            // todo
        },
        struct PlayerInput {
            // todo
        },
        struct Pong {
            transaction_id: VarInt
        },
        struct ChatSessionUpdate {
            // todo
        },
        struct RecipeBookChangeSettings {
            // todo
        },
        struct RecipeBookSeenRecipe {
            recipe_identifier: String
        },
        struct RenameItem {
            name: String
        },
        struct ResourcePack {
            // todo
        },
        struct SeenAdvancements {
            // todo
        },
        struct SelectTrade {
            item: VarInt
        },
        struct SetBeacon {
            // todo
        },
        struct SetCarriedItem {
            slot: u16
        },
        struct SetCommandBlock {
            // todo
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
            // todo
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
            // todo
        },
        struct UseItem {
            hand: InteractionHand,
            sequence: VarInt
        }
    }
}
