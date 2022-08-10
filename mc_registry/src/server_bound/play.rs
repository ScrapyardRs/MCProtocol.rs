use crate::shared_types::play::ResourceLocation;
use mc_serializer::primitive::{Identifier, VarInt};
use mc_serializer::serde::Contextual;

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct PluginMessage {
    pub identifier: ResourceLocation,
    pub data: Vec<u8>,
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(VarInt)]
pub enum ChatVisibility {
    #[key(VarInt::from(0))]
    Full,
    #[key(VarInt::from(1))]
    System,
    #[key(VarInt::from(2))]
    Hidden,
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(VarInt)]
pub enum HumanoidArm {
    #[key(VarInt::from(1))]
    Left,
    #[key(VarInt::from(2))]
    Right,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct ClientInformation {
    pub language: Identifier,
    pub view_distance: u8,
    pub chat_visibility: ChatVisibility,
    pub chat_colors: bool,
    pub model_customisation: u8,
    pub main_hand: HumanoidArm,
    pub text_filtering_enabled: bool,
    pub allows_listing: bool,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct AcceptTeleportation {
    pub id: VarInt,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct MovePlayerStatus {
    pub on_ground: bool,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct MovePlayerRot {
    pub y_rot: f32,
    pub x_rot: f32,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct MovePlayerPos {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct MovePlayerPosRot {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub y_rot: f32,
    pub x_rot: f32,
    pub on_ground: bool,
}

#[derive(mc_serializer_derive::Serial, Debug)]
#[key(VarInt)]
pub enum ClientCommand {
    #[key(VarInt::from(0))]
    PerformRespawn,
    #[key(VarInt::from(1))]
    RequestStats,
}

#[derive(mc_serializer_derive::Serial, Debug)]
pub struct Pong {
    pub id: i32,
}

crate::create_mappings! {
    AcceptTeleportation { def 0x0; }
    ClientCommand { def 0x7; }
    ClientInformation { def 0x8; }
    PluginMessage { def 0xD; }
    MovePlayerPos { def 0x14; }
    MovePlayerPosRot { def 0x15; }
    MovePlayerRot { def 0x16; }
    MovePlayerStatus { def 0x17; }
    Pong { def 0x20; }
}
