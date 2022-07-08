use minecraft_serde::primitive::VarInt;
use minecraft_serde::serde::ProtocolVersion;
use std::io::Cursor;

pub trait Mappings {
    type PacketType;

    fn attach_to_register<Context>(
        registry: &mut crate::registry::StateRegistry<Context>,
        handle: crate::registry::StateRegistryHandle<Context>,
    );

    fn create_packet(
        protocol_version: ProtocolVersion,
        buffer: Cursor<Vec<u8>>,
    ) -> crate::Result<Self::PacketType>;

    fn create_packet_buffer(
        protocol_version: ProtocolVersion,
        packet: Self::PacketType,
    ) -> crate::Result<Vec<u8>>;

    fn retrieve_packet_id(protocol_version: ProtocolVersion) -> VarInt;
}

#[macro_export]
macro_rules! create_mappings {
    ($($packet_ident:ident: $registrar_type:ty {
        def $packet_id:literal;
        $(def $latter_packet_id:literal ($protocol_from:path => $protocol_to:path) => $latter_mappings_type:ty;)*
    })*) => {
        $(
        pub struct $packet_ident;

        impl $crate::mappings::Mappings for $packet_ident {
            type PacketType = $registrar_type;

            fn attach_to_register<Context>(registry: &mut $crate::registry::StateRegistry<Context>, handle: $crate::registry::StateRegistryHandle<Context>) {
                registry.attach_mappings::<$packet_ident>(handle);
            }

            fn create_packet(_protocol_version: minecraft_serde::serde::ProtocolVersion, mut buffer: std::io::Cursor<Vec<u8>>) -> $crate::Result<Self::PacketType> {
                $(
                if _protocol_version >= $protocol_from && _protocol_version <= $protocol_to {
                    let intermediate: $latter_mappings_type = minecraft_serde::serde::Deserialize::deserialize_with_protocol(&mut buffer, _protocol_version)?;
                    return Ok(intermediate.into());
                }
                )*
                Ok(minecraft_serde::serde::Deserialize::deserialize_with_protocol(&mut buffer, _protocol_version)?)
            }

            fn create_packet_buffer(protocol_version: minecraft_serde::serde::ProtocolVersion, packet: Self::PacketType) -> $crate::Result<Vec<u8>> {
                let packet_id = Self::retrieve_packet_id(protocol_version);
                $(
                if _protocol_version >= $protocol_from && _protocol_version <= $protocol_to {
                    let mapped_packet: $latter_mappings_type = packet.into();

                    let packet_size = minecraft_serde::serde::Serialize::size(&packet_id)? +
                        minecraft_serde::serde::Serialize::size(&mapped_packet)?;
                    let mut buf = std::io::Cursor::new(Vec::with_capacity(packet_size as usize));

                    minecraft_serde::serde::Serialize::serialize(&packet_id, &mut buf)?;
                    minecraft_serde::serde::Serialize::serialize_with_protocol(&mapped_packet, &mut buf, protocol_version)?;
                    return Ok(buf.into_inner());
                }
                )*
                let packet_size = minecraft_serde::serde::Serialize::size(&packet_id)? +
                    minecraft_serde::serde::Serialize::size(&packet)?;
                let mut buf = std::io::Cursor::new(Vec::with_capacity(packet_size as usize));

                minecraft_serde::serde::Serialize::serialize(&packet_id, &mut buf)?;
                minecraft_serde::serde::Serialize::serialize_with_protocol(&packet, &mut buf, protocol_version)?;
                return Ok(buf.into_inner());
            }

            fn retrieve_packet_id(_protocol_version: minecraft_serde::serde::ProtocolVersion) -> minecraft_serde::primitive::VarInt {
                $(
                if _protocol_version >= $protocol_from && _protocol_version <= $protocol_to {
                    return minecraft_serde::primitive::VarInt::from($latter_packet_id)
                }
                )*
                return minecraft_serde::primitive::VarInt::from($packet_id);
            }
        }
        )*
    }
}

pub fn create_packet<M: Mappings>(
    protocol: ProtocolVersion,
    buffer: Cursor<Vec<u8>>,
) -> crate::Result<M::PacketType> {
    M::create_packet(protocol, buffer)
}

#[macro_export]
macro_rules! packet_handlers {
    ($(
        fn $function_ident:ident<$packet_mappings:ty, $context:ty>(
            $context_ident:ident,
            $registry_ident:ident,
            $packet_ident:ident
        ) -> anyhow::Result<()> {
            $($function_tokens:tt)*
        }
    )*) => {
        $(
        $crate::state_registry_handle! {
            fn $function_ident<$context>($context_ident, $registry_ident, protocol, buffer) -> anyhow::Result<()> {
                let $packet_ident = $crate::mappings::create_packet::<$packet_mappings>(protocol, buffer)?;
                $($function_tokens)*
                Ok(())
            }
        }
        )*
    }
}
