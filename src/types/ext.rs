use crate::encoding::*;
use crate::types::VarInt;
use anyhow::Context;
use nbt::Blob;
use std::convert::TryInto;
use std::io::{Read, Write};
use uuid::Uuid;

impl<T: Decodable> Decodable for Vec<T> {
    fn decode<R: Read>(reader: &mut R) -> anyhow::Result<Self> {
        let mut items: Vec<T> = Vec::new();
        let mut remaining_bytes = Vec::new();
        let length = reader.read_to_end(&mut remaining_bytes)? as u64;
        let mut cursor = std::io::Cursor::new(remaining_bytes);

        while cursor.position() < length {
            items.push(T::decode(&mut cursor)?);
        }
        Ok(items)
    }
}

impl<T: Encodable> Encodable for Vec<T> {
    fn encode<W: Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        for item in self {
            item.encode(writer)?;
        }
        Ok(())
    }

    fn size(&self) -> anyhow::Result<VarInt> {
        self.iter()
            .map(|item| item.size())
            .try_fold(0.into(), |bubble, item| Ok(bubble + item?))
    }
}

impl<T: Decodable> SizeDecodable for Vec<T> {
    fn decode_sized<R: Read>(reader: &mut R, size: &VarInt) -> anyhow::Result<Self> {
        let mut items = Vec::with_capacity(size.try_into()?);
        for _ in 0..size.into() {
            items.push(T::decode(reader)?);
        }
        Ok(items)
    }
}

impl<T: Encodable> SizeEncodable for Vec<T> {
    fn encode_sized<W: Write>(&self, writer: &mut W, size: &VarInt) -> anyhow::Result<()> {
        size.encode(writer)?;
        self.iter().try_for_each(|item| item.encode(writer))
    }

    fn predicted_size(&self) -> anyhow::Result<VarInt> {
        let mut size = self.size()?;
        size += size.size()?;
        Ok(size)
    }
}

impl<T: SizeDecodable> Decodable for (VarInt, T) {
    fn decode<R: Read>(reader: &mut R) -> anyhow::Result<Self> {
        let size = VarInt::decode(reader)?;
        let item = T::decode_sized(reader, &size)?;
        Ok((size, item))
    }
}

impl<T: SizeEncodable> Encodable for (VarInt, T) {
    fn encode<W: Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        self.1.encode_sized(writer, &self.0)
    }

    fn size(&self) -> anyhow::Result<VarInt> {
        self.1.predicted_size()
    }
}

impl<T: Decodable> Decodable for Option<T> {
    fn decode<R: Read>(reader: &mut R) -> anyhow::Result<Self> {
        Ok(Some(T::decode(reader)?))
    }
}

impl<T: Encodable> Encodable for Option<T> {
    fn encode<W: Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        self.as_ref().map_or(Ok(()), |item| item.encode(writer))
    }

    fn size(&self) -> anyhow::Result<VarInt> {
        self.as_ref()
            .map_or(Ok(VarInt::from(0)), |item| item.size())
    }
}

impl<T: Decodable> Decodable for (bool, Option<T>) {
    fn decode<R: Read>(reader: &mut R) -> anyhow::Result<Self> {
        let present = bool::decode(reader)?;
        if present {
            Ok((true, Some(T::decode(reader)?)))
        } else {
            Ok((false, None))
        }
    }
}

impl<T: Encodable> Encodable for (bool, Option<T>) {
    fn encode<W: Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        self.0.encode(writer)?;
        if self.0 {
            match &self.1 {
                Some(item) => item.encode(writer),
                None => anyhow::bail!("Expected some value but found None."),
            }
        } else {
            Ok(())
        }
    }

    fn size(&self) -> anyhow::Result<VarInt> {
        let size = self.0.size()?;
        if self.0 {
            match &self.1 {
                Some(item) => Ok(size + item.size()?),
                None => Ok(size),
            }
        } else {
            Ok(size)
        }
    }
}

impl<X: Decodable, Y: Decodable, Z: Decodable> Decodable for (X, Y, Z) {
    fn decode<R: Read>(reader: &mut R) -> anyhow::Result<Self> {
        Ok((X::decode(reader)?, Y::decode(reader)?, Z::decode(reader)?))
    }
}

impl<X: Encodable, Y: Encodable, Z: Encodable> Encodable for (X, Y, Z) {
    fn encode<W: Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        self.0.encode(writer)?;
        self.1.encode(writer)?;
        self.2.encode(writer)
    }

    fn size(&self) -> anyhow::Result<VarInt> {
        Ok(self.0.size()? + self.1.size()? + self.2.size()?)
    }
}

impl Decodable for Uuid {
    fn decode<R: Read>(reader: &mut R) -> anyhow::Result<Self> {
        let mut bytes: [u8; 16] = [0u8; 16];
        reader.read_exact(&mut bytes)?;
        Ok(Uuid::from_bytes(bytes))
    }
}

impl Encodable for Uuid {
    fn encode<W: Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        writer
            .write_all(self.as_bytes())
            .context(format!("Failed to write {:?} into bytes.", self))
    }

    fn size(&self) -> anyhow::Result<VarInt> {
        Ok(VarInt::from(16))
    }
}

impl Decodable for Blob {
    fn decode<R: Read>(reader: &mut R) -> anyhow::Result<Self> {
        Blob::from_reader(reader).context("Failed to read bytes into nbt string.")
    }
}

impl Encodable for Blob {
    fn encode<W: Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        self.to_writer(writer)
            .context("Failed to write nbt string into bytes.")
    }

    fn size(&self) -> anyhow::Result<VarInt> {
        self.len_bytes().try_into().context(format!(
            "Failed to turn {} into a VarInt.",
            self.len_bytes()
        ))
    }
}
