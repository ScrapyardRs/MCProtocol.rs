use crate::types::prelude::*;
use crate::Aes128Cfb8Enc;
use aes::cipher::BlockEncryptMut;
use flate2::bufread::ZlibEncoder;
use flate2::Compression;
use std::io::{Cursor, Read, Write};

pub type ProtocolVersionSpec = (i32, String);

#[derive(Debug, Eq, PartialOrd, PartialEq, Hash, Copy, Clone)]
pub enum ProtocolVersion {
    Handshake,
    V118R1,
    V118R2,
}

impl ProtocolVersion {
    pub fn to_spec(&self) -> ProtocolVersionSpec {
        match self {
            ProtocolVersion::Handshake => (-1, "n/a".to_string()),
            ProtocolVersion::V118R1 => (757, "1.18.1".to_string()),
            ProtocolVersion::V118R2 => (758, "1.18.2".to_string()),
        }
    }

    pub fn from_varint(varint: VarInt) -> Option<ProtocolVersion> {
        match *varint {
            757 => Some(ProtocolVersion::V118R1),
            758 => Some(ProtocolVersion::V118R2),
            _ => None,
        }
    }
}

pub trait Decodable: Sized {
    fn decode<R: std::io::Read>(reader: &mut R) -> anyhow::Result<Self>;
}

pub trait ProtocolDecodable: Sized {
    fn decode_from_protocol<R: std::io::Read>(
        protocol: ProtocolVersion,
        reader: &mut R,
    ) -> anyhow::Result<Self>;
}

impl<T> ProtocolDecodable for T
where
    T: Decodable,
{
    fn decode_from_protocol<R: Read>(_: ProtocolVersion, reader: &mut R) -> anyhow::Result<Self> {
        T::decode(reader)
    }
}

pub trait SizeDecodable: Sized {
    fn decode_sized<R: std::io::Read>(reader: &mut R, size: &VarInt) -> anyhow::Result<Self>;
}

pub trait Encodable {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> anyhow::Result<()>;

    fn size(&self) -> anyhow::Result<VarInt>;
}

pub trait ProtocolEncodable: Sized {
    fn encode_from_protocol<W: std::io::Write>(
        &self,
        protocol_target: ProtocolVersion,
        writer: &mut W,
    ) -> anyhow::Result<()>;

    fn size_from_protocol(&self, protocol_target: ProtocolVersion) -> anyhow::Result<VarInt>;
}

pub trait PacketEncodable: ProtocolEncodable {
    fn encode_packet_id<W: std::io::Write>(
        protocol_target: ProtocolVersion,
        writer: &mut W,
    ) -> anyhow::Result<()>;

    fn size_packet_id(protocol_target: ProtocolVersion) -> anyhow::Result<VarInt>;
}

pub trait PacketToCursor {
    fn to_cursor(
        &self,
        protocol_version: ProtocolVersion,
        encryption: Option<&mut crate::encryption::Aes128Cfb8Enc>,
        compression: Option<i32>,
    ) -> anyhow::Result<Cursor<Vec<u8>>>;
}

impl<P: PacketEncodable> PacketToCursor for P {
    fn to_cursor(
        &self,
        protocol_version: ProtocolVersion,
        encryption: Option<&mut Aes128Cfb8Enc>,
        compression: Option<i32>,
    ) -> anyhow::Result<Cursor<Vec<u8>>> {
        let initial_size =
            P::size_packet_id(protocol_version)? + self.size_from_protocol(protocol_version)?;

        let mut res: Vec<u8> = if let Some(compression_threshold) = compression {
            if initial_size > compression_threshold {
                let mut result = Cursor::new(Vec::with_capacity(
                    (initial_size.size()? + initial_size).try_into()?,
                ));

                P::encode_packet_id(protocol_version, &mut result)?;
                P::encode_from_protocol(self, protocol_version, &mut result)?;

                let inner = result.into_inner();
                let mut encoder = ZlibEncoder::new(inner.as_slice(), Compression::default());
                let mut compressed = Vec::new();
                encoder.read_to_end(&mut compressed)?;

                let compressed_length = VarInt::try_from(compressed.len())?;
                let uncompressed_length_length = initial_size.size()?;
                let total_length_data = compressed_length + uncompressed_length_length;

                let mut result = Cursor::new(Vec::with_capacity(
                    (total_length_data.size()? + uncompressed_length_length + compressed_length)
                        .try_into()?,
                ));
                total_length_data.encode(&mut result)?;
                initial_size.encode(&mut result)?;

                let mut inner = result.into_inner();
                inner.append(&mut compressed);
                inner
            } else {
                let compressed_length = VarInt::from(0i32);
                let uncompressed_length_length = initial_size.size()?;
                let total_length_data = VarInt::from(1i32) + initial_size;

                let mut result = Cursor::new(Vec::with_capacity(
                    (total_length_data.size()? + uncompressed_length_length + compressed_length)
                        .try_into()?,
                ));
                total_length_data.encode(&mut result)?;
                initial_size.encode(&mut result)?;
                P::encode_packet_id(protocol_version, &mut result)?;
                P::encode_from_protocol(self, protocol_version, &mut result)?;

                result.into_inner()
            }
        } else {
            let mut result = Cursor::new(Vec::with_capacity(
                (initial_size.size()? + initial_size).try_into()?,
            ));

            initial_size.encode(&mut result)?;
            P::encode_packet_id(protocol_version, &mut result)?;
            P::encode_from_protocol(self, protocol_version, &mut result)?;

            result.into_inner()
        };

        if let Some(encryption) = encryption {
            let mutable_inner = res.as_mut_slice();
            encryption.encrypt_block_mut(mutable_inner.into());
        }
        Ok(Cursor::new(res))
    }
}

impl<T> ProtocolEncodable for T
where
    T: Encodable,
{
    fn encode_from_protocol<W: Write>(
        &self,
        _: ProtocolVersion,
        writer: &mut W,
    ) -> anyhow::Result<()> {
        T::encode(self, writer)
    }

    fn size_from_protocol(&self, _: ProtocolVersion) -> anyhow::Result<VarInt> {
        T::size(self)
    }
}

pub trait SizeEncodable {
    fn encode_sized<W: std::io::Write>(&self, writer: &mut W, size: &VarInt) -> anyhow::Result<()>;

    fn predicted_size(&self) -> anyhow::Result<VarInt>;
}
