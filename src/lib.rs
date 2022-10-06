//! # MCProtocol.rs
//!
//! MCProtocol is built as a version of the Minecraft protocol specification. <br/>
//! Most of the work done here was work through reverse engineering the vanilla client. <br />
//! This project does *NOT* aim to mimic or act like a vanilla client or server. <br />

pub mod registry;
pub mod crypto;
pub mod protocol;
pub mod chat;
pub mod prelude;
