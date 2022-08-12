use crate::primitive::{read_string, size_string, write_string, VarInt};
use crate::serde::{
    Contextual, Deserialize, Error, InternalSizer, ProtocolVersion, Result, Serialize,
    SerializerContext,
};
use crate::{wrap_indexed_struct_context, wrap_struct_context};
use bytes::Buf;

use mc_level::codec::Codec;

use std::collections::HashMap;
use std::hash::Hash;
use std::io::{Cursor, ErrorKind, Read, Write};

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
        let mut result: Vec<T> = Vec::with_capacity(v_size.try_into().map_err(|err| {
            Error::TryFromIntError(
                err,
                SerializerContext::new(
                    Self::context(),
                    format!("Failed to map {} to a valid usize.", v_size),
                ),
            )
        })?);
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
        reader
            .read_to_end(&mut remaining)
            .map_err(|err| Error::IoError(err, Self::base_context()))?;
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

impl<T: Serialize> Serialize for Option<T> {
    fn serialize<W: Write>(&self, writer: &mut W, protocol_version: ProtocolVersion) -> Result<()> {
        if let Some(t) = self.as_ref() {
            T::serialize(t, writer, protocol_version)?;
        }
        Ok(())
    }

    fn size(&self, protocol_version: ProtocolVersion) -> Result<i32> {
        if let Some(t) = self.as_ref() {
            return T::size(t, protocol_version);
        }
        Ok(0)
    }
}

impl<T: Deserialize> Deserialize for Option<T> {
    fn deserialize<R: Read>(reader: &mut R, protocol_version: ProtocolVersion) -> Result<Self> {
        T::deserialize(reader, protocol_version).map(|t| Some(t))
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
                self.1.as_ref().map(Ok).unwrap_or_else(|| {
                    Err(Error::Generic(SerializerContext::new(
                        Self::context(),
                        "Found None after serializing true.".to_string(),
                    )))
                })?,
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
        let (most_significant, least_significant) = (
            u64::deserialize(reader, protocol_version)?,
            u64::deserialize(reader, protocol_version)?,
        );
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
        self.to_writer(writer)
            .map_err(|err| Error::NbtError(err, Self::base_context()))
    }

    fn size(&self, _: ProtocolVersion) -> Result<i32> {
        self.len_bytes()
            .try_into()
            .map_err(|err| Error::TryFromIntError(err, Self::base_context()))
    }
}

impl Deserialize for nbt::Blob {
    fn deserialize<R: Read>(reader: &mut R, _: ProtocolVersion) -> Result<Self> {
        nbt::Blob::from_reader(reader).map_err(|err| Error::NbtError(err, Self::base_context()))
    }
}

pub fn write_json<T, W: Write>(
    max_length: usize,
    value: &T,
    writer: &mut W,
    protocol_version: ProtocolVersion,
) -> Result<()>
where
    T: Contextual + serde::ser::Serialize,
{
    let value_to_string = serde_json::to_string(value)
        .map_err(|err| Error::SerdeJsonError(err, T::base_context()))?;
    write_string::<W>(max_length, &value_to_string, writer, protocol_version)
}

pub fn size_json<T>(value: &T, protocol_version: ProtocolVersion) -> Result<i32>
where
    T: Contextual + serde::ser::Serialize,
{
    let value_to_string = serde_json::to_string(value)
        .map_err(|err| Error::SerdeJsonError(err, T::base_context()))?;
    size_string::<T>(&value_to_string, protocol_version)
}

pub fn read_json<T, R: Read>(
    max_length: usize,
    reader: &mut R,
    protocol_version: ProtocolVersion,
) -> Result<T>
where
    T: Contextual + for<'de> serde::de::Deserialize<'de>,
{
    let json_string = read_string::<R>(max_length, reader, protocol_version)?;
    serde_json::from_slice(json_string.as_bytes())
        .map_err(|err| Error::SerdeJsonError(err, T::base_context()))
}

pub fn write_nbt<T, W: Write>(value: &T, writer: &mut W, _: ProtocolVersion) -> Result<()>
where
    T: Contextual + serde::ser::Serialize,
{
    nbt::ser::to_writer(writer, value, None).map_err(|err| Error::NbtError(err, T::base_context()))
}

