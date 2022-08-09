use std::io::{Read, Write};
use mc_serializer::primitive::Identifier;
use mc_serializer::primitive::VarInt;
use mc_serializer::serde::{Contextual, Deserialize, Error, ProtocolVersion, Serialize, SerializerContext};
use mc_serializer::wrap_struct_context;
use crate::NodeStub::{Literal, Root};

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(VarInt)]
pub enum StringType {
    #[key(VarInt::from(0))]
    SingleWord,
    #[key(VarInt::from(1))]
    QuotablePhrase,
    #[key(VarInt::from(2))]
    GreedyPhrase,
}

#[derive(mc_serializer_derive::SerialBitMap, Debug)]
pub struct MinMaxBitFlag {
    pub min: bool,
    pub max: bool,
}

#[derive(mc_serializer_derive::SerialBitMap, Debug)]
pub struct EntityBitFlag {
    pub single: bool,
    pub players_only: bool,
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(VarInt)]
pub enum Argument {
    #[key(VarInt::from(0))]
    Bool,
    #[key(VarInt::from(1))]
    Float {
        flags: MinMaxBitFlag,
        #[serial_if(__serde_flags.min)]
        min: Option<f32>,
        #[serial_if(__serde_flags.max)]
        max: Option<f32>,
    },
    #[key(VarInt::from(2))]
    Double {
        flags: MinMaxBitFlag,
        #[serial_if(__serde_flags.min)]
        min: Option<f64>,
        #[serial_if(__serde_flags.max)]
        max: Option<f64>,
    },
    #[key(VarInt::from(3))]
    Integer {
        flags: MinMaxBitFlag,
        #[serial_if(__serde_flags.min)]
        min: Option<i32>,
        #[serial_if(__serde_flags.max)]
        max: Option<i32>,
    },
    #[key(VarInt::from(4))]
    Long {
        flags: MinMaxBitFlag,
        #[serial_if(__serde_flags.min)]
        min: Option<i64>,
        #[serial_if(__serde_flags.max)]
        max: Option<i64>,
    },
    #[key(VarInt::from(5))]
    String(StringType),
    #[key(VarInt::from(6))]
    Entity(EntityBitFlag),
    #[key(VarInt::from(7))]
    GameProfile,
    #[key(VarInt::from(8))]
    BlockPos,
    #[key(VarInt::from(9))]
    ColumnPos,
    #[key(VarInt::from(10))]
    Vec3,
    #[key(VarInt::from(11))]
    Vec2,
    #[key(VarInt::from(12))]
    BlockState,
    #[key(VarInt::from(13))]
    BlockPredicate,
    #[key(VarInt::from(14))]
    ItemStack,
    #[key(VarInt::from(15))]
    ItemPredicate,
    #[key(VarInt::from(16))]
    Color,
    #[key(VarInt::from(17))]
    Component,
    #[key(VarInt::from(18))]
    Message,
    #[key(VarInt::from(19))]
    NbtCompoundTag,
    #[key(VarInt::from(20))]
    NbtTag,
    #[key(VarInt::from(21))]
    NbtPath,
    #[key(VarInt::from(22))]
    Objective,
    #[key(VarInt::from(23))]
    ObjectiveCriteria,
    #[key(VarInt::from(24))]
    Operation,
    #[key(VarInt::from(25))]
    Particle,
    #[key(VarInt::from(26))]
    Angle,
    #[key(VarInt::from(27))]
    Rotation,
    #[key(VarInt::from(28))]
    ScoreboardSlot,
    #[key(VarInt::from(29))]
    ScoreHolder(bool),
    #[key(VarInt::from(30))]
    Swizzle,
    #[key(VarInt::from(31))]
    Team,
    #[key(VarInt::from(32))]
    ItemSlot,
    #[key(VarInt::from(33))]
    ResourceLocation,
    #[key(VarInt::from(34))]
    MobEffect,
    #[key(VarInt::from(35))]
    Function,
    #[key(VarInt::from(36))]
    EntityAnchor,
    #[key(VarInt::from(37))]
    IntRange,
    #[key(VarInt::from(38))]
    FloatRange,
    #[key(VarInt::from(39))]
    ItemEnchantment,
    #[key(VarInt::from(40))]
    EntitySummon,
    #[key(VarInt::from(41))]
    Dimension,
    #[key(VarInt::from(42))]
    Time,
    #[key(VarInt::from(43))]
    ResourceOrTag(Identifier),
    #[key(VarInt::from(44))]
    Resource(Identifier),
    #[key(VarInt::from(45))]
    TemplateMirror,
    #[key(VarInt::from(46))]
    TemplateRotation,
    #[key(VarInt::from(47))]
    Uuid,
}

#[derive(Debug)]
pub enum NodeStub {
    Root,
    Argument {
        id: Identifier,
        argument: Argument,
        resource_location: Option<Identifier>,
    },
    Literal(Identifier),
}

