//! # hibitset
//!
//! Provides hierarchical bit sets,
//! which allow very fast iteration
//! on sparse data structures.

#![deny(missing_docs)]

extern crate typenum;
extern crate generic_array;
extern crate atom;
#[cfg(feature="parallel")]
extern crate rayon;
#[cfg(test)]
extern crate rand;

use typenum::{Add1, B1};
use generic_array::{ArrayLength, GenericArray};

use std::ops::Add;

mod atomic;
mod iter;
mod ops;
mod util;

pub use atomic::AtomicBitSet;
pub use iter::BitIter;
#[cfg(feature="parallel")]
pub use iter::{BitParIter, BitProducer};
pub use ops::{BitSetAnd, BitSetNot, BitSetOr, BitSetXor};

use util::*;

/// How many layers are there by default.
pub type DefaultLayers = typenum::U3;

/// A `BitSet` is a simple set designed to track entity indices for which
/// a certain component exists. It does not track the `Generation` of the
/// entities that it contains.
///
/// Note, a `BitSet` is limited by design to only `1,048,576` indices.
/// Adding beyond this limit will cause the `BitSet` to panic.
#[derive(Clone, Debug, Default)]
pub struct BitSet<N: ArrayLength<Vec<usize>> = DefaultLayers> {
    top_layer: usize,
    layers: GenericArray<Vec<usize>, N>,
}

impl<N: ArrayLength<Vec<usize>>> BitSet<N> {
    /// Creates an empty `BitSet`.
    pub fn new() -> Self {
        BitSet {
            top_layer: 0,
            layers: GenericArray::generate(|_| vec![]),
        }
    }

    #[inline]
    fn valid_range(max: Index) {
        if (MAX_EID as u32) < max {
            panic!("Expected index to be less then {}, found {}", MAX_EID, max);
        }
    }

    /// Creates an empty `BitSet`, preallocated for up to `max` indices.
    pub fn with_capacity(max: Index) -> Self {
        Self::valid_range(max);
        let mut value = BitSet::new();
        value.extend(max);
        value
    }

    #[inline(never)]
    fn extend(&mut self, id: Index) {
        Self::valid_range(id);
        for i in (0..self.layers.len()).rev() {
            let p = id.offset(BITS * (i + 1));
            Self::fill_up(&mut self.layers.as_mut_slice()[i], p);
        }
    }

    fn fill_up(vec: &mut Vec<usize>, upper_index: usize) {
        if vec.len() <= upper_index {
            vec.resize(upper_index + 1, 0);
        }
    }

    /// This is used to set the levels in the hierarchy
    /// when the lowest layer was set from 0.
    #[inline(never)]
    fn add_slow(&mut self, id: Index) {
        for i in 1..self.layers.len() {
            let p = id.offset(BITS * (i + 1));
            self.layers.as_mut_slice()[i][p] |= id.mask(BITS * i);
        }
        self.top_layer |= id.mask(BITS * self.layers.len());
    }

    /// Adds `id` to the `BitSet`. Returns `true` if the value was
    /// already in the set.
    #[inline]
    pub fn add(&mut self, id: Index) -> bool {
        let (p0, mask) = (id.offset(SHIFT1), id.mask(SHIFT0));

        if p0 >= self.layers[0].len() {
            self.extend(id);
        }

        if self.layers[0][p0] & mask != 0 {
            return true;
        }

        // we need to set the bit on every layer to indicate
        // that the value can be found here.
        let old = self.layers[0][p0];
        self.layers[0][p0] |= mask;
        if old == 0 {
            self.add_slow(id);
        } else {
            self.layers[0][p0] |= mask;
        }
        false
    }

    fn layer_mut(&mut self, level: usize, idx: usize) -> &mut usize {
        if level == self.layers.len() {
            &mut self.top_layer
        } else {
            let mut layer = &mut self.layers[level];
            Self::fill_up(&mut layer, idx);
            &mut layer[idx]
        }
    }

