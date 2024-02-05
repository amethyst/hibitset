//! # hibitset
//!
//! Provides hierarchical bit sets,
//! which allow very fast iteration
//! on sparse data structures.
//!
//! ## What it does
//!
//! A `BitSet` may be considered analogous to a `HashSet<u32>`. It
//! tracks whether or not certain indices exist within it. Its
//! implementation is very different, however.
//!
//! At its root, a `BitSet` relies on an array of bits, which express
//! whether or not indices exist. This provides the functionality to
//! `add( )` and `remove( )` indices.
//!
//! This array is referred to as Layer 0. Above it, there is another
//! layer: Layer 1. Layer 1 acts as a 'summary' of Layer 0. It contains
//! one bit for each `usize` bits of Layer 0. If any bit in that `usize`
//! of Layer 0 is set, the bit in Layer 1 will be set.
//!
//! There are, in total, four layers. Layers 1 through 3 are each a
//! summary of the layer immediately below them.
//!
//! ```no_compile
//! Example, with an imaginary 4-bit usize:
//!
//! Layer 3: 1------------------------------------------------ ...
//! Layer 2: 1------------------ 1------------------ 0-------- ...
//! Layer 1: 1--- 0--- 0--- 0--- 1--- 0--- 1--- 0--- 0--- 0--- ...
//! Layer 0: 0010 0000 0000 0000 0011 0000 1111 0000 0000 0000 ...
//! ```
//!
//! This method makes operations that operate over the whole `BitSet`,
//! such as unions, intersections, and iteration, very fast (because if
//! any bit in any summary layer is zero, an entire range of bits
//! below it can be skipped.)
//!
//! However, there is a maximum on index size. The top layer (Layer 3)
//! of the BitSet is a single integer long (depending on the underlying type used).
//! This makes the maximum index `BITS**4` (`1,048,576` for a 32-bit `usize`,
//! `16,777,216` for a 64-bit `usize`). Attempting to add indices larger than that
//! will cause the `BitSet` to panic.
//!

#![deny(missing_docs)]

#[cfg(test)]
extern crate rand;
#[cfg(feature = "parallel")]
extern crate rayon;

mod atomic;
mod iter;
mod ops;
mod util;

pub use atomic::AtomicBitSet;
pub use iter::{BitIter, DrainBitIter};
#[cfg(feature = "parallel")]
pub use iter::{BitParIter, BitProducer};
pub use ops::{BitSetAll, BitSetAnd, BitSetNot, BitSetOr, BitSetXor};

use util::*;

/// A `GenericBitSet` is a simple set designed to track which indices are placed
/// into it. Is is based on an underlying type `T` which is supposed to represent an
/// unsigned integer type (in particular `u32`, `u64`, or `usize`).
///
/// Note, a `BitSet` is limited by design to only `T::NB_BITS**4` indices.
/// Adding beyond this limit will cause the `BitSet` to panic.
#[derive(Clone, Debug, Default)]
pub struct GenericBitSet<T: UnsignedInteger> {
    layer3: T,
    layer2: Vec<T>,
    layer1: Vec<T>,
    layer0: Vec<T>,
}

/// A `BitSet` is a simple set designed to track which indices are placed
/// into it.
///
/// Note, a `BitSet` is limited by design to only `usize**4` indices.
/// Adding beyond this limit will cause the `BitSet` to panic.
pub type BitSet = GenericBitSet<usize>;

impl<T: UnsignedInteger> GenericBitSet<T> {
    /// Creates an empty `BitSet`.
    pub fn new() -> Self {
        Default::default()
    }

    #[inline]
    fn valid_range(max: Index) {
        if T::MAX_EID < max {
            panic!(
                "Expected index to be less then {}, found {}",
                T::MAX_EID,
                max
            );
        }
    }

    /// Creates an empty `BitSet`, preallocated for up to `max` indices.
    pub fn with_capacity(max: Index) -> Self {
        Self::valid_range(max);
        let mut value = Self::new();
        value.extend(max);
        value
    }

