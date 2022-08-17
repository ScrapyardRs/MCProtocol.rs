use crate::primitive::{read_string, size_string, write_string, VarInt};
use crate::serde::{
    Contextual, Deserialize, Error, InternalSizer, ProtocolVersion, Result, Serialize,
    SerializerContext,
};
use crate::{wrap_indexed_struct_context, wrap_struct_context};
use bytes::Buf;

use crate::ext::BitStorage::SimpleStorage;
use nbt::ser::Encoder;
use serde::de::{EnumAccess, MapAccess, SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserializer, Serializer};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::io::{Cursor, ErrorKind, Read, Write};
use BitStorage::ZeroStorage;

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
            result.push(wrap_indexed_struct_context!(
                "result",
                result.len(),
                T::deserialize(&mut remaining, protocol_version)
            )?);
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

macro_rules! map_size {
    ($v:expr, $t:ty) => {
        wrap_struct_context!(
            "map_size",
            VarInt::try_from($v.len()).map_err(|err| super::serde::Error::TryFromIntError(
                err,
                SerializerContext::new(
                    <$t>::context(),
                    format!("Failed to create var int from usize {}", $v.len())
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

const fn false0() -> bool {
    false
}

#[derive(serde_derive::Serialize, Debug)]
#[serde(untagged)]
pub enum BitStorage {
    ZeroStorage {
        #[serde(skip)]
        size: i32,
        #[serde(serialize_with = "nbt::i64_array")]
        raw: Vec<i64>,
    },
    SimpleStorage {
        #[serde(skip)]
        size: i32,
        #[serde(skip)]
        bits: i32,
        #[serde(serialize_with = "nbt::i64_array")]
        raw: Vec<i64>,
    },
}

pub struct BitSetVisitor {
    seeded_bitset: BitStorage,
}

impl BitSetVisitor {
    pub fn new(seed: BitStorage) -> Self {
        Self {
            seeded_bitset: seed,
        }
    }

    fn expected_size(&self) -> i32 {
        match &self.seeded_bitset {
            ZeroStorage { .. } => 0,
            SimpleStorage { size, bits, .. } => BitStorage::expected_size(*size, *bits),
        }
    }
}

impl<'de> Visitor<'de> for BitSetVisitor {
    type Value = BitStorage;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "A long array of {} length.",
            &self.expected_size()
        )
    }

    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut raw = Vec::with_capacity(self.expected_size() as usize);
        let expecting = self.expected_size();
        loop {
            match seq.next_element_seed(BitSetSeedDeserializer)? {
                None => {
                    return if raw.len() == expecting as usize {
                        Ok(match self.seeded_bitset {
                            ZeroStorage { size, raw } => ZeroStorage { size, raw },
                            SimpleStorage { size, bits, .. } => SimpleStorage { size, bits, raw },
                        })
                    } else {
                        Err(serde::de::Error::custom(format!(
                            "Invalid length {} expected {}.",
                            raw.len(),
                            self.expected_size(),
                        )))
                    }
                }
                Some(next) => {
                    if raw.len() == expecting as usize {
                        return Err(serde::de::Error::custom(format!(
                            "Array too big, expected size {}.",
                            self.expected_size(),
                        )));
                    } else {
                        raw.push(next);
                        continue;
                    }
                }
            }
        }
    }
}

struct BitSetSeedVisitor;

impl<'de> Visitor<'de> for BitSetSeedVisitor {
    type Value = i64;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "A long.")
    }

    fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v)
    }

    fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v as i64)
    }
}

struct BitSetSeedDeserializer;

impl<'de> serde::de::DeserializeSeed<'de> for BitSetSeedDeserializer {
    type Value = i64;

    fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_i64(BitSetSeedVisitor)
    }
}

#[derive(Debug)]
pub struct BitSetValidationError(pub String);

impl std::error::Error for BitSetValidationError {}

impl Display for BitSetValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error validating BitSet entry. {}", self.0)
    }
}

impl BitStorage {
    pub fn new(size: i32, bits: i32) -> Self {
        if size == 0 || bits == 0 {
            ZeroStorage {
                size: 0,
                raw: vec![],
            }
        } else {
            SimpleStorage {
                size,
                bits,
                raw: vec![0; Self::expected_size(size, bits) as usize],
            }
        }
    }

    pub fn with_seeded_raw(
        size: i32,
        bits: i32,
        raw: Vec<i64>,
    ) -> std::result::Result<Self, BitSetValidationError> {
        if Self::expected_size(size, bits) as usize != raw.len() {
            return Err(BitSetValidationError(format!(
                "Invalid bitset seeded raw length of {}, expected {}.",
                raw.len(),
                Self::expected_size(size, bits)
            )));
        }
        Ok(SimpleStorage { size, bits, raw })
    }

