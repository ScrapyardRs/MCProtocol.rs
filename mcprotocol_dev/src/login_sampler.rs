use bytes::Buf;
use futures::future::BoxFuture;
use mc_buffer::buffer::PacketReader;
use mc_buffer::buffer::{OwnedPacketReader, PacketReaderGeneric};
use mc_buffer::encryption::{Codec, Compressor};
use mc_chat::Chat;
use mc_registry::client_bound::login::{LoginSuccess, SetCompression};
use mc_registry::client_bound::play::{
    ChangeDifficulty, DeclareCommands, DeclareRecipes, EntityEvent, JoinGame, LevelChunkWithLight,
    LightUpdate, Ping, PlayerAbilities, PlayerInfo, PlayerPosition, PluginMessage, SetCarriedItem,
    SetChunkCacheCenter, SetChunkCacheRadius, SetDefaultSpawnPosition, SystemChat, UpdateRecipes,
    UpdateTags, WorldBorder,
};
use mc_registry::mappings::Mappings;
use mc_registry::registry::{
    arc_lock, LockedContext, LockedStateRegistry, StateRegistry, UnhandledContext,
};
use mc_registry::server_bound::handshaking::{Handshake, NextState, ServerAddress};
use mc_registry::server_bound::login::LoginStart;
use mc_registry::server_bound::play::Pong;
use mc_registry::shared_types::login::LoginUsername;
use mc_registry::shared_types::play::ResourceLocation;
use mc_serializer::ext::{read_nbt, write_nbt};
use mc_serializer::primitive::{read_string, Identifier, VarInt};
use mc_serializer::serde::ProtocolVersion;
use std::io::Cursor;
use std::process::exit;
use std::sync::Arc;
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::tcp::WriteHalf;
use tokio::net::TcpStream;

struct Test {
    owned_write: OwnedWriteHalf,
}

pub type PacketWriterFuture<'a> = BoxFuture<'a, anyhow::Result<()>>;

pub trait PacketWriter<W: AsyncWrite + Unpin + Send + Sync>: Send + Sync {
    fn writer(&mut self) -> &mut W;

    fn compressor(&self) -> Option<&Compressor>;

    fn encrypt(&mut self, buffer: &mut Vec<u8>);

    fn protocol_version(&self) -> ProtocolVersion;

    fn send_packet<'a, Packet: Mappings<PacketType = Packet> + Send + Sync + 'a>(
        &'a mut self,
        packet: Packet,
    ) -> PacketWriterFuture<'a> {
        Box::pin(async move {
            let buffer = Packet::create_packet_buffer(self.protocol_version(), packet)?;

            let mut buffer = if let Some(compressor) = self.compressor() {
                compressor.compress(buffer)?
            } else {
                Compressor::uncompressed(buffer)?
            };

            self.encrypt(&mut buffer);

            let mut buffer = Cursor::new(buffer);

            while buffer.has_remaining() {
                self.writer().write_buf(&mut buffer).await?;
            }
            Ok(())
        })
    }
}

impl PacketWriter<OwnedWriteHalf> for Test {
    fn writer(&mut self) -> &mut OwnedWriteHalf {
        &mut self.owned_write
    }

    fn compressor(&self) -> Option<&Compressor> {
        None
    }

    #[inline]
    fn encrypt(&mut self, _: &mut Vec<u8>) {}

    fn protocol_version(&self) -> ProtocolVersion {
        ProtocolVersion::V119_1
    }
}

#[mc_registry_derive::packet_handler]
fn handle_login_success(
    packet: LoginSuccess,
    _context: LockedContext<Test>,
    registry: LockedStateRegistry<Test>,
) {
    let mut lock = registry.write().await;
    lock.clear_mappings();
    JoinGame::attach_to_register(&mut lock, handle_join_game);
    Ping::attach_to_register(&mut lock, handle_ping);
    PluginMessage::attach_to_register(&mut lock, handle_plugin_message);
    ChangeDifficulty::attach_to_register(&mut lock, handle_change_difficult);
    PlayerAbilities::attach_to_register(&mut lock, handle_player_abilities);
    SetCarriedItem::attach_to_register(&mut lock, handle_set_carried_item);
    UpdateRecipes::attach_to_register(&mut lock, handle_update_recipes);
    UpdateTags::attach_to_register(&mut lock, handle_update_tags);
    EntityEvent::attach_to_register(&mut lock, handle_update_tags);
    DeclareCommands::attach_to_register(&mut lock, handle_declare_commands);
    DeclareRecipes::attach_to_register(&mut lock, handle_declare_recipes);
    PlayerInfo::attach_to_register(&mut lock, handle_player_info);
    SetChunkCacheCenter::attach_to_register(&mut lock, handle_set_chunk_cache_center);
    LightUpdate::attach_to_register(&mut lock, handle_light_update);
    LevelChunkWithLight::attach_to_register(&mut lock, handle_light_update_with_chunk);
    WorldBorder::attach_to_register(&mut lock, handle_world_border);
    SetDefaultSpawnPosition::attach_to_register(&mut lock, handle_set_default_spawn_position);
    PlayerPosition::attach_to_register(&mut lock, handle_player_position);
    SetChunkCacheRadius::attach_to_register(&mut lock, handle_set_chunk_radius)
}

