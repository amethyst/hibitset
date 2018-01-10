use util::*;
use {BitSet, BitSetLike};

use typenum::{Add1, B1, Unsigned};
use generic_array::{ArrayLength, GenericArray};

use std::ops::Add;

#[cfg(feature="parallel")]
pub use self::parallel::{BitParIter, BitProducer};

#[cfg(feature="parallel")]
mod parallel;

/// Trait to clean up signatures of bitset iteration.
pub trait BitIterableNum: Add<B1> + ArrayLength<u32> + ArrayLength<Vec<usize>> {}

impl<N> BitIterableNum for N
where N: Unsigned +
         Add<B1> +
         ArrayLength<u32> +
         ArrayLength<Vec<usize>>
{}

/// An `Iterator` over a [`BitSetLike`] structure.
///
/// [`BitSetLike`]: ../trait.BitSetLike.html
#[derive(Debug)]
pub struct BitIter<T: BitSetLike<N>, N>
    where N: BitIterableNum,
          Add1<N>: ArrayLength<usize>
{
    pub(crate) set: T,
    pub(crate) masks: GenericArray<usize, Add1<N>>,
    pub(crate) prefix: GenericArray<u32, N>,
}

impl<T: BitSetLike<N>, N> BitIter<T, N>
    where N: BitIterableNum,
          Add1<N>: ArrayLength<usize>
{
    /// Creates a new `BitIter`. You usually don't call this function
    /// but just [`.iter()`] on a bit set.
    ///
    /// [`.iter()`]: ../trait.BitSetLike.html#method.iter
    pub fn new(set: T, masks: GenericArray<usize, Add1<N>>, prefix: GenericArray<u32, N>) -> Self {
        BitIter {
            set: set,
            masks: masks,
            prefix: prefix,
        }
    }
}

impl<T: BitSetLike<N>, N> BitIter<T, N>
    where N: BitIterableNum,
          Add1<N>: ArrayLength<usize>
{
    /// Allows checking if set bit is contained in underlying bit set.
    pub fn contains(&self, i: Index) -> bool {
        self.set.contains(i)
    }
}

impl<'a, N> BitIter<&'a mut BitSet<N>, N>
    where N: BitIterableNum,
          Add1<N>: ArrayLength<usize>
{
    /// Clears the rest of the bitset starting from the next inner layer.
    pub(crate) fn clear(&mut self) {
        use self::State::Continue;
        while let Some(level) = (1..self.masks.len()).find(|&level| self.handle_level(level) == Continue) {
            let lower = level - 1;
            let idx = (self.prefix[lower] >> BITS) as usize;
            *self.set.layer_mut(lower, idx) = 0;
            if level == self.masks.len() - 1 {
                *self.set.layer_mut(self.masks.len(), 0) &= !((2 << idx) - 1);
            }
        }
    }
}

#[derive(PartialEq)]
pub(crate) enum State {
    Empty,
    Continue,
    Value(Index)
}

impl<T: BitSetLike<N>, N> Iterator for BitIter<T, N>
    where N: BitIterableNum,
          Add1<N>: ArrayLength<usize>
{
    type Item = Index;

    fn next(&mut self) -> Option<Self::Item> {
        use self::State::*;
        'find: loop {
            for level in 0..(N::to_usize() + 1) {
                match self.handle_level(level) {
                    Value(v) => return Some(v),
                    Continue => continue 'find,
                    Empty => {},
                }
            }
            // There is no set bits left
            return None;
        }
    }
}

impl<T: BitSetLike<N>, N> BitIter<T, N>
    where N: BitIterableNum,
          Add1<N>: ArrayLength<usize>
{
    pub(crate) fn handle_level(&mut self, level: usize) -> State {
        use self::State::*;
        if self.masks[level] == 0 {
            Empty
        } else {
            // Take the first bit that isn't zero
            let first_bit = self.masks[level].trailing_zeros();
            // Remove it from the mask
            self.masks[level] &= !(1 << first_bit);
            // Calculate the index of it
            let idx = self.prefix.get(level).cloned().unwrap_or(0) | first_bit;
            if level == 0 {
                // It's the lowest layer, so the `idx` is the next set bit
                Value(idx)
            } else {
                // Take the corresponding `usize` from the layer below
                self.masks[level - 1] = self.set.get_from_layer(level - 1, idx as usize);
                self.prefix[level - 1] = idx << BITS;
                Continue
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ::{BitSet, BitSetLike};
    
    #[test]
    fn iterator_clear_empties() {
        use rand::{Rng, weak_rng};
        let mut set = BitSet::<::DefaultLayers>::new();
        let mut rng = weak_rng();
        let limit = 1_048_576;
        for _ in 0..(limit / 10) {
            set.add(rng.gen_range(0, limit));
        }
        (&mut set).iter().clear();
        assert_eq!(0, set.top_layer());
        for i in set.layers.iter() {
            for &i in i {
                assert_eq!(0, i);
            }
        }
    }
}