    /// Removes `id` from the set, returns `true` if the value
    /// was removed, and `false` if the value was not set
    /// to begin with.
    #[inline]
    pub fn remove(&mut self, id: Index) -> bool {
        let p0 = id.offset(SHIFT1);
        if p0 >= self.layers[0].len() {
            return false;
        }

        if self.layers[0][p0] & id.mask(SHIFT0) == 0 {
            return false;
        }

        // if the bitmask was set we need to clear
        // its bit from layer0 to 3. the layers abover only
        // should be cleared if the bit cleared was the last bit
        // in its set
        for i in 0..self.layers.len() {
            let p = id.offset(BITS * (i + 1));
            self.layers[i][p] &= !id.mask(BITS * i);
            if self.layers[i][p] != 0 {
                return true;
            }
        }

        self.top_layer &= !id.mask(BITS * self.layers.len());
        return true;
    }

    /// Returns `true` if `id` is in the set.
    #[inline]
    pub fn contains(&self, id: Index) -> bool {
        let p0 = id.offset(SHIFT1);
        p0 < self.layers[0].len() && (self.layers[0][p0] & id.mask(SHIFT0)) != 0
    }

    /// Completely wipes out the bit set.
    pub fn clear(&mut self) {
        for layer in self.layers.as_mut_slice() {
            layer.clear();
        }
        self.top_layer = 0;
    }
}

/// A generic interface for [`BitSetLike`]-like types.
///
/// Every `BitSetLike` is hierarchical, meaning that there
/// are multiple levels that branch out in a tree like structure.
///
/// Layer0 each bit represents one Index of the set
/// Layer1 each bit represents one `usize` of Layer0, and will be
/// set only if the word below it is not zero.
/// Layer2 has the same arrangement but with Layer1, and Layer3 with Layer2.
///
/// This arrangement allows for rapid jumps across the key-space.
///
/// [`BitSetLike`]: ../trait.BitSetLike.html
pub trait BitSetLike<N: ArrayLength<Vec<usize>>>
    where N: Add<B1>,
          Add1<N>: ArrayLength<usize>,
          N: ArrayLength<u32>,
          N: ArrayLength<Vec<usize>>,
{
    /// Gets the `usize` corresponding to layer and index.
    ///
    /// The `layer` should be in the range [0, N + 1]
    fn get_from_layer(&self, layer: usize, idx: usize) -> usize;

    /// Returns an `usize` where each bit represents which words in the second highest
    /// layer has been set.
    fn top_layer(&self) -> usize;

    /// Allows checking if set bit is contained in the bit set.
    fn contains(&self, i: Index) -> bool;

    /// Create an iterator that will scan over the keyspace
    fn iter(self) -> BitIter<Self, N>
        where Self: Sized
    {
        let mut masks = GenericArray::default();
        let top_layer = masks.len() - 1;
        masks[top_layer] = self.top_layer();
        BitIter::new(self, masks, GenericArray::default())
    }

    /// Create a parallel iterator that will scan over the keyspace
    #[cfg(feature="parallel")]
    fn par_iter(self) -> BitParIter<Self, N>
        where Self: Sized
    {
        BitParIter::new(self)
    }
}

impl<'a, T, N> BitSetLike<N> for &'a T
    where T: BitSetLike<N>,
          N: Add<B1>,
          Add1<N>: ArrayLength<usize>,
          N: ArrayLength<u32>,
          N: ArrayLength<Vec<usize>>,
{
    fn get_from_layer(&self, layer: usize, idx: usize) -> usize {
        (*self).get_from_layer(layer, idx)
    }

    #[inline]
    fn top_layer(&self) -> usize {
        (*self).top_layer()
    }

    #[inline]
    fn contains(&self, i: Index) -> bool {
        (*self).contains(i)
    }
}

impl<'a, T, N> BitSetLike<N> for &'a mut T
    where T: BitSetLike<N>,
          N: Add<B1>,
          Add1<N>: ArrayLength<usize>,
          N: ArrayLength<u32>,
          N: ArrayLength<Vec<usize>>,
{
    fn get_from_layer(&self, layer: usize, idx: usize) -> usize {
        (**self).get_from_layer(layer, idx)
    }

    #[inline]
    fn top_layer(&self) -> usize {
        (**self).top_layer()
    }

    #[inline]
    fn contains(&self, i: Index) -> bool {
        (**self).contains(i)
    }
}