    #[inline(never)]
    fn extend(&mut self, id: Index) {
        Self::valid_range(id);
        let (p0, p1, p2) = offsets::<T>(id);

        Self::fill_up(&mut self.layer2, p2);
        Self::fill_up(&mut self.layer1, p1);
        Self::fill_up(&mut self.layer0, p0);
    }

    fn fill_up(vec: &mut Vec<T>, upper_index: usize) {
        if vec.len() <= upper_index {
            vec.resize(upper_index + 1, T::ZERO);
        }
    }

    /// This is used to set the levels in the hierarchy
    /// when the lowest layer was set from 0.
    #[inline(never)]
    fn add_slow(&mut self, id: Index) {
        let (_, p1, p2) = offsets::<T>(id);
        self.layer1[p1] |= id.mask::<T>(T::SHIFT1);
        self.layer2[p2] |= id.mask::<T>(T::SHIFT2);
        self.layer3 |= id.mask::<T>(T::SHIFT3);
    }

    /// Adds `id` to the `BitSet`. Returns `true` if the value was
    /// already in the set.
    #[inline]
    pub fn add(&mut self, id: Index) -> bool {
        let (p0, mask) = (id.offset(T::SHIFT1), id.mask::<T>(T::SHIFT0));

        if p0 >= self.layer0.len() {
            self.extend(id);
        }

        if self.layer0[p0] & mask != T::ZERO {
            return true;
        }

        // we need to set the bit on every layer to indicate
        // that the value can be found here.
        let old = self.layer0[p0];
        self.layer0[p0] |= mask;
        if old == T::ZERO {
            self.add_slow(id);
        }
        false
    }

    fn layer_mut(&mut self, level: usize, idx: usize) -> &mut T {
        match level {
            0 => {
                Self::fill_up(&mut self.layer0, idx);
                &mut self.layer0[idx]
            }
            1 => {
                Self::fill_up(&mut self.layer1, idx);
                &mut self.layer1[idx]
            }
            2 => {
                Self::fill_up(&mut self.layer2, idx);
                &mut self.layer2[idx]
            }
            3 => &mut self.layer3,
            _ => panic!("Invalid layer: {}", level),
        }
    }

    /// Removes `id` from the set, returns `true` if the value
    /// was removed, and `false` if the value was not set
    /// to begin with.
    #[inline]
    pub fn remove(&mut self, id: Index) -> bool {
        let (p0, p1, p2) = offsets::<T>(id);

        if p0 >= self.layer0.len() {
            return false;
        }

        if self.layer0[p0] & id.mask::<T>(T::SHIFT0) == T::ZERO {
            return false;
        }

        // if the bitmask was set we need to clear
        // its bit from layer0 to 3. the layers abover only
        // should be cleared if the bit cleared was the last bit
        // in its set
        self.layer0[p0] &= !id.mask::<T>(T::SHIFT0);
        if self.layer0[p0] != T::ZERO {
            return true;
        }

        self.layer1[p1] &= !id.mask::<T>(T::SHIFT1);
        if self.layer1[p1] != T::ZERO {
            return true;
        }

        self.layer2[p2] &= !id.mask::<T>(T::SHIFT2);
        if self.layer2[p2] != T::ZERO {
            return true;
        }

        self.layer3 &= !id.mask::<T>(T::SHIFT3);
        return true;
    }

    /// Returns `true` if `id` is in the set.
    #[inline]
    pub fn contains(&self, id: Index) -> bool {
        let p0 = id.offset(T::SHIFT1);
        p0 < self.layer0.len() && (self.layer0[p0] & id.mask::<T>(T::SHIFT0)) != T::ZERO
    }

