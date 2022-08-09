use crate::serde::{
    Contextual, Deserialize, Error, ProtocolVersion, ProtocolVersionSpec, Result, Serialize,
    SerializerContext,
};
use std::io::{Read, Write};

macro_rules! serde_primitive {
    ($prim_type:ty, $byte_count:literal) => {
        impl Contextual for $prim_type {
            fn context() -> String {
                format!("{}", stringify!($prim_type))
            }
        }

        impl Deserialize for $prim_type {
            fn deserialize<R: Read>(reader: &mut R, _: ProtocolVersion) -> Result<Self> {
                let mut bytes = [0u8; $byte_count];
                reader
                    .read_exact(&mut bytes)
                    .map_err(|err| Error::IoError(err, Self::base_context()))?;
                Ok(<$prim_type>::from_be_bytes(bytes))
            }
        }

        impl Serialize for $prim_type {
            fn serialize<W: Write>(&self, writer: &mut W, _: ProtocolVersion) -> Result<()> {
                writer
                    .write_all(&<$prim_type>::to_be_bytes(*self))
                    .map_err(|err| Error::IoError(err, Self::base_context()))
            }

            fn size(&self, _: ProtocolVersion) -> Result<i32> {
                Ok($byte_count)
            }
        }
    };
}

serde_primitive!(u8, 1);
serde_primitive!(i8, 1);
serde_primitive!(u16, 2);
serde_primitive!(i16, 2);
serde_primitive!(u32, 4);
serde_primitive!(i32, 4);
serde_primitive!(u64, 8);
serde_primitive!(i64, 8);
serde_primitive!(u128, 16);
serde_primitive!(i128, 16);
serde_primitive!(f32, 4);
serde_primitive!(f64, 8);

impl Contextual for bool {
    fn context() -> String {
        "bool".to_string()
    }
}

impl Serialize for bool {
    fn serialize<W: Write>(&self, writer: &mut W, _: ProtocolVersion) -> Result<()> {
        writer
            .write(&[if *self { 0x01 } else { 0x00 }])
            .map_err(|err| Error::IoError(err, Self::base_context()))
            .map(|_| ())
    }

    fn size(&self, _: ProtocolVersion) -> Result<i32> {
        Ok(1)
    }
}

impl Deserialize for bool {
    fn deserialize<R: Read>(reader: &mut R, _: ProtocolVersion) -> Result<Self> {
        let mut bytes = [0u8; 1];
        reader
            .read_exact(&mut bytes)
            .map_err(|err| Error::IoError(err, Self::base_context()))?;
        Ok(bytes[0] != 0x0)
    }
}

macro_rules! impl_into_num_bind {
    ($name:ident, $sim:ty, $prim:ty, $relationship:ident) => {
        impl From<$prim> for $name {
            fn from(prim: $prim) -> $name {
                $name(<$sim>::$relationship(prim))
            }
        }
    };
    ($name:ident, $sim:ty, $prim:ty, $relationship:ident | $rel_err:ty) => {
        impl TryFrom<$prim> for $name {
            type Error = $rel_err;
            fn try_from(prim: $prim) -> std::result::Result<$name, Self::Error> {
                Ok($name(<$sim>::$relationship(prim)?))
            }
        }
    };
}

macro_rules! impl_into_prim_bind {
    ($name:ident, $prim:ty, $alt_relationship:ident) => {
        impl From<$name> for $prim {
            fn from(item: $name) -> $prim {
                <$prim>::$alt_relationship(item.0)
            }
        }

        impl From<&$name> for $prim {
            fn from(item: &$name) -> $prim {
                <$prim>::$alt_relationship(item.0)
            }
        }
    };
    ($name:ident, $prim:ty, $alt_relationship:ident | $alt_err:ty) => {
        impl TryFrom<$name> for $prim {
            type Error = $alt_err;
            fn try_from(item: $name) -> std::result::Result<$prim, Self::Error> {
                <$prim>::$alt_relationship(item.0)
            }
        }

        impl TryFrom<&$name> for $prim {
            type Error = $alt_err;
            fn try_from(item: &$name) -> std::result::Result<$prim, Self::Error> {
                <$prim>::$alt_relationship(item.0)
            }
        }
    };
}

