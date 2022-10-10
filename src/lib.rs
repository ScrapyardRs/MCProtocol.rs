#![feature(closure_lifetime_binder)]
//! # MCProtocol.rs
//!
//! MCProtocol is built as a version of the Minecraft protocol specification. <br/>
//! Most of the work done here was work through reverse engineering the vanilla client. <br />
//! This project does *NOT* aim to mimic or act like a vanilla client or server. <br />

pub const MC_BUFFER_CAPACITY: usize = 2097154; // static value from wiki.vg

pub mod auth;
pub mod chat;
pub mod crypto;
pub mod pipeline;
pub mod prelude;
pub mod protocol;
pub mod registry;
pub mod server_loop;
pub mod status;
pub mod commands;
