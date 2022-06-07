use crate::packets::server_bound::status::StatusRequest;
use crate::{encryption_from_secret, Aes128Cfb8Enc, PacketToCursor, ProtocolVersion};
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio::net::tcp::OwnedWriteHalf;

pub struct PacketStub<'a> {
    protocol_version: ProtocolVersion,
    encryption: Option<&'a mut Aes128Cfb8Enc>,
    compression: Option<i32>,
}

impl From<ProtocolVersion> for PacketStub<'_> {
    fn from(protocol_version: ProtocolVersion) -> Self {
        Self {
            protocol_version,
            encryption: None,
            compression: None,
        }
    }
}

impl<'a> From<(ProtocolVersion, Option<&'a mut Aes128Cfb8Enc>)> for PacketStub<'a> {
    fn from(item: (ProtocolVersion, Option<&'a mut Aes128Cfb8Enc>)) -> Self {
        Self {
            protocol_version: item.0,
            encryption: item.1,
            compression: None,
        }
    }
}

impl From<(ProtocolVersion, Option<i32>)> for PacketStub<'_> {
    fn from(item: (ProtocolVersion, Option<i32>)) -> Self {
        Self {
            protocol_version: item.0,
            encryption: None,
            compression: item.1,
        }
    }
}

impl<'a> From<(ProtocolVersion, Option<&'a mut Aes128Cfb8Enc>, Option<i32>)> for PacketStub<'a> {
    fn from(item: (ProtocolVersion, Option<&'a mut Aes128Cfb8Enc>, Option<i32>)) -> Self {
        Self {
            protocol_version: item.0,
            encryption: item.1,
            compression: item.2,
        }
    }
}

pub async fn write_packet<
    'a,
    Packet: PacketToCursor,
    Stub: Into<PacketStub<'a>>,
    Write: AsyncWrite + Unpin,
>(
    packet: Packet,
    packet_stub: Stub,
    writer: &mut Write,
) -> anyhow::Result<()> {
    let stub: PacketStub = packet_stub.into();
    let cursor = packet.to_cursor(stub.protocol_version, stub.encryption, stub.compression)?;
    writer.write(cursor.into_inner().as_slice()).await?;
    Ok(())
}
