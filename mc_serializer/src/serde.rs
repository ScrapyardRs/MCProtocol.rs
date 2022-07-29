use crate::primitive::VarInt;
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::num::TryFromIntError;

use std::string::FromUtf8Error;

/// Defines the "context" around a serialization or deserialization.
/// ```
/// struct Example {
///     field: String,
/// }
/// ```
/// In this example, `Example` would be the current struct, `field` would be the current field,
/// `String` would be the serial type, and any extra details which the serializer can give it will.
#[derive(Debug)]
pub struct SerializerContext {
    current_struct: Option<String>,
    current_field: Option<String>,
    serial_type: String,
    details: String,
}

impl SerializerContext {
    /// Creates a new serializer context at the bottom level, given a serial type and other details.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::io::{Read, Write};
    /// # use mc_serializer::serde::{Result, Deserialize, Serialize, SerializerContext, Contextual, Error};
    ///
    /// fn serialize<W: Write>(option: bool, writer: &mut W) -> Result<()> {
    ///     writer
    ///         .write(&[if option { 0x01 } else { 0x00 }])
    ///         .map_err(|err| Error::IoError(err, SerializerContext::new(bool::context(), format!("N/A"))))
    ///         .map(|_| ())
    /// }
    /// ```
    pub fn new(serial_type: String, details: String) -> Self {
        Self {
            current_struct: None,
            current_field: None,
            serial_type,
            details,
        }
    }

    /// Updates the current field of the given serializer context, usually used in upwards chains.
    ///
    /// # Examples
    /// ```
    /// # use std::io::{Read, Write};
    /// # use mc_serializer::serde::{Result, Deserialize, Serialize, SerializerContext, Contextual, Error};
    ///
    /// fn serialize_bool<W: Write>(option: bool, writer: &mut W) -> Result<()> {
    ///     writer
    ///         .write(&[if option { 0x01 } else { 0x00 }])
    ///         .map_err(|err| Error::IoError(err, SerializerContext::new(format!("bool"), format!("N/A"))))
    ///         .map(|_| ())
    /// }
    ///
    /// struct Object { inner: bool }
    ///
    /// fn serialize_obj<W: Write>(obj: Object, writer: &mut W) -> Result<()> {
    ///     serialize_bool(obj.inner, writer).map_err(|err| err.update_context(|ctx| {
    ///         ctx.current_field(format!("inner"));
    ///     }))
    /// }
    /// ```
    pub fn current_field(&mut self, current_field: String) -> &mut Self {
        self.current_field = Some(current_field);
        self
    }

    /// Updates the current struct of the given serializer context, usually used in upwards chains.
    ///
    /// # Examples
    /// ```
    /// # use std::io::{Read, Write};
    /// # use mc_serializer::serde::{Result, Deserialize, Serialize, SerializerContext, Error};
    ///
    /// fn serialize_bool<W: Write>(option: bool, writer: &mut W) -> Result<()> {
    ///     writer
    ///         .write(&[if option { 0x01 } else { 0x00 }])
    ///         .map_err(|err| Error::IoError(err, SerializerContext::new(format!("bool"), format!("N/A"))))
    ///         .map(|_| ())
    /// }
    ///
    /// struct Object { inner: bool }
    ///
    /// fn serialize_obj<W: Write>(obj: Object, writer: &mut W) -> Result<()> {
    ///     serialize_bool(obj.inner, writer).map_err(|err| err.update_context(|ctx| {
    ///         ctx.current_struct(format!("Object"));
    ///     }))
    /// }
    /// ```
    pub fn current_struct(&mut self, current_struct: String) -> &mut Self {
        self.current_struct = Some(current_struct);
        self
    }
}

impl Display for SerializerContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Serializer Context: (current_struct: {}, current_field: {}, serial_type: {}), more info: {}",
            self.current_struct.as_ref().unwrap_or(&"unknown".to_string()),
            self.current_field.as_ref().unwrap_or(&"unknown".to_string()),
            self.serial_type,
            self.details.as_str()
        )
    }
}

/// Defines an error type generic over multiple possible errors coming from serialization.
/// This error type will always contain a `SerializerContext`.
#[derive(Debug)]
pub enum Error {
    /// Represents a case where no underlying error was thrown.
    Generic(SerializerContext),
    /// Represents an underlying `std::io::Error` being thrown during serialization.
    IoError(std::io::Error, SerializerContext),
    /// Represents an underlying `TryFromIntError` being thrown during serialization.
    TryFromIntError(TryFromIntError, SerializerContext),
    /// Represents an underlying `nbt::Error` being thrown during serialization.
    NbtError(nbt::Error, SerializerContext),
    /// Represents a UTF-8 error when deserializing a `MCString`.
    FromUtf8Error(FromUtf8Error, SerializerContext),
    /// Represents an unknown `std::error::Error` being thrown during serialization.
    Bubbled(Box<dyn std::error::Error>, SerializerContext),
}

