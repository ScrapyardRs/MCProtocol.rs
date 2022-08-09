use mc_serializer::serde::ProtocolVersion;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Error {
    ProtocolInvalid(ProtocolVersion, ProtocolVersion),
    AnyhowError(anyhow::Error),
    SerdeError(mc_serializer::serde::Error),
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            Error::ProtocolInvalid(since, current) => write!(
                f,
                "The player's protocol {} attempted to access a packet from protocol {}.",
                current, since
            ),
            Error::AnyhowError(anyhow) => write!(f, "{}", anyhow),
            Error::SerdeError(serde) => write!(f, "{}", serde),
        }
    }
}

impl From<anyhow::Error> for Error {
    fn from(error: anyhow::Error) -> Self {
        Error::AnyhowError(error)
    }
}

impl From<mc_serializer::serde::Error> for Error {
    fn from(error: mc_serializer::serde::Error) -> Self {
        Error::SerdeError(error)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