    /// Returns `true` if all ids in `other` are contained in this set
    #[inline]
    pub fn contains_set<S>(&self, other: &S) -> bool
    where
        S: BitSetLike<Underlying = T>,
    {
        for id in other.iter() {
            if !self.contains(id) {
                return false;
            }
        }
        true
    }

    /// Completely wipes out the bit set.
    pub fn clear(&mut self) {
        self.layer0.clear();
        self.layer1.clear();
        self.layer2.clear();
        self.layer3 = T::ZERO;
    }

    /// How many bits are in a `usize`.
    ///
    /// This value can be trivially determined. It is provided here as a constant for clarity.
    ///
    /// # Example
    ///
    /// ```
    /// use hibitset::BitSet;
    /// assert_eq!(BitSet::BITS_PER_USIZE, std::mem::size_of::<usize>()*8);
    /// ```
    #[cfg(target_pointer_width = "32")]
    pub const BITS_PER_USIZE: usize = 32;

    /// How many bits are in a `usize`.
    ///
    /// This value can be trivially determined. It is provided here as a constant for clarity.
    ///
    /// # Example
    ///
    /// ```
    /// use hibitset::BitSet;
    /// assert_eq!(BitSet::BITS_PER_USIZE, std::mem::size_of::<usize>()*8);
    /// ```
    #[cfg(target_pointer_width = "64")]
    pub const BITS_PER_USIZE: usize = 64;

    /// Returns the bottom layer of the bitset as a slice. Each bit in this slice refers to a single
    /// `Index`.
    ///
    /// The slice's length will be at least the length needed to reflect all the `1`s in the bitset,
    /// but is not otherwise guaranteed. Consider it to be an implementation detail.
    ///
    /// # Example
    ///
    /// ```
    /// use hibitset::BitSet;
    ///
    /// let index: u32 = 12345;
    ///
    /// let mut bitset = BitSet::new();
    /// bitset.add(index);
    ///
    /// // layer 0 is 1:1 with Indexes, so we expect that bit in the slice to be set
    /// let slice = bitset.layer0_as_slice();
    /// let bit_index = index as usize;
    ///
    /// // map that bit index to a usize in the slice and a bit within that usize
    /// let slice_index = bit_index / BitSet::BITS_PER_USIZE;
    /// let bit_at_index = bit_index % BitSet::BITS_PER_USIZE;
    ///
    /// assert_eq!(slice[slice_index], 1 << bit_at_index);
    /// ```
    pub fn layer0_as_slice(&self) -> &[T] {
        self.layer0.as_slice()
    }

    /// How many `Index`es are described by as single layer 1 bit, intended for use with
    /// `BitSet::layer1_as_slice()`.
    ///
    /// `BitSet`s are defined in terms of `usize`s summarizing `usize`s, so this value can be
    /// trivially determined. It is provided here as a constant for clarity.
    ///
    /// # Example
    ///
    /// ```
    /// use hibitset::BitSet;
    /// assert_eq!(BitSet::LAYER1_GRANULARITY, BitSet::BITS_PER_USIZE);
    /// ```
    pub const LAYER1_GRANULARITY: usize = Self::BITS_PER_USIZE;

    /// Returns the second layer of the bitset as a slice. Each bit in this slice summarizes a
    /// corresponding `usize` from `layer0`. (If `usize` is 64 bits, bit 0 will be set if any
    /// `Index`es 0-63 are set, bit 1 will be set if any `Index`es 64-127 are set, etc.)
    /// `BitSet::LAYER1_GRANULARITY` reflects how many indexes are summarized per layer 1 bit.
    ///
    /// The slice's length is not guaranteed, except that it will be at least the length needed to
    /// reflect all the `1`s in the bitset.
    ///
    /// # Example
    ///
    /// ```
    /// use hibitset::BitSet;
    ///
    /// let index: u32 = 12345;
    ///
    /// let mut bitset = BitSet::new();
    /// bitset.add(index);
    ///
    /// // layer 1 summarizes multiple indexes per bit, so divide appropriately
    /// let slice = bitset.layer1_as_slice();
    /// let bit_index = index as usize / BitSet::LAYER1_GRANULARITY;
    ///
    /// // map that bit index to a usize in the slice and a bit within that usize
    /// let slice_index = bit_index / BitSet::BITS_PER_USIZE;
    /// let bit_at_index = bit_index % BitSet::BITS_PER_USIZE;
    ///
    /// assert_eq!(slice[slice_index], 1 << bit_at_index);
    /// ```
    pub fn layer1_as_slice(&self) -> &[T] {
        self.layer1.as_slice()
    }

