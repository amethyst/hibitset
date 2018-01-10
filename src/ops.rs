
use generic_array::{GenericArray, ArrayLength};

use typenum::{Add1, B1};

use std::ops::{Add, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};
use std::iter::{FromIterator, IntoIterator};
use std::marker::PhantomData as Phantom;

use util::*;

use {AtomicBitSet, BitIter, BitSet, BitSetLike};

impl<'a, B, N> BitOrAssign<&'a B> for BitSet<N>
    where B: BitSetLike<N>,
          N: Add<B1>,
          Add1<N>: ArrayLength<usize>,
          N: ArrayLength<u32>,
          N: ArrayLength<Vec<usize>>
{
    fn bitor_assign(&mut self, lhs: &B) {
        use iter::State::Continue;
        let mut iter = lhs.iter();
        while let Some(level) = (1..LAYERS).find(|&level| iter.handle_level(level) == Continue) {
            let lower = level - 1;
            let idx = iter.prefix[lower] as usize >> BITS;
            *self.layer_mut(lower, idx) |= lhs.get_from_layer(lower, idx);
        }
        self.top_layer |= lhs.top_layer();
    }
}

impl<'a, B, N> BitAndAssign<&'a B> for BitSet<N>
    where B: BitSetLike<N>,
          N: Add<B1>,
          Add1<N>: ArrayLength<usize>,
          N: ArrayLength<u32>,
          N: ArrayLength<Vec<usize>>
{
    fn bitand_assign(&mut self, lhs: &B) {
        use iter::State::*;
        let mut iter = lhs.iter();
        iter.masks[LAYERS - 1] &= self.top_layer();
        while let Some(level) = (1..LAYERS).find(|&level| iter.handle_level(level) == Continue) {
            let lower = level - 1;
            let idx = iter.prefix[lower] as usize >> BITS;
            let our_layer = self.get_from_layer(lower, idx);
            let their_layer = lhs.get_from_layer(lower, idx);

            iter.masks[lower] &= our_layer;

            let mut masks = GenericArray::default();
            masks[lower] = our_layer & !their_layer;
            BitIter::new(&mut *self, masks, iter.prefix.clone()).clear();

            *self.layer_mut(lower, idx) &= their_layer;
        }
        let mut masks = GenericArray::default();
        masks[LAYERS - 1] =  self.top_layer() & !lhs.top_layer();
        BitIter::new(&mut *self, masks, GenericArray::default()).clear();

        self.top_layer &= lhs.top_layer();
    }
}

impl<'a, B, N> BitXorAssign<&'a B> for BitSet<N>
    where B: BitSetLike<N>,
          N: Add<B1>,
          Add1<N>: ArrayLength<usize>,
          N: ArrayLength<u32>,
          N: ArrayLength<Vec<usize>>
{
    fn bitxor_assign(&mut self, lhs: &B) {
        use iter::State::*;
        let mut iter = lhs.iter();
        while let Some(level) = (1..LAYERS).find(|&level| iter.handle_level(level) == Continue) {
            let lower = level - 1;
            let idx = iter.prefix[lower] as usize >> BITS;

            if lower == 0 {
                *self.layer_mut(lower, idx) ^= lhs.get_from_layer(lower, idx);

                let mut change_bit = |level| {
                    let lower = level - 1;
                    let h = iter.prefix.get(level).cloned().unwrap_or(0) as usize;
                    let l = iter.prefix[lower] as usize >> BITS;
                    let mask = 1 << (l & !h);

                    if self.get_from_layer(lower, l) == 0 {
                        *self.layer_mut(level, h >> BITS) &= !mask;
                    } else {
                        *self.layer_mut(level, h >> BITS) |= mask;
                    }
                };

                change_bit(level);
                if iter.masks[level] == 0 {
                    (2..LAYERS).for_each(change_bit);
                }
            }
        }
    }
}

