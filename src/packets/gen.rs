#[macro_export]
macro_rules! packets {
    ($($packet_name:ident {
        $(
            field $field_name:ident: $field_type:ty,
        )*
        $(
            mapping $protocol_version:path => $packet_id:literal $(& $inner_protocol_version:path)* {
                $(for $mapping_field_name:ident = {$self_ident:ident -> $($equal_tt:tt)+}
                    $(| auto as $auto_field_type:ty $(, if ($($auto_bool_expr:tt)*))?)?
                    $(| ser as $ser_field_type:ty $(, if ($($ser_bool_expr:tt)*))?)?
                    $(| de as $de_field_type:ty $(, if ($($de_bool_expr:tt)*))?)?
                )*

                = deserializer {
                    $($de_response_token:tt)+
                }
            }
        )*
    })*) => {
        $(
            #[derive(Debug)]
            pub struct $packet_name {$(
                pub $field_name: $field_type,
            )*}

            impl $packet_name {
                #[allow(clippy::new_without_default)]
                pub fn new($($field_name: $field_type,)*) -> Self {
                    Self { $($field_name,)* }
                }
            }

            impl $crate::packets::StaticProtocolMappings for $packet_name {
                fn get_protocol_mappings() -> Vec<($crate::encoding::ProtocolVersion, $crate::types::nums::VarInt)> {
                    vec![
                        $(
                        ($protocol_version, $crate::types::nums::VarInt::from($packet_id))
                        $(
                            ,($inner_protocol_version, $crate::types::nums::VarInt::from($inner_packet_id))
                        )*
                        ,)*
                    ]
                }
            }

            impl $crate::encoding::ProtocolDecodable for $packet_name {
                fn decode_from_protocol<R: std::io::Read>(protocol: ProtocolVersion, _reader: &mut R) -> anyhow::Result<Self> {
                    match protocol {
                        $(
                            $protocol_version $(|$inner_protocol_version)* => {
                                $(
                                    #[allow(unused_variables)]
                                    let $mapping_field_name =
                                    $(
                                        $(
                                            if !$($auto_bool_expr)* {
                                                None
                                            } else
                                        )?
                                        { <$auto_field_type>::decode(_reader)? };
                                    )?
                                    $(
                                        $(
                                            if !$($de_bool_expr)* {
                                                None
                                            } else
                                        )?
                                        { <$de_field_type>::decode(_reader)? };
                                    )?
                                )*

                                $($de_response_token)*
                            }
                        )*
                        #[allow(unreachable_patterns)]
                        _ => anyhow::bail!("Protocol unsupported for {}. Protocol required {protocol:?}", stringify!($packet_name)),
                    }
                }
            }

            impl $crate::encoding::ProtocolEncodable for $packet_name {
                fn encode_from_protocol<W: std::io::Write>(&self, protocol_target: ProtocolVersion, _writer: &mut W) -> anyhow::Result<()> {
                    match protocol_target {
                        $(
                            $protocol_version $(|$inner_protocol_version)*
                            => {
                                $(
                                    let $mapping_field_name = {
                                        let $self_ident = self;
                                        $($equal_tt)*
                                    };
                                    $(
                                        $(
                                            if !$($auto_bool_expr)*
                                        )?
                                        { <$auto_field_type>::encode(&$mapping_field_name, _writer)? };
                                    )?
                                    $(
                                        $(
                                            if $($ser_bool_expr)*
                                        )?
                                        { <$ser_field_type>::encode(&$mapping_field_name, _writer)? };
                                    )?
                                )*

                                Ok(())
                            }
                        )*
                        #[allow(unreachable_patterns)]
                        _ => anyhow::bail!("Protocol unsupported for {}. Protocol required: {protocol_target:?}", stringify!($packet_name)),
                    }
                }

                fn size_from_protocol(&self, protocol_target: ProtocolVersion) -> anyhow::Result<VarInt> {
                    match protocol_target {
                        $(
                            $protocol_version $(|$inner_protocol_version)*
                            => {
                                #[allow(unused_mut)]
                                let mut size: $crate::types::VarInt = 0i32.into();
                                $(
                                    let $mapping_field_name = {
                                        let $self_ident = self;
                                        $($equal_tt)*
                                    };
                                    $(
                                        $(
                                            if !$($auto_bool_expr)*
                                        )?
                                        { size = size + <$auto_field_type>::size(&$mapping_field_name)? };
                                    )?
                                    $(
                                        $(
                                            if $($ser_bool_expr)*
                                        )?
                                        { size = size + <$ser_field_type>::size(&$mapping_field_name)? };
                                    )?
                                )*
                                Ok(size.into())
                            }
                        )*
                        #[allow(unreachable_patterns)]
                        _ => anyhow::bail!("Protocol unsupported for {}. Protocol required: {protocol_target:?}", stringify!($packet_name)),
                    }
                }
            }

            impl $crate::encoding::PacketEncodable for $packet_name {
                fn encode_packet_id<W: std::io::Write>(protocol_target: ProtocolVersion, writer: &mut W) -> anyhow::Result<()> {
                    match protocol_target {
                        $(
                            $protocol_version $(|$inner_protocol_version)*
                            => {
                                $crate::types::VarInt::encode(&From::<i32>::from($packet_id), writer)
                            }
                        )*
                        #[allow(unreachable_patterns)]
                        _ => anyhow::bail!("Protocol unsupported for {}. Protocol required: {protocol_target:?}", stringify!($packet_name)),
                    }
                }

                fn size_packet_id(protocol_target: ProtocolVersion) -> anyhow::Result<VarInt> {
                    match protocol_target {
                        $(
                            $protocol_version $(|$inner_protocol_version)*
                            => {
                                $crate::types::VarInt::size(&From::<i32>::from($packet_id))
                            }
                        )*
                        #[allow(unreachable_patterns)]
                        _ => anyhow::bail!("Protocol unsupported for {}. Protocol required: {protocol_target:?}", stringify!($packet_name)),
                    }
                }
            }

            impl $crate::packets::packet::ProtocolSheetEchoCandidate for $packet_name {
                fn echo_packet_handle<Context: Send + Sync>() -> $crate::packets::packet::MetaPacketHandle<Context, Self> {
                    Box::new(|_, _, handle| {
                        println!("Echo {handle:?}");
                        Ok(())
                    })
                }
            }
        )*
    }
}