pub fn size_nbt<T>(value: &T, protocol_version: ProtocolVersion) -> Result<i32>
where
    T: Contextual + serde::ser::Serialize,
{
    let mut sizer = InternalSizer::default();
    write_nbt(value, &mut sizer, protocol_version)?;
    Ok(sizer.current_size())
}

pub fn size_stripped_nbt<T>(value: &T, protocol_version: ProtocolVersion) -> Result<i32>
where
    T: Contextual + serde::ser::Serialize,
{
    let mut outer_size = InternalSizer::default();
    let mut sizer = strip_fake_nbt_header(&mut outer_size);
    write_nbt(value, &mut sizer, protocol_version)?;
    Ok(outer_size.current_size())
}

pub fn read_nbt<T, R: Read>(reader: &mut R, _: ProtocolVersion) -> Result<T>
where
    T: Contextual + for<'de> serde::de::Deserialize<'de>,
{
    nbt::de::from_reader(reader).map_err(|err| Error::NbtError(err, T::base_context()))
}

impl<K: Contextual, V: Contextual> Contextual for HashMap<K, V> {
    fn context() -> String {
        format!("HashMap<{}, {}>", K::context(), V::context())
    }
}

pub struct FakeNbtHeaderStripper<'a, W: Write> {
    inner: &'a mut W,
    cursor: usize,
    skip_state: (i32, u16),
    prep_bytes: Option<u8>,
    buf_to_forward: Vec<u8>,
}

impl<'a, W: Write> FakeNbtHeaderStripper<'a, W> {
    fn handle_bytes(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.len() == self.cursor {
            return Ok(0);
        }
        println!("FAKE NBT HEADER STRIPPER: ({}, {})", buf.len(), self.cursor);

        match self.skip_state {
            (0, _) => {
                println!("Skip state 0");
                self.cursor += buf.len().min(1); // consume
                println!("Next cursor {}", self.cursor);
                if self.cursor == 0 {
                    Ok(0)
                } else {
                    println!("Skip state up");
                    self.skip_state = (1, 0);
                    self.handle_bytes(buf).map(|r| r + 1)
                }
            }
            (1, _) if matches!(self.prep_bytes, None) => {
                // read in first byte
                match buf.len() - self.cursor {
                    x if x >= 2 => {
                        println!("Reading both length bytes.");
                        self.buf_to_forward.push(buf[self.cursor]);
                        self.buf_to_forward.push(buf[self.cursor + 1]);
                        println!("Pushing buffer. {:?}", self.buf_to_forward);
                        self.skip_state = (
                            2,
                            u16::from_be_bytes([buf[self.cursor], buf[self.cursor + 1]]),
                        );
                        println!("Post: {:?}", self.skip_state);
                        self.cursor += 2;
                        self.handle_bytes(buf).map(|r| r + 2)
                    }
                    1 => {
                        println!("Forwarding byte.");
                        self.buf_to_forward.push(buf[self.cursor]);
                        self.prep_bytes = Some(buf[self.cursor]);
                        Ok(1)
                    }
                    _ => unreachable!(),
                }
            }
            (1, _) => {
                println!("Reading forwarded byte!");
                self.buf_to_forward.push(buf[0]);
                self.skip_state = (2, u16::from_be_bytes([self.prep_bytes.unwrap(), buf[0]]));
                self.handle_bytes(buf).map(|r| r + 1)
            }
            // here we're reading the string in preparation
            // after the string we'll read either a 0x00 or a 0x0a
            // in the case of a 0x00 we must forward a 0x00 immediately
            // and close, the nbt writer should handle that, we just need to
            // buffer the string send - it isn't skipped if 0x00
            // so we must consume it early for "under reads"
            (2, to_read) => {
                println!("Reading in string.");
                let available_read = buf.len().min(to_read as usize) - self.cursor;
                self.cursor += available_read;
                self.buf_to_forward
                    .extend_from_slice(&buf[self.cursor..self.cursor + available_read]);
                if available_read as u16 == to_read {
                    self.skip_state = (3, 0);
                } else {
                    self.skip_state = (2, to_read - available_read as u16);
                }
                println!("Wrote bytes: {}", available_read);
                self.handle_bytes(buf).map(|r| r + available_read)
            }
            (3, _) => {
                println!("Reading true header.");
                let heading_value = buf[self.cursor];
                self.cursor += 1;
                match heading_value {
                    0x00 => {
                        println!("Null...");
                        if self.inner.write(&[0x00])? == 1 {
                            self.skip_state = (5, 0);
                            Ok(1)
                        } else {
                            Ok(0)
                        }
                    }
                    x => {
                        println!("Read Tag: {}", x);
                        if self.inner.write(&[x])? == 1 {
                            self.skip_state = (4, 0);
                            self.handle_bytes(buf).map(|r| r + 1)
                        } else {
                            Ok(0)
                        }
                    }
                    _ => Err(std::io::Error::new(
                        ErrorKind::InvalidInput,
                        "Invalid value coming from internal mapper.",
                    )),
                }
            }
            (4, _) => {
                println!("Writing string buffer. {}", self.cursor);
                self.inner.write_all(&self.buf_to_forward)?;
                self.inner.write_all(&buf[self.cursor..])?;
                println!("Wrote size: {}", buf[self.cursor..].len());
                Ok(buf[self.cursor..].len())
            }
            (5, 0) => self.inner.write(buf),
            _ => Ok(0),
        }
    }
}