/// `BitSetAnd` takes two [`BitSetLike`] items, and merges the masks
/// returning a new virtual set, which represents an intersection of the
/// two original sets.
///
/// [`BitSetLike`]: ../trait.BitSetLike.html
#[derive(Debug)]
pub struct BitSetAnd<A: BitSetLike<N>, B: BitSetLike<N>, N>(pub A, pub B, pub Phantom<N>)
    where N: Add<B1>,
          Add1<N>: ArrayLength<usize>,
          N: ArrayLength<u32>,
          N: ArrayLength<Vec<usize>>;


impl<A: BitSetLike<N>, B: BitSetLike<N>, N> BitSetAnd<A, B, N>
    where N: Add<B1>,
          Add1<N>: ArrayLength<usize>,
          N: ArrayLength<u32>,
          N: ArrayLength<Vec<usize>>
{
    /// Constructs lazy *and* operation between `BitSetLike`s
    pub fn new(a: A, b: B) -> Self {
        BitSetAnd(a, b, Phantom)
    }
}

impl<A: BitSetLike<N>, B: BitSetLike<N>, N> BitSetLike<N> for BitSetAnd<A, B, N>
    where N: Add<B1>,
          Add1<N>: ArrayLength<usize>,
          N: ArrayLength<u32>,
          N: ArrayLength<Vec<usize>>
{
    #[inline]
    fn get_from_layer(&self, layer: usize, idx: usize) -> usize {
        self.0.get_from_layer(layer, idx) & self.1.get_from_layer(layer, idx)
    }

    #[inline]
    fn top_layer(&self) -> usize {
        self.0.top_layer() & self.1.top_layer()
    }

    #[inline]
    fn contains(&self, i: Index) -> bool {
        self.0.contains(i) && self.1.contains(i)
    }
}

/// `BitSetOr` takes two [`BitSetLike`] items, and merges the masks
/// returning a new virtual set, which represents an merged of the
/// two original sets.
///
/// [`BitSetLike`]: ../trait.BitSetLike.html
#[derive(Debug)]
pub struct BitSetOr<A: BitSetLike<N>, B: BitSetLike<N>, N>(pub A, pub B, pub Phantom<N>)
    where N: Add<B1>,
          Add1<N>: ArrayLength<usize>,
          N: ArrayLength<u32>,
          N: ArrayLength<Vec<usize>>;

impl<A: BitSetLike<N>, B: BitSetLike<N>, N> BitSetOr<A, B, N>
    where N: Add<B1>,
          Add1<N>: ArrayLength<usize>,
          N: ArrayLength<u32>,
          N: ArrayLength<Vec<usize>>
{
    /// Constructs lazy *or* operation between `BitSetLike`s
    pub fn new(a: A, b: B) -> Self {
        BitSetOr(a, b, Phantom)
    }
}

impl<A: BitSetLike<N>, B: BitSetLike<N>, N> BitSetLike<N> for BitSetOr<A, B, N>
    where N: Add<B1>,
          Add1<N>: ArrayLength<usize>,
          N: ArrayLength<u32>,
          N: ArrayLength<Vec<usize>>
{
    #[inline]
    fn get_from_layer(&self, layer: usize, idx: usize) -> usize {
        self.0.get_from_layer(layer, idx) | self.1.get_from_layer(layer, idx)
    }

    #[inline]
    fn top_layer(&self) -> usize {
        self.0.top_layer() | self.1.top_layer()
    }

    #[inline]
    fn contains(&self, i: Index) -> bool {
        self.0.contains(i) || self.1.contains(i)
    }
}

/// `BitSetNot` takes a [`BitSetLike`] item, and produced an inverted virtual set.
/// Note: the implementation is sub-optimal because layers 1-3 are not active.
///
/// [`BitSetLike`]: ../trait.BitSetLike.html
#[derive(Debug)]
pub struct BitSetNot<A: BitSetLike<N>, N>(pub A, pub Phantom<N>)
    where N: Add<B1>,
          Add1<N>: ArrayLength<usize>,
          N: ArrayLength<u32>,
          N: ArrayLength<Vec<usize>>;

