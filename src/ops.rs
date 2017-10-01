
use std::ops::{BitAnd, BitOr, Not};

use {BitSet, BitSetLike};

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
}

macro_rules! operator {
    ( $bitset:ident ( $( $arg:ident ),* ) ) => {
        impl<$( $arg ),*> Not for $bitset<$( $arg ),*>
            where $( $arg: BitSetLike ),*
        {
            type Output = BitSetNot<Self>;
            fn not(self) -> Self::Output {
                BitSetNot(self)
            }
        }

        impl<$( $arg,  )* T> BitAnd<T> for $bitset<$( $arg ),*>
            where T: BitSetLike,
                  $( $arg: BitSetLike ),*
        {
            type Output = BitSetAnd<Self, T>;
            fn bitand(self, rhs: T) -> Self::Output {
                BitSetAnd(self, rhs)
            }
        }

        impl<$( $arg, )* T> BitOr<T> for $bitset<$( $arg ),*>
            where T: BitSetLike,
                  $( $arg: BitSetLike ),*
        {
            type Output = BitSetOr<Self, T>;
            fn bitor(self, rhs: T) -> Self::Output {
                BitSetOr(self, rhs)
            }
        }

    }
}

operator!(BitSet());
operator!(BitSetAnd(A, B));
operator!(BitSetNot(A));
operator!(BitSetOr(A, B));
operator!(BitSetXor(A, B));