#[mc_registry_derive::packet_handler]
fn handle_set_chunk_radius(packet: SetChunkCacheRadius, _context: LockedContext<Test>) {
    println!("Set Chunk Cache radius: {:?}", packet);
}

#[mc_registry_derive::packet_handler]
fn handle_player_position(packet: PlayerPosition, _context: LockedContext<Test>) {
    println!("Player Position: {:?}", packet);
}

#[mc_registry_derive::packet_handler]
fn handle_set_default_spawn_position(
    packet: SetDefaultSpawnPosition,
    _context: LockedContext<Test>,
) {
    println!("Set Default Spawn Position: {:?}", packet);
}

#[mc_registry_derive::packet_handler]
fn handle_world_border(packet: WorldBorder, _context: LockedContext<Test>) {
    println!("WorldBorder: {:?}", packet);
}

#[mc_registry_derive::packet_handler]
fn handle_light_update_with_chunk(packet: LevelChunkWithLight, _context: LockedContext<Test>) {
    println!("Level chunk with light");
}

#[mc_registry_derive::packet_handler]
fn handle_light_update(packet: LightUpdate, _context: LockedContext<Test>) {
    println!("Light update packet");
}

#[mc_registry_derive::packet_handler]
fn handle_set_chunk_cache_center(packet: SetChunkCacheCenter, _context: LockedContext<Test>) {
    println!("Set Chunk Cache Center: {:?}", packet);
}

#[mc_registry_derive::packet_handler]
fn handle_player_info(packet: PlayerInfo, _context: LockedContext<Test>) {
    println!("Player Info: {:?}", packet);
}

#[mc_registry_derive::packet_handler]
fn handle_declare_recipes(packet: DeclareRecipes, _context: LockedContext<Test>) {
    println!("Declare Recipes");
}

#[mc_registry_derive::packet_handler]
fn handle_declare_commands(packet: DeclareCommands, _context: LockedContext<Test>) {
    println!("Declare Commands");
}

#[mc_registry_derive::packet_handler]
fn handle_entity_event(packet: EntityEvent, _context: LockedContext<Test>) {
    println!("Entity Event: {:?}", packet);
}

#[mc_registry_derive::packet_handler]
fn handle_set_compression(packet: SetCompression, _context: LockedContext<Test>) {
    println!("Set Compression: {:?}", packet);
}

#[mc_registry_derive::packet_handler]
fn handle_join_game(packet: JoinGame, _context: LockedContext<Test>) {
    println!("Join Game");
    let mut bytes = Vec::new();
    write_nbt(&packet.codec, &mut bytes, ProtocolVersion::V119_1)?;
    let mut bytes = Cursor::new(bytes);
    let codec: mc_level::codec::Codec = read_nbt(&mut bytes, ProtocolVersion::V119_1)?;

    // println!("Join Game: {:#?}", packet);
}

#[mc_registry_derive::packet_handler]
fn handle_plugin_message(packet: PluginMessage, _context: LockedContext<Test>) {
    if packet.identifier == ResourceLocation::from("minecraft:brand") {
        let mut cursor = Cursor::new(packet.data);
        let brand = read_string(32767, &mut cursor, ProtocolVersion::V119_1)?;
        println!("Custom Payload Brand: {}", brand);
    } else {
        println!("Custom Payload {}", packet.identifier);
    }
}

#[mc_registry_derive::packet_handler]
fn handle_change_difficult(packet: ChangeDifficulty, _context: LockedContext<Test>) {
    println!("Change difficulty: {:?}", packet);
}

#[mc_registry_derive::packet_handler]
fn handle_player_abilities(packet: PlayerAbilities, _context: LockedContext<Test>) {
    println!("Player Abilities: {:?}", packet);
}