impl<A: BitSetLike<N>, N> BitSetNot<A, N>
    where N: Add<B1>,
          Add1<N>: ArrayLength<usize>,
          N: ArrayLength<u32>,
          N: ArrayLength<Vec<usize>>
{
    /// Constructs lazy *not* operation for `BitSetLike`
    pub fn new(a: A) -> Self {
        BitSetNot(a, Phantom)
    }
}

impl<A: BitSetLike<N>, N> BitSetLike<N> for BitSetNot<A, N>
    where N: Add<B1>,
          Add1<N>: ArrayLength<usize>,
          N: ArrayLength<u32>,
          N: ArrayLength<Vec<usize>>
{
    #[inline]
    fn get_from_layer(&self, layer: usize, idx: usize) -> usize {
        if layer > 0 {
            !0
        } else {
            !self.0.get_from_layer(0, idx)
        }
    }

    #[inline]
    fn top_layer(&self) -> usize {
        !0
    }

    #[inline]
    fn contains(&self, i: Index) -> bool {
        !self.0.contains(i)
    }
}

/// `BitSetXor` takes two [`BitSetLike`] items, and merges the masks
/// returning a new virtual set, which represents an merged of the
/// two original sets.
///
/// [`BitSetLike`]: ../trait.BitSetLike.html
#[derive(Debug)]
pub struct BitSetXor<A: BitSetLike<N>, B: BitSetLike<N>, N>(pub A, pub B, pub Phantom<N>)
    where N: Add<B1>,
          Add1<N>: ArrayLength<usize>,
          N: ArrayLength<u32>,
          N: ArrayLength<Vec<usize>>;

impl<A: BitSetLike<N>, B: BitSetLike<N>, N> BitSetXor<A, B, N>
    where N: Add<B1>,
          Add1<N>: ArrayLength<usize>,
          N: ArrayLength<u32>,
          N: ArrayLength<Vec<usize>>
{
    /// Constructs lazy *xor* operation between `BitSetLike`s
    pub fn new(a: A, b: B) -> Self {
        BitSetXor(a, b, Phantom)
    }
}

impl<A: BitSetLike<N>, B: BitSetLike<N>, N> BitSetLike<N> for BitSetXor<A, B, N>
    where N: Add<B1>,
          Add1<N>: ArrayLength<usize>,
          N: ArrayLength<u32>,
          N: ArrayLength<Vec<usize>>
{

    #[inline]
    fn get_from_layer(&self, layer: usize, idx: usize) -> usize {
        use BitSetAnd as And;
        use BitSetOr as Or;
        use BitSetNot as Not;
        let xor = And::new(Or::new(&self.0, &self.1), Not::new(And::new(&self.0, &self.1)));
        xor.get_from_layer(layer, idx)
    }

    #[inline]
    fn top_layer(&self) -> usize {
        use BitSetAnd as And;
        use BitSetOr as Or;
        use BitSetNot as Not;
        let xor = And::new(Or::new(&self.0, &self.1), Not::new(And::new(&self.0, &self.1)));
        xor.top_layer()
    }

    #[inline]
    fn contains(&self, i: Index) -> bool {
        use BitSetAnd as And;
        use BitSetOr as Or;
        use BitSetNot as Not;
        And::new(Or::new(&self.0, &self.1), Not::new(And::new(&self.0, &self.1))).contains(i)
    }

}

