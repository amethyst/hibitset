use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXorAssign, Div, Not, Shl, Shr, Sub};

/// Type used for indexing.
pub type Index = u32;

/// helper function to get the base 2 log of a const number
const fn base_2_log<const N: usize>() -> usize {
    match N {
        32 => 5,
        64 => 6,
        _ => unimplemented!(),
    }
}

/// Specifies the interface necessary for a `BitSet` to be built on top of `Self`
pub trait UnsignedInteger:
    Sized
    + Clone
    + Copy
    + Default
    + std::fmt::Debug
    + PartialEq
    + Not<Output = Self>
    + BitAnd<Output = Self>
    + BitAndAssign
    + BitOr<Output = Self>
    + BitOrAssign
    + BitXorAssign
    + Shl<Output = Self>
    + Shr<Output = Self>
    + Div<Output = Self>
    + Sub<Output = Self>
{
    /// value of zero
    const ZERO: Self;
    /// value of one
    const ONE: Self;
    /// all ones
    const MAX: Self;
    /// Number of bits per integer
    const BITS: usize;
    /// Base two log of the number of bits.
    const LOG_BITS: usize;
    /// Maximum amount of bits per bitset.
    const MAX_EID: u32 = (2 << (Self::LOG_BITS * LAYERS) - 1) as u32;
    /// Layer0 shift (bottom layer, true bitset).
    const SHIFT0: usize = 0;
    /// Layer1 shift (third layer).
    const SHIFT1: usize = Self::SHIFT0 + Self::LOG_BITS;
    /// Layer2 shift (second layer).
    const SHIFT2: usize = Self::SHIFT1 + Self::LOG_BITS;
    /// Top layer shift.
    const SHIFT3: usize = Self::SHIFT2 + Self::LOG_BITS;

    /// conversion function from Index type
    fn from_u32(val: u32) -> Self;
    /// conversion function from u64
    fn from_u64(val: u64) -> Self;
    /// conversion to u32
    fn to_u32(self) -> u32;
    /// conversion to u64
    fn to_u64(self) -> u64;
    /// Returns the number of trailing zeros in the binary representation of self.
    fn trailing_zeros(self) -> u32;
}

macro_rules! from_primitive_uint {
    ($type:ident) => {
        impl UnsignedInteger for $type {
            const ZERO: Self = 0;
            const ONE: Self = 1;
            const MAX: Self = Self::MAX;
            const BITS: usize = Self::BITS as usize;
            const LOG_BITS: usize = base_2_log::<{ Self::BITS as usize }>();
            #[inline(always)]
            fn from_u32(val: u32) -> Self {
                val as Self
            }
            #[inline(always)]
            fn from_u64(val: u64) -> Self {
                val as Self
            }
            #[inline(always)]
            fn to_u32(self) -> u32 {
                self as u32
            }
            #[inline(always)]
            fn to_u64(self) -> u64 {
                self as u64
            }
            #[inline(always)]
            fn trailing_zeros(self) -> u32 {
                self.trailing_zeros()
            }
        }
    };
}

from_primitive_uint!(usize);
from_primitive_uint!(u64);
from_primitive_uint!(u32);

/// Amount of layers in the hierarchical bitset.
pub const LAYERS: usize = 4;

pub trait Row: Sized + Copy {
    /// Location of the bit in the row.
    fn row<T: UnsignedInteger>(self, shift: usize) -> T;

    /// Index of the row that the bit is in.
    fn offset(self, shift: usize) -> usize;

    /// Bitmask of the row the bit is in.
    #[inline(always)]
    fn mask<T: UnsignedInteger>(self, shift: usize) -> T {
        T::ONE << self.row(shift)
    }
}

impl Row for Index {
    #[inline(always)]
    fn row<T: UnsignedInteger>(self, shift: usize) -> T {
        T::from_u32(self >> shift) & T::from_u32((1 << T::LOG_BITS) - 1)
    }

    #[inline(always)]
    fn offset(self, shift: usize) -> usize {
        self as usize / (1 << shift)
    }
}

/// Helper method for getting parent offsets of 3 layers at once.
///
/// Returns them in (Layer0, Layer1, Layer2) order.
#[inline]
pub fn offsets<T: UnsignedInteger>(bit: Index) -> (usize, usize, usize) {
    (
        bit.offset(T::SHIFT1),
        bit.offset(T::SHIFT2),
        bit.offset(T::SHIFT3),
    )
}

