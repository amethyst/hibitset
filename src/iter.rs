use util::*;
use BitSetLike;

/// An `Iterator` over a [`BitSetLike`] structure.
///
/// [`BitSetLike`]: ../trait.BitSetLike.html
#[derive(Debug)]
pub struct BitIter<T> {
    set: T,
    masks: [usize; 4],
    prefix: [u32; 3],
}

impl<T> BitIter<T> {
    /// Creates a new `BitIter`. You usually don't call this function
    /// but just [`.iter()`] on a bit set.
    ///
    /// [`.iter()`]: ../trait.BitSetLike.html#method.iter
    pub fn new(set: T, masks: [usize; 4], prefix: [u32; 3]) -> Self {
        BitIter {
            set: set,
            masks: masks,
            prefix: prefix,
        }
    }
}

impl<T> Iterator for BitIter<T>
    where T: BitSetLike
{
    type Item = Index;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.masks[0] != 0 {
                let bit = self.masks[0].trailing_zeros();
                self.masks[0] &= !(1 << bit);
                return Some(self.prefix[0] | bit);
            }

            if self.masks[1] != 0 {
                let bit = self.masks[1].trailing_zeros();
                self.masks[1] &= !(1 << bit);
                let idx = self.prefix[1] | bit;
                self.masks[0] = self.set.layer0(idx as usize);
                self.prefix[0] = idx << BITS;
                continue;
            }

            if self.masks[2] != 0 {
                let bit = self.masks[2].trailing_zeros();
                self.masks[2] &= !(1 << bit);
                let idx = self.prefix[2] | bit;
                self.masks[1] = self.set.layer1(idx as usize);
                self.prefix[1] = idx << BITS;
                continue;
            }

            if self.masks[3] != 0 {
                let bit = self.masks[3].trailing_zeros();
                self.masks[3] &= !(1 << bit);
                self.masks[2] = self.set.layer2(bit as usize);
                self.prefix[2] = bit << BITS;
                continue;
            }

            return None;
        }
    }
}