impl<'a, W: Write> Write for FakeNbtHeaderStripper<'a, W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.cursor = 0;
        let x = self.handle_bytes(buf)?;
        println!("Writing {} out of {}", x, buf.len());
        Ok(x)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

pub struct FakeNbtHeaderInserter<'a, R: Read> {
    skip_state: (usize, usize),
    inner: &'a mut R,
}

impl<'a, R: Read> FakeNbtHeaderInserter<'a, R> {
    pub fn into_inner(self) -> &'a mut R {
        self.inner
    }
}

impl<'a, R: Read> Read for FakeNbtHeaderInserter<'a, R> {
    fn read(&mut self, mut buf: &mut [u8]) -> std::io::Result<usize> {
        match self.skip_state {
            (0, _) => {
                // write initial compound tag byte
                let written = buf.write(&[0x0a])?;
                if written == 1 {
                    self.skip_state = (1, 0);
                } else {
                    return Ok(0);
                }
                let mut inner_read: [u8; 1] = [0];
                let inner_read_size = self.inner.read(&mut inner_read)?;
                if inner_read_size == 0 {
                    return Ok(0);
                }
                let intended_state = inner_read[0];
                if intended_state == 0x0A {
                    self.skip_state = (4, 0);
                } else if intended_state != 0x00 {
                    return Err(std::io::Error::new(
                        ErrorKind::InvalidInput,
                        format!("Invalid tag type: {}", intended_state),
                    ));
                }
                Ok(written)
            }
            (1, mut cursor) => {
                // write root tag string length
                const WRITE_BYTES: [u8; 2] = [0, 9];
                const TO_WRITE: usize = WRITE_BYTES.len();
                let written = buf.write(&WRITE_BYTES[cursor..])?;
                cursor += written;
                if cursor == TO_WRITE {
                    self.skip_state = (2, 0);
                }
                Ok(written)
            }
            (2, mut cursor) => {
                // write root tag string
                const WRITE_BYTES: [u8; 9] = [102, 97, 107, 101, 95, 114, 111, 111, 116];
                const TO_WRITE: usize = WRITE_BYTES.len();
                let written = buf.write(&WRITE_BYTES[cursor..])?;
                cursor += written;
                if cursor == TO_WRITE {
                    self.skip_state = (3, 0);
                }
                Ok(written)
            }
            (3, _) => {
                if buf.write(&[0x00])? == 0 {
                    Ok(0)
                } else {
                    self.skip_state = (4, 0);
                    Ok(1)
                }
            }
            (4, _) => {
                // free spin
                self.inner.read(buf)
            }
            (_, _) => panic!("Invalid skip state"),
        }
    }
}

pub fn strip_fake_nbt_header<W: Write>(writer: &mut W) -> FakeNbtHeaderStripper<W> {
    FakeNbtHeaderStripper {
        inner: writer,
        cursor: 0,
        skip_state: (0, 0),
        prep_bytes: None,
        buf_to_forward: vec![],
    }
}

pub fn insert_fake_nbt_header<R: Read>(reader: &mut R) -> FakeNbtHeaderInserter<R> {
    FakeNbtHeaderInserter {
        skip_state: (0, 0),
        inner: reader,
    }
}