    /// How many `Index`es are described by as single layer 2 bit, intended for use with
    /// `BitSet::layer2_as_slice()`.
    ///
    /// `BitSet`s are defined in terms of `usize`s summarizing `usize`s, so this value can be
    /// trivially determined. It is provided here as a constant for clarity.
    ///
    /// # Example
    ///
    /// ```
    /// use hibitset::BitSet;
    /// assert_eq!(BitSet::LAYER2_GRANULARITY, BitSet::LAYER1_GRANULARITY * BitSet::BITS_PER_USIZE);
    /// ```
    pub const LAYER2_GRANULARITY: usize = Self::LAYER1_GRANULARITY * Self::BITS_PER_USIZE;

    /// Returns the third layer of the bitset as a slice. Each bit in this slice summarizes a
    /// corresponding `usize` from `layer1`. If `usize` is 64 bits, bit 0 will be set if any
    /// `Index`es 0-4095 are set, bit 1 will be set if any `Index`es 4096-8191 are set, etc.
    ///
    /// The slice's length is not guaranteed, except that it will be at least the length needed to
    /// reflect all the `1`s in the bitset.
    ///
    /// # Example
    ///
    /// ```
    /// use hibitset::BitSet;
    ///
    /// let index: u32 = 12345;
    ///
    /// let mut bitset = BitSet::new();
    /// bitset.add(index);
    ///
    /// // layer 2 summarizes multiple indexes per bit, so divide appropriately
    /// let slice = bitset.layer2_as_slice();
    /// let bit_index = index as usize / BitSet::LAYER2_GRANULARITY;
    ///
    /// // map that bit index to a usize in the slice and a bit within that usize
    /// let slice_index = bit_index / BitSet::BITS_PER_USIZE;
    /// let bit_at_index = bit_index % BitSet::BITS_PER_USIZE;
    ///
    /// assert_eq!(slice[slice_index], 1 << bit_at_index);
    /// ```
    pub fn layer2_as_slice(&self) -> &[T] {
        self.layer2.as_slice()
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
pub trait BitSetLike {
    /// Type of the underlying bit storage
    type Underlying: UnsignedInteger;

    /// Gets the `usize` corresponding to layer and index.
    ///
    /// The `layer` should be in the range [0, 3]
    fn get_from_layer(&self, layer: usize, idx: usize) -> Self::Underlying {
        match layer {
            0 => self.layer0(idx),
            1 => self.layer1(idx),
            2 => self.layer2(idx),
            3 => self.layer3(),
            _ => panic!("Invalid layer: {}", layer),
        }
    }

    /// Returns true if this `BitSetLike` contains nothing, and false otherwise.
    fn is_empty(&self) -> bool {
        self.layer3() == Self::Underlying::ZERO
    }

    /// Return a `usize` where each bit represents if any word in layer2
    /// has been set.
    fn layer3(&self) -> Self::Underlying;

    /// Return the `usize` from the array of usizes that indicates if any
    /// bit has been set in layer1
    fn layer2(&self, i: usize) -> Self::Underlying;

    /// Return the `usize` from the array of usizes that indicates if any
    /// bit has been set in layer0
    fn layer1(&self, i: usize) -> Self::Underlying;

    /// Return a `usize` that maps to the direct 1:1 association with
    /// each index of the set
    fn layer0(&self, i: usize) -> Self::Underlying;

    /// Allows checking if set bit is contained in the bit set.
    fn contains(&self, i: Index) -> bool;

    /// Create an iterator that will scan over the keyspace
    fn iter(self) -> BitIter<Self>
    where
        Self: Sized,
    {
        let layer3 = self.layer3();

        BitIter::new(
            self,
            [
                Self::Underlying::ZERO,
                Self::Underlying::ZERO,
                Self::Underlying::ZERO,
                layer3,
            ],
            [0; LAYERS - 1],
        )
    }

    /// Create a parallel iterator that will scan over the keyspace
    #[cfg(feature = "parallel")]
    fn par_iter(self) -> BitParIter<Self>
    where
        Self: Sized,
    {
        BitParIter::new(self)
    }
}

/// A extension to the [`BitSetLike`] trait which allows draining it.
pub trait DrainableBitSet: BitSetLike {
    /// Removes bit from the bit set.
    ///
    /// Returns `true` if removal happened and `false` otherwise.
    fn remove(&mut self, i: Index) -> bool;

    /// Create a draining iterator that will scan over the keyspace and clears it while doing so.
    fn drain<'a>(&'a mut self) -> DrainBitIter<'a, Self>
    where
        Self: Sized,
    {
        let layer3 = self.layer3();

        DrainBitIter::new(
            self,
            [
                Self::Underlying::ZERO,
                Self::Underlying::ZERO,
                Self::Underlying::ZERO,
                layer3,
            ],
            [0; LAYERS - 1],
        )
    }
}

impl<'a, T> BitSetLike for &'a T
where
    T: BitSetLike + ?Sized,
{
    type Underlying = T::Underlying;

    #[inline]
    fn layer3(&self) -> T::Underlying {
        (*self).layer3()
    }

    #[inline]
    fn layer2(&self, i: usize) -> T::Underlying {
        (*self).layer2(i)
    }

    #[inline]
    fn layer1(&self, i: usize) -> T::Underlying {
        (*self).layer1(i)
    }

    #[inline]
    fn layer0(&self, i: usize) -> T::Underlying {
        (*self).layer0(i)
    }

    #[inline]
    fn contains(&self, i: Index) -> bool {
        (*self).contains(i)
    }
}

impl<'a, T> BitSetLike for &'a mut T
where
    T: BitSetLike + ?Sized,
{
    type Underlying = T::Underlying;

    #[inline]
    fn layer3(&self) -> T::Underlying {
        (**self).layer3()
    }

    #[inline]
    fn layer2(&self, i: usize) -> T::Underlying {
        (**self).layer2(i)
    }

    #[inline]
    fn layer1(&self, i: usize) -> T::Underlying {
        (**self).layer1(i)
    }

    #[inline]
    fn layer0(&self, i: usize) -> T::Underlying {
        (**self).layer0(i)
    }

    #[inline]
    fn contains(&self, i: Index) -> bool {
        (**self).contains(i)
    }
}

impl<'a, T> DrainableBitSet for &'a mut T
where
    T: DrainableBitSet,
{
    #[inline]
    fn remove(&mut self, i: Index) -> bool {
        (**self).remove(i)
    }
}

impl<T: UnsignedInteger> BitSetLike for GenericBitSet<T> {
    type Underlying = T;

