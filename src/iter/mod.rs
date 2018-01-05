use util::*;
use BitSetLike;

#[cfg(feature="parallel")]
pub use self::parallel::{BitParIter, BitProducer};

#[cfg(feature="parallel")]
mod parallel;

/// An `Iterator` over a [`BitSetLike`] structure.
///
/// [`BitSetLike`]: ../trait.BitSetLike.html
#[derive(Debug)]
pub struct BitIter<T> {
    pub(crate) set: T,
    pub(crate) masks: [usize; 4],
    pub(crate) prefix: [u32; 3],
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

impl<T: BitSetLike> BitIter<T> {
    /// Allows checking if set bit is contained in underlying bit set.
    pub fn contains(&self, i: Index) -> bool {
        self.set.contains(i)
    }
}

pub(crate) enum State {
    Empty,
    Continue,
    Value(Index)
}

impl<T> Iterator for BitIter<T>
    where T: BitSetLike
{
    type Item = Index;

    fn next(&mut self) -> Option<Self::Item> {
        use self::State::*;
        'find: loop {
            for level in 0..LAYERS {
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

impl<T: BitSetLike> BitIter<T> {
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