/// Finds the highest bit that splits set bits of the `usize`
/// to half (rounding up).
///
/// Returns `None` if the `usize` has only one or zero set bits.
///
/// # Examples
/// ````rust,ignore
/// use hibitset::util::average_ones;
///
/// assert_eq!(Some(4), average_ones(0b10110));
/// assert_eq!(Some(5), average_ones(0b100010));
/// assert_eq!(None, average_ones(0));
/// assert_eq!(None, average_ones(1));
/// ````
// TODO: Can 64/32 bit variants be merged to one implementation?
// Seems that this would need integer generics to do.
#[cfg(feature = "parallel")]
pub fn average_ones<T: UnsignedInteger>(n: T) -> Option<T> {
    let average = match T::BITS {
        32 => average_ones_u32(n.to_u32()).map(T::from_u32),
        64 => average_ones_u64(n.to_u64()).map(T::from_u64),
        _ => unimplemented!(),
    };

    average
}

#[cfg(feature = "parallel")]
fn average_ones_u32(n: u32) -> Option<u32> {
    // !0 / ((1 << (1 << n)) | 1)
    const PAR: [u32; 5] = [!0 / 0x3, !0 / 0x5, !0 / 0x11, !0 / 0x101, !0 / 0x10001];

    // Counting set bits in parallel
    let a = n - ((n >> 1) & PAR[0]);
    let b = (a & PAR[1]) + ((a >> 2) & PAR[1]);
    let c = (b + (b >> 4)) & PAR[2];
    let d = (c + (c >> 8)) & PAR[3];
    let mut cur = d >> 16;
    let count = (d + cur) & PAR[4];
    if count <= 1 {
        return None;
    }

    // Amount of set bits that are wanted for both sides
    let mut target = count / 2;

    // Binary search
    let mut result = 32;
    {
        let mut descend = |child, child_stride, child_mask| {
            if cur < target {
                result -= 2 * child_stride;
                target -= cur;
            }
            // Descend to upper half or lower half
            // depending on are we over or under
            cur = (child >> (result - child_stride)) & child_mask;
        };
        //(!PAR[n] & (PAR[n] + 1)) - 1
        descend(c, 8, 16 - 1); // PAR[3]
        descend(b, 4, 8 - 1); // PAR[2]
        descend(a, 2, 4 - 1); // PAR[1]
        descend(n, 1, 2 - 1); // PAR[0]
    }
    if cur < target {
        result -= 1;
    }

    Some(result - 1)
}

#[cfg(feature = "parallel")]
fn average_ones_u64(n: u64) -> Option<u64> {
    // !0 / ((1 << (1 << n)) | 1)
    const PAR: [u64; 6] = [
        !0 / 0x3,
        !0 / 0x5,
        !0 / 0x11,
        !0 / 0x101,
        !0 / 0x10001,
        !0 / 0x100000001,
    ];

    // Counting set bits in parallel
    let a = n - ((n >> 1) & PAR[0]);
    let b = (a & PAR[1]) + ((a >> 2) & PAR[1]);
    let c = (b + (b >> 4)) & PAR[2];
    let d = (c + (c >> 8)) & PAR[3];
    let e = (d + (d >> 16)) & PAR[4];
    let mut cur = e >> 32;
    let count = (e + cur) & PAR[5];
    if count <= 1 {
        return None;
    }

    // Amount of set bits that are wanted for both sides
    let mut target = count / 2;

    // Binary search
    let mut result = 64;
    {
        let mut descend = |child, child_stride, child_mask| {
            if cur < target {
                result -= 2 * child_stride;
                target -= cur;
            }
            // Descend to upper half or lower half
            // depending on are we over or under
            cur = (child >> (result - child_stride)) & child_mask;
        };
        //(!PAR[n] & (PAR[n] + 1)) - 1
        descend(d, 16, 256 - 1); // PAR[4]
        descend(c, 8, 16 - 1); // PAR[3]
        descend(b, 4, 8 - 1); // PAR[2]
        descend(a, 2, 4 - 1); // PAR[1]
        descend(n, 1, 2 - 1); // PAR[0]
    }
    if cur < target {
        result -= 1;
    }

    Some(result - 1)
}

#[cfg(all(test, feature = "parallel"))]
mod test_average_ones {
    use super::*;
    #[test]
    fn parity_0_average_ones_u32() {
        struct EvenParity(u32);

        impl Iterator for EvenParity {
            type Item = u32;
            fn next(&mut self) -> Option<Self::Item> {
                if self.0 == u32::max_value() {
                    return None;
                }
                self.0 += 1;
                while self.0.count_ones() & 1 != 0 {
                    if self.0 == u32::max_value() {
                        return None;
                    }
                    self.0 += 1;
                }
                Some(self.0)
            }
        }

        let steps = 1000;
        for i in 0..steps {
            let pos = i * (u32::max_value() / steps);
            for i in EvenParity(pos).take(steps as usize) {
                let mask = (1 << average_ones_u32(i).unwrap_or(31)) - 1;
                assert_eq!((i & mask).count_ones(), (i & !mask).count_ones(), "{:x}", i);
            }
        }
    }