macro_rules! impl_ord_bind {
    ($name:ident, $sim:ty, $prim:ty, $relationship:ident) => {
        impl std::cmp::PartialEq<$prim> for $name {
            fn eq(&self, other: &$prim) -> bool {
                Self::eq(self, &<$sim>::$relationship(*other))
            }
        }

        impl std::cmp::PartialOrd<$prim> for $name {
            fn partial_cmp(&self, other: &$prim) -> Option<std::cmp::Ordering> {
                Self::partial_cmp(self, &<$sim>::$relationship(*other))
            }
        }
    };
}

macro_rules! impl_try_operators {
    ($name:ident, $sim:ty, $(($($to_impl:tt)*), $fn_name:ident;)*) => {
        $(
            impl $($to_impl)*<std::result::Result<$sim, std::num::TryFromIntError>> for $name {
                type Output = std::result::Result<$name, std::num::TryFromIntError>;
                fn $fn_name(self, rhs: std::result::Result<$sim, std::num::TryFromIntError>) -> Self::Output {
                    Ok($name(rhs?.$fn_name(self.0)))
                }
            }
            impl $($to_impl)*<std::result::Result<$sim, std::num::TryFromIntError>> for &$name {
                type Output = std::result::Result<$name, std::num::TryFromIntError>;
                fn $fn_name(self, rhs: std::result::Result<$sim, std::num::TryFromIntError>) -> Self::Output {
                    Ok($name(rhs?.$fn_name(self.0)))
                }
            }
        )*
    }
}

macro_rules! impl_primitive_operators {
    ($name:ident, $self:ty, $sim:ty, $prim:ty, $relationship:ident, $(($($to_impl:tt)*), $fn_name:ident,)*) => {
        $(
            impl $($to_impl)*<$self> for $prim {
                type Output = $name;
                fn $fn_name(self, rhs: $self) -> Self::Output {
                    <$sim>::$relationship(self).$fn_name(rhs)
                }
            }

            impl $($to_impl)*<$prim> for $self {
                type Output = $name;
                fn $fn_name(self, rhs: $prim) -> Self::Output {
                    self.$fn_name(<$sim>::$relationship(rhs))
                }
            }
        )*
    };
    ($name:ident, $self:ty, $sim:ty, $prim:ty, $relationship:ident, $rel_err:ty, $(($($to_impl:tt)*), $fn_name:ident,)*) => {
        $(
            impl $($to_impl)*<$self> for $prim {
                type Output = std::result::Result<$name, $rel_err>;
                fn $fn_name(self, rhs: $self) -> Self::Output {
                    Ok(<$sim>::$relationship(self)?.$fn_name(rhs))
                }
            }

            impl $($to_impl)*<$prim> for $self {
                type Output = std::result::Result<$name, $rel_err>;
                fn $fn_name(self, rhs: $prim) -> Self::Output {
                    Ok(self.$fn_name(<$sim>::$relationship(rhs)?))
                }
            }
        )*
    }
}

