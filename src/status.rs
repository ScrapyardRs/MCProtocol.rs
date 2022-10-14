use crate::pipeline::{AsyncMinecraftProtocolPipeline, MinecraftProtocolWriter};
use crate::protocol::handshaking::sb::Handshake;
use crate::protocol::status::cb::{
    Pong, Response, StatusResponse, StatusResponsePlayers, StatusResponseVersion,
};
use crate::protocol::status::sb::{Ping, Request};
use crate::registry::{RegistryError, UNKNOWN_VERSION};
use crate::{chat, pin_fut};
use drax::prelude::BoxFuture;
use drax::VarInt;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};

const fn proto_to_string(proto: VarInt) -> &'static str {
    match proto {
        760 => "1.19.2",
        _ => "???",
    }
}

pub enum StatusFunctionResponse {
    RequestForward,
    PingForward { start_time: i64 },
}

pub async fn handle_request(_: &mut (), _: Request) -> StatusFunctionResponse {
    StatusFunctionResponse::RequestForward
}

pub async fn handle_ping(_: &mut (), ping: Ping) -> StatusFunctionResponse {
    StatusFunctionResponse::PingForward {
        start_time: ping.start_time,
    }
}

pub struct StatusBuilder {
    pub players: StatusResponsePlayers,
    pub description: chat::Chat,
    pub favicon: Option<String>,
}

pub async fn handle_status_client<
    R: AsyncRead + Unpin + Sized + Send + Sync,
    W: AsyncWrite + Unpin + Sized + Send + Sync,
    Func: (Fn(Handshake) -> BoxFuture<'static, StatusBuilder>) + 'static,
>(
    read: R,
    write: W,
    handshake: Handshake,
    status_responder: Arc<Func>,
) -> Result<(), RegistryError> {
    let protocol_version = handshake.protocol_version;

    let mut status_pipeline = AsyncMinecraftProtocolPipeline::from_handshake(read, &handshake);
    let mut packet_writer = MinecraftProtocolWriter::from_handshake(write, &handshake);

    status_pipeline.register(pin_fut!(handle_request));
    status_pipeline.register(pin_fut!(handle_ping));

    match status_pipeline.execute_next_packet(&mut ()).await? {
        StatusFunctionResponse::RequestForward => {
            log::trace!("Listening to status request forward.");
            let StatusBuilder {
                players,
                description,
                favicon,
            } = (status_responder)(handshake).await;
            log::trace!("Responding with: {:?}, {:?}, {:?}", players, description, favicon);
            let status = StatusResponse {
                version: StatusResponseVersion {
                    name: proto_to_string(protocol_version).to_string(),
                    protocol: protocol_version,
                },
                players,
                description,
                favicon,
                previews_chat: Some(false),
            };
            packet_writer.write_packet(Response(status)).await?;
        }
        StatusFunctionResponse::PingForward { .. } => {
            return Err(RegistryError::DraxTransportError(
                drax::transport::Error::Unknown(Some(format!(
                    "Invalid ping forward when request expected."
                ))),
            ))
        }
    }

    match status_pipeline.execute_next_packet(&mut ()).await? {
        StatusFunctionResponse::RequestForward => {
            return Err(RegistryError::DraxTransportError(
                drax::transport::Error::Unknown(Some(format!(
                    "Invalid ping forward when request expected."
                ))),
            ))
        }
        StatusFunctionResponse::PingForward { start_time } => {
            packet_writer.write_packet(Pong { start_time }).await?;
        }
    }

    Ok(())
}
