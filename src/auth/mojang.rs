// use std::future::Future;
// use std::pin::Pin;
// use std::sync::Arc;
//
// use crate::protocol::handshaking::sb::Handshake;
// use crate::protocol::login::sb::{EncryptionResponse, LoginStart};
// use drax::prelude::{AsyncRead, AsyncWrite};
// use drax::transport::{Result, TransportProcessorContext};
// use crate::registry::AsyncTypeTransport;
//
// pub enum AuthPacketResponse {
//     LoginStart {},
//     EncryptionResponse {},
// }
//
// pub async fn handle_login_start(
//     login_start: LoginStart,
//     context: &mut TransportProcessorContext,
// ) -> AuthPacketResponse {
//     AuthPacketResponse::LoginStart {}
// }
//
// pub async fn handle_encryption_response(
//     encryption_response: EncryptionResponse,
//     context: &mut TransportProcessorContext,
// ) -> AuthPacketResponse {
//     AuthPacketResponse::EncryptionResponse {}
// }
//
// pub enum AuthResult {
//     Denied(crate::chat::Chat),
//     Accepted {},
// }
//
// pub async fn authenticate_client<'a, R: AsyncRead, W: AsyncWrite>(
//     handshake: Handshake,
//     read: &mut R,
//     write: &mut W,
// ) -> Result<AuthResult> {
//     let mut registry = crate::registry::AsyncPacketRegistry::<AuthPacketResponse>::new(handshake.protocol_version);
//     AsyncTypeTransport {
//         inner: |t, ctx| Box::pin(handle_login_start(t, ctx)),
//         _phantom_output: Default::default(),
//         _phantom_t: Default::default()
//     };
//     // registry.register_with_context(handle_login_start);
//     // registry.register_with_context(handle_encryption_response);
//
//     Ok(AuthResult::Accepted {})
// }

use crate::pkt_ctx;
use crate::protocol::handshaking::sb::Handshake;
use crate::registry::AsyncPacketRegistry;

pub struct Context;

pub async fn test(_: &mut Context, handshake: Handshake) {}

fn test2() {
    let mut reg = AsyncPacketRegistry::default();
    reg.register(pkt_ctx!(test));
}