impl Error {
    /// Updates the context of an error in-line.
    ///
    /// # Examples
    ///
    /// ```
    /// use mc_serializer::serde::{Error, Result, SerializerContext};
    ///
    /// fn throws() -> Result<()> {
    ///     Err(Error::Generic(SerializerContext::new(format!("throws"), format!("extra"))))
    /// }
    ///# assert!(
    /// throws().map_err(|err| err.update_context(|ctx| { ctx.current_field(format!("example")); } ))
    ///# .is_err())
    /// ```
    pub fn update_context<F: FnOnce(&mut SerializerContext)>(mut self, func: F) -> Error {
        match &mut self {
            Error::Generic(context) => (func)(context),
            Error::IoError(_, context) => (func)(context),
            Error::TryFromIntError(_, context) => (func)(context),
            Error::NbtError(_, context) => (func)(context),
            Error::FromUtf8Error(_, context) => (func)(context),
            Error::Bubbled(_, context) => (func)(context),
        };
        self
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Generic(context) => write!(f, "Failed to serialize data, {}", context),
            Error::IoError(io_error, context) => write!(f, "Failed to serialize data, {}, IoError: {}", context, io_error),
            Error::TryFromIntError(int_error, context) => write!(f, "Failed to serialize data, {}, IntError: {}", context, int_error),
            Error::NbtError(nbt_error, context) => write!(f, "Failed to serialize NBT data, {}, NbtError: {}", context, nbt_error),
            Error::FromUtf8Error(utf8_error, context) => write!(f, "Failed to parse utf8 data, {}, FromUtf8Error: {}", context, utf8_error),
            Error::Bubbled(error, context) => write!(f, "Generic error during serialization, {}, Error: {}", context, error),
        }
    }
}

impl std::error::Error for Error {}

/// Represents a `Result` of given type `<T>` and a `serde::Error`.
pub type Result<T> = std::result::Result<T, Error>;

/// Represents how a protocol is specified, a number value paired with a string version "name".
#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct ProtocolVersionSpec(pub i32, pub String);

/// Defines the protocols available in Minecraft which are supported in this library.
#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub enum ProtocolVersion {
    /// Post-protocol version when the protocol is clearly unknown to the library, the Unknown
    /// version is treated as the latest given revision.
    Unknown,
    /// Pre-protocol version when the protocol is unclear due to the lack of information rather
    /// the lack of ability to derive the information.
    Handshake,
    /// 1.19 Revision 1
    V119R1,
}

impl PartialOrd for ProtocolVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let that_protocol = Into::<ProtocolVersionSpec>::into(*other).0;
        let this_protocol = Into::<ProtocolVersionSpec>::into(*self).0;
        match that_protocol {
            x if this_protocol < x => Some(Ordering::Less),
            x if this_protocol > x => Some(Ordering::Greater),
            _ => Some(Ordering::Equal),
        }
    }
}

impl From<ProtocolVersion> for ProtocolVersionSpec {
    fn from(version: ProtocolVersion) -> Self {
        match version {
            ProtocolVersion::Unknown => ProtocolVersionSpec(99999, "n/a".to_string()),
            ProtocolVersion::Handshake => ProtocolVersionSpec(-1, "n/a".to_string()),
            ProtocolVersion::V119R1 => ProtocolVersionSpec(759, "1.19.1".to_string()),
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

/// Defines a contextual object which can provide debug information to itself during runtime.
pub trait Contextual {
    /// Represents a static representation of the serialization context.
    fn context() -> String;

    /// Returns a base serializer context with no details.
    fn base_context() -> SerializerContext {
        SerializerContext {
            current_struct: None,
            current_field: None,
            serial_type: Self::context(),
            details: "N/A".to_string(),
        }
    }
}

/// Represents an object which is serializable based on the Minecraft serialization specification.
pub trait Serialize: Contextual + Sized {
    /// Serializes a given object, given a `std::io::Writer` and a `ProtocolVersion` to map to.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::io::Cursor;
    /// # use mc_serializer::serde::Serialize;
    /// # use mc_serializer::serde::ProtocolVersion;
    /// let mut writer = Cursor::new(Vec::with_capacity(1));
    /// Serialize::serialize(&true, &mut writer, ProtocolVersion::Unknown).expect("Should serialize.");
    /// ```
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
        protocol_version: ProtocolVersion,
    ) -> Result<()>;

    /// Retrieves the size of a given serializable object. This is used to pre-condition buffers
    /// to hold the data of serialization.
    ///
    /// # Examples
    ///
    /// ```
    /// # use mc_serializer::serde::{ProtocolVersion, Serialize};
    /// assert_eq!(Serialize::size(&true).expect("Should size."), 1);
    /// ```
    fn size(&self) -> Result<i32>;
}

/// Represents an object which is deserializable based on the Minecraft serialization specification.
pub trait Deserialize: Contextual + Sized {
    /// Deserializes a given object, given a `std::io::Reader` and a `ProtocolVersion` to map to.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::io::Cursor;
    /// # use mc_serializer::serde::Deserialize;
    /// # use mc_serializer::serde::ProtocolVersion;
    /// let mut reader = Cursor::new(vec![0x01]);
    /// let out: bool = Deserialize::deserialize(&mut reader, ProtocolVersion::Unknown).expect("should deserialize");
    /// assert!(out);
    /// let mut reader = Cursor::new(vec![0x00]);
    /// let out: bool = Deserialize::deserialize(&mut reader, ProtocolVersion::Unknown).expect("should deserialize");
    /// assert!(!out);
    /// ```
    fn deserialize<R: std::io::Read>(
        reader: &mut R,
        protocol_version: ProtocolVersion,
    ) -> Result<Self>;
}