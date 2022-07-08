use crate::primitive::VarInt;
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug)]
pub enum Error {
    Generic(String),
    IoError(std::io::Error),
    AnyhowError(anyhow::Error),
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

pub type SerdeResult<T> = Result<T, Error>;

pub type ProtocolVersionSpec = (i32, String);

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub enum ProtocolVersion {
    Unknown,
    Handshake,
    V118R1,
    V118R2,
    V119R1,
}

impl PartialOrd for ProtocolVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let other_prot = other.to_spec().0;
        let this_prot = self.to_spec().0;
        if other_prot > this_prot {
            Some(Ordering::Less)
        } else if this_prot > other_prot {
            Some(Ordering::Greater)
        } else {
            Some(Ordering::Equal)
        }
    }
}

impl ProtocolVersion {
    pub fn to_spec(&self) -> ProtocolVersionSpec {
        match self {
            ProtocolVersion::Unknown => (-1, "n/a".to_string()),
            ProtocolVersion::Handshake => (-1, "n/a".to_string()),
            ProtocolVersion::V118R1 => (757, "1.18.1".to_string()),
            ProtocolVersion::V118R2 => (758, "1.18.2".to_string()),
            ProtocolVersion::V119R1 => (759, "1.19.1".to_string()),
        }
    }
}

impl From<i32> for ProtocolVersion {
    fn from(val: i32) -> Self {
        match val.into() {
            758 => ProtocolVersion::V118R2,
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
