use super::nums::VarInt;
use crate::prelude::*;
use anyhow::Context;
use std::convert::TryFrom;

#[macro_export]
macro_rules! auto_string {
    ($name:ident, $size:literal) => {
        #[derive(Debug)]
        pub struct $name(String);

        impl $crate::types::strings::McString for $name {
            fn new(internal: String) -> Self {
                $name(internal)
            }
            fn string(&self) -> &String {
                &self.0
            }
            fn limit() -> $crate::types::VarInt {
                $crate::types::VarInt::from($size as i32)
            }
        }

        impl From<String> for $name {
            fn from(internal: String) -> Self {
                $name(internal)
            }
        }

        impl From<&String> for $name {
            fn from(internal: &String) -> Self {
                $name(String::from(internal))
            }
        }

        impl From<&str> for $name {
            fn from(internal: &str) -> Self {
                $name(String::from(internal))
            }
        }

        impl From<$name> for String {
            fn from(internal: $name) -> Self {
                internal.0
            }
        }

        impl From<&$name> for String {
            fn from(internal: &$name) -> Self {
                String::from(&internal.0)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", &self)
            }
        }

        impl std::ops::Deref for $name {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                &$crate::types::McString::string(self)
            }
        }
    };
}

pub trait McString: Sized {
    fn new(internal: String) -> Self;

    fn string(&self) -> &String;

    fn limit() -> VarInt;
}

impl<T: McString> Decodable for T {
    fn decode<R: std::io::Read>(reader: &mut R) -> anyhow::Result<T> {
        let true_size = VarInt::decode(reader)?;

        if true_size > T::limit() * 4i32 {
            anyhow::bail!(
                "Failed to construct string with limit {:?} with given size {:?}.",
                T::limit(),
                true_size
            );
        }

        let mut bytes = vec![0u8; *true_size as usize];
        reader.read_exact(&mut bytes).context(format!(
            "Unexpected EOF while decoding string with size {:?}.",
            true_size
        ))?;
        let internal = String::from_utf8(bytes).context("Failed to build UTF-8 encoded string.")?;

        Ok(T::new(internal))
    }
}

impl<T: McString> Encodable for T {
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        let bytes = self.string().as_bytes();
        let length = super::VarInt::from(bytes.len() as i32);
        if length > T::limit() {
            anyhow::bail!(
                "Failed to encode string with limit {:?} with given size {}.",
                T::limit(),
                bytes.len()
            );
        }

        length.encode(writer)?;
        writer.write_all(bytes)?;

        Ok(())
    }

    fn size(&self) -> anyhow::Result<VarInt> {
        let string_len = VarInt::try_from(self.string().len())?;
        Ok(string_len.size()? + string_len)
    }
}

auto_string!(Identifier, 32767);
auto_string!(Chat, 262144);
