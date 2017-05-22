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
                // If the level is empty, there isn't anything go through
                if self.0.masks[level] == 0 {
                    return None;
                }
                // Find first bit that is set
                let first_bit = self.0.masks[level].trailing_zeros();
                // Find last bit that is set
                let last_bit = (size_of::<usize>() * 8) as u32 - self.0.masks[level].leading_zeros() - 1;
                // If this is the highest level, there is no prefix saved as it's always zero
                let level_prefix = self.0.prefix.get(level).cloned().unwrap_or(0);
                // If there is one bit left, descend
                if first_bit == last_bit {
                    let idx = (level_prefix | first_bit) as usize;
                    // When descending all of the iteration happens in the child of the bit that is left
                    self.0.prefix[level - 1] = (idx as u32) << BITS;
                    // And because all the iteration happens in the child, it's parent can be removed.
                    self.0.masks[level] = 0;
                    // Get the mask of the child layer from the set
                    self.0.masks[level - 1] = get_from_layer(self.0.set, level - 1, idx);
                    return None;
                }
                // Make the split point to be the avarage of first and last bit
                let average = (first_bit + last_bit) / 2 + 1;
                // A bit mask to get the lower half of the mask
                let mask = (1 << average) - 1;
                let mut other = BitProducer(BitIter::new(self.0.set, [0, 0, 0, 0], [0, 0, 0]));
                let original_mask = self.0.masks[level];
                // Take the higher half of the mask
                other.0.masks[level] = original_mask & !mask;
                // The higher levels of masks don't need to preserved as they are empty.
                for n in &self.0.masks[(level + 1)..] {
                    debug_assert_eq!(*n, 0);
                }
                // The higher half starts iterating from the average
                other.0.prefix[level - 1] = (level_prefix | average) << BITS;
                // And preserve the prefix of the higher levels
                other.0.prefix[level..].copy_from_slice(&self.0.prefix[level..]);
                // Take the lower half the mask
                self.0.masks[level] = original_mask & mask;
                // Combined mask of the current level should now equal the original mask
                debug_assert_eq!(self.0.masks[level] | other.0.masks[level], original_mask);
                // The lower half starts iterating from the first bit
                self.0.prefix[level - 1] = (level_prefix | first_bit) << BITS;
                Some(other)
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

fn get_from_layer<T: BitSetLike>(set: &T, layer: usize, idx: usize) -> usize {
    match layer {
        0 => set.layer0(idx),
        1 => set.layer1(idx),
        2 => set.layer2(idx),
        3 => set.layer3(),
        _ => unreachable!("Invalid layer {}", layer),
    }
}
