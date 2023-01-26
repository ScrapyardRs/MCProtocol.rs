use crate::common::chat::Chat;
use drax::nbt::EnsuredCompoundTag;
use drax::prelude::{
    AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, DraxWriteExt, PacketComponent, Size, Uuid,
};
use drax::transport::packet::option::Maybe;
use drax::transport::packet::primitive::{VarInt, VarLong};
use drax::transport::packet::serde_json::JsonDelegate;
use drax::transport::packet::string::LimitedString;
use drax::transport::packet::vec::{LimitedVec, VecU8};
use drax::{throw_explain, PinnedLivelyResult};
use std::mem::size_of;

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

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
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

impl<C: Send + Sync> PacketComponent<C> for BlockPos {
    type ComponentType = BlockPos;

    fn decode<'a, A: AsyncRead + Unpin + Send + Sync + ?Sized>(
        context: &'a mut C,
        read: &'a mut A,
    ) -> PinnedLivelyResult<'a, Self::ComponentType> {
        Box::pin(async move {
            let value = i64::decode(context, read).await?;
            let x = (value << (64 - Self::X_OFFSET - Self::PACKED_X_LENGTH)
                >> (64 - Self::PACKED_X_LENGTH)) as i32;
            let y = (value << (64 - Self::PACKED_Y_LENGTH) >> (64 - Self::PACKED_Y_LENGTH)) as i32;
            let z = (value << (64 - Self::Z_OFFSET - Self::PACKED_Z_LENGTH)
                >> (64 - Self::PACKED_Z_LENGTH)) as i32;
            Ok(Self { x, y, z })
        })
    }

    fn encode<'a, A: AsyncWrite + Unpin + Send + Sync + ?Sized>(
        component_ref: &'a Self::ComponentType,
        context: &'a mut C,
        write: &'a mut A,
    ) -> PinnedLivelyResult<'a, ()> {
        Box::pin(async move {
            let mut value: i64 = 0;
            value |= (component_ref.x as i64 & Self::PACKED_X_MASK) << Self::X_OFFSET;
            value |= component_ref.y as i64 & Self::PACKED_Y_MASK;
            value |= (component_ref.z as i64 & Self::PACKED_Z_MASK) << Self::Z_OFFSET;
            i64::encode(&value, context, write).await
        })
    }

    fn size(_: &Self::ComponentType, _: &mut C) -> drax::prelude::Result<Size> {
        Ok(Size::Constant(size_of::<u64>()))
    }
}

pub type MessageSignature = [u8; 256];

#[derive(Debug)]
pub enum PackedMessageSignature {
    IdBase(i32),
    Signature(MessageSignature),
}

impl<C: Send + Sync> PacketComponent<C> for PackedMessageSignature {
    type ComponentType = PackedMessageSignature;

    fn decode<'a, A: AsyncRead + Unpin + Send + Sync + ?Sized>(
        context: &'a mut C,
        read: &'a mut A,
    ) -> PinnedLivelyResult<'a, Self::ComponentType> {
        Box::pin(async move {
            let id = VarInt::decode(context, read).await? - 1;
            if id == -1 {
                let mut signature = [0u8; 256];
                read.read_exact(&mut signature).await?;
                Ok(PackedMessageSignature::Signature(signature))
            } else {
                Ok(PackedMessageSignature::IdBase(id))
            }
        })
    }

    fn encode<'a, A: AsyncWrite + Unpin + Send + Sync + ?Sized>(
        component_ref: &'a Self::ComponentType,
        context: &'a mut C,
        write: &'a mut A,
    ) -> PinnedLivelyResult<'a, ()> {
        Box::pin(async move {
            match component_ref {
                PackedMessageSignature::IdBase(id) => {
                    VarInt::encode(&(id + 1), context, write).await
                }
                PackedMessageSignature::Signature(signature) => {
                    VarInt::encode(&0, context, write).await?;
                    write.write_all(signature).await?;
                    Ok(())
                }
            }
        })
    }

    fn size(input: &Self::ComponentType, context: &mut C) -> drax::prelude::Result<Size> {
        match input {
            PackedMessageSignature::IdBase(id) => VarInt::size(&id, context),
            PackedMessageSignature::Signature(_) => Ok(Size::Constant(257)),
        }
    }
}

