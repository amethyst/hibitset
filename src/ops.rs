use BitSetLike;

/// `BitSetAnd` takes two [`BitSetLike`] items, and merges the masks
/// returning a new virtual set, which represents an intersection of the
/// two original sets.
///
/// [`BitSetLike`]: ../trait.BitSetLike.html
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
