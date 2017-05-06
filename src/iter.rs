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
            // Look at first level
            if self.masks[0] != 0 {
                // Take first bit that isn't zero
                let bit = self.masks[0].trailing_zeros();
                // Remove it from masks
                self.masks[0] &= !(1 << bit);
                // and returns it's index
                return Some(self.prefix[0] | bit);
            }
            // Look at second level
            if self.masks[1] != 0 {
                // Take first bit that isn't zero
                let bit = self.masks[1].trailing_zeros();
                // Remove it from masks
                self.masks[1] &= !(1 << bit);
                // Calculate index of the bit in first level
                let idx = self.prefix[1] | bit;
                // Take corresponding usize from layer below
                self.masks[0] = self.set.layer0(idx as usize);
                // Prefix of the complete index
                self.prefix[0] = idx << BITS;
                continue;
            }
            // Look at third level
            if self.masks[2] != 0 {
                // Take first bit that isn't zero
                let bit = self.masks[2].trailing_zeros();
                // Remove it from masks
                self.masks[2] &= !(1 << bit);
                // Calculate index of the bit in second level
                let idx = self.prefix[2] | bit;
                // Take corresponding usize from layer below
                self.masks[1] = self.set.layer1(idx as usize);
                // Prefix of the index of the second level
                self.prefix[1] = idx << BITS;
                continue;
            }
            // Look at the 4th and highest level
            if self.masks[3] != 0 {
                // Take first bit that isn't zero
                let bit = self.masks[3].trailing_zeros();
                // Remove it from masks
                self.masks[3] &= !(1 << bit);
                // Take corresponding usize from layer below
                self.masks[2] = self.set.layer2(bit as usize);
                // Prefix of the index of the third level
                self.prefix[2] = bit << BITS;
                continue;
            }
            // There is no set indices left
            return None;
        }
    }
}

#[cfg(feature="parallel")]
pub use self::par_iter::*;
/// A `ParallelIterator` over a [`BitSetLike`] structure.
///
/// [`BitSetLike`]: ../trait.BitSetLike.html
#[cfg(feature="parallel")]
pub struct BitParIter<T>(T);
#[cfg(feature="parallel")]
mod par_iter {
    use rayon::iter::ParallelIterator;
    use rayon::iter::internal::{UnindexedProducer, UnindexedConsumer, Folder, bridge_unindexed};
    use super::*;

    #[cfg(target_pointer_width="64")]
    pub const BITS_IN_USIZE: usize = 64;
    #[cfg(target_pointer_width="32")]
    pub const BITS_IN_USIZE: usize = 32;

    impl<T> BitParIter<T> {
        /// Creates a new `BitParIter`. You usually don't call this function
        /// but just [`.par_iter()`] on a bit set.
        ///
        /// [`.par_iter()`]: ../trait.BitSetLike.html#method.par_iter
        pub fn new(set: T) -> Self {
            BitParIter(set)
        }
    }

    impl<T> ParallelIterator for BitParIter<T>
        where T: BitSetLike + Send + Sync,
    {
        type Item = Index;

        fn drive_unindexed<C>(self, consumer: C) -> C::Result
            where C: UnindexedConsumer<Self::Item>
        {
            bridge_unindexed(BitProducer((&self.0).iter()), consumer)
        }
    }

    pub struct BitProducer<'a, T: 'a + Send + Sync>(pub BitIter<&'a T>);

    impl<'a, T: 'a + Send + Sync> UnindexedProducer for BitProducer<'a, T>
        where T: BitSetLike
    {
        type Item = Index;
        fn split(mut self) -> (Self, Option<Self>) {
            // Look at the 4th and highest level
            if self.0.masks[3] != 0 {
                // Find first bit set
                let first_bit = self.0.masks[3].trailing_zeros();
                // Find last bit set
                let last_bit = BITS_IN_USIZE as u32 - self.0.masks[3].leading_zeros() - 1;
                // Check that there is more than one bit set
                if first_bit != last_bit {
                    // Make the split point to be the avarage of first and last bit
                    let avarage = (first_bit + last_bit) / 2;
                    // Bit mask to get the lower half of the mask
                    let mask = (1 << avarage) - 1;
                    let other = BitProducer(BitIter {
                        set: self.0.set,
                        // Take the higher half of the mask
                        masks: [0, 0, 0, self.0.masks[3] & !mask],
                        // The higher half starts iterating from the avarage
                        prefix: [0, 0, avarage << BITS],
                    });
                    // Take the lower half the mask
                    self.0.masks[3] &= mask;
                    // The lower half starts iterating from the first bit
                    self.0.prefix[2] = first_bit << BITS;
                    return (self, Some(other));
                }
            }
            // Look at third level
            if self.0.masks[2] != 0 {
                // Find first bit set
                let first_bit = self.0.masks[2].trailing_zeros();
                // Find last bit set
                let last_bit = BITS_IN_USIZE as u32 - self.0.masks[2].leading_zeros() - 1;
                // Check that there is more than one bit set
                if first_bit != last_bit {
                    // Make the split point to be the avarage of first and last bit
                    let avarage = (first_bit + last_bit) / 2;
                    // Bit mask to get the lower half of the mask
                    let mask = (1 << avarage) - 1;
                    let other = BitProducer(BitIter {
                        set: self.0.set,
                        // Take the higher half of the mask
                        masks: [0, 0, self.0.masks[2] & !mask, 0],
                        // The higher half starts iterating from the avarage
                        prefix: [0, (self.0.prefix[2] | avarage) << BITS, self.0.prefix[2]],
                    });
                    // Take the lower half the mask
                    self.0.masks[2] &= mask;
                    // The lower half starts iterating from the first bit
                    self.0.prefix[2] = (self.0.prefix[2] | first_bit) << BITS;
                    return (self, Some(other));
                }
            }
            // Look at second level
            if self.0.masks[1] != 0 {
                // Find first bit set
                let first_bit = self.0.masks[1].trailing_zeros();
                // Find last bit set
                let last_bit = BITS_IN_USIZE as u32 - self.0.masks[1].leading_zeros() - 1;
                // Check that there is more than one bit set
                if first_bit != last_bit {
                    // Make the split point to be the avarage of first and last bit
                    let avarage = (first_bit + last_bit) / 2;
                    // Bit mask to get the lower half of the mask
                    let mask = (1 << avarage) - 1;
                    let other = BitProducer(BitIter {
                        set: self.0.set,
                        // Take the higher half of the mask
                        masks: [0, self.0.masks[1] & !mask, 0, 0],
                        // The higher half starts iterating from the avarage
                        prefix: [(self.0.prefix[1] | avarage) << BITS, self.0.prefix[1], self.0.prefix[2]],
                    });
                    // Take the lower half the mask
                    self.0.masks[1] &= mask;
                    // The lower half starts iterating from the first bit
                    self.0.prefix[0] = (self.0.prefix[1] | first_bit) << BITS;
                    return (self, Some(other));
                }
            }
            // No more splitting
            (self, None)
        }

        fn fold_with<F>(self, folder: F) -> F
            where F: Folder<Self::Item>
        {
            folder.consume_iter(self.0)
        }
    }
}
