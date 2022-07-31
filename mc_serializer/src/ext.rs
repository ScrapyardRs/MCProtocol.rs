use crate::primitive::{read_string, size_string, VarInt, write_string};
use crate::serde::{Contextual, Deserialize, Error, InternalSizer, ProtocolVersion, Result, Serialize, SerializerContext};
use bytes::Buf;
use std::io::{Cursor, Read, Write};

impl<T: Contextual> Contextual for (VarInt, Vec<T>) {
    fn context() -> String {
        format!("({}, {})", VarInt::context(), Vec::<T>::context())
    }
}

impl<T: Serialize> Serialize for (VarInt, Vec<T>) {
    fn serialize<W: Write>(&self, writer: &mut W, protocol_version: ProtocolVersion) -> Result<()> {
        VarInt::serialize(&self.0, writer, protocol_version)?;
        for item in &self.1 {
            T::serialize(item, writer, protocol_version)?;
        }
        Ok(())
    }

    fn size(&self, protocol_version: ProtocolVersion) -> Result<i32> {
        let mut size = self.0.size(protocol_version)?;
        for item in &self.1 {
            size += item.size(protocol_version)?;
        }
        Ok(size)
    }
}

impl<T: Deserialize> Deserialize for (VarInt, Vec<T>) {
    fn deserialize<R: Read>(reader: &mut R, protocol_version: ProtocolVersion) -> Result<Self> {
        let v_size = VarInt::deserialize(reader, protocol_version)?;
        let mut result: Vec<T> = Vec::with_capacity(v_size.try_into().map_err(|err| Error::TryFromIntError(err, SerializerContext::new(Self::context(), format!("Failed to map {} to a valid usize.", v_size))))?);
        for _ in 0..v_size.into() {
            result.push(T::deserialize(reader, protocol_version)?);
        }
        Ok((v_size, result))
    }
}

impl<T: Contextual> Contextual for Vec<T> {
    fn context() -> String {
        format!("Vec<{}>", T::context())
    }
}

impl<T: Serialize> Serialize for Vec<T> {
    fn serialize<W: Write>(&self, writer: &mut W, protocol_version: ProtocolVersion) -> Result<()> {
        for item in self {
            T::serialize(item, writer, protocol_version)?;
        }
        Ok(())
    }

    fn size(&self, protocol_version: ProtocolVersion) -> Result<i32> {
        let mut size = 0;
        for item in self {
            size += T::size(item, protocol_version)?;
        }
        Ok(size)
    }
}

impl<T: Deserialize> Deserialize for Vec<T> {
    fn deserialize<R: Read>(reader: &mut R, protocol_version: ProtocolVersion) -> Result<Self> {
        let mut remaining = Vec::new();
        reader.read_to_end(&mut remaining).map_err(|err| Error::IoError(err, Self::base_context()))?;
        let mut remaining = Cursor::new(remaining);
        let mut result = Vec::new();
        while remaining.has_remaining() {
            result.push(T::deserialize(&mut remaining, protocol_version)?);
        }
        Ok(result)
    }
}

impl<T: Contextual> Contextual for Option<T> {
    fn context() -> String {
        format!("Option<{}>", T::context())
    }
}

impl<T: Contextual> Contextual for (bool, Option<T>) {
    fn context() -> String {
        format!("({}, {})", bool::context(), Option::<T>::context())
    }
}

impl<T: Serialize> Serialize for (bool, Option<T>) {
    fn serialize<W: Write>(&self, writer: &mut W, protocol_version: ProtocolVersion) -> Result<()> {
        bool::serialize(&self.0, writer, protocol_version)?;
        if self.0 {
            T::serialize(
                self.1
                    .as_ref()
                    .map(Ok)
                    .unwrap_or_else(|| Err(Error::Generic(SerializerContext::new(Self::context(), "Found None after deserializing true.".to_string()))))?,
                writer,
                protocol_version,
            )?;
        }
        Ok(())
    }

    fn size(&self, protocol_version: ProtocolVersion) -> Result<i32> {
        Ok(1 + match &self.1 {
            None => 0,
            Some(item) => T::size(item, protocol_version)?,
        })
    }
}

