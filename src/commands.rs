use drax::extension::*;
use drax::transport::Error;
use drax::transport::Result;
use std::io::{Read, Write};

use drax::transport::TransportProcessorContext;
use drax::{SizedVec, VarInt};

#[derive(drax_derive::DraxTransport, Debug)]
#[drax(key = {match VarInt})]
pub enum StringType {
    SingleWord,
    QuotablePhrase,
    GreedyPhrase,
}

#[derive(drax_derive::BitMapTransport, Debug)]
pub struct MinMaxBitFlag {
    pub min: bool,
    pub max: bool,
}

#[derive(drax_derive::BitMapTransport, Debug)]
pub struct EntityBitFlag {
    pub single: bool,
    pub players_only: bool,
}

#[derive(drax_derive::DraxTransport, Debug)]
#[drax(key = {match VarInt})]
pub enum Argument {
    Bool,
    Float {
        flags: MinMaxBitFlag,
        #[drax(skip_if = {!flags.min})]
        min: Option<f32>,
        #[drax(skip_if = {!flags.max})]
        max: Option<f32>,
    },
    Double {
        flags: MinMaxBitFlag,
        #[drax(skip_if = {!flags.min})]
        min: Option<f64>,
        #[drax(skip_if = {!flags.max})]
        max: Option<f64>,
    },
    Integer {
        flags: MinMaxBitFlag,
        #[drax(skip_if = {!flags.min})]
        min: Option<i32>,
        #[drax(skip_if = {!flags.max})]
        max: Option<i32>,
    },
    Long {
        flags: MinMaxBitFlag,
        #[drax(skip_if = {!flags.min})]
        min: Option<i64>,
        #[drax(skip_if = {!flags.max})]
        max: Option<i64>,
    },
    String(StringType),
    Entity(EntityBitFlag),
    GameProfile,
    BlockPos,
    ColumnPos,
    Vec3,
    Vec2,
    BlockState,
    BlockPredicate,
    ItemStack,
    ItemPredicate,
    Color,
    Component,
    Message,
    NbtCompoundTag,
    NbtTag,
    NbtPath,
    Objective,
    ObjectiveCriteria,
    Operation,
    Particle,
    Angle,
    Rotation,
    ScoreboardSlot,
    ScoreHolder(bool),
    Swizzle,
    Team,
    ItemSlot,
    ResourceLocation,
    MobEffect,
    Function,
    EntityAnchor,
    IntRange,
    FloatRange,
    ItemEnchantment,
    EntitySummon,
    Dimension,
    Time,
    ResourceOrTag(String),
    Resource(String),
    TemplateMirror,
    TemplateRotation,
    Uuid,
}

#[derive(Debug)]
pub enum NodeStub {
    Root,
    Argument {
        id: String,
        argument: Argument,
        resource_location: Option<String>,
    },
    Literal(String),
}

#[derive(Debug)]
pub struct Command {
    pub command_flags: u8,
    pub children: SizedVec<VarInt>,
    pub redirect: Option<VarInt>,
    pub node_stub: NodeStub,
}

impl drax::transport::DraxTransport for Command {
    fn write_to_transport(
        &self,
        context: &mut TransportProcessorContext,
        writer: &mut std::io::Cursor<Vec<u8>>,
    ) -> Result<()> {
        self.command_flags.write_to_transport(context, writer)?;
        write_var_int_sync(self.children.len() as i32, context, writer)?;
        for child in self.children.iter() {
            write_var_int_sync(*child, context, writer)?;
        }
        if self.command_flags & 8 != 0 {
            match self.redirect {
                None => {
                    return Error::cause("Found `None` redirect when one was specified.");
                }
                Some(redirect) => {
                    write_var_int_sync(redirect, context, writer)?;
                }
            }
        }
        match &self.node_stub {
            NodeStub::Argument {
                id,
                argument,
                resource_location,
            } => {
                write_string(32767, id, context, writer)?;
                argument.write_to_transport(context, writer)?;
                if let Some(loc) = resource_location {
                    write_string(32767, loc, context, writer)?;
                }
            }
            NodeStub::Literal(literal) => {
                write_string(32767, literal, context, writer)?;
            }
            NodeStub::Root => (),
        }
        Ok(())
    }

    fn read_from_transport<R: Read>(
        context: &mut TransportProcessorContext,
        read: &mut R,
    ) -> Result<Self>
    where
        Self: Sized,
    {
        let flags = i8::read_from_transport(context, read)?;
        let children_size = read_var_int_sync(context, read)?;
        let mut children = Vec::with_capacity(children_size as usize);
        for _ in 0i32..children_size.into() {
            let next = read_var_int_sync(context, read)?;
            children.push(next);
        }

        let redirect = if flags & 8 != 0 {
            Some(read_var_int_sync(context, read)?)
        } else {
            None
        };
        let node_stub = match flags & 3 {
            2 => {
                let id = read_string(32767, context, read)?;
                let argument = Argument::read_from_transport(context, read)?;
                let resource_location = if flags & 0x10 != 0 {
                    Some(read_string(32767, context, read)?)
                } else {
                    None
                };
                NodeStub::Argument {
                    id,
                    argument,
                    resource_location,
                }
            }
            1 => NodeStub::Literal(read_string(32767, context, read)?),
            0 => NodeStub::Root,
            x => {
                return Error::cause(format!("Failed to understand command node of type {}", x));
            }
        };

        Ok(Command {
            command_flags: flags as u8,
            children,
            redirect,
            node_stub,
        })
    }

    fn precondition_size(&self, context: &mut TransportProcessorContext) -> Result<usize> {
        let mut size = 0;
        size += self.command_flags.precondition_size(context)?;
        size += size_var_int(self.children.len() as i32, context)?;
        for child in self.children.iter() {
            size += size_var_int(*child, context)?;
        }
        if self.command_flags & 8 != 0 {
            match self.redirect {
                None => {
                    return Error::cause("Found `None` redirect when one was specified");
                }
                Some(redirect) => {
                    size += size_var_int(redirect, context)?;
                }
            }
        }
        match &self.node_stub {
            NodeStub::Argument {
                id,
                argument,
                resource_location,
            } => {
                size += size_string(id, context)?;
                size += argument.precondition_size(context)?;
                if let Some(resource_location) = resource_location.as_ref() {
                    size += size_string(resource_location, context)?;
                }
            }
            NodeStub::Literal(literal) => {
                size += size_string(literal, context)?;
            }
            NodeStub::Root => (),
        }
        Ok(size)
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
