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

impl<T: BitSetLike> BitIter<T> {
    /// Allows checking if set bit is contained in underlying bit set.
    pub fn contains(&self, i: Index) -> bool {
        self.set.contains(i)
    }
}

enum State<T> {
    Empty,
    Continue,
    Value(T)
}

impl<T> Iterator for BitIter<T>
    where T: BitSetLike
{
    type Item = Index;

    fn next(&mut self) -> Option<Self::Item> {
        use self::State::*;
        let mut handle_level = |level: usize| if self.masks[level] == 0 {
            Empty
        } else {
            // Take first bit that isn't zero
            let first_bit = self.masks[level].trailing_zeros();
            // Remove it from masks
            self.masks[level] &= !(1 << first_bit);
            // Calculate index of the bit
            let idx = self.prefix.get(level).cloned().unwrap_or(0) | first_bit;
            if level == 0 {
                // It's the lowest layer so idx is the next bit in the set
                Value(idx)
            } else {
                // Take corresponding usize from layer below
                self.masks[level - 1] = get_from_layer(&self.set, level - 1, idx as usize);
                // Prefix of the complete index
                self.prefix[level - 1] = idx << BITS;
                Continue
            }
        };
        'find: loop {
            for level in 0..LAYERS {
                match handle_level(level) {
                    Value(v) => return Some(v),
                    Continue => continue 'find,
                    Empty => {},
                }
            }
            // There is no set indices left
            return None;
        }
    }
}
