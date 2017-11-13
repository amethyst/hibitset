
/// Type used for indexing.
pub type Index = u32;

/// Base two log of the number of bits in a usize.
#[cfg(target_pointer_width= "64")]
pub const BITS: usize = 6;
#[cfg(target_pointer_width= "32")]
pub const BITS: usize = 5;
/// Amount of layers in the hierarchical bitset.
pub const LAYERS: usize = 4;
pub const MAX: usize = BITS * LAYERS;
/// Maximum amount of bits per bitset.
pub const MAX_EID: usize = 2 << MAX - 1;

/// Layer0 shift (bottom layer, true bitset).
pub const SHIFT0: usize = 0;
/// Layer1 shift (third layer).
pub const SHIFT1: usize = SHIFT0 + BITS;
/// Layer2 shift (second layer).
pub const SHIFT2: usize = SHIFT1 + BITS;
/// Top layer shift.
pub const SHIFT3: usize = SHIFT2 + BITS;

pub trait Row: Sized + Copy {
    /// Location of the bit in the row.
    fn row(self, shift: usize) -> usize;

    /// Index of the row that the bit is in.
    fn offset(self, shift: usize) -> usize;

    /// Bitmask of the row the bit is in.
    #[inline(always)]
    fn mask(self, shift: usize) -> usize {
        1usize << self.row(shift)
    }
}

impl Row for Index {
    #[inline(always)]
    fn row(self, shift: usize) -> usize {
        ((self >> shift) as usize) & ((1 << BITS) - 1)
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
pub fn offsets(bit: Index) -> (usize, usize, usize) {
    (bit.offset(SHIFT1), bit.offset(SHIFT2), bit.offset(SHIFT3))
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
/// assert_eq!(Some(3), average_ones(0b10110));
/// assert_eq!(Some(6), average_ones(0b100010));
/// assert_eq!(None, average_ones(0));
/// assert_eq!(None, average_ones(1));
/// ````
// TODO: Can 64/32 bit variants be merged to one implementation?
pub fn average_ones(n: usize) -> Option<usize> {
    #[cfg(target_pointer_width= "64")]
    fn average(n: usize) -> Option<usize> {
        average_ones_u64(n as u64).map(|n| n as usize)
    }
    #[cfg(target_pointer_width= "32")]
    fn average(n: usize) -> Option<usize> {
        average_ones_u64(n as u32).map(|n| n as usize)
    }
    average(n)
}

#[allow(dead_code)]
fn average_ones_u32(n: u32) -> Option<u32> {
    use std::num::Wrapping as W;
    let n = W(n);
    const PAR: [W<u32>; 5] = [
        W(0x55555555),
        W(0x33333333),
        W(0x0F0F0F0F),
        W(0x00FF00FF),
        W(0x0000FFFF),
    ];

    // Counting set bits in parallel
    let a = n - ((n >> 1) & PAR[0]);
    let b = (a & PAR[1]) + ((a >> 2) & PAR[1]);
    let c = (b + (b >> 4)) & PAR[2];
    let d = (c + (c >> 8)) & PAR[3];
    let mut cur = d >> 16;
    let e = (d + cur) & PAR[4];
    if e <= W(1) {
        return None;
    }
    let mut target = e / W(2);

    // Branchless binary search
    let mut result = W(32);
    {
        let mut descend = |child, to_bits, child_stride, child_mask| {
            let diff = cur - target;
            // If cur < target then result -= (256 >> to_bits)
            result -= (diff & W(256)) >> to_bits;
            // If cur < target then target -= t
            target -= cur & (diff >> 8);
            // Descend to upper half or lower half
            // depending on are we over or under
            cur = (child >> (result - W(child_stride)).0 as usize) & W(child_mask);
        };
        descend(c, 4/*16*/,  8, 0b00001111);// PAR[3]
        descend(b, 5/* 8*/,  4, 0b00000111);// PAR[2]
        descend(a, 6/* 4*/,  2, 0b00000011);// PAR[1]
        descend(n, 7/* 2*/,  1, 0b00000001);// PAR[0]
    }
    result -= (cur - target & W(256)) >> 8;

    Some(result.0)
}

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
            let mask = (1 << (average_ones_u32(i).unwrap_or(32) - 1)) - 1;
            assert_eq!(
                (i & mask).count_ones(),
                (i & !mask).count_ones(),
                "{}", i
            );
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
            let mask = (1 << (average_ones_u32(i).unwrap_or(32) - 1)) - 1;
            let a = (i & mask).count_ones();
            let b = (i & !mask).count_ones();
            if a < b {
                assert_eq!(a + 1, b, "{:x}", i);
            } else if b < a{
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
        assert_eq!(
            None,
            average_ones_u32(1 << i),
            "{:x}", i
        );
    }
}

#[allow(dead_code)]
fn average_ones_u64(n: u64) -> Option<u64> {
    use std::num::Wrapping as W;
    let n = W(n);
    const PAR: [W<u64>; 6] = [
        W(0x5555555555555555),
        W(0x3333333333333333),
        W(0x0F0F0F0F0F0F0F0F),
        W(0x00FF00FF00FF00FF),
        W(0x0000FFFF0000FFFF),
        W(0x00000000FFFFFFFF)
    ];

    // Counting set bits in parallel
    let a = n - ((n >> 1) & PAR[0]);
    let b = (a & PAR[1]) + ((a >> 2) & PAR[1]);
    let c = (b + (b >> 4)) & PAR[2];
    let d = (c + (c >> 8)) & PAR[3];
    let e = (d + (d >> 16)) & PAR[4];
    let mut cur = e >> 32;
    let f = (e + cur) & PAR[5];
    if f <= W(1) {
        return None;
    }

    let mut target = f / W(2);

    // Branchless binary search
    let mut result = W(64);
    {
        let mut descend = |child, to_bits, child_stride, child_mask| {
            let diff = cur - target;
            // If cur < target then result -= (256 >> to_bits)
            result -= (diff & W(256)) >> to_bits;
            // If cur < target then target -= t
            target -= cur & (diff >> 8);
            // Descend to upper half or lower half
            // depending on are we over or under
            cur = (child >> (result - W(child_stride)).0 as usize) & W(child_mask);
        };
        descend(d, 3/*32*/, 16, 0xFF);// PAR[4]
        descend(c, 4/*16*/,  8, 0x0F);// PAR[3]
        descend(b, 5/* 8*/,  4, 0x07);// PAR[2]
        descend(a, 6/* 4*/,  2, 0x03);// PAR[1]
        descend(n, 7/* 2*/,  1, 0x01);// PAR[0]
    }
    result -= (cur - target & W(256)) >> 8;

    Some(result.0)
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
            let mask = (1 << (average_ones_u64(i).unwrap_or(64) - 1)) - 1;
            assert_eq!(
                (i & mask).count_ones(),
                (i & !mask).count_ones(),
                "{}", i
            );
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
            let mask = (1 << (average_ones_u64(i).unwrap_or(64) - 1)) - 1;
            let a = (i & mask).count_ones();
            let b = (i & !mask).count_ones();
            if a < b {
                assert_eq!(a + 1, b, "{:x}", i);
            } else if b < a{
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
        assert_eq!(
            None,
            average_ones_u64(1 << i),
            "{:x}", i
        );
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
                "{:x}", i
            );
        }
    }
}