macro_rules! impl_variable_number_bind {
    ($name:ident, $self:ty, $sim:ty, $prim:ty, $relationship:ident $(|$rel_err:ty)?) => {
        impl_primitive_operators!($name, $self, $sim, $prim, $relationship, $($rel_err,)*
            (std::ops::Mul), mul,
            (std::ops::Add), add,
            (std::ops::Sub), sub,
        );
    };
    ($name:ident, $sim:ty, $($prim:ty: ($relationship:ident $(|$rel_err:ty)?, $alt_relationship:ident $(|$alt_err:ty)?),)*) => {
        impl_try_operators!($name, $sim,
            (std::ops::Mul), mul;
            (std::ops::Add), add;
            (std::ops::Sub), sub;
        );
        impl std::cmp::PartialEq<std::result::Result<$sim, std::num::TryFromIntError>> for $name {
            fn eq(&self, other: &std::result::Result<$sim, std::num::TryFromIntError>) -> bool {
                if let Ok(internal) = other {
                    internal == &self.0
                } else {
                    false
                }
            }
        }
        impl std::cmp::PartialOrd<std::result::Result<$sim, std::num::TryFromIntError>> for $name {
            fn partial_cmp(&self, other: &std::result::Result<$sim, std::num::TryFromIntError>) -> Option<std::cmp::Ordering> {
                if let Ok(internal) = other {
                    self.0.partial_cmp(&internal)
                } else {
                    None
                }
            }
        }
        $(
            impl_variable_number_bind!($name, &$name, $sim, $prim, $relationship $(|$rel_err)*);
            impl_variable_number_bind!($name, $name, $sim, $prim, $relationship $(|$rel_err)*);
            impl_ord_bind!($name, $sim, $prim, $relationship);
            impl_into_prim_bind!($name, $prim, $alt_relationship $(|$alt_err)*);
            impl_into_num_bind!($name, $sim, $prim, $relationship $(|$rel_err)*);
        )*
    }
}