macro_rules! map_size {
    ($v:expr, $t:ty) => {
        wrap_struct_context!(
            "map_size",
            VarInt::try_from($v.len()).map_err(|err| super::serde::Error::TryFromIntError(
                err,
                SerializerContext::new(
                    <$t>::context(),
                    format!("Failed to create varint from usize {}", $v.len())
                )
            ))
        )?
    };
}

impl<K: Contextual + Serialize + Hash + Eq, V: Contextual + Serialize> Serialize for HashMap<K, V> {
    fn serialize<W: Write>(&self, writer: &mut W, protocol_version: ProtocolVersion) -> Result<()> {
        let map_size = map_size!(self, Self);
        wrap_struct_context!("map_size", map_size.serialize(writer, protocol_version))?;
        for (index, (key, value)) in self.iter().enumerate() {
            wrap_indexed_struct_context!("key", index, key.serialize(writer, protocol_version))?;
            wrap_indexed_struct_context!(
                "value",
                index,
                value.serialize(writer, protocol_version)
            )?;
        }
        Ok(())
    }

    fn size(&self, protocol_version: ProtocolVersion) -> Result<i32> {
        let mut size = 0;
        let map_size = map_size!(self, Self);
        size += wrap_struct_context!("map_size", map_size.size(protocol_version))?;
        for (index, (key, value)) in self.iter().enumerate() {
            size += wrap_indexed_struct_context!("key", index, key.size(protocol_version))?;
            size += wrap_indexed_struct_context!("value", index, value.size(protocol_version))?;
        }
        Ok(size)
    }
}

impl<K: Contextual + Deserialize + Hash + Eq, V: Contextual + Deserialize> Deserialize
    for HashMap<K, V>
{
    fn deserialize<R: Read>(reader: &mut R, protocol_version: ProtocolVersion) -> Result<Self> {
        let map_size =
            wrap_struct_context!("map_size", VarInt::deserialize(reader, protocol_version))?;
        let mut map = HashMap::with_capacity(wrap_struct_context!(
            "map_size",
            TryInto::<usize>::try_into(map_size).map_err(|err| {
                super::serde::Error::TryFromIntError(
                    err,
                    SerializerContext::new(
                        Self::context(),
                        format!("Failed to turn {} into a usize.", map_size),
                    ),
                )
            })
        )?);
        for index in 0..Into::<i32>::into(map_size) {
            map.insert(
                wrap_indexed_struct_context!(
                    "key",
                    index,
                    K::deserialize(reader, protocol_version)
                )?,
                wrap_indexed_struct_context!(
                    "value",
                    index,
                    V::deserialize(reader, protocol_version)
                )?,
            );
        }
        Ok(map)
    }
}

impl<T: Contextual> Contextual for Box<T> {
    fn context() -> String {
        format!("Box<{}>", T::context())
    }
}

impl<T: Serialize> Serialize for Box<T> {
    fn serialize<W: Write>(&self, writer: &mut W, protocol_version: ProtocolVersion) -> Result<()> {
        T::serialize(self, writer, protocol_version)
    }

    fn size(&self, protocol_version: ProtocolVersion) -> Result<i32> {
        T::size(self, protocol_version)
    }
}

impl<T: Deserialize> Deserialize for Box<T> {
    fn deserialize<R: Read>(reader: &mut R, protocol_version: ProtocolVersion) -> Result<Self> {
        T::deserialize(reader, protocol_version).map(Box::new)
    }
}

impl Contextual for Codec {
    fn context() -> String {
        "Codec".to_string()
    }
}

macro_rules! context {
    ($ty:ty) => {
        impl Contextual for $ty {
            fn context() -> String {
                format!("{}", stringify!($ty))
            }
        }
    };
}

context!(String);

impl Deserialize for String {
    fn deserialize<R: Read>(reader: &mut R, protocol_version: ProtocolVersion) -> Result<Self> {
        read_string::<R>(32767, reader, protocol_version)
    }
}

impl Serialize for String {
    fn serialize<W: Write>(&self, writer: &mut W, protocol_version: ProtocolVersion) -> Result<()> {
        write_string::<W>(32767, &self.to_string(), writer, protocol_version)
    }

    fn size(&self, protocol_version: ProtocolVersion) -> Result<i32> {
        size_string::<Self>(&self.to_string(), protocol_version)
    }
}

context!(nbt::Value);