#[macro_export]
macro_rules! auto_enum {
    ($(
        $enum_name:ident by $index_type:ty $(| $arbitrary_star:tt)? {$(
            $prim_lit:expr; $enum_variant:ident {
                $(
                    $enum_field:ident: $enum_type:ty,
                )*
            }
        )*}
    )+) => {
        $(
            #[derive(Debug)]
            pub enum $enum_name {
                $(
                    $enum_variant {
                        $(
                            $enum_field: $enum_type,
                        )*
                    },
                )*
            }

            impl $crate::encoding::Decodable for ($index_type, $enum_name) {
                fn decode<R: std::io::Read>(reader: &mut R) -> anyhow::Result<Self> {
                    let index: $index_type = <$index_type>::decode(reader)?;
                    match $($arbitrary_star)?index {
                        $($prim_lit => {
                            Ok((index, $enum_name::$enum_variant {
                                $(
                                    $enum_field: <$enum_type>::decode(reader)?,
                                )*
                            }))
                        },)*
                        _ => anyhow::bail!("Did not understand index {index:?}."),
                    }
                }
            }

            impl $crate::encoding::Encodable for ($index_type, $enum_name) {
                fn encode<W: std::io::Write>(&self, writer: &mut W) -> anyhow::Result<()> {
                    match &self.1 {
                        $(
                            $enum_name::$enum_variant {
                                $(
                                    $enum_field,
                                )*
                            } => {
                                self.0.encode(writer)?;
                                $($enum_field.encode(writer)?;)*
                                Ok(())
                            }
                        )*
                    }
                }

                fn size(&self) -> anyhow::Result<$crate::types::VarInt> {
                    match &self.1 {
                        $(
                            $enum_name::$enum_variant {
                                $(
                                    $enum_field,
                                )*
                            } => {
                                #[allow(unused_mut)]
                                let mut size = VarInt::from(1);
                                $(size = size + $enum_field.size()?;)*
                                Ok(size)
                            }
                        )*
                    }
                }
            }
        )+
    }
}
