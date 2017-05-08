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
        // Look at the 4th and highest level
        if self.0.masks[3] != 0 {
            // Find first bit set
            let first_bit = self.0.masks[3].trailing_zeros();
            // Find last bit set
            let last_bit = (size_of::<usize>() * 8) as u32 - self.0.masks[3].leading_zeros() - 1;
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
            let last_bit = (size_of::<usize>() * 8) as u32 as u32 - self.0.masks[2].leading_zeros() - 1;
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
            let last_bit = (size_of::<usize>() * 8) as u32 as u32 - self.0.masks[1].leading_zeros() - 1;
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
