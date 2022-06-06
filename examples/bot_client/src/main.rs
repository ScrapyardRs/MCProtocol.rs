use mc_protocol::packets::server_bound::status::Ping;
use mc_protocol::packets::*;
use mc_protocol::prelude::*;
use std::io::Cursor;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use tokio::net::TcpStream;
use custom_packets::CustomPayload;
use mc_protocol::packets::client_bound::status::StatusResponse;

const STATUS: bool = true;
const TARGET_IP: &'static str = "127.0.0.1";
const TARGET_PORT: u16 = 25565;
const NATIVE_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::V118R2;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if STATUS {
        println!("Calling status of: {TARGET_IP}:{TARGET_PORT}");
        spin_status_req().await?;
    }
    Ok(())
}

async fn spin_status_req() -> anyhow::Result<StatusResponse> {
    use client_bound::status::{Pong, StatusResponse};
    use server_bound::handshaking::{Handshake, NextState};
    use server_bound::status::StatusRequest;

    #[derive(Default)]
    struct Context {
        packet_queue: Vec<Cursor<Vec<u8>>>,
        complete: bool,
    }

    type Sheet = ProtocolSheet<Context>;

    impl Context {
        fn spin_queue(self) -> (Vec<Cursor<Vec<u8>>>, Context) {
            return (self.packet_queue, Context { packet_queue: Vec::new(), complete: false });
        }
    }

    let mut protocol_sheet = ProtocolSheet::new(NATIVE_PROTOCOL_VERSION);

    fn status_response_handle(
        sheet: &mut Sheet,
        context: &mut Context,
        response: StatusResponse,
    ) -> anyhow::Result<()> {
        println!("Status: {:?}", response.json_response);
        context.packet_queue.push(
            Ping {
                payload: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_millis() as i64,
            }.to_cursor(sheet.protocol_version, None, None)?,
        );
        Ok(())
    }

    fn pong_handle(_: &mut Sheet, context: &mut Context, pong: Pong) -> anyhow::Result<()> {
        let latency = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis()
            - pong.payload as u128;
        println!("Got pong response! {pong:?}... Latency: {latency}ms");
        context.complete = true;
        Ok(())
    }

    protocol_sheet.register_packet_handle(Box::new(status_response_handle));
    protocol_sheet.register_packet_handle(Box::new(pong_handle));

    let handshake = Handshake::new(
        758i32.into(),
        TARGET_IP.into(),
        TARGET_PORT,
        (1i32.into(), NextState::Status {}),
    );
    let status_request = StatusRequest {};

    let mut buffer = MinecraftPacketBuffer::new();

    let mut stream = TcpStream::connect(format!("{TARGET_IP}:{TARGET_PORT}")).await?;
    println!("Writing handshake & status request.");
    stream.write(CustomPayload {
        f1: VarInt::from(25),
        f2: VarInt::from(28),
    }.to_cursor(ProtocolVersion::Handshake, None, None)?.into_inner().as_slice()).await?;
    stream.write(handshake.to_cursor(ProtocolVersion::Handshake, None, None)?.into_inner().as_slice()).await?;
    stream.write(status_request.to_cursor(NATIVE_PROTOCOL_VERSION, None, None)?.into_inner().as_slice()).await?;

    let mut context = Context {
        packet_queue: Vec::new(),
        complete: false,
    };

    loop {
        match buffer.poll() {
            BufferState::PacketReady => {
                protocol_sheet.call_generic(&mut context, buffer.packet_reader()?)?;

                if context.complete {
                    return Ok(());
                }

                let context_poll = context.spin_queue();
                context = context_poll.1;
                for packet in context_poll.0 {
                    stream.write(packet.into_inner().as_slice()).await?;
                }
            }
            BufferState::Waiting => {
                stream.read_buf(buffer.inner_buf()).await?;
            }
            BufferState::Error(error) => {
                anyhow::bail!("Found error {} while polling buffer.", error);
            }
        }
    }
}
