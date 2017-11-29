use rayon::iter::ParallelIterator;
use rayon::iter::internal::{UnindexedProducer, UnindexedConsumer, Folder, bridge_unindexed};

use iter::{BITS, BitSetLike, BitIter, Index};
use util::average_ones;

/// A `ParallelIterator` over a [`BitSetLike`] structure.
///
/// [`BitSetLike`]: ../../trait.BitSetLike.html
#[derive(Debug)]
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
#[derive(Debug)]
pub struct BitProducer<'a, T: 'a + Send + Sync>(pub BitIter<&'a T>);

impl<'a, T: 'a + Send + Sync> UnindexedProducer for BitProducer<'a, T>
    where T: BitSetLike
{
    type Item = Index;

    /// How the splitting is done:
    ///
    /// 1) First the highest layer that has at least one set bit
    ///    is searched.
    ///
    /// 2) If the layer that was found only has one set bit,
    ///    it's cleared, the correct prefix for the bit is figured
    ///    out and descending is continued.
    ///
    /// 3) If the layer has more than one set bit, a mask is created
    ///    that splits the set bits of the layer as close to half
    ///    as possible.
    ///    After that the layer is masked by either the mask or
    ///    it's complement constructing two distinct producers which
    ///    are then returned.
    ///
    /// 4) If there isn't any layers that have more than one set bit,
    ///    splitting doesn't happen.
    ///
    /// The actual iteration is performed by the sequential iterator
    /// `BitIter` which internals are modified by this splitting
    ///  algorithm.
    ///
    /// The splitting is only done for 3 highest levels of the bitset
    /// and thus if all of the bits are set then the smallest possible unit
    /// of work is `usize` bits.
    fn split(mut self) -> (Self, Option<Self>) {
        let other = {
            let mut handle_level = |level: usize| if self.0.masks[level] == 0 {
                None
            } else {
                // Top levels prefix is zero because there is nothing before it
                let level_prefix = self.0.prefix.get(level).cloned().unwrap_or(0);
                let first_bit = self.0.masks[level].trailing_zeros();
                average_ones(self.0.masks[level])
                    .and_then(|average_bit| {
                        let mask = (1 << average_bit) - 1;
                        let mut other = BitProducer(BitIter::new(self.0.set, [0; 4], [0; 3]));
                        // `other` is the more significant half of the mask
                        other.0.masks[level] = self.0.masks[level] & !mask;
                        other.0.prefix[level - 1] = (level_prefix | average_bit as u32) << BITS;
                        // Upper portion prefix is maintained, because `other`
                        // will iterate the same subtree as `self`
                        other.0.prefix[level..].copy_from_slice(&self.0.prefix[level..]);
                        // And `self` is the less significant one
                        self.0.masks[level] &= mask;
                        self.0.prefix[level - 1] = (level_prefix | first_bit) << BITS;
                        Some(other)
                    }).or_else(|| {
                        // Because there is only one bit left we descend to it
                        let idx = level_prefix as usize | first_bit as usize;
                        self.0.prefix[level - 1] = (idx as u32) << BITS;
                        // The level that is descended from doesn't have anything
                        // interesting so it can be skipped in future.
                        self.0.masks[level] = 0;
                        self.0.masks[level - 1] = get_from_layer(self.0.set, level - 1, idx);
                        None
                    })
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

/// Gets usize by layer and index from bit set.
fn get_from_layer<T: BitSetLike>(set: &T, layer: usize, idx: usize) -> usize {
    match layer {
        0 => set.layer0(idx),
        1 => set.layer1(idx),
        2 => set.layer2(idx),
        3 => set.layer3(),
        _ => unreachable!("Invalid layer {}", layer),
    }
}

#[cfg(test)]
mod test_bit_producer {
    use rayon::iter::internal::UnindexedProducer;

    use super::BitProducer;
    use iter::BitSetLike;

    #[test]
    fn max_splitting_of_two_top_bits() {
        fn visit<T>(mut us: BitProducer<T>, d: usize, i: usize, mut trail: String, c: &mut usize)
            where T: Send +
                     Sync +
                     BitSetLike
        {
            if d == 0 {
                assert!(us.split().1.is_none());
                *c += 1;
            } else {
                for j in 1..(i + 1) {
                    let (new_us, them) = us.split();
                    us = new_us;
                    let them = them.expect(&trail);
                    let mut trail = trail.clone();
                    trail.push_str(&i.to_string());
                    visit(them, d, i - j, trail, c);
                }
                trail.push_str("u");
                visit(us, d - 1, 6, trail, c);
            }
        }

        let usize_bits = ::std::mem::size_of::<usize>() * 8;

        let mut c = ::BitSet::new();
        for i in 0..(usize_bits.pow(3) * 2) {
            assert!(!c.add(i as u32));
        }

        let us = BitProducer((&c).iter());
        let (us, them) = us.split();

        let mut count = 0;
        visit(us, 2, 6, "u".to_owned(), &mut count);
        visit(them.expect("Splitting top level"), 2, 6, "t".to_owned(), &mut count);
        assert_eq!(usize_bits.pow(2) * 2, count);
    }
}