macro_rules! operator {
    ( impl < ( $( $lifetime:tt )* ) ( $( $arg:ident ),* ) > for $($bitset:tt)+ ) => {
        impl<$( $lifetime, )* $( $arg, )* N> IntoIterator for $($bitset)+<$( $arg, )* N>
            where $( $arg: BitSetLike<N>, )*
                  N: ::std::ops::Add<::typenum::B1>,
                  ::typenum::Add1<N>: ::generic_array::ArrayLength<usize>,
                  N: ::generic_array::ArrayLength<u32>,
                  N: ::generic_array::ArrayLength<Vec<usize>>,
        {
            type Item = <BitIter<Self, N> as Iterator>::Item;
            type IntoIter = BitIter<Self, N>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }

        impl<$( $lifetime, )* $( $arg, )* N> Not for $($bitset)+<$( $arg, )* N>
            where $( $arg: BitSetLike<N>, )*
                  N: ::std::ops::Add<::typenum::B1>,
                  ::typenum::Add1<N>: ::generic_array::ArrayLength<usize>,
                  N: ::generic_array::ArrayLength<u32>,
                  N: ::generic_array::ArrayLength<Vec<usize>>,
        {
            type Output = BitSetNot<Self, N>;
            fn not(self) -> Self::Output {
                BitSetNot(self, Phantom)
            }
        }

        impl<$( $lifetime, )* $( $arg, )* T, N> BitAnd<T> for $($bitset)+<$( $arg, )* N>
            where T: BitSetLike<N>,
                  $( $arg: BitSetLike<N>, )*
                  N: ::std::ops::Add<::typenum::B1>,
                  ::typenum::Add1<N>: ::generic_array::ArrayLength<usize>,
                  N: ::generic_array::ArrayLength<u32>,
                  N: ::generic_array::ArrayLength<Vec<usize>>,
        {
            type Output = BitSetAnd<Self, T, N>;
            fn bitand(self, rhs: T) -> Self::Output {
                BitSetAnd(self, rhs, Phantom)
            }
        }

        impl<$( $lifetime, )* $( $arg, )* T, N> BitOr<T> for $($bitset)+<$( $arg, )* N>
            where T: BitSetLike<N>,
                  $( $arg: BitSetLike<N>, )*
                  N: ::std::ops::Add<::typenum::B1>,
                  ::typenum::Add1<N>: ::generic_array::ArrayLength<usize>,
                  N: ::generic_array::ArrayLength<u32>,
                  N: ::generic_array::ArrayLength<Vec<usize>>,
        {
            type Output = BitSetOr<Self, T, N>;
            fn bitor(self, rhs: T) -> Self::Output {
                BitSetOr(self, rhs, Phantom)
            }
        }

        impl<$( $lifetime, )* $( $arg, )* T, N> BitXor<T> for $($bitset)+<$( $arg, )* N>
            where T: BitSetLike<N>,
                  $( $arg: BitSetLike<N>, )*
                  N: ::std::ops::Add<::typenum::B1>,
                  ::typenum::Add1<N>: ::generic_array::ArrayLength<usize>,
                  N: ::generic_array::ArrayLength<u32>,
                  N: ::generic_array::ArrayLength<Vec<usize>>,
        {
            type Output = BitSetXor<Self, T, N>;
            fn bitxor(self, rhs: T) -> Self::Output {
                BitSetXor(self, rhs, Phantom)
            }
        }

    }
}

