use num_traits::PrimInt;
use rand::Rng;

// Global Constants
pub const SECURITY_PARAM: usize = 256; // bits used total
pub const LENGTH: usize = SECURITY_PARAM / 8; // bytes used

#[inline]
pub fn rng(max: u16) -> u16 {
    rand::thread_rng().gen_range(0..max)
}

#[inline]
pub fn log2<N: PrimInt>(x: N) -> u32 {
    (std::mem::size_of::<N>() * 8) as u32 - (x - N::one()).leading_zeros()
}

pub fn u8_vec_to_bool_vec(str: &[u8]) -> Vec<bool> {
    let mut bits = Vec::with_capacity(8 * str.len());
    for s in str {
        for i in 0..8 {
            bits.push((s >> i) & 1 == 1);
        }
    }
    bits
}

pub type Bytes = [u8; LENGTH];

pub fn to_array(bytes: &[u8]) -> [u8; LENGTH] {
    debug_assert!(bytes.len() == LENGTH, "Should be {} bytes", LENGTH);
    let mut array = [0u8; LENGTH];
    array.copy_from_slice(bytes);
    array
}

/// Variadic Hashing
/// Hashing based on Sha256 producing a 32 byte hash
/// Arguments are hashed in order.
#[macro_export]
macro_rules! hash {
    // Decompose
    ($($ls:expr),+) => {{
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hash!(@next hasher, $($ls),+);
        let digest = hasher.finalize();
        <[u8; 256/8]>::try_from(digest.as_ref()).expect("digest too long")
    }};

    // Recursive update
    (@next $hasher:expr, $e:expr, $($ls:expr),+) => {{
        $hasher.update($e);
        hash!(@next $hasher, $($ls),+)
    }};

    // Last
    (@next $hasher:expr, $e:expr) => {{
        $hasher.update($e);
    }};
}
pub(crate) use hash;

pub fn xor(a: Bytes, b: Bytes) -> Bytes {
    let mut result = [0u8; LENGTH];
    for i in 0..LENGTH {
        result[i] = a[i] ^ b[i];
    }
    result
}

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    use super::*;

    #[test]
    fn test_hash() {
        let h1 = hash!("hello", "world");
        let mut hasher = Sha256::new();
        hasher.update("hello");
        hasher.update("world");
        let h2 = hasher.finalize();
        let h2 = <[u8; LENGTH]>::try_from(h2.as_ref()).expect("digest too long");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_log2() {
        assert!(log2(1) == 0);
        assert!(log2(2) == 1);
        assert!(log2(3) == 2);
        assert!(log2(4) == 2);
        assert!(log2(5) == 3);
        assert!(log2(8) == 3);
        assert!(log2(9) == 4);
    }
}