    #[inline]
    fn layer3(&self) -> T {
        self.layer3
    }

    #[inline]
    fn layer2(&self, i: usize) -> T {
        self.layer2.get(i).map(|&x| x).unwrap_or(T::ZERO)
    }

    #[inline]
    fn layer1(&self, i: usize) -> T {
        self.layer1.get(i).map(|&x| x).unwrap_or(T::ZERO)
    }

    #[inline]
    fn layer0(&self, i: usize) -> T {
        self.layer0.get(i).map(|&x| x).unwrap_or(T::ZERO)
    }

    #[inline]
    fn contains(&self, i: Index) -> bool {
        self.contains(i)
    }
}

impl<T: UnsignedInteger> DrainableBitSet for GenericBitSet<T> {
    #[inline]
    fn remove(&mut self, i: Index) -> bool {
        self.remove(i)
    }
}

impl<T: UnsignedInteger> PartialEq for GenericBitSet<T> {
    #[inline]
    fn eq(&self, rhv: &GenericBitSet<T>) -> bool {
        if self.layer3 != rhv.layer3 {
            return false;
        }
        if self.layer2.len() != rhv.layer2.len()
            || self.layer1.len() != rhv.layer1.len()
            || self.layer0.len() != rhv.layer0.len()
        {
            return false;
        }

        for i in 0..self.layer2.len() {
            if self.layer2(i) != rhv.layer2(i) {
                return false;
            }
        }
        for i in 0..self.layer1.len() {
            if self.layer1(i) != rhv.layer1(i) {
                return false;
            }
        }
        for i in 0..self.layer0.len() {
            if self.layer0(i) != rhv.layer0(i) {
                return false;
            }
        }

        true
    }
}
impl<T: UnsignedInteger> Eq for GenericBitSet<T> {}

#[cfg(test)]
mod tests {
    extern crate typed_test_gen;
    use self::typed_test_gen::test_with;

