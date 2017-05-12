use std::mem::size_of;

use rayon::iter::ParallelIterator;
use rayon::iter::internal::{UnindexedProducer, UnindexedConsumer, Folder, bridge_unindexed};

use iter::{BITS, BitSetLike, BitIter, Index};

/// An `ParallelIterator` over a [`BitSetLike`] structure.
///
/// [`BitSetLike`]: ../../trait.BitSetLike.html
pub struct BitParIter<T>(T);

impl<T> BitParIter<T> {
    /// Creates a new `BitParIter`. You usually don't call this function
    /// but just [`.par_iter()`] on a bit set.
    ///
    /// [`.par_iter()`]: ../../trait.BitSetLike.html#method.par_iter
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


/// Allows splitting and internally iterating through `BitSet`.
///
/// Usually used internally by `BitParIter`.
pub struct BitProducer<'a, T: 'a + Send + Sync>(pub BitIter<&'a T>);

impl<'a, T: 'a + Send + Sync> UnindexedProducer for BitProducer<'a, T>
    where T: BitSetLike
{
    type Item = Index;
    fn split(mut self) -> (Self, Option<Self>) {
        let other = {
            let mut handle_level = |level: usize| {
                // Check that the level isn't empty
                if self.0.masks[level] != 0 {
                    // Find first bit that is set
                    let first_bit = self.0.masks[level].trailing_zeros();
                    // Find last bit that is set
                    let last_bit = (size_of::<usize>() * 8) as u32 - self.0.masks[level].leading_zeros() - 1;
                    // Check that there is more than one bit that is set
                    if first_bit != last_bit {
                        // Make the split point to be the avarage of first and last bit
                        let avarage = (first_bit + last_bit) / 2;
                        // A bit mask to get the lower half of the mask
                        let mask = (1 << avarage) - 1;
                        let level_prefix = if level == self.0.prefix.len() {
                            0
                        } else {
                            self.0.prefix[level]
                        };
                        let mut other = BitProducer(BitIter {
                            set: self.0.set,
                            masks: [0, 0, 0, 0],
                            prefix: [0, 0, 0],
                        });
                        // Take the higher half of the mask
                        other.0.masks[level] = self.0.masks[level] & !mask;
                        // The higher half starts iterating from the avarage
                        other.0.prefix[level - 1] = (level_prefix | avarage) << BITS;
                        // And preserves the prefix of higher levels
                        for n in level..self.0.prefix.len() {
                            other.0.prefix[n] = self.0.prefix[n];
                        }
                        // Take the lower half the mask
                        self.0.masks[level] &= mask;
                        // The lower half starts iterating from the first bit
                        self.0.prefix[level - 1] = (level_prefix | first_bit) << BITS;
                        return Some(other);
                    }
                }
                None
            };
            handle_level(3)
                .or_else(|| handle_level(2))
                .or_else(|| handle_level(1))
        };
        (self, other)
    }

    fn fold_with<F>(self, folder: F) -> F
        where F: Folder<Self::Item>
    {
        folder.consume_iter(self.0)
    }
}