macro_rules! min_max_arg_type {
    ($(
        $enum_name:ident, $arg_ty:ty,
    )*) => {
        registry! {
            components {
                $(
                enum $enum_name<key: u8> {
                    @match {key & 3},
                    NoMinMax {},
                    Min {
                        min: $arg_ty
                    },
                    Max {
                        max: $arg_ty
                    },
                    MinMax {
                        min: $arg_ty,
                        max: $arg_ty
                    }
                }
                ),*
            }
        }
    };
}

min_max_arg_type! {
    LongArgumentInfo, i64,
    IntegerArgumentInfo, i32,
    DoubleArgumentInfo, f64,
    FloatArgumentInfo, f32,
}

registry! {
    components {
        struct BlockHitResult {
            block_pos: BlockPos,
            direction: Direction,
            xo: f32,
            yo: f32,
            zo: f32,
            inside: bool
        },

        enum Direction<key: VarInt> {
            Down {},
            Up {},
            North {},
            South {},
            West {},
            East {}
        },

        struct PackedLastSeenMessages {
            messages: LimitedVec<PackedMessageSignature, 20>
        },

        struct PackedMessageBody {
            content: LimitedString<256>,
            timestamp: u64,
            salt: u64,
            last_seen: PackedLastSeenMessages
        },

        enum StringArgumentType<key: VarInt> {
            SingleWord {},
            QuotablePhrase {},
            GreedyPhrase {}
        },

        enum IntegerArgumentType<key: u8> {
            @match {key & 3},
            NoMinMax {},
            Min {
                min: i32
            },
            Max {
                max: i32
            },
            MinMax {
                min: i32,
                max: i32
            }
        },

        enum LongArgumentType<key: u8> {
            @match {key & 3},
            NoMinMax {},
            Min {
                min: u64
            },
            Max {
                max: u64
            },
            MinMax {
                min: u64,
                max: u64
            }
        },

        enum ArgumentTypeInfo<key: VarInt> {
            Bool {},
            Float {
                argument_type: FloatArgumentInfo
            },
            Double {
                argument_type: DoubleArgumentInfo
            },
            Integer {
                argument_type: IntegerArgumentInfo
            },
            Long {
                argument_type: LongArgumentInfo
            },
            String {
                argument_type: StringArgumentType
            },
            Entity {
                mask: i8
            },
            GameProfile {},
            BlockPos {},
            ColumnPos {},
            Vec3 {},
            Vec2 {},
            BlockState {},
            BlockPredicate {},
            ItemStack {},
            ItemPredicate {},
            Color {},
            Component {},
            Message {},
            NbtCompoundTag {},
            NbtTag {},
            NbtPath {},
            Objective {},
            ObjectiveCriteria {},
            Operation {},
            Particle {},
            Angle {},
            Rotation {},
            ScoreboardSlot {},
            ScoreHolder {
                multiple: bool
            },
            Swizzle {},
            Team {},
            ItemSlot {},
            ResourceLocation {},
            MobEffect {},
            Function {},
            EntityAnchor {},
            IntRange {},
            FloatRange {},
            Dimension {},
            Gamemode {},
            Time {},
            ResourceOrTag {
                resource_location: String
            },
            ResourceOrTagKey {
                resource_location: String
            },
            Resource {
                resource_location: String
            },
            ResourceKey {
                resource_location: String
            },
            TemplateMirror {},
            TemplateRotation {},
            Uuid {}
        },

        struct ChatBind {
            chat_type: VarInt,
            name: JsonDelegate<Chat>,
            target: Maybe<JsonDelegate<Chat>>
        }
    }
}