macro_rules! declare_variable_number {
    ($name:ident, $primitive_signed:ty, $bit_limit:literal, $primitive_unsigned:ty, $and_check:literal $(,
        $prim:ty: ($relationship:ident $(|$rel_err:ty)?, $alt_relationship:ident $(|$alt_err:ty)?)
    )*) => {
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
        pub struct $name($primitive_signed);

        impl_variable_number_bind!($name, $primitive_signed, $(
            $prim: ($relationship $(|$rel_err)*, $alt_relationship $(|$alt_err)*),
        )*);

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl $name {
            pub fn decode_and_size(reader: &mut impl std::io::Read) -> Result<(i32, Self)> {
                let mut running_size = 0;
                let mut value: $primitive_signed = 0;
                let mut bit_offset = 0u32;
                loop {
                    if bit_offset == $bit_limit {
                        return Err(Error::Generic(SerializerContext::new(Self::context(), format!("More bytes than expected, current value: {}.", value))));
                    }

                    let mut buf = [0; 1];
                    reader.read_exact(&mut buf).map_err(|err| Error::IoError(err, Self::base_context()))?;
                    running_size += 1;
                    let byte = buf[0];
                    value |= <$primitive_signed>::from(byte & 0b01111111)
                        .overflowing_shl(bit_offset)
                        .0;
                    bit_offset += 7;

                    if byte & 0b10000000 == 0 {
                        break;
                    }
                }
                Ok((running_size, $name(value)))
            }
        }

        impl Contextual for $name {
            fn context() -> String {
                format!("{}", stringify!($name))
            }
        }

        impl Deserialize for $name {
            fn deserialize<R: std::io::Read>(reader: &mut R, _: ProtocolVersion) -> Result<Self> {
                <$name>::decode_and_size(reader).map(|res| res.1)
            }
        }

        impl Serialize for $name {
            fn serialize<W: std::io::Write>(&self, writer: &mut W, _: ProtocolVersion) -> Result<()> {
                let mut temp = self.0.clone() as $primitive_unsigned;
                loop {
                    if temp & $and_check == 0 {
                        writer.write_all(&[temp as u8]).map_err(|err| Error::IoError(err, Self::base_context()))?;
                        return Ok(());
                    }
                    writer.write_all(&[(temp & 0x7F | 0x80) as u8]).map_err(|err| Error::IoError(err, Self::base_context()))?;
                    temp = temp.overflowing_shr(7).0;
                }
            }

            fn size(&self, _: ProtocolVersion) -> Result<i32> {
                let mut running_size: i32 = 0;
                let mut temp = self.0.clone() as $primitive_unsigned;
                loop {
                    if temp & $and_check == 0 {
                        running_size += 1;
                        return Ok(running_size);
                    }
                    running_size += 1;
                    temp = temp.overflowing_shr(7).0;
                }
            }
        }

        // extensions

        impl std::ops::Mul for &$name {
            type Output = $name;

            fn mul(self, rhs: Self) -> Self::Output {
                $name(self.0 * rhs.0)
            }
        }

        impl std::ops::Mul for $name {
            type Output = $name;

            fn mul(self, rhs: Self) -> Self::Output {
                $name(self.0 * rhs.0)
            }
        }

        impl std::ops::Add for &$name {
            type Output = $name;

            fn add(self, rhs: Self) -> Self::Output {
                $name(self.0 + rhs.0)
            }
        }

        impl std::ops::Add for $name {
            type Output = Self;

            fn add(self, rhs: Self) -> Self::Output {
                $name(self.0 + rhs.0)
            }
        }

        impl std::ops::Deref for $name {
            type Target = $primitive_signed;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        // prim sign functions
        impl std::ops::Mul<&$name> for $primitive_signed {
            type Output = $name;
            fn mul(self, rhs: &$name) -> Self::Output {
                $name(self * rhs.0)
            }
        }
        impl std::ops::Mul<$primitive_signed> for &$name {
            type Output = $name;
            fn mul(self, rhs: $primitive_signed) -> Self::Output {
                $name(self.0 * rhs)
            }
        }
        impl std::ops::Add<&$name> for $primitive_signed {
            type Output = $name;
            fn add(self, rhs: &$name) -> Self::Output {
                $name(self + rhs.0)
            }
        }
        impl std::ops::Add<$primitive_signed> for &$name {
            type Output = $name;
            fn add(self, rhs: $primitive_signed) -> Self::Output {
                $name(self.0 + rhs)
            }
        }
        impl std::ops::Sub<&$name> for $primitive_signed {
            type Output = $name;
            fn sub(self, rhs: &$name) -> Self::Output {
                $name(self - rhs.0)
            }
        }
        impl std::ops::Sub<$primitive_signed> for &$name {
            type Output = $name;
            fn sub(self, rhs: $primitive_signed) -> Self::Output {
                $name(self.0 - rhs)
            }
        }
        impl std::ops::Mul<$name> for $primitive_signed {
            type Output = $name;
            fn mul(self, rhs: $name) -> Self::Output {
                $name(self * rhs.0)
            }
        }
        impl std::ops::Mul<$primitive_signed> for $name {
            type Output = $name;
            fn mul(self, rhs: $primitive_signed) -> Self::Output {
                $name(self.0 * rhs)
            }
        }
        impl std::ops::Add<$name> for $primitive_signed {
            type Output = $name;
            fn add(self, rhs: $name) -> Self::Output {
                $name(self + rhs.0)
            }
        }
        impl std::ops::Add<$primitive_signed> for $name {
            type Output = $name;
            fn add(self, rhs: $primitive_signed) -> Self::Output {
                $name(self.0 + rhs)
            }
        }
        impl std::ops::Sub<$name> for $primitive_signed {
            type Output = $name;
            fn sub(self, rhs: $name) -> Self::Output {
                $name(self - rhs.0)
            }
        }
        impl std::ops::Sub<$primitive_signed> for $name {
            type Output = $name;
            fn sub(self, rhs: $primitive_signed) -> Self::Output {
                $name(self.0 - rhs)
            }
        }
        impl std::cmp::PartialEq<$primitive_signed> for $name {
            fn eq(&self, other: &$primitive_signed) -> bool {
                self.0 == *other
            }
        }
        impl std::cmp::PartialOrd<$primitive_signed> for $name {
            fn partial_cmp(&self, other: &$primitive_signed) -> Option<std::cmp::Ordering> {
                self.0.partial_cmp(other)
            }
        }
        impl From<$primitive_signed> for $name {
            fn from(prim: $primitive_signed) -> Self {
                Self(prim)
            }
        }
        impl From<$name> for $primitive_signed {
            fn from(t: $name) -> $primitive_signed {
                t.0
            }
        }

        impl From<&$name> for $primitive_signed {
            fn from(t: &$name) -> $primitive_signed {
                t.0
            }
        }

        impl std::ops::AddAssign for $name {
            fn add_assign(&mut self, rhs: Self) {
                self.0 = self.0 + rhs.0;
            }
        }
    };
}

declare_variable_number!(VarInt, i32, 35, u32, 0xFFFFFF80,
    u8: (from, try_from | std::num::TryFromIntError),
    i8: (from, try_from | std::num::TryFromIntError),
    u16: (from, try_from | std::num::TryFromIntError),
    i16: (from, try_from | std::num::TryFromIntError),
    u32: (try_from | std::num::TryFromIntError, try_from | std::num::TryFromIntError),
    u64: (try_from | std::num::TryFromIntError, try_from | std::num::TryFromIntError),
    i64: (try_from | std::num::TryFromIntError, from),
    u128: (try_from | std::num::TryFromIntError, try_from | std::num::TryFromIntError),
    i128: (try_from | std::num::TryFromIntError, from),
    usize: (try_from | std::num::TryFromIntError, try_from | std::num::TryFromIntError),
    isize: (try_from | std::num::TryFromIntError, try_from | std::num::TryFromIntError)
);

declare_variable_number!(VarLong, i64, 70, u64, 0xFFFFFFFFFFFFFF80,
    u8: (from, try_from | std::num::TryFromIntError),
    i8: (from, try_from | std::num::TryFromIntError),
    u16: (from, try_from | std::num::TryFromIntError),
    i16: (from, try_from | std::num::TryFromIntError),
    u32: (from, try_from | std::num::TryFromIntError),
    i32: (from, try_from | std::num::TryFromIntError),
    u64: (try_from | std::num::TryFromIntError, try_from | std::num::TryFromIntError),
    u128: (try_from | std::num::TryFromIntError, try_from | std::num::TryFromIntError),
    i128: (try_from | std::num::TryFromIntError, from),
    usize: (try_from | std::num::TryFromIntError, try_from | std::num::TryFromIntError),
    isize: (try_from | std::num::TryFromIntError, try_from | std::num::TryFromIntError)
);

use crate::serde::Error::Generic;
use std::convert::TryFrom;

/// Creates a length-bounded serializable string.
///
/// # Examples
///
/// ```
/// # use mc_serializer::{auto_string, primitive::McString};
/// # use mc_serializer::primitive::VarInt;
/// auto_string!(Example, 200);
/// assert_eq!(Example::limit(), VarInt::from(200));
/// assert_eq!("Example", Example::from("Example").string().as_str())
/// ```
#[macro_export]
macro_rules! auto_string {
    ($name:ident, $size:literal) => {
        #[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct $name(String);

        impl $crate::primitive::McString for $name {
            fn new(internal: String) -> Self {
                $name(internal)
            }
            fn string(&self) -> &String {
                &self.0
            }
            fn limit() -> $crate::primitive::VarInt {
                $crate::primitive::VarInt::from($size as i32)
            }
            fn context() -> String {
                format!("{}", stringify!($name))
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
                internal.0.to_string()
            }
        }

        impl From<&$name> for String {
            fn from(internal: &$name) -> Self {
                String::from(&internal.0)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", &self.0)
            }
        }

        impl std::ops::Deref for $name {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                &$crate::primitive::McString::string(self)
            }
        }
    };
}

