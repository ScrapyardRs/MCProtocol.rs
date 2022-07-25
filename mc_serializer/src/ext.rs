use std::io::{Cursor, Read, Write};
use crate::primitive::VarInt;
use crate::serde::{Deserialize, SerdeResult, Serialize};
use bytes::Buf;

impl<T: Serialize> Serialize for (VarInt, Vec<T>) {
    fn serialize<W: Write>(&self, writer: &mut W) -> SerdeResult<()> {
        VarInt::serialize(&self.0, writer)?;
        for item in &self.1 {
            T::serialize(item, writer)?;
        }
        Ok(())
    }

    fn size(&self) -> SerdeResult<i32> {
        let mut size = self.0.size()?;
        for item in &self.1 {
            size += item.size()?;
        }
        Ok(size)
    }
}

impl<T: Deserialize> Deserialize for (VarInt, Vec<T>) {
    fn deserialize<R: Read>(reader: &mut R) -> SerdeResult<Self> {
        let v_size = VarInt::deserialize(reader)?;
        let mut result: Vec<T> = Vec::with_capacity(v_size.try_into()?);
        for _ in 0..v_size.into() {
            result.push(T::deserialize(reader)?);
        }
        Ok((v_size, result))
    }
}

impl<T: Serialize> Serialize for Vec<T> {
    fn serialize<W: Write>(&self, writer: &mut W) -> SerdeResult<()> {
        for item in self {
            T::serialize(item, writer)?;
        }
        Ok(())
    }

    fn size(&self) -> SerdeResult<i32> {
        let mut size = 0;
        for item in self {
            size += T::size(item)?;
        }
        Ok(size)
    }
}

impl<T: Deserialize> Deserialize for Vec<T> {
    fn deserialize<R: Read>(reader: &mut R) -> SerdeResult<Self> {
        let mut remaining = Vec::new();
        reader.read_to_end(&mut remaining)?;
        let mut remaining = Cursor::new(remaining);
        let mut result = Vec::new();
        while remaining.has_remaining() {
            result.push(T::deserialize(&mut remaining)?);
        }
        Ok(result)
    }
}

impl<T: Serialize> Serialize for (bool, Option<T>) {
    fn serialize<W: Write>(&self, writer: &mut W) -> SerdeResult<()> {
        bool::serialize(&self.0, writer)?;
        if self.0 {
            T::serialize(self.1.as_ref().expect("When bool is true option should have Some(x)."), writer)?;
        }
        Ok(())
    }

    fn size(&self) -> SerdeResult<i32> {
        Ok(1 + match &self.1 {
            None => 0,
            Some(item) => T::size(item)?,
        })
    }
}

impl <T: Deserialize> Deserialize for (bool, Option<T>) {
    fn deserialize<R: Read>(reader: &mut R) -> SerdeResult<Self> {
        let exists = bool::deserialize(reader)?;
        if exists {
            Ok((true, Some(T::deserialize(reader)?)))
        } else {
            Ok((exists, None))
        }
    }
}

impl Serialize for uuid::Uuid {
    fn serialize<W: Write>(&self, writer: &mut W) -> SerdeResult<()> {
        let (most_significant, least_significant) = self.as_u64_pair();
        u64::serialize(&most_significant, writer)?;
        u64::serialize(&least_significant, writer)
    }

    fn size(&self) -> SerdeResult<i32> {
        Ok(16)
    }
}

impl Deserialize for uuid::Uuid {
    fn deserialize<R: Read>(reader: &mut R) -> SerdeResult<Self> {
        let (most_significant, least_significant) = (u64::deserialize(reader)?, u64::deserialize(reader)?);
        Ok(uuid::Uuid::from_u64_pair(most_significant, least_significant))
    }
}