#[derive(Debug)]
pub struct CommandEntry {
    pub flags: i8,
    pub redirect: i32,
    pub children: Vec<i32>,
}

#[derive(Debug)]
pub enum CommandNode {
    Root {
        entry: CommandEntry,
    },
    Literal {
        entry: CommandEntry,
        literal: String,
    },
    Argument {
        entry: CommandEntry,
        argument_id: String,
        argument_type_info: ArgumentTypeInfo,
        resource_location: Option<String>,
    },
}

impl<C: Send + Sync> PacketComponent<C> for CommandNode {
    type ComponentType = CommandNode;

    fn decode<'a, A: AsyncRead + Unpin + Send + Sync + ?Sized>(
        context: &'a mut C,
        read: &'a mut A,
    ) -> PinnedLivelyResult<'a, Self::ComponentType> {
        Box::pin(async move {
            let flags = i8::decode(context, read).await?;
            let children = Vec::<VarInt>::decode(context, read).await?;
            let redirect = if flags & 8 != 0 {
                VarInt::decode(context, read).await?
            } else {
                0
            };
            let entry = CommandEntry {
                flags,
                redirect,
                children,
            };
            match flags & 3 {
                0 => Ok(CommandNode::Root { entry }),
                1 => Ok(CommandNode::Literal {
                    entry,
                    literal: String::decode(context, read).await?,
                }),
                2 => {
                    let argument_id = String::decode(context, read).await?;
                    let argument_type_info = ArgumentTypeInfo::decode(context, read).await?;
                    let resource_location = if flags & 0x10 != 0 {
                        Some(String::decode(context, read).await?)
                    } else {
                        None
                    };
                    Ok(CommandNode::Argument {
                        entry,
                        argument_id,
                        argument_type_info,
                        resource_location,
                    })
                }
                _ => throw_explain!("Invalid command node type 3"),
            }
        })
    }

    fn encode<'a, A: AsyncWrite + Unpin + Send + Sync + ?Sized>(
        component_ref: &'a Self::ComponentType,
        context: &'a mut C,
        write: &'a mut A,
    ) -> PinnedLivelyResult<'a, ()> {
        Box::pin(async move {
            let entry = match component_ref {
                CommandNode::Root { entry } => entry,
                CommandNode::Literal { entry, .. } => entry,
                CommandNode::Argument { entry, .. } => entry,
            };
            i8::encode(&entry.flags, context, write).await?;
            Vec::<VarInt>::encode(&entry.children, context, write).await?;
            if entry.flags & 8 != 0 {
                write.write_var_int(entry.redirect).await?;
            }
            match component_ref {
                CommandNode::Root { .. } => {}
                CommandNode::Literal { literal, .. } => {
                    String::encode(literal, context, write).await?;
                }
                CommandNode::Argument {
                    argument_id,
                    argument_type_info,
                    resource_location,
                    ..
                } => {
                    String::encode(argument_id, context, write).await?;
                    ArgumentTypeInfo::encode(argument_type_info, context, write).await?;
                    if let Some(location) = resource_location.as_ref() {
                        String::encode(&location, context, write).await?;
                    }
                }
            }
            Ok(())
        })
    }

    fn size(input: &Self::ComponentType, context: &mut C) -> drax::prelude::Result<Size> {
        let entry = match input {
            CommandNode::Root { entry } => entry,
            CommandNode::Literal { entry, .. } => entry,
            CommandNode::Argument { entry, .. } => entry,
        };
        let mut size = i8::size(&entry.flags, context)?;
        size = size + Vec::<VarInt>::size(&entry.children, context)?;
        if entry.flags & 8 != 0 {
            size = size + VarInt::size(&entry.redirect, context)?;
        }
        match input {
            CommandNode::Root { .. } => {}
            CommandNode::Literal { literal, .. } => {
                size = size + String::size(literal, context)?;
            }
            CommandNode::Argument {
                argument_id,
                argument_type_info,
                resource_location,
                ..
            } => {
                size = size + String::size(argument_id, context)?;
                size = size + ArgumentTypeInfo::size(argument_type_info, context)?;
                if let Some(location) = resource_location.as_ref() {
                    size = size + String::size(&location, context)?;
                }
            }
        }
        Ok(size)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct SectionPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl<C: Send + Sync> PacketComponent<C> for SectionPos {
    type ComponentType = SectionPos;

    fn decode<'a, A: AsyncRead + Unpin + Send + Sync + ?Sized>(
        context: &'a mut C,
        read: &'a mut A,
    ) -> PinnedLivelyResult<'a, Self::ComponentType> {
        Box::pin(async move {
            let v = u64::decode(context, read).await?;
            Ok(SectionPos {
                x: (v >> 42) as i32,
                y: (v << 44 >> 44) as i32,
                z: (v << 22 >> 42) as i32,
            })
        })
    }

    fn encode<'a, A: AsyncWrite + Unpin + Send + Sync + ?Sized>(
        component_ref: &'a Self::ComponentType,
        context: &'a mut C,
        write: &'a mut A,
    ) -> PinnedLivelyResult<'a, ()> {
        Box::pin(async move {
            let v = ((component_ref.x as u64 & 4194303) << 42)
                | (component_ref.y as u64 & 1048575)
                | ((component_ref.z as u64 & 4194303) << 20);
            u64::encode(&v, context, write).await
        })
    }

    fn size(_: &Self::ComponentType, _: &mut C) -> drax::prelude::Result<Size> {
        Ok(Size::Constant(size_of::<u64>()))
    }
}