    #[rustfmt::skip]
    const MAGIC: [i32; 192] = [-1, -1, 0, -2147483648, 0, 0, 1431655765, 1431655765,
        0, -2147483648, 0, 1, 858993459, 858993459, 0,
        715827882, 715827882, 0, 613566756, 613566756, 0,
        -2147483648, 0, 2, 477218588, 477218588, 0, 429496729,
        429496729, 0, 390451572, 390451572, 0, 357913941,
        357913941, 0, 330382099, 330382099, 0, 306783378,
        306783378, 0, 286331153, 286331153, 0, -2147483648,
        0, 3, 252645135, 252645135, 0, 238609294,
        238609294, 0, 226050910, 226050910, 0, 214748364, 214748364,
        0, 204522252, 204522252, 0, 195225786, 195225786,
        0, 186737708, 186737708, 0, 178956970, 178956970,
        0, 171798691, 171798691, 0, 165191049, 165191049, 0, 159072862,
        159072862, 0, 153391689, 153391689,
        0, 148102320, 148102320, 0, 143165576, 143165576, 0, 138547332,
        138547332, 0, -2147483648, 0, 4, 130150524, 130150524,
        0, 126322567, 126322567, 0, 122713351, 122713351, 0,
        119304647, 119304647, 0, 116080197, 116080197, 0, 113025455,
        113025455, 0, 10127366, 110127366, 0, 107374182,
        107374182, 0, 104755299, 104755299,
        0, 102261126, 102261126, 0, 99882960, 99882960, 0,
        97612893, 97612893, 0, 95443717, 95443717, 0, 93368854, 93368854,
        0, 91382282, 91382282, 0, 89478485, 89478485, 0, 87652393, 87652393,
        0, 85899345, 85899345, 0, 84215045, 84215045, 0, 82595524, 82595524,
        0, 81037118, 81037118, 0, 79536431, 79536431, 0, 78090314, 78090314,
        0, 76695844, 76695844, 0, 75350303, 75350303, 0, 74051160,
        74051160, 0, 72796055, 72796055, 0, 71582788, 71582788, 0,
        70409299, 70409299, 0, 69273666, 69273666, 0, 68174084,
        68174084, 0, -2147483648, 0, 5,
    ];

    const fn mask(bits: i32) -> i64 {
        match bits {
            0 => 0,
            _ => (1 << bits as i64) - 1,
        }
    }

    const fn values_per_long(bits: i32) -> i32 {
        match bits {
            0 => 0,
            _ => 64 / bits,
        }
    }

    const fn divide_mul(bits: i32) -> i32 {
        match bits {
            0 => 0,
            _ => Self::MAGIC[(3 * (Self::values_per_long(bits) - 1)) as usize],
        }
    }

    const fn divide_add(bits: i32) -> i32 {
        match bits {
            0 => 0,
            _ => Self::MAGIC[1 + (3 * (Self::values_per_long(bits) - 1)) as usize],
        }
    }

    const fn divide_shift(bits: i32) -> i32 {
        match bits {
            0 => 0,
            _ => Self::MAGIC[2 + (3 * (Self::values_per_long(bits) - 1)) as usize],
        }
    }

    const fn cell_index(bits: i32, n: i32) -> i32 {
        let l = 4294967295 & Self::divide_mul(bits) as u64;
        let l2 = 4294967295 & Self::divide_add(bits) as u64;
        let n = n as u64;
        ((((n * l) + l2) >> 32) >> Self::divide_shift(bits) as u64) as i32
    }

    fn validate_n32(n1: i32, n2: i32, n3: i32) -> std::result::Result<(), BitSetValidationError> {
        if !(n1..n2).contains(&n3) {
            return Err(BitSetValidationError(format!(
                "i32 base {} is not between {} and {}.",
                n3, n1, n2
            )));
        }
        Ok(())
    }

    fn validate_n64(n1: i64, n2: i64, n3: i64) -> std::result::Result<(), BitSetValidationError> {
        if !(n1..n2).contains(&n3) {
            return Err(BitSetValidationError(format!(
                "i64 base {} is not between {} and {}.",
                n3, n1, n2
            )));
        }
        Ok(())
    }

    pub const fn expected_size(size: i32, bits: i32) -> i32 {
        (size + Self::values_per_long(bits) - 1) / Self::values_per_long(bits)
    }

    pub fn get_and_set(
        &mut self,
        n: i32,
        n2: i32,
    ) -> std::result::Result<i32, BitSetValidationError> {
        match self {
            ZeroStorage { size, .. } => {
                Self::validate_n32(0, *size - 1, n)?;
                Self::validate_n64(0, 0, n2 as i64)?;
                Ok(0)
            }
            SimpleStorage { size, bits, raw } => {
                Self::validate_n32(0, *size - 1, n)?;
                Self::validate_n64(0, Self::mask(*bits), n2 as i64)?;
                let n3 = Self::cell_index(*bits, n);
                let larr = raw[n3 as usize] as u64;
                let u64mask = Self::mask(*bits) as u64;
                let n4 = (n - (n3 * Self::values_per_long(*bits))) * *bits;
                let n5 = (u64mask & (larr >> n4 as u64)) as i32;
                raw[n3 as usize] = (larr as u64 & ((u64mask << n4 as u64) ^ 0xFFFFFFFFFFFFFFFFu64)
                    | (n2 as u64 & Self::mask(*bits) as u64) << n4 as u64)
                    as i64;
                Ok(n5)
            }
        }
    }

