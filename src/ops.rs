
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};
use std::iter::{FromIterator, IntoIterator};

use util::*;

use {AtomicBitSet, BitIter, BitSet, BitSetLike};

impl<'a, B> BitOrAssign<&'a B> for BitSet
    where B: BitSetLike
{
    fn bitor_assign(&mut self, lhs: &B) {
        use iter::State::*;
        let mut iter = lhs.iter();
        'find: loop {
            for level in 1..LAYERS {
                match iter.handle_level(level) {
                    Value(_) => unreachable!("Lowest level is not iterated directly."),
                    Continue => {
                        let lower = level - 1;
                        let idx = (iter.prefix[lower] >> BITS) as usize;

                        *self.layer_mut(lower, idx) |= lhs.get_from_layer(lower, idx);
                        continue 'find;
                    }
                    Empty => {},
                }
            }
            break;
        }
        self.layer3 |= lhs.layer3();
    }
}

impl<'a, B> BitAndAssign<&'a B> for BitSet
    where B: BitSetLike
{
    fn bitand_assign(&mut self, lhs: &B) {
        use iter::State::*;
        let mut iter = lhs.iter();
        'find: loop {
            for level in 1..LAYERS {
                match iter.handle_level(level) {
                    Value(_) => unreachable!("Lowest level is not iterated directly."),
                    Continue => {
                        let lower = level - 1;
                        let idx = (iter.prefix[lower] >> BITS) as usize;
                        let our_layer = self.get_from_layer(lower, idx);
                        let their_layer = lhs.get_from_layer(lower, idx);

                        let mut mask = [0; 4];
                        mask[lower] = our_layer & !their_layer;
                        BitIter::new(&mut *self, mask, iter.prefix).clear();

                        *self.layer_mut(lower, idx) &= their_layer;
                        continue 'find;
                    }
                    Empty => {},
                }
            }
            break;
        }

        let mask = [0, 0, 0, self.layer3() & !lhs.layer3()];
        BitIter::new(&mut *self, mask, [0; 3]).clear();

        self.layer3 &= lhs.layer3();
    }
}

/// `BitSetAnd` takes two [`BitSetLike`] items, and merges the masks
/// returning a new virtual set, which represents an intersection of the
/// two original sets.
///
/// [`BitSetLike`]: ../trait.BitSetLike.html
#[derive(Debug)]
pub struct BitSetAnd<A: BitSetLike, B: BitSetLike>(pub A, pub B);