impl<N> BitSetLike<N> for BitSet<N>
    where N: Add<B1>,
          Add1<N>: ArrayLength<usize>,
          N: ArrayLength<u32>,
          N: ArrayLength<Vec<usize>>,
{

    fn get_from_layer(&self, layer: usize, idx: usize) -> usize {
        self.layers[layer].get(idx).cloned().unwrap_or(0)
    }

    #[inline]
    fn top_layer(&self) -> usize {
        self.top_layer
    }

    #[inline]
    fn contains(&self, i: Index) -> bool {
        (&self).contains(i)
    }
}

#[cfg(test)]
mod tests {
    use super::{BitSet, BitSetAnd, BitSetNot, BitSetLike};

    #[test]
    fn insert() {
        let mut c = BitSet::<::DefaultLayers>::new();
        for i in 0..1_000 {
            assert!(!c.add(i));
            assert!(c.add(i));
        }

        for i in 0..1_000 {
            assert!(c.contains(i));
        }
    }

    #[test]
    fn insert_100k() {
        let mut c = BitSet::<::DefaultLayers>::new();
        for i in 0..100_000 {
            assert!(!c.add(i));
            assert!(c.add(i));
        }

        for i in 0..100_000 {
            assert!(c.contains(i));
        }
    }
    #[test]
    fn remove() {
        let mut c = BitSet::<::DefaultLayers>::new();
        for i in 0..1_000 {
            assert!(!c.add(i));
        }

        for i in 0..1_000 {
            assert!(c.contains(i));
            assert!(c.remove(i));
            assert!(!c.contains(i));
            assert!(!c.remove(i));
        }
    }

    #[test]
    fn iter() {
        let mut c = BitSet::<::DefaultLayers>::new();
        for i in 0..100_000 {
            c.add(i);
        }

        let mut count = 0;
        for (idx, i) in c.iter().enumerate() {
            count += 1;
            assert_eq!(idx, i as usize);
        }
        assert_eq!(count, 100_000);
    }

    #[test]
    fn iter_odd_even() {
        let mut odd = BitSet::<::DefaultLayers>::new();
        let mut even = BitSet::<::DefaultLayers>::new();
        for i in 0..100_000 {
            if i % 2 == 1 {
                odd.add(i);
            } else {
                even.add(i);
            }
        }

        assert_eq!((&odd).iter().count(), 50_000);
        assert_eq!((&even).iter().count(), 50_000);
        assert_eq!(BitSetAnd::new(&odd, &even).iter().count(), 0);
    }

    #[test]
    fn iter_random_add() {
        use rand::{Rng, weak_rng};
        let mut set = BitSet::<::DefaultLayers>::new();
        let mut rng = weak_rng();
        let limit = 1_048_576;
        let mut added = 0;
        for _ in 0..(limit / 10) {
            let index = rng.gen_range(0, limit);
            if !set.add(index) {
                added += 1;
            }
        }
        assert_eq!(set.iter().count(), added as usize);
    }

    #[test]
    fn iter_clusters() {
        let mut set = BitSet::<::DefaultLayers>::new();
        for x in 0..8 {
            let x = (x * 3) << (::BITS * 2); // scale to the last slot
            for y in 0..8 {
                let y = (y * 3) << (::BITS);
                for z in 0..8 {
                    let z = z * 2;
                    set.add(x + y + z);
                }
            }
        }
        assert_eq!(set.iter().count(), 8usize.pow(3));
    }

    #[test]
    fn not() {
        let mut c = BitSet::<::DefaultLayers>::new();
        for i in 0..10_000 {
            if i % 2 == 1 {
                c.add(i);
            }
        }
        let d = BitSetNot::new(c);
        for (idx, i) in d.iter().take(5_000).enumerate() {
            assert_eq!(idx * 2, i as usize);
        }
    }
}

#[cfg(all(test, feature="parallel"))]
mod test_parallel {
    use super::{BitSet, BitSetAnd, BitSetLike};
    use rayon::iter::ParallelIterator;