    pub fn set(&mut self, n: i32, n2: i32) -> std::result::Result<(), BitSetValidationError> {
        match self {
            ZeroStorage { size, .. } => {
                Self::validate_n32(0, *size - 1, n)?;
                Self::validate_n64(0, 0, n2 as i64)?;
                Ok(())
            }
            SimpleStorage { size, bits, raw } => {
                Self::validate_n32(0, *size - 1, n)?;
                Self::validate_n64(0, Self::mask(*bits), n2 as i64)?;
                let n3 = Self::cell_index(*bits, n);
                let larr = raw[n3 as usize];
                let n4 = (n - (n3 * Self::values_per_long(*bits))) * *bits;
                let u64mask = Self::mask(*bits) as u64;
                raw[n3 as usize] = (larr as u64 & ((u64mask << n4 as u64) ^ 0xFFFFFFFFFFFFFFFFu64)
                    | (n2 as u64 & Self::mask(*bits) as u64) << n4 as u64)
                    as i64;
                Ok(())
            }
        }
    }

    pub fn get(&self, n: i32) -> std::result::Result<i32, BitSetValidationError> {
        match self {
            ZeroStorage { size, .. } => {
                Self::validate_n32(0, *size - 1, n)?;
                Ok(0)
            }
            SimpleStorage { size, bits, raw } => {
                Self::validate_n32(0, *size - 1, n)?;
                let n3 = Self::cell_index(*bits, n);
                let larr = raw[n3 as usize] as u64;
                let n4 = (n - (n3 * Self::values_per_long(*bits))) * *bits;
                let u64mask = Self::mask(*bits) as u64;
                let n5 = (u64mask & (larr >> n4 as u64)) as i32;
                Ok(n5)
            }
        }
    }

    pub fn from_reader<R: Read>(
        reader: &mut R,
        bits: u8,
        storage_size: i32,
        protocol_version: ProtocolVersion,
    ) -> Result<Self> {
        if bits == 0 {
            let deserialized_size = VarInt::deserialize(reader, protocol_version)?;
            if *deserialized_size != 0 {
                return Err(Error::Generic(SerializerContext::new(
                    Self::context(),
                    format!(
                        "Incorrect value length given {} expected {}.",
                        0, deserialized_size
                    ),
                )));
            }
            Ok(ZeroStorage {
                size: storage_size,
                raw: vec![],
            })
        } else {
            let expected_size = Self::expected_size(storage_size, bits as i32);
            let deserialized_size = VarInt::deserialize(reader, protocol_version)?;
            if expected_size != *deserialized_size {
                return Err(Error::Generic(SerializerContext::new(
                    Self::context(),
                    format!(
                        "Incorrect value length given {} expected {}.",
                        expected_size, deserialized_size
                    ),
                )));
            }

            let mut raw = Vec::with_capacity(expected_size as usize);
            for index in 0..expected_size {
                raw.push(wrap_indexed_struct_context!(
                    "raw",
                    index,
                    Deserialize::deserialize(reader, protocol_version)
                )?);
            }

            Ok(SimpleStorage {
                size: storage_size,
                bits: bits as i32,
                raw,
            })
        }
    }

    pub fn to_writer<W: Write>(
        &self,
        writer: &mut W,
        protocol_version: ProtocolVersion,
    ) -> Result<()> {
        match self {
            ZeroStorage { .. } => {
                wrap_struct_context!("size", VarInt::from(0).serialize(writer, protocol_version))?
            }
            SimpleStorage { raw, .. } => {
                wrap_struct_context!(
                    "raw",
                    (VarInt::from(raw.len() as i32), raw.clone())
                        .serialize(writer, protocol_version)
                )?;
            }
        }
        Ok(())
    }

    pub fn check_size(&self, protocol_version: ProtocolVersion) -> Result<i32> {
        Ok(match self {
            ZeroStorage { .. } => 1, // VarInt(0) is size of 1 byte
            SimpleStorage { size, raw, bits } => {
                VarInt::from(raw.len() as i32).size(protocol_version)? + (raw.len() as i32 * 8)
            }
        })
    }

    pub fn get_raw(&self) -> Vec<i64> {
        match self {
            BitStorage::ZeroStorage { raw, .. } => raw.clone(),
            BitStorage::SimpleStorage { raw, .. } => raw.clone(),
        }
    }
}

context!(BitStorage);