impl<A: BitSetLike, B: BitSetLike> BitSetLike for BitSetAnd<A, B> {
    #[inline]
    fn layer3(&self) -> usize {
        self.0.layer3() & self.1.layer3()
    }
    #[inline]
    fn layer2(&self, i: usize) -> usize {
        self.0.layer2(i) & self.1.layer2(i)
    }
    #[inline]
    fn layer1(&self, i: usize) -> usize {
        self.0.layer1(i) & self.1.layer1(i)
    }
    #[inline]
    fn layer0(&self, i: usize) -> usize {
        self.0.layer0(i) & self.1.layer0(i)
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
pub struct BitSetOr<A: BitSetLike, B: BitSetLike>(pub A, pub B);

impl<A: BitSetLike, B: BitSetLike> BitSetLike for BitSetOr<A, B> {
    #[inline]
    fn layer3(&self) -> usize {
        self.0.layer3() | self.1.layer3()
    }
    #[inline]
    fn layer2(&self, i: usize) -> usize {
        self.0.layer2(i) | self.1.layer2(i)
    }
    #[inline]
    fn layer1(&self, i: usize) -> usize {
        self.0.layer1(i) | self.1.layer1(i)
    }
    #[inline]
    fn layer0(&self, i: usize) -> usize {
        self.0.layer0(i) | self.1.layer0(i)
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
pub struct BitSetNot<A: BitSetLike>(pub A);

impl<A: BitSetLike> BitSetLike for BitSetNot<A> {
    #[inline]
    fn layer3(&self) -> usize {
        !0
    }
    #[inline]
    fn layer2(&self, _: usize) -> usize {
        !0
    }
    #[inline]
    fn layer1(&self, _: usize) -> usize {
        !0
    }
    #[inline]
    fn layer0(&self, i: usize) -> usize {
        !self.0.layer0(i)
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
pub struct BitSetXor<A: BitSetLike, B: BitSetLike>(pub A, pub B);

impl<A: BitSetLike, B: BitSetLike> BitSetLike for BitSetXor<A, B> {
    #[inline]
    fn layer3(&self) -> usize {
        let xor = BitSetAnd(BitSetOr(&self.0, &self.1), BitSetNot(BitSetAnd(&self.0, &self.1)));
        xor.layer3()
    }
    #[inline]
    fn layer2(&self, id: usize) -> usize {
        let xor = BitSetAnd(BitSetOr(&self.0, &self.1), BitSetNot(BitSetAnd(&self.0, &self.1)));
        xor.layer2(id)
    }
    #[inline]
    fn layer1(&self, id: usize) -> usize {
        let xor = BitSetAnd(BitSetOr(&self.0, &self.1), BitSetNot(BitSetAnd(&self.0, &self.1)));
        xor.layer1(id)
    }
    #[inline]
    fn layer0(&self, id: usize) -> usize {
        let xor = BitSetAnd(BitSetOr(&self.0, &self.1), BitSetNot(BitSetAnd(&self.0, &self.1)));
        xor.layer0(id)
    }
    #[inline]
    fn contains(&self, i: Index) -> bool {
        BitSetAnd(BitSetOr(&self.0, &self.1), BitSetNot(BitSetAnd(&self.0, &self.1))).contains(i)
    }

}

macro_rules! operator {
    ( impl < ( $( $lifetime:tt )* ) ( $( $arg:ident ),* ) > for $bitset:ty ) => {
        impl<$( $lifetime, )* $( $arg ),*> IntoIterator for $bitset
            where $( $arg: BitSetLike ),*
        {
            type Item = <BitIter<Self> as Iterator>::Item;
            type IntoIter = BitIter<Self>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }

        impl<$( $lifetime, )* $( $arg ),*> Not for $bitset
            where $( $arg: BitSetLike ),*
        {
            type Output = BitSetNot<Self>;
            fn not(self) -> Self::Output {
                BitSetNot(self)
            }
        }

        impl<$( $lifetime, )* $( $arg, )* T> BitAnd<T> for $bitset
            where T: BitSetLike,
                  $( $arg: BitSetLike ),*
        {
            type Output = BitSetAnd<Self, T>;
            fn bitand(self, rhs: T) -> Self::Output {
                BitSetAnd(self, rhs)
            }
        }

        impl<$( $lifetime, )* $( $arg, )* T> BitOr<T> for $bitset
            where T: BitSetLike,
                  $( $arg: BitSetLike ),*
        {
            type Output = BitSetOr<Self, T>;
            fn bitor(self, rhs: T) -> Self::Output {
                BitSetOr(self, rhs)
            }
        }

        impl<$( $lifetime, )* $( $arg, )* T> BitXor<T> for $bitset
            where T: BitSetLike,
                  $( $arg: BitSetLike ),*
        {
            type Output = BitSetXor<Self, T>;
            fn bitxor(self, rhs: T) -> Self::Output {
                BitSetXor(self, rhs)
            }
        }

    }
}

operator!(impl<()()> for BitSet);
operator!(impl<('a)()> for &'a BitSet);
operator!(impl<()()> for AtomicBitSet);
operator!(impl<('a)()> for &'a AtomicBitSet);
operator!(impl<()(A)> for BitSetNot<A>);
operator!(impl<('a)(A)> for &'a BitSetNot<A>);
operator!(impl<()(A, B)> for BitSetAnd<A, B>);
operator!(impl<('a)(A, B)> for &'a BitSetAnd<A, B>);
operator!(impl<()(A, B)> for BitSetOr<A, B>);
operator!(impl<('a)(A, B)> for &'a BitSetOr<A, B>);
operator!(impl<()(A, B)> for BitSetXor<A, B>);
operator!(impl<('a)(A, B)> for &'a BitSetXor<A, B>);

macro_rules! iterator {
    ( $bitset:ident ) => {
        impl FromIterator<Index> for $bitset {
            fn from_iter<T>(iter: T) -> Self
            where
                T: IntoIterator<Item = Index>,
            {
                let mut bitset = $bitset::new();
                for item in iter {
                    bitset.add(item);
                }
                bitset
            }
        }
        
        impl<'a> FromIterator<&'a Index> for $bitset {
            fn from_iter<T>(iter: T) -> Self
            where
                T: IntoIterator<Item = &'a Index>,
            {
                let mut bitset = $bitset::new();
                for item in iter {
                    bitset.add(*item);
                }
                bitset
            }
        }

        impl Extend<Index> for $bitset {
            fn extend<T>(&mut self, iter: T)
            where
                T: IntoIterator<Item = Index>,
            {
                for item in iter {
                    self.add(item);
                }
            }
        }

        impl<'a> Extend<&'a Index> for $bitset {
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
iterator!(AtomicBitSet);

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

        let mut c1: BitSet = (0..n).map(f1).collect();
        let c2: BitSet = (0..n).map(f2).collect();
        c1 |= &c2;

        let h1: HashSet<_> = (0..n).map(f1).collect();
        let h2: HashSet<_> = (0..n).map(f2).collect();
        assert_eq!(
            c1.iter().collect::<HashSet<_>>(),
            &h1 | &h2
        );
    }

    #[test]
    fn and_assign() {
        use std::mem::size_of;
        use std::collections::HashSet;

        let usize_bits = size_of::<usize>() as u32 * 8;
        let n = 10_000;
        let f1 = &|n| 7 * usize_bits * n;
        let f2 = &|n| 13 * usize_bits * n;

        let mut c1: BitSet = (0..n).map(f1).collect();
        let c2: BitSet = (0..n).map(f2).collect();
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
        let mut c1 = BitSet::new();
        c1.add(0);
        let common = ((1 << BITS) << BITS) << BITS;
        c1.add(common);
        c1.add((((1 << BITS) << BITS) + 1) << BITS);
        let mut c2: BitSet = BitSet::new();
        c2.add(common);
        c2.add((((1 << BITS) << BITS) + 2) << BITS);
        c1 &= &c2;
        assert_eq!(c1.iter().collect::<Vec<_>>(), [common]);
    }

    #[test]
    fn and_assign_with_modification() {
        use util::BITS;
        let mut c1 = BitSet::new();
        c1.add(0);
        c1.add((1 << BITS) << BITS);
        let mut c2: BitSet = BitSet::new();
        c2.add(0);
        c1 &= &c2;
        let added = ((1 << BITS) + 1) << BITS;
        c1.add(added);
        assert_eq!(c1.iter().collect::<Vec<_>>(), [0, added]);
    }

    #[test]
    fn operators() {
        let mut bitset = BitSet::new();
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
        let mut bitset = BitSet::new();
        bitset.add(2);
        bitset.add(3);
        bitset.add(50000);

        // 0101
        let mut other = BitSet::new();
        other.add(1);
        other.add(3);
        other.add(50000);
        other.add(50001);

        {
            // 0110
            let xor = BitSetXor(&bitset, &other);
            let collected = xor.iter().collect::<Vec<Index>>();
            assert_eq!(collected, vec![1, 2, 50001]);
        }
    }
}