#[derive(Copy, Clone, Debug)]
pub struct BlockUpdate {
    pub block_id: i32,
    pub block_pos: BlockPos,
}

impl BlockUpdate {
    fn pack_pos(&self) -> u16 {
        let pos = self.block_pos;
        (((pos.x & 15) << 8) | ((pos.z & 15) << 4) | (pos.y & 15)) as u16
    }

    fn unpack_pos(packed: u16) -> BlockPos {
        BlockPos {
            x: ((packed >> 8) & 15) as i32,
            y: (packed & 15) as i32,
            z: ((packed >> 4) & 15) as i32,
        }
    }
}

impl<C: Send + Sync> PacketComponent<C> for BlockUpdate {
    type ComponentType = BlockUpdate;

    fn decode<'a, A: AsyncRead + Unpin + Send + Sync + ?Sized>(
        context: &'a mut C,
        read: &'a mut A,
    ) -> PinnedLivelyResult<'a, Self::ComponentType> {
        Box::pin(async move {
            let packed = VarLong::decode(context, read).await?;
            let block_id = (packed >> 12) as i32;
            let packed_section = (packed & 4095) as u32 as u16;
            let block_pos = BlockUpdate::unpack_pos(packed_section);
            Ok(BlockUpdate {
                block_id,
                block_pos,
            })
        })
    }

    fn encode<'a, A: AsyncWrite + Unpin + Send + Sync + ?Sized>(
        component_ref: &'a Self::ComponentType,
        context: &'a mut C,
        write: &'a mut A,
    ) -> PinnedLivelyResult<'a, ()> {
        Box::pin(async move {
            let packed_section = component_ref.pack_pos();
            let packed = ((component_ref.block_id as u64) << 12) | (packed_section as u64);
            VarLong::encode(&(packed as i64), context, write).await
        })
    }

    fn size(input: &Self::ComponentType, context: &mut C) -> drax::prelude::Result<Size> {
        let packed_section = input.pack_pos();
        let packed = ((input.block_id as u64) << 12) | (packed_section as u64);
        VarLong::size(&(packed as i64), context)
    }
}

