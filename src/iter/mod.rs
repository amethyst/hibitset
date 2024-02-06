use util::*;
use {BitSetLike, GenericBitSet};

pub use self::drain::DrainBitIter;

#[cfg(feature = "parallel")]
pub use self::parallel::{BitParIter, BitProducer};

mod drain;
#[cfg(feature = "parallel")]
mod parallel;

/// An `Iterator` over a [`BitSetLike`] structure.
///
/// [`BitSetLike`]: ../trait.BitSetLike.html
#[derive(Debug, Clone)]
pub struct BitIter<T: BitSetLike> {
    pub(crate) set: T,
    pub(crate) masks: [T::Underlying; LAYERS],
    pub(crate) prefix: [u32; LAYERS - 1],
}

impl<T: BitSetLike> BitIter<T> {
    /// Creates a new `BitIter`. You usually don't call this function
    /// but just [`.iter()`] on a bit set.
    ///
    /// [`.iter()`]: ../trait.BitSetLike.html#method.iter
    pub fn new(set: T, masks: [T::Underlying; LAYERS], prefix: [u32; LAYERS - 1]) -> Self {
        BitIter {
            set: set,
            masks: masks,
            prefix: prefix,
        }
    }
}

impl<T: BitSetLike> BitIter<T> {
    /// Allows checking if set bit is contained in underlying bit set.
    pub fn contains(&self, i: Index) -> bool {
        self.set.contains(i)
    }
}

impl<'a, T: UnsignedInteger> BitIter<&'a mut GenericBitSet<T>> {
    /// Clears the rest of the bitset starting from the next inner layer.
    pub(crate) fn clear(&mut self) {
        use self::State::Continue;
        while let Some(level) = (1..LAYERS).find(|&level| self.handle_level(level) == Continue) {
            let lower = level - 1;
            let idx = (self.prefix[lower] >> T::LOG_BITS) as usize;
            *self.set.layer_mut(lower, idx) = T::ZERO;
            if level == LAYERS - 1 {
                self.set.layer3 &= T::from_u64(!((2usize << idx) - 1) as u64);
            }
        }
    }
}

#[derive(PartialEq)]
pub(crate) enum State {
    Empty,
    Continue,
    Value(Index),
}

impl<T> Iterator for BitIter<T>
where
    T: BitSetLike,
{
    type Item = Index;

    fn next(&mut self) -> Option<Self::Item> {
        use self::State::*;
        'find: loop {
            for level in 0..LAYERS {
                match self.handle_level(level) {
                    Value(v) => return Some(v),
                    Continue => continue 'find,
                    Empty => {}
                }
            }
            // There is no set bits left
            return None;
        }
    }
}

impl<T: BitSetLike> BitIter<T> {
    pub(crate) fn handle_level(&mut self, level: usize) -> State {
        use self::State::*;
        if self.masks[level] == T::Underlying::ZERO {
            Empty
        } else {
            // Take the first bit that isn't zero
            let first_bit = self.masks[level].trailing_zeros();
            // Remove it from the mask
            self.masks[level] &= !(T::Underlying::ONE << T::Underlying::from_u32(first_bit));
            // Calculate the index of it
            let idx = self.prefix.get(level).cloned().unwrap_or(0) | first_bit;
            if level == 0 {
                // It's the lowest layer, so the `idx` is the next set bit
                Value(idx)
            } else {
                // Take the corresponding `usize` from the layer below
                self.masks[level - 1] = self.set.get_from_layer(level - 1, idx as usize);
                self.prefix[level - 1] = idx << T::Underlying::LOG_BITS;
                Continue
            }
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate typed_test_gen;
    use self::typed_test_gen::test_with;

    use {BitSetLike, GenericBitSet, UnsignedInteger};

    #[test_with(u32, u64, usize)]
    fn iterator_clear_empties<T: UnsignedInteger>() {
        use rand::prelude::*;

        let mut set = GenericBitSet::<T>::new();
        let mut rng = thread_rng();
        let limit = 1_048_576;
        for _ in 0..(limit / 10) {
            set.add(rng.gen_range(0, limit));
        }
        (&mut set).iter().clear();
        assert_eq!(T::ZERO, set.layer3);
        for &i in &set.layer2 {
            assert_eq!(T::ZERO, i);
        }
        for &i in &set.layer1 {
            assert_eq!(T::ZERO, i);
        }
        for &i in &set.layer0 {
            assert_eq!(T::ZERO, i);
        }
    }

    #[test_with(u32, u64, usize)]
    fn iterator_clone<T: UnsignedInteger>() {
        let mut set = GenericBitSet::<T>::new();
        set.add(1);
        set.add(3);
        let iter = set.iter().skip(1);
        for (a, b) in iter.clone().zip(iter) {
            assert_eq!(a, b);
        }
    }
}