operator!(impl<()()> for BitSet);
operator!(impl<('a)()> for &'a BitSet);
//operator!(impl<()()> for AtomicBitSet);
//operator!(impl<('a)()> for &'a AtomicBitSet);
operator!(impl<()(A)> for BitSetNot);
operator!(impl<('a)(A)> for &'a BitSetNot);
operator!(impl<()(A, B)> for BitSetAnd);
operator!(impl<('a)(A, B)> for &'a BitSetAnd);
operator!(impl<()(A, B)> for BitSetOr);
operator!(impl<('a)(A, B)> for &'a BitSetOr);
operator!(impl<()(A, B)> for BitSetXor);
operator!(impl<('a)(A, B)> for &'a BitSetXor);

macro_rules! iterator {
    ( $($bitset:tt)+ ) => {
        impl<N> FromIterator<Index> for $($bitset)+<N>
        where N: ::std::ops::Add<::typenum::B1>,
                ::typenum::Add1<N>: ::generic_array::ArrayLength<usize>,
                N: ::generic_array::ArrayLength<u32>,
                N: ::generic_array::ArrayLength<Vec<usize>>,
        {
            fn from_iter<T>(iter: T) -> Self
            where
                T: IntoIterator<Item = Index>,
            {
                let mut bitset = $($bitset)+::new();
                for item in iter {
                    bitset.add(item);
                }
                bitset
            }
        }
        
        impl<'a, N> FromIterator<&'a Index> for $($bitset)+<N>
        where N: ::std::ops::Add<::typenum::B1>,
                ::typenum::Add1<N>: ::generic_array::ArrayLength<usize>,
                N: ::generic_array::ArrayLength<u32>,
                N: ::generic_array::ArrayLength<Vec<usize>>,
        {
            fn from_iter<T>(iter: T) -> Self
            where
                T: IntoIterator<Item = &'a Index>,
            {
                let mut bitset = $($bitset)+::new();
                for item in iter {
                    bitset.add(*item);
                }
                bitset
            }
        }

        impl<N> Extend<Index> for $($bitset)+<N>
            where N: ::std::ops::Add<::typenum::B1>,
                  ::typenum::Add1<N>: ::generic_array::ArrayLength<usize>,
                  N: ::generic_array::ArrayLength<u32>,
                  N: ::generic_array::ArrayLength<Vec<usize>>,
        {
            fn extend<T>(&mut self, iter: T)
            where
                T: IntoIterator<Item = Index>,
            {
                for item in iter {
                    self.add(item);
                }
            }
        }

        impl<'a, N> Extend<&'a Index> for $($bitset)+<N>
        where N: ::std::ops::Add<::typenum::B1>,
                ::typenum::Add1<N>: ::generic_array::ArrayLength<usize>,
                N: ::generic_array::ArrayLength<u32>,
                N: ::generic_array::ArrayLength<Vec<usize>>,
        {
            fn extend<T>(&mut self, iter: T)
            where
                T: IntoIterator<Item = &'a Index>,
            {
                for item in iter {
                    self.add(*item);
                }
            }
        }
    };

}

iterator!(BitSet);
//iterator!(AtomicBitSet);

#[cfg(test)]
mod tests {
    use {Index, BitSet, BitSetLike, BitSetXor};

    #[test]
    fn or_assign() {
        use std::mem::size_of;
        use std::collections::HashSet;

        let usize_bits = size_of::<usize>() as u32 * 8;
        let n = 10_000;
        let f1 = &|n| 7 * usize_bits * n;
        let f2 = &|n| 13 * usize_bits * n;

        let mut c1: BitSet<::DefaultLayers> = (0..n).map(f1).collect();
        let c2: BitSet<::DefaultLayers> = (0..n).map(f2).collect();

        c1 |= &c2;

        let h1: HashSet<_> = (0..n).map(f1).collect();
        let h2: HashSet<_> = (0..n).map(f2).collect();
        assert_eq!(
            c1.iter().collect::<HashSet<_>>(),
            &h1 | &h2
        );
    }

    #[test]
    fn or_assign_random() {
        use rand::{Rng, weak_rng};
        use std::collections::HashSet;
        let limit = 1_048_576;
        let mut rng = weak_rng();

        let mut set1 = BitSet::<::DefaultLayers>::new();
        let mut check_set1 = HashSet::new();
        for _ in 0..(limit / 100) {
            let index = rng.gen_range(0, limit);
            set1.add(index);
            check_set1.insert(index);
        }
        
        let mut set2 = BitSet::<::DefaultLayers>::new();
        let mut check_set2 = HashSet::new();
        for _ in 0..(limit / 100) {
            let index = rng.gen_range(0, limit);
            set2.add(index);
            check_set2.insert(index);
        }

        let hs1 = (&set1).iter().collect::<HashSet<_>>();
        let hs2 = (&set2).iter().collect::<HashSet<_>>();
        let mut hs = (&hs1 | &hs2).iter().cloned().collect::<HashSet<_>>();

        set1 |= &set2;

        for _ in 0..(limit / 1000) {
            let index = rng.gen_range(0, limit);
            set1.add(index);
            hs.insert(index);
        }

        assert_eq!(hs, set1.iter().collect());
    }

    #[test]
    fn and_assign() {
        use std::mem::size_of;
        use std::collections::HashSet;

        let usize_bits = size_of::<usize>() as u32 * 8;
        let n = 10_000;
        let f1 = &|n| 7 * usize_bits * n;
        let f2 = &|n| 13 * usize_bits * n;

        let mut c1: BitSet<::DefaultLayers> = (0..n).map(f1).collect();
        let c2: BitSet<::DefaultLayers> = (0..n).map(f2).collect();

        c1 &= &c2;

        let h1: HashSet<_> = (0..n).map(f1).collect();
        let h2: HashSet<_> = (0..n).map(f2).collect();
        assert_eq!(
            c1.iter().collect::<HashSet<_>>(),
            &h1 & &h2
        );
    }

    #[test]
    fn and_assign_specific() {
        use util::BITS;

        let mut c1 = BitSet::default();
        c1.add(0);
        let common = ((1 << BITS) << BITS) << BITS;
        c1.add(common);
        c1.add((((1 << BITS) << BITS) + 1) << BITS);

        let mut c2 = BitSet::default();
        c2.add(common);
        c2.add((((1 << BITS) << BITS) + 2) << BITS);

        c1 &= &c2;

        assert_eq!(c1.iter().collect::<Vec<_>>(), [common]);
    }

    #[test]
    fn and_assign_with_modification() {
        use util::BITS;

        let mut c1 = BitSet::default();
        c1.add(0);
        c1.add((1 << BITS) << BITS);

        let mut c2 = BitSet::default();
        c2.add(0);

        c1 &= &c2;

        let added = ((1 << BITS) + 1) << BITS;
        c1.add(added);

        assert_eq!(c1.iter().collect::<Vec<_>>(), [0, added]);
    }

    #[test]
    fn and_assign_random() {
        use rand::{Rng, weak_rng};
        use std::collections::HashSet;
        let limit = 1_048_576;
        let mut rng = weak_rng();

        let mut set1 = BitSet::<::DefaultLayers>::new();
        let mut check_set1 = HashSet::new();
        for _ in 0..(limit / 100) {
            let index = rng.gen_range(0, limit);
            set1.add(index);
            check_set1.insert(index);
        }
        
        let mut set2 = BitSet::<::DefaultLayers>::new();
        let mut check_set2 = HashSet::new();
        for _ in 0..(limit / 100) {
            let index = rng.gen_range(0, limit);
            set2.add(index);
            check_set2.insert(index);
        }

        let hs1 = (&set1).iter().collect::<HashSet<_>>();
        let hs2 = (&set2).iter().collect::<HashSet<_>>();
        let mut hs = (&hs1 & &hs2).iter().cloned().collect::<HashSet<_>>();

        set1 &= &set2;

        for _ in 0..(limit / 1000) {
            let index = rng.gen_range(0, limit);
            set1.add(index);
            hs.insert(index);
        }

        assert_eq!(hs, set1.iter().collect());
    }

    #[test]
    fn xor_assign() {
        use std::mem::size_of;
        use std::collections::HashSet;

        let usize_bits = size_of::<usize>() as u32 * 8;
        let n = 10_000;
        let f1 = &|n| 7 * usize_bits * n;
        let f2 = &|n| 13 * usize_bits * n;

        let mut c1: BitSet<::DefaultLayers> = (0..n).map(f1).collect();
        let c2: BitSet<::DefaultLayers> = (0..n).map(f2).collect();
        c1 ^= &c2;

        let h1: HashSet<_> = (0..n).map(f1).collect();
        let h2: HashSet<_> = (0..n).map(f2).collect();
        assert_eq!(
            c1.iter().collect::<HashSet<_>>(),
            &h1 ^ &h2
        );
    }

    #[test]
    fn xor_assign_specific() {
        use util::BITS;

        let mut c1 = BitSet::default();
        c1.add(0);
        let common = ((1 << BITS) << BITS) << BITS;
        c1.add(common);
        let a = (((1 << BITS) + 1) << BITS) << BITS;
        c1.add(a);

        let mut c2 = BitSet::default();
        c2.add(common);
        let b = (((1 << BITS) + 2) << BITS) << BITS;
        c2.add(b);

        c1 ^= &c2;

        assert_eq!(c1.iter().collect::<Vec<_>>(), [0, a, b]);
    }

    #[test]
    fn xor_assign_random() {
        use rand::{Rng, weak_rng};
        use std::collections::HashSet;
        let limit = 1_048_576;
        let mut rng = weak_rng();

        let mut set1 = BitSet::<::DefaultLayers>::new();
        let mut check_set1 = HashSet::new();
        for _ in 0..(limit / 100) {
            let index = rng.gen_range(0, limit);
            set1.add(index);
            check_set1.insert(index);
        }
        
        let mut set2 = BitSet::<::DefaultLayers>::new();
        let mut check_set2 = HashSet::new();
        for _ in 0..(limit / 100) {
            let index = rng.gen_range(0, limit);
            set2.add(index);
            check_set2.insert(index);
        }

        let hs1 = (&set1).iter().collect::<HashSet<_>>();
        let hs2 = (&set2).iter().collect::<HashSet<_>>();
        let mut hs = (&hs1 ^ &hs2).iter().cloned().collect::<HashSet<_>>();

        set1 ^= &set2;

        for _ in 0..(limit / 1000) {
            let index = rng.gen_range(0, limit);
            set1.add(index);
            hs.insert(index);
        }

        assert_eq!(hs, set1.iter().collect());
    }

    #[test]
    fn operators() {
        let mut bitset = BitSet::default();
        bitset.add(1);
        bitset.add(3);
        bitset.add(5);
        bitset.add(15);
        bitset.add(200);
        bitset.add(50001);

        let mut other = BitSet::new();
        other.add(1);
        other.add(3);
        other.add(50000);
        other.add(50001);

        {
            let not = &bitset & !&bitset;
            assert_eq!(not.iter().count(), 0);
        }

        {
            let either = &bitset | &other;
            let collected = either.iter().collect::<Vec<Index>>();
            assert_eq!(collected, vec![1, 3, 5, 15, 200, 50000, 50001]);

            let either_sanity = bitset.clone() | other.clone();
            assert_eq!(collected, either_sanity.iter().collect::<Vec<Index>>());
        }

        {
            let same = &bitset & &other;
            let collected = same.iter().collect::<Vec<Index>>();
            assert_eq!(collected, vec![1, 3, 50001]);

            let same_sanity = bitset.clone() & other.clone();
            assert_eq!(collected, same_sanity.iter().collect::<Vec<Index>>());
        }

        {
            let exclusive = &bitset ^ &other;
            let collected = exclusive.iter().collect::<Vec<Index>>();
            assert_eq!(collected, vec![5, 15, 200, 50000]);

            let exclusive_sanity = bitset.clone() ^ other.clone();
            assert_eq!(collected, exclusive_sanity.iter().collect::<Vec<Index>>());
        }
    }

    #[test]
    fn xor() {
        // 0011
        let mut bitset = BitSet::default();
        bitset.add(2);
        bitset.add(3);
        bitset.add(50000);

        // 0101
        let mut other = BitSet::default();
        other.add(1);
        other.add(3);
        other.add(50000);
        other.add(50001);

        {
            // 0110
            let xor = BitSetXor::new(&bitset, &other);
            let collected = xor.iter().collect::<Vec<Index>>();
            assert_eq!(collected, vec![1, 2, 50001]);
        }
    }
}