    use super::{BitSetAnd, BitSetLike, BitSetNot, GenericBitSet, UnsignedInteger};

    #[test_with(u32, u64, usize)]
    fn insert<T: UnsignedInteger>() {
        let mut c = GenericBitSet::<T>::new();
        for i in 0..1_000 {
            assert!(!c.add(i));
            assert!(c.add(i));
        }

        for i in 0..1_000 {
            assert!(c.contains(i));
        }
    }

    #[test_with(u32, u64, usize)]
    fn insert_100k<T: UnsignedInteger>() {
        let mut c = GenericBitSet::<T>::new();
        for i in 0..100_000 {
            assert!(!c.add(i));
            assert!(c.add(i));
        }

        for i in 0..100_000 {
            assert!(c.contains(i));
        }
    }

    #[test_with(u32, u64, usize)]
    fn remove<T: UnsignedInteger>() {
        let mut c = GenericBitSet::<T>::new();
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

    #[test_with(u32, u64, usize)]
    fn iter<T: UnsignedInteger>() {
        let mut c = GenericBitSet::<T>::new();
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

    #[test_with(u32, u64, usize)]
    fn iter_odd_even<T: UnsignedInteger>() {
        let mut odd = GenericBitSet::<T>::new();
        let mut even = GenericBitSet::<T>::new();
        for i in 0..100_000 {
            if i % 2 == 1 {
                odd.add(i);
            } else {
                even.add(i);
            }
        }

        assert_eq!((&odd).iter().count(), 50_000);
        assert_eq!((&even).iter().count(), 50_000);
        assert_eq!(BitSetAnd(&odd, &even).iter().count(), 0);
    }

    #[test_with(u32, u64, usize)]
    fn iter_random_add<T: UnsignedInteger>() {
        use rand::prelude::*;

        let mut set = GenericBitSet::<T>::new();
        let mut rng = thread_rng();
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

    #[test_with(u32, u64, usize)]
    fn iter_clusters<T: UnsignedInteger>() {
        let mut set = GenericBitSet::<T>::new();
        for x in 0..8 {
            let x = (x * 3) << (T::LOG_BITS * 2); // scale to the last slot
            for y in 0..8 {
                let y = (y * 3) << (T::LOG_BITS);
                for z in 0..8 {
                    let z = z * 2;
                    set.add(x + y + z);
                }
            }
        }
        assert_eq!(set.iter().count(), 8usize.pow(3));
    }

    #[test_with(u32, u64, usize)]
    fn not<T: UnsignedInteger>() {
        let mut c = GenericBitSet::<T>::new();
        for i in 0..10_000 {
            if i % 2 == 1 {
                c.add(i);
            }
        }
        let d = BitSetNot(c);
        for (idx, i) in d.iter().take(5_000).enumerate() {
            assert_eq!(idx * 2, i as usize);
        }
    }
}

#[cfg(all(test, feature = "parallel"))]
mod test_parallel {
    extern crate typed_test_gen;
    use self::typed_test_gen::test_with;

    use super::{BitSetAnd, BitSetLike, GenericBitSet, UnsignedInteger};
    use rayon::iter::ParallelIterator;

    #[test_with(u32, u64, usize)]
    fn par_iter_one<T: UnsignedInteger + Send + Sync>() {
        let step = 5000;
        let tests = 1_048_576 / step;
        for n in 0..tests {
            let n = n * step;
            let mut set = GenericBitSet::<T>::new();
            set.add(n);
            assert_eq!(set.par_iter().count(), 1);
        }
        let mut set = GenericBitSet::<T>::new();
        set.add(1_048_576 - 1);
        assert_eq!(set.par_iter().count(), 1);
    }

    #[test_with(u32, u64, usize)]
    fn par_iter_random_add<T: UnsignedInteger + Send + Sync>() {
        use rand::prelude::*;
        use std::collections::HashSet;
        use std::sync::{Arc, Mutex};

        let mut set = GenericBitSet::<T>::new();
        let mut check_set = HashSet::new();
        let mut rng = thread_rng();
        let limit = 1_048_576;
        for _ in 0..(limit / 10) {
            let index = rng.gen_range(0, limit);
            set.add(index);
            check_set.insert(index);
        }
        let check_set = Arc::new(Mutex::new(check_set));
        let missing_set = Arc::new(Mutex::new(HashSet::new()));
        set.par_iter().for_each(|n| {
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
            panic!(
                "There were values that didn't get iterated: {:?}
            There were values that got iterated, but that shouldn't be: {:?}",
                *check_set, *missing_set
            );
        }
        if !check_set.is_empty() {
            panic!(
                "There were values that didn't get iterated: {:?}",
                *check_set
            );
        }
        if !missing_set.is_empty() {
            panic!(
                "There were values that got iterated, but that shouldn't be: {:?}",
                *missing_set
            );
        }
    }

    #[test_with(u32, u64, usize)]
    fn par_iter_odd_even<T: UnsignedInteger + Send + Sync>() {
        let mut odd = GenericBitSet::<T>::new();
        let mut even = GenericBitSet::<T>::new();
        for i in 0..100_000 {
            if i % 2 == 1 {
                odd.add(i);
            } else {
                even.add(i);
            }
        }

        assert_eq!((&odd).par_iter().count(), 50_000);
        assert_eq!((&even).par_iter().count(), 50_000);
        assert_eq!(BitSetAnd(&odd, &even).par_iter().count(), 0);
    }

    #[test_with(u32, u64, usize)]
    fn par_iter_clusters<T: UnsignedInteger + Send + Sync>() {
        use std::collections::HashSet;
        use std::sync::{Arc, Mutex};
        let mut set = GenericBitSet::<T>::new();
        let mut check_set = HashSet::new();
        for x in 0..8 {
            let x = (x * 3) << (T::LOG_BITS * 2); // scale to the last slot
            for y in 0..8 {
                let y = (y * 3) << (T::LOG_BITS);
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
        set.par_iter().for_each(|n| {
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
            panic!(
                "There were values that didn't get iterated: {:?}
            There were values that got iterated, but that shouldn't be: {:?}",
                *check_set, *missing_set
            );
        }
        if !check_set.is_empty() {
            panic!(
                "There were values that didn't get iterated: {:?}",
                *check_set
            );
        }
        if !missing_set.is_empty() {
            panic!(
                "There were values that got iterated, but that shouldn't be: {:?}",
                *missing_set
            );
        }
    }
}