impl<T: Deserialize> Deserialize for (bool, Option<T>) {
    fn deserialize<R: Read>(reader: &mut R, protocol_version: ProtocolVersion) -> Result<Self> {
        let exists = bool::deserialize(reader, protocol_version)?;
        if exists {
            Ok((true, Some(T::deserialize(reader, protocol_version)?)))
        } else {
            Ok((exists, None))
        }
    }
}

impl Contextual for uuid::Uuid {
    fn context() -> String {
        "Uuid".to_string()
    }
}

impl Serialize for uuid::Uuid {
    fn serialize<W: Write>(&self, writer: &mut W, protocol_version: ProtocolVersion) -> Result<()> {
        let (most_significant, least_significant) = self.as_u64_pair();
        u64::serialize(&most_significant, writer, protocol_version)?;
        u64::serialize(&least_significant, writer, protocol_version)
    }

    fn size(&self, _: ProtocolVersion) -> Result<i32> {
        Ok(16)
    }
}

impl Deserialize for uuid::Uuid {
    fn deserialize<R: Read>(reader: &mut R, protocol_version: ProtocolVersion) -> Result<Self> {
        let (most_significant, least_significant) =
            (u64::deserialize(reader, protocol_version)?, u64::deserialize(reader, protocol_version)?);
        Ok(uuid::Uuid::from_u64_pair(
            most_significant,
            least_significant,
        ))
    }
}

impl Contextual for nbt::Blob {
    fn context() -> String {
        "NbtBlob".to_string()
    }
}

impl Serialize for nbt::Blob {
    fn serialize<W: Write>(&self, writer: &mut W, _: ProtocolVersion) -> Result<()> {
        self.to_writer(writer).map_err(|err| Error::Bubbled(Box::new(err), Self::base_context()))
    }

    fn size(&self, _: ProtocolVersion) -> Result<i32> {
        self.len_bytes().try_into().map_err(|err| Error::TryFromIntError(err, Self::base_context()))
    }
}

impl Deserialize for nbt::Blob {
    fn deserialize<R: Read>(reader: &mut R, _: ProtocolVersion) -> Result<Self> {
        nbt::Blob::from_reader(reader).map_err(|err| Error::NbtError(err, Self::base_context()))
    }
}

pub fn write_json<T, W: Write>(max_length: usize, value: &T, writer: &mut W, protocol_version: ProtocolVersion) -> Result<()>
    where
        T: Contextual + serde::ser::Serialize {
    let value_to_string = serde_json::to_string(value).map_err(|err| Error::SerdeJsonError(err, T::base_context()))?;
    write_string::<T, W>(max_length, &value_to_string, writer, protocol_version)
}

pub fn size_json<T>(value: &T, protocol_version: ProtocolVersion) -> Result<i32>
    where
        T: Contextual + serde::ser::Serialize {
    let value_to_string = serde_json::to_string(value).map_err(|err| Error::SerdeJsonError(err, T::base_context()))?;
    size_string::<T>(&value_to_string, protocol_version)
}

pub fn read_json<T, R: Read>(max_length: usize, reader: &mut R, protocol_version: ProtocolVersion) -> Result<T>
    where
        T: Contextual + for<'de> serde::de::Deserialize<'de> {
    let json_string = read_string::<T, R>(max_length, reader, protocol_version)?;
    serde_json::from_slice(json_string.as_bytes()).map_err(|err| Error::SerdeJsonError(err, T::base_context()))
}

pub fn write_nbt<T, W: Write>(value: &T, writer: &mut W, _: ProtocolVersion) -> Result<()>
    where
        T: Contextual + serde::ser::Serialize {
    nbt::ser::to_writer(writer, value, None).map_err(|err| Error::NbtError(err, T::base_context()))
}

pub fn size_nbt<T>(value: &T, protocol_version: ProtocolVersion) -> Result<i32>
    where
        T: Contextual + serde::ser::Serialize {
    let mut sizer = InternalSizer::default();
    write_nbt(value, &mut sizer, protocol_version)?;
    Ok(sizer.current_size())
}

pub fn read_nbt<T, R: Read>(reader: &mut R, _: ProtocolVersion) -> Result<T>
    where
        T: Contextual + for<'de> serde::de::Deserialize<'de> {
    nbt::de::from_reader(reader).map_err(|err| Error::NbtError(err, T::base_context()))
}
