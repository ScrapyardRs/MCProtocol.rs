pub mod mappings;
pub mod registry;

pub mod client_bound;
pub mod server_bound;

pub type Result<T> = anyhow::Result<T>;