#[mc_registry_derive::packet_handler]
fn handle_set_carried_item(packet: SetCarriedItem, _context: LockedContext<Test>) {
    println!("Set Carried Item: {:?}", packet);
}

#[mc_registry_derive::packet_handler]
fn handle_update_recipes(packet: UpdateRecipes, _context: LockedContext<Test>) {
    // println!("Update Recipes: {:#?}", packet);
    println!("Update recipes.");
}

#[allow(unreachable_code)]
#[mc_registry_derive::packet_handler]
fn handle_update_tags(packet: UpdateTags, _context: LockedContext<Test>) {
    // println!("Update Tags: {:#?}", packet);
    println!("Updating tags.");
}

#[mc_registry_derive::packet_handler]
fn handle_ping(packet: Ping, context: LockedContext<Test>) {
    println!("Server ping.");
    let mut lock = context.write().await;
    lock.send_packet(Pong { id: packet.id }).await?;
}

pub async fn run() -> anyhow::Result<()> {
    let mut registry = StateRegistry::new(ProtocolVersion::V119_1);
    LoginSuccess::attach_to_register(&mut registry, handle_login_success);
    SetCompression::attach_to_register(&mut registry, handle_set_compression);
    let registry = arc_lock(registry);

    let connection = TcpStream::connect("localhost:25565").await?;
    let (read, write) = connection.into_split();

    let mut packet_buffer = OwnedPacketReader::new(read);
    let mut context = Test { owned_write: write };

    let handshake = Handshake {
        protocol_version: ProtocolVersion::V119_1.get_protocol_id().into(),
        server_address: ServerAddress::from("localhost"),
        server_port: 25565,
        next_state: NextState::Login,
    };
    let login_start = LoginStart {
        name: LoginUsername::from("KekW"),
        sig_data: (false, None),
        sig_holder: (false, None),
    };

    context.send_packet(handshake).await?;
    context.send_packet(login_start).await?;

    let context = arc_lock(context);

    loop {
        let next_packet = packet_buffer.loop_read().await?;
        match StateRegistry::emit(
            Arc::clone(&registry),
            Arc::clone(&context),
            Cursor::new(next_packet),
        )
        .await?
        {
            None => {}
            Some(unhandled) => {
                println!(
                    "Received packet ID {} of size {}",
                    unhandled.packet_id,
                    unhandled.bytes.len()
                );
                if unhandled.packet_id == VarInt::from(98) {
                    // system chat
                    continue;
                }
                if unhandled.packet_id == VarInt::from(90) {
                    // set simulation distance
                    continue;
                }
                if unhandled.packet_id == VarInt::from(0) {
                    // add entity
                    continue;
                }
                if unhandled.packet_id == VarInt::from(80) {
                    // set entity data
                    continue;
                }
                if unhandled.packet_id == VarInt::from(104) {
                    // update attributes
                    continue;
                }
                if unhandled.packet_id == VarInt::from(63) {
                    // rotate head
                    continue;
                }
                if unhandled.packet_id == VarInt::from(83) {
                    // set equipment
                    continue;
                }
                if unhandled.packet_id == VarInt::from(92) {
                    // set time
                    continue;
                }
                if unhandled.packet_id == VarInt::from(29) {
                    // Game event
                    continue;
                }
                if unhandled.packet_id == VarInt::from(66) {
                    // Server data
                    continue;
                }
                if unhandled.packet_id == VarInt::from(17) {
                    // Container set content
                    continue;
                }
                if unhandled.packet_id == VarInt::from(85) {
                    // set health
                    continue;
                }
                if unhandled.packet_id == VarInt::from(84) {
                    // set experience
                    continue;
                }
                if unhandled.packet_id == VarInt::from(103) {
                    // Update advancements
                    continue;
                }
                if unhandled.packet_id == VarInt::from(40) {
                    // Move entity pos
                    continue;
                }
                if unhandled.packet_id == VarInt::from(41) {
                    // Move entity pos rot
                    continue;
                }
                if unhandled.packet_id == VarInt::from(42) {
                    // Move entity rot
                    continue;
                }
                if unhandled.packet_id == VarInt::from(82) {
                    // Set entity motion
                    continue;
                }
                if unhandled.packet_id == VarInt::from(1) {
                    // Add experience orb
                    continue;
                }
                if unhandled.packet_id == VarInt::from(102) {
                    // Client teleport entity
                    continue;
                }
                if unhandled.packet_id == VarInt::from(9) {
                    // Block update
                    continue;
                }
                if unhandled.packet_id == VarInt::from(59) {
                    // Remove entities
                    continue;
                }
                return Ok(());
            }
        }
    }
}
