use std::cmp::max;
use std::future::Future;
use std::pin::Pin;

use drax::prelude::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, PacketComponent, Size};

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

    fn value_of(l_arr: Vec<u64>) -> drax::prelude::Result<BitSet> {
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

    fn to_long_array(&self) -> Vec<u64> {
        self.words[..self.words_in_use].to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> drax::prelude::Result<BitSet> {
        let used_bits = bytes.iter().rposition(|x| *x != 0).unwrap_or(0);
        let mut words = vec![0; (used_bits + 7) / 8];
        let mut iter = bytes.iter();
        while {
            match iter.next_chunk::<8>() {
                Ok(chunk) => {
                    words.push(u64::from_be_bytes(chunk.map(|x| *x)));
                    true
                }
                Err(remaining) => {
                    let mut b: u64 = 0;
                    for (idx, item) in remaining.enumerate() {
                        b |= (*item as u64 & 0xffu64) << (8 * (idx as u64))
                    }
                    words.push(b);
                    false
                }
            }
        } {}
        let bit_set = BitSet {
            words_in_use: words.len(),
            words,
        };
        bit_set.assert_invariants()?;
        Ok(bit_set)
    }

    fn to_byte_array(&self) -> Vec<u8> {
        let mut extra = 0;
        for b in self.words[self.words_in_use].to_be_bytes().iter() {
            if *b == 0 {
                break;
            }
            extra += 1;
        }
        let mut bytes = vec![0; ((self.words_in_use - 1) * 8) + extra];
        for (idx, word) in self.words.iter().enumerate() {
            if idx == self.words_in_use {
                break;
            }
            bytes[idx * 8..(idx + 1) * 8].copy_from_slice(&word.to_be_bytes());
        }
        bytes.extend_from_slice(&self.words[self.words_in_use].to_be_bytes()[..extra]);
        bytes
    }
}

impl<C> PacketComponent<C> for BitSet {
    type ComponentType = BitSet;

    fn decode<'a, A: AsyncRead + Unpin + ?Sized>(
        context: &'a mut C,
        read: &'a mut A,
    ) -> Pin<Box<dyn Future<Output = drax::prelude::Result<Self::ComponentType>> + 'a>> {
        Box::pin(async move { BitSet::value_of(Vec::<u64>::decode(context, read).await?) })
    }

    fn encode<'a, A: AsyncWrite + Unpin + ?Sized>(
        component_ref: &'a Self::ComponentType,
        context: &'a mut C,
        write: &'a mut A,
    ) -> Pin<Box<dyn Future<Output = drax::prelude::Result<()>> + 'a>> {
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

impl<C, const N: i32> PacketComponent<C> for FixedBitSet<N> {
    type ComponentType = BitSet;

    fn decode<'a, A: AsyncRead + Unpin + ?Sized>(
        _: &'a mut C,
        read: &'a mut A,
    ) -> Pin<Box<dyn Future<Output = drax::prelude::Result<Self::ComponentType>> + 'a>> {
        Box::pin(async move {
            let mut bytes = vec![0; Self::FLOORED_SIZE];
            read.read_exact(&mut bytes).await?;
            BitSet::from_bytes(&bytes)
        })
    }

    fn encode<'a, A: AsyncWrite + Unpin + ?Sized>(
        component_ref: &'a Self::ComponentType,
        _: &'a mut C,
        write: &'a mut A,
    ) -> Pin<Box<dyn Future<Output = drax::prelude::Result<()>> + 'a>> {
        Box::pin(async move {
            write
                .write_all(&component_ref.to_byte_array()[0..Self::FLOORED_SIZE])
                .await?;
            Ok(())
        })
    }

    fn size(_: &Self::ComponentType, _: &mut C) -> drax::prelude::Result<Size> {
        Ok(Size::Constant(Self::FLOORED_SIZE))
    }
}