    #[test]
    fn parity_1_average_ones_u32() {
        struct OddParity(u32);

        impl Iterator for OddParity {
            type Item = u32;
            fn next(&mut self) -> Option<Self::Item> {
                if self.0 == u32::max_value() {
                    return None;
                }
                self.0 += 1;
                while self.0.count_ones() & 1 == 0 {
                    if self.0 == u32::max_value() {
                        return None;
                    }
                    self.0 += 1;
                }
                Some(self.0)
            }
        }

        let steps = 1000;
        for i in 0..steps {
            let pos = i * (u32::max_value() / steps);
            for i in OddParity(pos).take(steps as usize) {
                let mask = (1 << average_ones_u32(i).unwrap_or(31)) - 1;
                let a = (i & mask).count_ones();
                let b = (i & !mask).count_ones();
                if a < b {
                    assert_eq!(a + 1, b, "{:x}", i);
                } else if b < a {
                    assert_eq!(a, b + 1, "{:x}", i);
                } else {
                    panic!("Odd parity shouldn't split in exactly half");
                }
            }
        }
    }

    #[test]
    fn empty_average_ones_u32() {
        assert_eq!(None, average_ones_u32(0));
    }

    #[test]
    fn singleton_average_ones_u32() {
        for i in 0..32 {
            assert_eq!(None, average_ones_u32(1 << i), "{:x}", i);
        }
    }

    #[test]
    fn parity_0_average_ones_u64() {
        struct EvenParity(u64);

        impl Iterator for EvenParity {
            type Item = u64;
            fn next(&mut self) -> Option<Self::Item> {
                if self.0 == u64::max_value() {
                    return None;
                }
                self.0 += 1;
                while self.0.count_ones() & 1 != 0 {
                    if self.0 == u64::max_value() {
                        return None;
                    }
                    self.0 += 1;
                }
                Some(self.0)
            }
        }

        let steps = 1000;
        for i in 0..steps {
            let pos = i * (u64::max_value() / steps);
            for i in EvenParity(pos).take(steps as usize) {
                let mask = (1 << average_ones_u64(i).unwrap_or(63)) - 1;
                assert_eq!((i & mask).count_ones(), (i & !mask).count_ones(), "{:x}", i);
            }
        }
    }

    #[test]
    fn parity_1_average_ones_u64() {
        struct OddParity(u64);

        impl Iterator for OddParity {
            type Item = u64;
            fn next(&mut self) -> Option<Self::Item> {
                if self.0 == u64::max_value() {
                    return None;
                }
                self.0 += 1;
                while self.0.count_ones() & 1 == 0 {
                    if self.0 == u64::max_value() {
                        return None;
                    }
                    self.0 += 1;
                }
                Some(self.0)
            }
        }

        let steps = 1000;
        for i in 0..steps {
            let pos = i * (u64::max_value() / steps);
            for i in OddParity(pos).take(steps as usize) {
                let mask = (1 << average_ones_u64(i).unwrap_or(63)) - 1;
                let a = (i & mask).count_ones();
                let b = (i & !mask).count_ones();
                if a < b {
                    assert_eq!(a + 1, b, "{:x}", i);
                } else if b < a {
                    assert_eq!(a, b + 1, "{:x}", i);
                } else {
                    panic!("Odd parity shouldn't split in exactly half");
                }
            }
        }
    }

    #[test]
    fn empty_average_ones_u64() {
        assert_eq!(None, average_ones_u64(0));
    }

    #[test]
    fn singleton_average_ones_u64() {
        for i in 0..64 {
            assert_eq!(None, average_ones_u64(1 << i), "{:x}", i);
        }
    }

    #[test]
    fn average_ones_agree_u32_u64() {
        let steps = 1000;
        for i in 0..steps {
            let pos = i * (u32::max_value() / steps);
            for i in pos..steps {
                assert_eq!(
                    average_ones_u32(i),
                    average_ones_u64(i as u64).map(|n| n as u32),
                    "{:x}",
                    i
                );
            }
        }
    }

    #[test]
    fn specific_values() {
        assert_eq!(Some(4), average_ones_u32(0b10110));
        assert_eq!(Some(5), average_ones_u32(0b100010));
        assert_eq!(None, average_ones_u32(0));
        assert_eq!(None, average_ones_u32(1));

        assert_eq!(Some(4), average_ones_u64(0b10110));
        assert_eq!(Some(5), average_ones_u64(0b100010));
        assert_eq!(None, average_ones_u64(0));
        assert_eq!(None, average_ones_u64(1));
    }
}
