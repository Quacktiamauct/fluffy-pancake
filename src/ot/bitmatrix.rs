use std::mem;
use std::ops::Range;
use std::ops::RangeFrom;
use std::ops::RangeFull;
use std::ops::RangeInclusive;
use std::ops::BitXor;
use std::ops::BitAnd;
use std::ops::BitXorAssign;
use std::ops::BitAndAssign;
use std::ops::Index;

// BitMatrix and BitVector
use bitvec::prelude::*;
use serde::{Deserialize, Serialize};

// PERF: Change to u128 or u64
pub type Block = u64;
pub const BLOCK_SIZE: usize = mem::size_of::<Block>() * 8;

#[repr(transparent)]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BitVector (
    pub BitVec<Block, Lsb0>,
);

impl BitVector {
    pub fn zeros(size : usize) -> Self {
        Self::from_bytes(vec![0x00u8; size / 8])
    }

    pub fn ones(size : usize) -> Self {
        Self::from_bytes(vec![0xFFu8; size / 8])
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn from_vec(vec: Vec<Block>) -> Self {
        Self(BitVec::from_vec(vec))
    }

    pub fn from_bytes(vec: Vec<u8>) -> Self {
        unsafe { // TODO: Fallback if alignment fails.
            let (head, body, tail) = vec.align_to::<Block>();
            debug_assert!(tail.is_empty());
            debug_assert!(head.is_empty());
            Self::from_vec(body.to_vec())
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            let (head, body, tail) = self.0.as_raw_slice().align_to::<u8>();
            debug_assert!(tail.is_empty());
            debug_assert!(head.is_empty());
            body
        }
    }

    pub fn as_mut_bytes(&mut self) -> &mut [u8] {
        unsafe {
            let (head, body, tail) = self.0.as_raw_mut_slice().align_to_mut::<u8>();
            debug_assert!(tail.is_empty());
            debug_assert!(head.is_empty());
            body
        }
    }
}

impl Index<usize> for BitVector {
    type Output = bool;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl BitXor for BitVector {
    type Output = Self;

    #[inline]
    fn bitxor(self, rhs: Self) -> Self::Output {
        BitVector(self.0 ^ rhs.0)
    }
}

impl BitXor for &BitVector {
    type Output = BitVector;

    #[inline]
    fn bitxor(self, rhs: Self) -> Self::Output {
        BitVector(self.0.clone() ^ rhs.0.clone())
    }
}

impl BitAnd for BitVector {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        BitVector(self.0 & rhs.0)
    }
}

impl BitXorAssign for BitVector {
    #[inline]
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl BitXorAssign<&Self> for BitVector {
    #[inline]
    fn bitxor_assign(&mut self, rhs: &Self) {
        debug_assert_eq!(self.0.len(), rhs.0.len(), "BitVector lengths must be equal");
        let lhs = self.0.as_raw_mut_slice();
        let rhs = rhs.0.as_raw_slice();
        for i in 0..lhs.len() {
            lhs[i] ^= rhs[i];
        }
    }
}


impl BitAndAssign for BitVector {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

#[repr(transparent)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BitMatrix {
    rows: Vec<BitVector>,
}

impl BitMatrix {
    pub fn new(rows: Vec<BitVector>) -> BitMatrix {
        BitMatrix { rows }
    }

    pub fn dims(&self) -> (usize, usize) {
        (self.rows.len(), self.rows[0].len())
    }

    // PERF: Work on bytes instead of booleans. See below.
    // // https://stackoverflow.com/questions/31742483/how-would-you-transpose-a-binary-matrix
    pub fn transpose(&self) -> BitMatrix {
        let (rows, cols) = self.dims();
        let mut new_rows = Vec::with_capacity(cols);
        for col in 0..cols {
            let mut new_row = BitVec::with_capacity(rows);
            for row in 0..rows {
                new_row.push(self.rows[row][col]);
            }
            new_rows.push(BitVector(new_row));
        }
        BitMatrix::new(new_rows)
    }
}

impl FromIterator<BitVector> for BitMatrix {
    fn from_iter<I: IntoIterator<Item = BitVector>>(iter: I) -> Self {
        let mut rows = Vec::new();
        for row in iter {
            rows.push(row);
        }
        BitMatrix::new(rows)
    }
}

impl IntoIterator for BitMatrix {
    type Item = BitVector;
    type IntoIter = std::vec::IntoIter<BitVector>;

    fn into_iter(self) -> Self::IntoIter {
        self.rows.into_iter()
    }
}

impl<'a> IntoIterator for &'a BitMatrix {
    type Item = &'a BitVector;
    type IntoIter = std::slice::Iter<'a, BitVector>;

    fn into_iter(self) -> Self::IntoIter {
        self.rows.iter()
    }
}

impl Index<usize> for BitMatrix {
    type Output = BitVector;

    fn index(&self, index: usize) -> &Self::Output {
        &self.rows[index]
    }
}

use std::ops::RangeTo;
impl Index<RangeTo<usize>> for BitMatrix {
    type Output = [BitVector];

    fn index(&self, index: RangeTo<usize>) -> &Self::Output {
        &self.rows[index]
    }
}

impl Index<Range<usize>> for BitMatrix {
    type Output = [BitVector];

    fn index(&self, index: Range<usize>) -> &Self::Output {
        &self.rows[index]
    }
}

impl Index<RangeInclusive<usize>> for BitMatrix {
    type Output = [BitVector];

    fn index(&self, index: RangeInclusive<usize>) -> &Self::Output {
        &self.rows[index]
    }
}

impl Index<RangeFrom<usize>> for BitMatrix {
    type Output = [BitVector];

    fn index(&self, index: RangeFrom<usize>) -> &Self::Output {
        &self.rows[index]
    }
}

impl Index<RangeFull> for BitMatrix {
    type Output = [BitVector];

    fn index(&self, _index: RangeFull) -> &Self::Output {
        &self.rows[..]
    }
}