    #[test]
    fn par_iter_one() {
        let step = 5000;
        let tests = 1_048_576 / step;
        for n in 0..tests {
            let n = n * step;
            let mut set = BitSet::<::DefaultLayers>::new();
            set.add(n);
            assert_eq!(set.par_iter().count(), 1);
        }
        let mut set = BitSet::<::DefaultLayers>::new();
        set.add(1_048_576 - 1);
        assert_eq!(set.par_iter().count(), 1);
    }

    #[test]
    fn par_iter_random_add() {
        use rand::{Rng, weak_rng};
        use std::collections::HashSet;
        use std::sync::{Arc, Mutex};
        let mut set = BitSet::<::DefaultLayers>::new();
        let mut check_set = HashSet::new();
        let mut rng = weak_rng();
        let limit = 1_048_576;
        for _ in 0..(limit / 10) {
            let index = rng.gen_range(0, limit);
            set.add(index);
            check_set.insert(index);
        }
        let check_set = Arc::new(Mutex::new(check_set));
        let missing_set = Arc::new(Mutex::new(HashSet::new()));
        set.par_iter()
            .for_each(|n| {
                let check_set = check_set.clone();
                let missing_set = missing_set.clone();
                let mut check = check_set.lock().unwrap();
                if !check.remove(&n) {
                    let mut missing = missing_set.lock().unwrap();
                    missing.insert(n);
                }
            });
        let check_set = check_set.lock().unwrap();
        let missing_set = missing_set.lock().unwrap();
        if !check_set.is_empty() && !missing_set.is_empty() {
            panic!("There were values that didn't get iterated: {:?}
            There were values that got iterated, but that shouldn't be: {:?}", *check_set, *missing_set);
        }
        if !check_set.is_empty() {
            panic!("There were values that didn't get iterated: {:?}", *check_set);
        }
        if !missing_set.is_empty() {
            panic!("There were values that got iterated, but that shouldn't be: {:?}", *missing_set);
        }
    }

    #[test]
    fn par_iter_odd_even() {
        let mut odd = BitSet::<::DefaultLayers>::new();
        let mut even = BitSet::<::DefaultLayers>::new();
        for i in 0..100_000 {
            if i % 2 == 1 {
                odd.add(i);
            } else {
                even.add(i);
            }
        }

        assert_eq!((&odd).par_iter().count(), 50_000);
        assert_eq!((&even).par_iter().count(), 50_000);
        assert_eq!(BitSetAnd::new(&odd, &even).par_iter().count(), 0);
    }

    #[test]
    fn par_iter_clusters() {
        use std::collections::HashSet;
        use std::sync::{Arc, Mutex};
        let mut set = BitSet::<::DefaultLayers>::new();
        let mut check_set = HashSet::new();
        for x in 0..8 {
            let x = (x * 3) << (::BITS * 2); // scale to the last slot
            for y in 0..8 {
                let y = (y * 3) << (::BITS);
                for z in 0..8 {
                    let z = z * 2;
                    let index = x + y + z;
                    set.add(index);
                    check_set.insert(index);
                }
            }
        }
        let check_set = Arc::new(Mutex::new(check_set));
        let missing_set = Arc::new(Mutex::new(HashSet::new()));
        set.par_iter()
            .for_each(|n| {
                let check_set = check_set.clone();
                let missing_set = missing_set.clone();
                let mut check = check_set.lock().unwrap();
                if !check.remove(&n) {
                    let mut missing = missing_set.lock().unwrap();
                    missing.insert(n);
                }
            });
        let check_set = check_set.lock().unwrap();
        let missing_set = missing_set.lock().unwrap();
        if !check_set.is_empty() && !missing_set.is_empty() {
            panic!("There were values that didn't get iterated: {:?}
            There were values that got iterated, but that shouldn't be: {:?}", *check_set, *missing_set);
        }
        if !check_set.is_empty() {
            panic!("There were values that didn't get iterated: {:?}", *check_set);
        }
        if !missing_set.is_empty() {
            panic!("There were values that got iterated, but that shouldn't be: {:?}", *missing_set);
        }
    }
}
