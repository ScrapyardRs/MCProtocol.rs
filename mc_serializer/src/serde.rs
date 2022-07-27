use crate::primitive::VarInt;
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::num::TryFromIntError;

#[derive(Debug)]
pub enum Error {
    Generic(String),
    IoError(std::io::Error),
    AnyhowError(anyhow::Error),
    TryFromIntError(TryFromIntError),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}

impl From<anyhow::Error> for Error {
    fn from(error: anyhow::Error) -> Self {
        Error::AnyhowError(error)
    }
}

impl From<TryFromIntError> for Error {
    fn from(error: TryFromIntError) -> Self {
        Error::TryFromIntError(error)
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::IoError(error)
    }
}

pub type SerdeResult<T> = Result<T, Error>;

pub type ProtocolVersionSpec = (i32, String);

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub enum ProtocolVersion {
    Unknown,
    Handshake,
    V119R1,
}

impl PartialOrd for ProtocolVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let that_protocol = other.to_spec().0;
        let this_protocol = self.to_spec().0;
        match that_protocol {
            x if this_protocol < x => Some(Ordering::Less),
            x if this_protocol > x => Some(Ordering::Greater),
            _ => Some(Ordering::Equal),
        }
    }
}

impl ProtocolVersion {
    pub fn to_spec(&self) -> ProtocolVersionSpec {
        match self {
            ProtocolVersion::Unknown => (99999, "n/a".to_string()), // treat unknown as latest
            ProtocolVersion::Handshake => (-1, "n/a".to_string()),
            ProtocolVersion::V119R1 => (759, "1.19.1".to_string()),
        }
    }
}

impl From<i32> for ProtocolVersion {
    fn from(val: i32) -> Self {
        match val {
            759 => ProtocolVersion::V119R1,
            _ => ProtocolVersion::Unknown,
        }
    }
}

impl From<VarInt> for ProtocolVersion {
    fn from(val: VarInt) -> Self {
        let num: i32 = val.into();
        ProtocolVersion::from(num)
    }
}

pub trait Serialize: Sized {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> SerdeResult<()>;

    fn serialize_with_protocol<W: std::io::Write>(
        &self,
        writer: &mut W,
        _protocol_version: ProtocolVersion,
    ) -> SerdeResult<()> {
        Self::serialize(self, writer)
    }

    fn size(&self) -> SerdeResult<i32>;
}

pub trait Deserialize: Sized {
    fn deserialize<R: std::io::Read>(reader: &mut R) -> SerdeResult<Self>;

    fn deserialize_with_protocol<R: std::io::Read>(
        reader: &mut R,
        _: ProtocolVersion,
    ) -> SerdeResult<Self> {
        Self::deserialize(reader)
    }
}
