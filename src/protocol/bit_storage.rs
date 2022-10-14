use drax::extension::{read_var_int_sync, size_var_int, write_var_int_sync};
use drax::transport::{DraxTransport, Result, TransportProcessorContext};
use std::fmt::{Display, Formatter};
use std::io::Cursor;
use drax::SizedVec;

#[derive(Debug)]
pub enum BitStorage {
    ZeroStorage { size: i32, raw: Vec<i64> },
    SimpleStorage { size: i32, bits: i32, raw: Vec<i64> },
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
            BitStorage::ZeroStorage {
                size: 0,
                raw: vec![],
            }
        } else {
            BitStorage::SimpleStorage {
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
        Ok(BitStorage::SimpleStorage { size, bits, raw })
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
            BitStorage::ZeroStorage { size, .. } => {
                Self::validate_n32(0, *size - 1, n)?;
                Self::validate_n64(0, 0, n2 as i64)?;
                Ok(0)
            }
            BitStorage::SimpleStorage { size, bits, raw } => {
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
            BitStorage::ZeroStorage { size, .. } => {
                Self::validate_n32(0, *size - 1, n)?;
                Self::validate_n64(0, 0, n2 as i64)?;
                Ok(())
            }
            BitStorage::SimpleStorage { size, bits, raw } => {
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
            BitStorage::ZeroStorage { size, .. } => {
                Self::validate_n32(0, *size - 1, n)?;
                Ok(0)
            }
            BitStorage::SimpleStorage { size, bits, raw } => {
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

    pub fn from_reader<R: std::io::Read>(
        reader: &mut R,
        bits: u8,
        storage_size: i32,
        context: &mut TransportProcessorContext,
    ) -> Result<Self> {
        if bits == 0 {
            let deserialized_size = read_var_int_sync(context, reader)?;
            if deserialized_size != 0 {
                return drax::transport::Error::cause("Invalid zero bit storage size.");
            }
            Ok(BitStorage::ZeroStorage {
                size: storage_size,
                raw: vec![],
            })
        } else {
            let expected_size = Self::expected_size(storage_size, bits as i32);
            let deserialized_size = read_var_int_sync(context, reader)?;
            if expected_size != deserialized_size {
                return drax::transport::Error::cause(format!(
                    "Incorrect value length given {} expected {}.",
                    expected_size, deserialized_size
                ));
            }

            let mut raw = Vec::with_capacity(expected_size as usize);
            for _ in 0..expected_size {
                raw.push(i64::read_from_transport(context, reader)?);
            }

            Ok(BitStorage::SimpleStorage {
                size: storage_size,
                bits: bits as i32,
                raw,
            })
        }
    }

    pub fn to_writer(
        &self,
        writer: &mut Cursor<Vec<u8>>,
        context: &mut TransportProcessorContext,
    ) -> Result<()> {
        match self {
            BitStorage::ZeroStorage { .. } => {
                write_var_int_sync(0, context, writer)?;
            }
            BitStorage::SimpleStorage { raw, .. } => {
                write_var_int_sync(raw.len() as i32, context, writer)?;
                for t in raw {
                    t.write_to_transport(context, writer)?;
                }
            }
        };
        Ok(())
    }

    pub fn check_size(&self, context: &mut TransportProcessorContext) -> Result<usize> {
        Ok(match self {
            BitStorage::ZeroStorage { .. } => 1, // VarInt(0) is size of 1 byte
            BitStorage::SimpleStorage { raw, .. } => {
                size_var_int(raw.len() as i32, context)? + (raw.len() * 8)
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