pub trait McString: Sized {
    fn new(internal: String) -> Self;

    fn string(&self) -> &String;

    fn limit() -> VarInt;

    fn context() -> String;
}

impl<T: McString> Deserialize for T {
    fn deserialize<R: Read>(reader: &mut R, protocol_version: ProtocolVersion) -> Result<T> {
        Ok(T::new(read_string::<Self, R>(
            *T::limit() as usize,
            reader,
            protocol_version,
        )?))
    }
}

impl<T: McString> Contextual for T {
    fn context() -> String {
        T::context()
    }
}

impl<T: McString> Serialize for T {
    fn serialize<W: Write>(&self, writer: &mut W, protocol_version: ProtocolVersion) -> Result<()> {
        write_string::<Self, W>(
            *T::limit() as usize,
            self.string(),
            writer,
            protocol_version,
        )
    }

    fn size(&self, protocol_version: ProtocolVersion) -> Result<i32> {
        size_string::<Self>(self.string(), protocol_version)
    }
}

pub fn write_string_checked<C: Contextual, W: Write>(
    bytes: &[u8],
    writer: &mut W,
    protocol_version: ProtocolVersion,
) -> Result<()> {
    let length = VarInt::from(bytes.len() as i32);

    length.serialize(writer, protocol_version)?;
    writer
        .write_all(bytes)
        .map_err(|err| Error::IoError(err, C::base_context()))?;
    Ok(())
}

