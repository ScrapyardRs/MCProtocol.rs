use mc_protocol::{BufferState, MinecraftPacketBuffer, PacketToCursor, ProtocolSheet, ProtocolVersion};
use std::io::Cursor;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use custom_packets::CustomPayload;
use mc_protocol::packets::client_bound::status::{Pong, StatusResponse};
use mc_protocol::packets::server_bound::handshaking::{Handshake, NextState};
use mc_protocol::packets::server_bound::status::{Ping, StatusRequest};

const TARGET_IP: &'static str = "127.0.0.1";
const TARGET_PORT: u16 = 25565;
const NATIVE_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::V118R2;

type URes = anyhow::Result<()>;

async fn handle_client(mut stream: TcpStream) -> URes {
    struct Context {
        packet_queue: Vec<Cursor<Vec<u8>>>,
    }

    type Sheet = ProtocolSheet<Context>;

    impl Context {
        fn spin_queue(self) -> (Vec<Cursor<Vec<u8>>>, Context) {
            (self.packet_queue, Self { packet_queue: Vec::new() })
        }
    }

    fn custom_payload_handle(_: &mut Sheet, __: &mut Context, payload: CustomPayload) -> URes {
        println!("Custom payload received: {payload:?}");
        Ok(())
    }

    fn handshake_handle(sheet: &mut Sheet, _: &mut Context, handshake: Handshake) -> URes {
        println!("READING HANDSHAKE {handshake:?}");
        let true_protocol = ProtocolVersion::from_varint(handshake.protocol_version);

        if let Some(protocol) = true_protocol {
            sheet.protocol_version = protocol;
        } else {
            anyhow::bail!("Unsupported protocol version {:?}", handshake.protocol_version);
        }

        if let NextState::Status {} = handshake.next_state.1 {
            sheet.clear();
            sheet.register_packet_handle(Box::new(status_req_handle));
            sheet.register_packet_handle(Box::new(ping_handle));
        }
        Ok(())
    }

    fn status_req_handle(sheet: &mut Sheet, context: &mut Context, _: StatusRequest) -> URes {
        println!("READING STATUS REQUEST");
        let spec = NATIVE_PROTOCOL_VERSION.to_spec();
        context.packet_queue.push(StatusResponse {
            json_response: format!(r#"
            {{
                {{
                    "version": {{
                        "name": "{}",
                        "protocol": {}
                    }},
                    "players": {{
                        "max": 7,
                        "online": 12,
                        "sample": [
                            {{
                                "name": "thinkofdeath",
                                "id": "4566e69f-c907-48ee-8d71-d7ba5aa00d20"
                            }}
                        ]
                    }},
                    "description": {{
                        "text": "Hello world"
                    }}
                }}
            }}
            "#, spec.1, spec.0).into()
        }.to_cursor(sheet.protocol_version, None, None)?);
        Ok(())
    }

    fn ping_handle(sheet: &mut Sheet, context: &mut Context, ping: Ping) -> URes {
        println!("READING PING {ping:?}");
        context.packet_queue.push(Pong {
            payload: ping.payload,
        }.to_cursor(sheet.protocol_version, None, None)?);
        Ok(())
    }

    let mut protocol_sheet = ProtocolSheet::<Context>::new(ProtocolVersion::Handshake);
    protocol_sheet.register_packet_handle(Box::new(custom_payload_handle));
    protocol_sheet.register_packet_handle(Box::new(handshake_handle));

    let mut context = Context {
        packet_queue: Vec::new(),
    };

    let mut buffer = MinecraftPacketBuffer::new();

    loop {
        match buffer.poll() {
            BufferState::PacketReady => {
                protocol_sheet.call_generic(&mut context, buffer.packet_reader()?)?;
                let (to_send, new_context) = context.spin_queue();
                for sendable in to_send {
                    stream.write_all(sendable.into_inner().as_slice()).await?;
                }
                context = new_context;
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

#[tokio::main]
async fn main() -> URes {
    let addr = format!("{TARGET_IP}:{TARGET_PORT}");
    let listener = TcpListener::bind(&addr).await?;
    log::info!("Status Responder Listening on: {}", addr);

    loop {
        let (socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(error) = handle_client(socket).await {
                log::error!("An error occurred processing socket {error}");
            }
        });
    }
}
