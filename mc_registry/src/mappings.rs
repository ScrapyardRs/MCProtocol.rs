use mc_serializer::primitive::VarInt;
use mc_serializer::serde::ProtocolVersion;
use std::io::Cursor;

pub trait Mappings {
    type PacketType;

    fn attach_to_register<'a, 'b: 'a, Context>(
        registry: &'b mut crate::registry::StateRegistry<'a, Context>,
        handle: crate::registry::StateRegistryHandle<'a, Context>,
    );

    fn create_packet(
        protocol_version: ProtocolVersion,
        buffer: Cursor<Vec<u8>>,
    ) -> crate::error::Result<Self::PacketType>;

    fn create_packet_buffer(
        protocol_version: ProtocolVersion,
        packet: Self::PacketType,
    ) -> crate::error::Result<Vec<u8>>;

    fn retrieve_packet_id(protocol_version: ProtocolVersion) -> crate::error::Result<VarInt>;
}

#[macro_export]
macro_rules! create_mappings {
    ($($registrar_type:ty {
        def $packet_id:literal;
        $(def $latter_packet_id:literal ($protocol_from:path => $protocol_to:path) => $latter_mappings_type:ty;)*
        $(since $since_protocol:path;)?
    })*) => {
        $(
        impl $crate::mappings::Mappings for $registrar_type {
            type PacketType = $registrar_type;

            fn attach_to_register<'a, 'b: 'a, Context>(registry: &'b mut $crate::registry::StateRegistry<'a, Context>, handle: $crate::registry::StateRegistryHandle<'a, Context>) {
                registry.attach_mappings::<'a, $registrar_type>(handle);
            }

            fn create_packet(_protocol_version: mc_serializer::serde::ProtocolVersion, mut buffer: std::io::Cursor<Vec<u8>>) -> $crate::error::Result<Self::PacketType> {
                $(
                if _protocol_version >= $protocol_from && _protocol_version <= $protocol_to {
                    let intermediate: $latter_mappings_type = mc_serializer::serde::Deserialize::deserialize(&mut buffer, _protocol_version)?;
                    return Ok(intermediate.into());
                }
                )*
                $(if _protocol_version < $since_protocol {
                    return Err($crate::error::Error::ProtocolInvalid($since_protocol, _protocol_version));
                })?
                Ok(mc_serializer::serde::Deserialize::deserialize(&mut buffer, _protocol_version)?)
            }

            fn create_packet_buffer(protocol_version: mc_serializer::serde::ProtocolVersion, packet: Self::PacketType) -> $crate::error::Result<Vec<u8>> {
                let packet_id = Self::retrieve_packet_id(protocol_version)?;
                $(
                if _protocol_version >= $protocol_from && _protocol_version <= $protocol_to {
                    let mapped_packet: $latter_mappings_type = packet.into();

                    let packet_size = mc_serializer::serde::Serialize::size(&packet_id, protocol_version)? +
                        mc_serializer::serde::Serialize::size(&mapped_packet, protocol_version)?;
                    let mut buf = std::io::Cursor::new(Vec::with_capacity(packet_size as usize));

                    mc_serializer::serde::Serialize::serialize(&packet_id, &mut buf, protocol_version)?;
                    mc_serializer::serde::Serialize::serialize(&mapped_packet, &mut buf, protocol_version)?;
                    return Ok(buf.into_inner());
                }
                )*
                $(if _protocol_version < $since_protocol {
                    return Err($crate::error::Error::ProtocolInvalid($since_protocol, _protocol_version));
                })?
                let packet_size = mc_serializer::serde::Serialize::size(&packet_id, protocol_version)? +
                    mc_serializer::serde::Serialize::size(&packet, protocol_version)?;
                let mut buf = std::io::Cursor::new(Vec::with_capacity(packet_size as usize));

                mc_serializer::serde::Serialize::serialize(&packet_id, &mut buf, protocol_version)?;
                mc_serializer::serde::Serialize::serialize(&packet, &mut buf, protocol_version)?;
                return Ok(buf.into_inner());
            }

            fn retrieve_packet_id(_protocol_version: mc_serializer::serde::ProtocolVersion) -> $crate::error::Result<mc_serializer::primitive::VarInt> {
                $(
                if _protocol_version >= $protocol_from && _protocol_version <= $protocol_to {
                    return mc_serializer::primitive::VarInt::from($latter_packet_id)
                }
                )*
                $(if _protocol_version < $since_protocol {
                    return Err($crate::error::Error::ProtocolInvalid($since_protocol, _protocol_version));
                })?
                return Ok(mc_serializer::primitive::VarInt::from($packet_id));
            }
        }
        )*
    }
}

pub fn create_packet<M: Mappings>(
    protocol: ProtocolVersion,
    buffer: Cursor<Vec<u8>>,
) -> crate::error::Result<M::PacketType> {
    M::create_packet(protocol, buffer)
}
