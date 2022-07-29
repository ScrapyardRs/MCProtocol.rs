use std::io::Cursor;
use std::ops::{Add};
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use mc_buffer::buffer::MinecraftPacketBuffer;

use mc_registry::client_bound::status::{Pong, Response};
use mc_registry::mappings::Mappings;
use mc_registry::registry::{arc_lock, StateRegistry};
use mc_registry::server_bound::handshaking::{Handshake, NextState, ServerAddress};
use mc_registry::server_bound::status::{Ping, Request};
use mc_serializer::serde::ProtocolVersion;
use mc_registry_derive::packet_handler;
use bytes::Buf;
use mc_buffer::encryption::Compressor;

struct NoContext;

#[packet_handler(NoContext)]
async fn handle_status_response(packet: Response) {
    println!("{}", packet.json_response);
}

#[packet_handler(NoContext)]
async fn handle_pong(packet: Pong) {
    println!("{}", UNIX_EPOCH.add(Duration::from_millis(packet.start_time as u64)).elapsed()?.as_millis());
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = std::env::args().skip(1).take(2).collect::<Vec<String>>();

    let addr = args.get(0).unwrap();
    let port = str::parse::<u16>(args.get(1).unwrap())?;

    let stream = TcpStream::connect(format!("{}:{}", addr, port)).await?;

    let protocol_version = ProtocolVersion::from(759);

    let (read, mut write) = stream.into_split();

    let mut handshake = Compressor::uncompressed(Handshake::create_packet_buffer(protocol_version, Handshake {
        protocol_version: protocol_version.into(),
        server_address: ServerAddress::from(addr),
        server_port: port,
        next_state: NextState::Status,
    })?)?;
    let mut status_request = Compressor::uncompressed(Request::create_packet_buffer(protocol_version, Request)?)?;

    handshake.append(&mut status_request);

    let mut buffer = Cursor::new(handshake);

    while buffer.has_remaining() {
        write.write_buf(&mut buffer).await?;
    }

    let mut registry = StateRegistry::<NoContext>::new(protocol_version);
    Response::attach_to_register(&mut registry, handle_status_response);
    Pong::attach_to_register(&mut registry, handle_pong);

    let mut packet_buf = MinecraftPacketBuffer::new(read);

    let registry = arc_lock(registry);
    let context = arc_lock(NoContext);

    let next_packet = packet_buf.loop_read().await?; // status response
    StateRegistry::emit(Arc::clone(&registry), Arc::clone(&context), Cursor::new(next_packet)).await?;

    let ping = Compressor::uncompressed(Ping::create_packet_buffer(protocol_version, Ping {
        start_time: UNIX_EPOCH.elapsed()?.as_millis() as i64
    })?)?;

    let mut buffer = Cursor::new(ping);

    while buffer.has_remaining() {
        write.write_buf(&mut buffer).await?;
    }

    let next_packet = packet_buf.loop_read().await?; // pong
    StateRegistry::emit(Arc::clone(&registry), Arc::clone(&context), Cursor::new(next_packet)).await?;
    Ok(())
}