#[derive(Debug)]
pub struct Command {
    pub command_flags: u8,
    pub children: (VarInt, Vec<VarInt>),
    pub redirect: Option<VarInt>,
    pub node_stub: NodeStub,
}

impl Contextual for Command {
    fn context() -> String {
        "Command".to_string()
    }
}

impl Contextual for NodeStub {
    fn context() -> String {
        "NodeStub".to_string()
    }
}

impl Serialize for Command {
    fn serialize<W: Write>(&self, writer: &mut W, protocol_version: ProtocolVersion) -> mc_serializer::serde::Result<()> {
        wrap_struct_context!("command_flags", self.command_flags.serialize(writer, protocol_version))?;
        wrap_struct_context!("children", self.children.serialize(writer, protocol_version))?;
        if self.command_flags & 8 != 0 {
            match self.redirect {
                None => return Err(Error::Generic(SerializerContext::new(Self::context(), "Found `None` redirect when one was specified.".to_string()))),
                Some(redirect) => wrap_struct_context!("redirect", redirect.serialize(writer, protocol_version))?,
            }
        }
        match &self.node_stub {
            NodeStub::Argument {
                id,
                argument,
                resource_location
            } => {
                wrap_struct_context!("node_argument_id", id.serialize(writer, protocol_version))?;
                wrap_struct_context!("node_argument_argument", argument.serialize(writer, protocol_version))?;
                wrap_struct_context!("node_resource_location", resource_location.serialize(writer, protocol_version))?;
            }
            Literal(literal) => {
                wrap_struct_context!("node_literal", literal.serialize(writer, protocol_version))?;
            }
            Root => (),
        }
        Ok(())
    }

    fn size(&self, protocol_version: ProtocolVersion) -> mc_serializer::serde::Result<i32> {
        let mut size = 0;
        size += wrap_struct_context!("command_flags", self.command_flags.size(protocol_version))?;
        size += wrap_struct_context!("children", self.children.size(protocol_version))?;
        if self.command_flags & 8 != 0 {
            match self.redirect {
                None => return Err(mc_serializer::serde::Error::Generic(SerializerContext::new(Self::context(), "Found `None` redirect when one was specified.".to_string()))),
                Some(redirect) => size += wrap_struct_context!("redirect", redirect.size(protocol_version))?,
            }
        }
        match &self.node_stub {
            NodeStub::Argument {
                id,
                argument,
                resource_location
            } => {
                size += wrap_struct_context!("node_argument_id", id.size(protocol_version))?;
                size += wrap_struct_context!("node_argument_argument", argument.size(protocol_version))?;
                size += wrap_struct_context!("node_resource_location", resource_location.size(protocol_version))?;
            }
            Literal(literal) => {
                size += wrap_struct_context!("node_literal", literal.size(protocol_version))?;
            }
            Root => (),
        }
        Ok(size)
    }
}

impl Deserialize for Command {
    fn deserialize<R: Read>(reader: &mut R, protocol_version: ProtocolVersion) -> mc_serializer::serde::Result<Self> {
        let flags = wrap_struct_context!("flags", u8::deserialize(reader, protocol_version))?;
        let children = wrap_struct_context!("children", <(VarInt, Vec<VarInt>)>::deserialize(reader, protocol_version))?;
        let redirect = if flags & 8 != 0 {
            Some(wrap_struct_context!("redirect", VarInt::deserialize(reader, protocol_version))?)
        } else {
            None
        };
        let node_stub = match flags & 3 {
            1 => {
                let id = wrap_struct_context!("argument_identifier", Identifier::deserialize(reader, protocol_version))?;
                let argument = wrap_struct_context!("argument_argument", Argument::deserialize(reader, protocol_version))?;
                let resource_location = if flags & 0x10 != 0 {
                    Some(wrap_struct_context!("argument_resource_location", Identifier::deserialize(reader, protocol_version))?)
                } else {
                    None
                };
                NodeStub::Argument {
                    id,
                    argument,
                    resource_location,
                }
            }
            2 => Literal(wrap_struct_context!("literal_identifier", Identifier::deserialize(reader, protocol_version))?),
            _ => Root
        };

        Ok(Command {
            command_flags: flags,
            children,
            redirect,
            node_stub,
        })
    }
}

impl Command {
    pub fn is_literal(&self) -> bool {
        self.command_flags & 1 != 0
    }

    pub fn is_argument(&self) -> bool {
        self.command_flags & 2 != 0
    }

    pub fn executable(&self) -> bool {
        self.command_flags & 4 != 0
    }

    pub fn has_redirect(&self) -> bool {
        self.command_flags & 8 != 0
    }

    pub fn has_custom_suggestions(&self) -> bool {
        self.command_flags & 0x10 != 0
    }
}