pub fn write_string<C: Contextual, W: Write>(
    max_length: usize,
    string: &String,
    writer: &mut W,
    protocol_version: ProtocolVersion,
) -> Result<()> {
    let bytes = string.as_bytes();
    let length = VarInt::from(bytes.len() as i32);
    if length > max_length * 3 {
        return Err(Generic(SerializerContext::new(
            C::context(),
            format!(
                "Attempted to write string of length {} when max is {}.",
                length,
                max_length * 4
            ),
        )));
    }
    if length < 0 {
        return Err(Generic(SerializerContext::new(
            C::context(),
            format!(
                "Cannot read a string of less than 0 length. Given {}.",
                length
            ),
        )));
    }
    write_string_checked::<C, W>(bytes, writer, protocol_version)
}

pub fn read_string_checked<C: Contextual, R: Read>(
    length: usize,
    reader: &mut R,
    _: ProtocolVersion,
) -> Result<String> {
    let mut bytes = vec![0u8; length];
    reader
        .read_exact(&mut bytes)
        .map_err(|err| Error::IoError(err, C::base_context()))?;
    let internal =
        String::from_utf8(bytes).map_err(|err| Error::FromUtf8Error(err, C::base_context()))?;
    Ok(internal)
}

pub fn read_string<C: Contextual, R: Read>(
    max_length: usize,
    reader: &mut R,
    protocol_version: ProtocolVersion,
) -> Result<String> {
    let length = VarInt::deserialize(reader, protocol_version)?;
    if length > max_length * 3 {
        return Err(Generic(SerializerContext::new(
            C::context(),
            format!(
                "Attempted to read string of length {} when max is {}.",
                length,
                max_length * 4
            ),
        )));
    }
    if length < 0 {
        return Err(Generic(SerializerContext::new(
            C::context(),
            format!(
                "Cannot read a string of less than 0 length. Given {}.",
                length
            ),
        )));
    }
    read_string_checked::<C, R>(*length as usize, reader, protocol_version)
}

pub fn size_string<C: Contextual>(
    value: &String,
    protocol_version: ProtocolVersion,
) -> Result<i32> {
    let string_len = VarInt::try_from(value.len())
        .map_err(|err| Error::TryFromIntError(err, C::base_context()))?;
    Ok((string_len.size(protocol_version)? + string_len).into())
}

impl From<ProtocolVersion> for VarInt {
    fn from(ver: ProtocolVersion) -> Self {
        VarInt(Into::<ProtocolVersionSpec>::into(ver).0)
    }
}

auto_string!(Identifier, 32767);
