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

use typenum::Add1;
use generic_array::{ArrayLength, GenericArray};

mod atomic;
mod iter;
mod ops;
#[allow(missing_docs)]
pub mod util;

pub use atomic::AtomicBitSet;
pub use iter::{BitIter, BitIterableNum};
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
#[derive(Clone, Debug)]
pub struct BitSet<N: ArrayLength<Vec<usize>> = DefaultLayers> {
    top_layer: usize,
    layers: GenericArray<Vec<usize>, N>,
}

impl Default for BitSet<DefaultLayers> {
    fn default() -> Self {
        Self::new()
    }
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
    fn valid_range(i: Index) {
        let max = N::bitset_max_size();
        if (max as u32) < i {
            panic!("Expected index to be less then {}, found {}", max, i);
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
pub trait BitSetLike<N = DefaultLayers>
    where N: BitIterableNum,
          Add1<N>: ArrayLength<usize>
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
          N: BitIterableNum,
          Add1<N>: ArrayLength<usize>
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
          N: BitIterableNum,
          Add1<N>: ArrayLength<usize>
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
    where N: BitIterableNum,
          Add1<N>: ArrayLength<usize>
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
