use std::cmp::max;
use std::io::Cursor;

use drax::prelude::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, PacketComponent, Size};
use drax::PinnedLivelyResult;

const ADDRESS_BITS_PER_WORD: i32 = 6;

#[derive(Debug)]
pub struct BitSet {
    pub words: Vec<u64>,
    words_in_use: usize,
}

impl BitSet {
    pub fn get(&self, index: i32) -> drax::prelude::Result<bool> {
        self.assert_invariants()?;
        let word_index = Self::word_index(index) as usize;
        Ok((word_index < self.words_in_use) && (self.words[word_index] & (1 << index)) != 0)
    }

    fn expand_to(&mut self, word_index: usize) {
        let words_required = word_index + 1;
        if self.words_in_use < words_required {
            if self.words.len() < words_required {
                let new_load = max(2 * self.words.len(), words_required);
                self.words.resize(new_load, 0);
            }
            self.words_in_use = words_required;
        }
    }

    pub fn set(&mut self, index: i32) -> drax::prelude::Result<()> {
        let word_index = Self::word_index(index) as usize;
        self.expand_to(word_index);
        self.words[word_index] |= 1 << index;
        self.assert_invariants()
    }

    pub fn clear(&mut self, index: i32) -> drax::prelude::Result<()> {
        let word_index = Self::word_index(index) as usize;
        if word_index < self.words_in_use {
            self.words[word_index] &= !(1 << index);
            self.recalculate_words_in_use();
            self.assert_invariants()
        } else {
            Ok(())
        }
    }

    fn word_index(bit_index: i32) -> i32 {
        bit_index >> ADDRESS_BITS_PER_WORD
    }

    fn assert_invariants(&self) -> drax::prelude::Result<()> {
        macro_rules! assert_or_err {
            ($expr:expr) => {
                if !$expr {
                    drax::throw_explain!(format!("Assertion failed: {}", stringify!($expr)))
                }
            };
        }
        assert_or_err!(self.words_in_use == 0 || self.words[self.words_in_use - 1] != 0);
        assert_or_err!(self.words_in_use <= self.words.len());
        assert_or_err!(self.words_in_use == self.words.len() || self.words[self.words_in_use] == 0);
        Ok(())
    }

    fn recalculate_words_in_use(&mut self) {
        for i in (0..self.words_in_use).rev() {
            if self.words[i] != 0 {
                self.words_in_use = i + 1;
                return;
            }
        }
    }

    pub fn value_of(l_arr: Vec<u64>) -> drax::prelude::Result<BitSet> {
        let mut n = l_arr.len();
        while n > 0 && l_arr[n - 1] == 0 {
            n -= 1;
        }
        let bit_set = BitSet {
            words: l_arr[..n].to_vec(),
            words_in_use: n,
        };
        bit_set.assert_invariants()?;
        Ok(bit_set)
    }

    pub fn to_long_array(&self) -> Vec<u64> {
        self.words[..self.words_in_use].to_vec()
    }

    pub async fn from_bytes(bytes: &[u8]) -> drax::prelude::Result<Self> {
        let mut tip = bytes.len();
        if tip == 0 {
            let bit_set = BitSet {
                words: vec![],
                words_in_use: 0,
            };
            bit_set.assert_invariants()?;
            return Ok(bit_set);
        }
        while tip > 0 && bytes[tip - 1] == 0 {
            tip -= 1;
        }
        let bytes = &bytes[..tip];
        let len = bytes.len();
        let mut cursor = Cursor::new(bytes);
        let mut idx = 0;
        let mut words = vec![0u64; (tip + 7) / 8];
        while len - cursor.position() as usize >= 8 {
            let n_long = cursor.read_u64_le().await?;
            words[idx] = n_long;
            idx += 1;
        }
        let mut remaining = vec![0u8; tip % 8];
        cursor.read_exact(&mut remaining).await?;
        for (idx2, x) in remaining.iter().enumerate() {
            words[idx] |= (*x as u64 & 0xff) << (8 * idx2 as u64);
        }

        let bit_set = BitSet {
            words_in_use: words.len(),
            words,
        };
        bit_set.assert_invariants()?;
        return Ok(bit_set);
    }

    pub async fn to_byte_array(&self) -> drax::prelude::Result<Vec<u8>> {
        let n = self.words_in_use;
        if n == 0 {
            return Ok(vec![]);
        }
        let mut len = 8 * (n - 1);
        let mut x = self.words[n - 1];
        while x != 0 {
            x >>= 8;
            len += 1;
        }
        let ret = vec![0u8; len];
        let mut cursor = Cursor::new(ret);
        for i in 0..n - 1 {
            cursor.write_u64_le(self.words[i]).await?;
        }
        let mut x = self.words[n - 1];
        while x != 0 {
            cursor.write_u8((x & 0xff) as u8).await?;
            x >>= 8;
        }
        Ok(cursor.into_inner())
    }
}

impl<C: Send + Sync> PacketComponent<C> for BitSet {
    type ComponentType = BitSet;

    fn decode<'a, A: AsyncRead + Unpin + Send + Sync + ?Sized>(
        context: &'a mut C,
        read: &'a mut A,
    ) -> PinnedLivelyResult<'a, Self::ComponentType> {
        Box::pin(async move { BitSet::value_of(Vec::<u64>::decode(context, read).await?) })
    }

    fn encode<'a, A: AsyncWrite + Unpin + Send + Sync + ?Sized>(
        component_ref: &'a Self::ComponentType,
        context: &'a mut C,
        write: &'a mut A,
    ) -> PinnedLivelyResult<'a, ()> {
        Box::pin(
            async move { Vec::<u64>::encode(&component_ref.to_long_array(), context, write).await },
        )
    }

    fn size(input: &Self::ComponentType, context: &mut C) -> drax::prelude::Result<Size> {
        Vec::<u64>::size(&input.to_long_array(), context)
    }
}

pub struct FixedBitSet<const N: i32>;

impl<const N: i32> FixedBitSet<N> {
    const FLOORED_SIZE: usize = (-i32::div_floor(-N, 8)) as usize;
}

impl<C: Send + Sync, const N: i32> PacketComponent<C> for FixedBitSet<N> {
    type ComponentType = BitSet;

    fn decode<'a, A: AsyncRead + Unpin + Send + Sync + ?Sized>(
        _: &'a mut C,
        read: &'a mut A,
    ) -> PinnedLivelyResult<'a, Self::ComponentType> {
        Box::pin(async move {
            let mut bytes = vec![0; Self::FLOORED_SIZE];
            read.read_exact(&mut bytes).await?;
            BitSet::from_bytes(&bytes).await
        })
    }

    fn encode<'a, A: AsyncWrite + Unpin + Send + Sync + ?Sized>(
        component_ref: &'a Self::ComponentType,
        _: &'a mut C,
        write: &'a mut A,
    ) -> PinnedLivelyResult<'a, ()> {
        Box::pin(async move {
            write
                .write_all(&component_ref.to_byte_array().await?[0..Self::FLOORED_SIZE])
                .await?;
            Ok(())
        })
    }

    fn size(_: &Self::ComponentType, _: &mut C) -> drax::prelude::Result<Size> {
        Ok(Size::Constant(Self::FLOORED_SIZE))
    }
}
