pub type Index = u32;

#[cfg(target_pointer_width= "64")]
pub const BITS: usize = 6;
#[cfg(target_pointer_width= "32")]
pub const BITS: usize = 5;
pub const LAYERS: usize = 4;
pub const MAX: usize = BITS * LAYERS;
pub const MAX_EID: usize = 2 << MAX - 1;

pub const SHIFT0: usize = 0;
pub const SHIFT1: usize = SHIFT0 + BITS;
pub const SHIFT2: usize = SHIFT1 + BITS;
pub const SHIFT3: usize = SHIFT2 + BITS;

pub trait Row: Sized + Copy {
    fn row(self, shift: usize) -> usize;
    fn offset(self, shift: usize) -> usize;

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

#[inline]
pub fn offsets(bit: Index) -> (usize, usize, usize) {
    (bit.offset(SHIFT1), bit.offset(SHIFT2), bit.offset(SHIFT3))
}