registry! {
    components {
        #[derive(PartialEq, Eq, Clone, Copy, Hash)]
        enum RecipeBookType<key: VarInt> {
            Crafting {},
            Furnace {},
            BlastFurnace {},
            Smoker {}
        },

        #[derive(Clone)]
        struct ProfilePublicKey {
            expiry: u64,
            encoded_key: VecU8,
            key_sig: VecU8
        },

        #[derive(Clone)]
        struct RemoteChatSession {
            session_id: Uuid,
            key: ProfilePublicKey
        },

        #[derive(Clone, Copy, PartialEq)]
        struct SimpleLocation {
            x: f64,
            y: f64,
            z: f64
        },

        #[derive(Clone, Copy, PartialEq)]
        struct Location {
            inner_loc: SimpleLocation,
            yaw: f32,
            pitch: f32
        },

        #[derive(Clone, PartialEq)]
        struct ItemStack {
            item_id: VarInt,
            count: u8,
            tag: EnsuredCompoundTag<0>
        },

        #[derive(Clone)]
        struct GlobalPos {
            dimension: String,
            pos: BlockPos
        },

        #[derive(Clone, Copy)]
        enum InteractionHand<key: VarInt> {
            MainHand {},
            OffHand {}
        },

        #[derive(Clone, Copy)]
        enum Difficulty<key: u8> {
            Peaceful {},
            Easy {},
            Normal {},
            Hard {}
        }
    }
}

#[derive(Debug)]
pub enum MapColorPatch {
    Present {
        width: u8,
        height: u8,
        start_x: u8,
        start_y: u8,
        map_colors: Vec<u8>,
    },
    Absent,
}

impl<C: Send + Sync> PacketComponent<C> for MapColorPatch {
    type ComponentType = MapColorPatch;

    fn decode<'a, A: AsyncRead + Unpin + Send + Sync + ?Sized>(
        context: &'a mut C,
        read: &'a mut A,
    ) -> PinnedLivelyResult<'a, Self::ComponentType> {
        Box::pin(async move {
            let b1 = u8::decode(context, read).await?;
            if b1 != 0 {
                let width = b1;
                let height = u8::decode(context, read).await?;
                let start_x = u8::decode(context, read).await?;
                let start_y = u8::decode(context, read).await?;
                let map_colors = VecU8::decode(context, read).await?;
                Ok(MapColorPatch::Present {
                    width,
                    height,
                    start_x,
                    start_y,
                    map_colors,
                })
            } else {
                Ok(MapColorPatch::Absent)
            }
        })
    }

    fn encode<'a, A: AsyncWrite + Unpin + Send + Sync + ?Sized>(
        component_ref: &'a Self::ComponentType,
        context: &'a mut C,
        write: &'a mut A,
    ) -> PinnedLivelyResult<'a, ()> {
        Box::pin(async move {
            match component_ref {
                MapColorPatch::Present {
                    width,
                    height,
                    start_x,
                    start_y,
                    map_colors,
                } => {
                    u8::encode(width, context, write).await?;
                    u8::encode(height, context, write).await?;
                    u8::encode(start_x, context, write).await?;
                    u8::encode(start_y, context, write).await?;
                    VecU8::encode(map_colors, context, write).await?;
                }
                MapColorPatch::Absent => {
                    u8::encode(&0, context, write).await?;
                }
            }
            Ok(())
        })
    }

    fn size(input: &Self::ComponentType, context: &mut C) -> drax::prelude::Result<Size> {
        match input {
            MapColorPatch::Present {
                width,
                height,
                start_x,
                start_y,
                map_colors,
            } => {
                let mut size = u8::size(width, context)?;
                size = size + u8::size(height, context)?;
                size = size + u8::size(start_x, context)?;
                size = size + u8::size(start_y, context)?;
                size = size + VecU8::size(map_colors, context)?;
                Ok(size)
            }
            MapColorPatch::Absent => Ok(Size::Constant(1)),
        }
    }
}
