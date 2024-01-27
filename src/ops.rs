use std::iter::{FromIterator, IntoIterator};
use std::marker::PhantomData;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};
use std::usize;

use util::*;

use {AtomicBitSet, BitIter, BitSetLike, DrainableBitSet, GenericBitSet};

impl<'a, B, T> BitOrAssign<&'a B> for GenericBitSet<T>
where
    T: UnsignedInteger,
    B: BitSetLike<Underlying = T>,
{
    fn bitor_assign(&mut self, lhs: &B) {
        use iter::State::Continue;
        let mut iter = lhs.iter();
        while let Some(level) = (1..LAYERS).find(|&level| iter.handle_level(level) == Continue) {
            let lower = level - 1;
            let idx = iter.prefix[lower] as usize >> T::LOG_BITS;
            *self.layer_mut(lower, idx) |= lhs.get_from_layer(lower, idx);
        }
        self.layer3 |= lhs.layer3();
    }
}

impl<'a, B, T> BitAndAssign<&'a B> for GenericBitSet<T>
where
    T: UnsignedInteger,
    B: BitSetLike<Underlying = T>,
{
    fn bitand_assign(&mut self, lhs: &B) {
        use iter::State::*;
        let mut iter = lhs.iter();
        iter.masks[LAYERS - 1] &= self.layer3();
        while let Some(level) = (1..LAYERS).find(|&level| iter.handle_level(level) == Continue) {
            let lower = level - 1;
            let idx = iter.prefix[lower] as usize >> T::LOG_BITS;
            let our_layer = self.get_from_layer(lower, idx);
            let their_layer = lhs.get_from_layer(lower, idx);

            iter.masks[lower] &= our_layer;

            let mut masks = [T::ZERO; LAYERS];
            masks[lower] = our_layer & !their_layer;
            BitIter::new(&mut *self, masks, iter.prefix).clear();

            *self.layer_mut(lower, idx) &= their_layer;
        }
        let mut masks = [T::ZERO; LAYERS];
        masks[LAYERS - 1] = self.layer3() & !lhs.layer3();
        BitIter::new(&mut *self, masks, [0; LAYERS - 1]).clear();

        self.layer3 &= lhs.layer3();
    }
}

impl<'a, B, T> BitXorAssign<&'a B> for GenericBitSet<T>
where
    T: UnsignedInteger,
    B: BitSetLike<Underlying = T>,
{
    fn bitxor_assign(&mut self, lhs: &B) {
        use iter::State::*;
        let mut iter = lhs.iter();
        while let Some(level) = (1..LAYERS).find(|&level| iter.handle_level(level) == Continue) {
            let lower = level - 1;
            let idx = iter.prefix[lower] as usize >> T::LOG_BITS;

            if lower == 0 {
                *self.layer_mut(lower, idx) ^= lhs.get_from_layer(lower, idx);

                let mut change_bit = |level| {
                    let lower = level - 1;
                    let h = iter.prefix.get(level).cloned().unwrap_or(0);
                    let l = iter.prefix[lower] >> T::LOG_BITS;
                    let mask = T::ONE << T::from_u32(l & !h);

                    if self.get_from_layer(lower, l as usize) == T::ZERO {
                        *self.layer_mut(level, h as usize >> T::LOG_BITS) &= !mask;
                    } else {
                        *self.layer_mut(level, h as usize >> T::LOG_BITS) |= mask;
                    }
                };

                change_bit(level);
                if iter.masks[level] == T::ZERO {
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
#[derive(Debug, Clone)]
pub struct BitSetAnd<A, B>(pub A, pub B)
where
    A: BitSetLike,
    B: BitSetLike<Underlying = A::Underlying>;

impl<A, B> BitSetLike for BitSetAnd<A, B>
where
    A: BitSetLike,
    B: BitSetLike<Underlying = A::Underlying>,
{
    type Underlying = A::Underlying;

    #[inline]
    fn layer3(&self) -> Self::Underlying {
        self.0.layer3() & self.1.layer3()
    }
    #[inline]
    fn layer2(&self, i: usize) -> Self::Underlying {
        self.0.layer2(i) & self.1.layer2(i)
    }
    #[inline]
    fn layer1(&self, i: usize) -> Self::Underlying {
        self.0.layer1(i) & self.1.layer1(i)
    }
    #[inline]
    fn layer0(&self, i: usize) -> Self::Underlying {
        self.0.layer0(i) & self.1.layer0(i)
    }
    #[inline]
    fn contains(&self, i: Index) -> bool {
        self.0.contains(i) && self.1.contains(i)
    }
}

impl<A, B> DrainableBitSet for BitSetAnd<A, B>
where
    A: DrainableBitSet,
    B: DrainableBitSet<Underlying = A::Underlying>,
{
    #[inline]
    fn remove(&mut self, i: Index) -> bool {
        if self.contains(i) {
            self.0.remove(i);
            self.1.remove(i);
            true
        } else {
            false
        }
    }
}

/// `BitSetOr` takes two [`BitSetLike`] items, and merges the masks
/// returning a new virtual set, which represents an merged of the
/// two original sets.
///
/// [`BitSetLike`]: ../trait.BitSetLike.html
#[derive(Debug, Clone)]
pub struct BitSetOr<A, B>(pub A, pub B)
where
    A: BitSetLike,
    B: BitSetLike<Underlying = A::Underlying>;

impl<A, B> BitSetLike for BitSetOr<A, B>
where
    A: BitSetLike,
    B: BitSetLike<Underlying = A::Underlying>,
{
    type Underlying = A::Underlying;

    #[inline]
    fn layer3(&self) -> Self::Underlying {
        self.0.layer3() | self.1.layer3()
    }
    #[inline]
    fn layer2(&self, i: usize) -> Self::Underlying {
        self.0.layer2(i) | self.1.layer2(i)
    }
    #[inline]
    fn layer1(&self, i: usize) -> Self::Underlying {
        self.0.layer1(i) | self.1.layer1(i)
    }
    #[inline]
    fn layer0(&self, i: usize) -> Self::Underlying {
        self.0.layer0(i) | self.1.layer0(i)
    }
    #[inline]
    fn contains(&self, i: Index) -> bool {
        self.0.contains(i) || self.1.contains(i)
    }
}

impl<A, B> DrainableBitSet for BitSetOr<A, B>
where
    A: DrainableBitSet,
    B: DrainableBitSet<Underlying = A::Underlying>,
{
    #[inline]
    fn remove(&mut self, i: Index) -> bool {
        if self.contains(i) {
            self.0.remove(i);
            self.1.remove(i);
            true
        } else {
            false
        }
    }
}

/// `BitSetNot` takes a [`BitSetLike`] item, and produced an inverted virtual set.
/// Note: the implementation is sub-optimal because layers 1-3 are not active.
///
/// [`BitSetLike`]: ../trait.BitSetLike.html
#[derive(Debug, Clone)]
pub struct BitSetNot<A>(pub A)
where
    A: BitSetLike;

impl<A> BitSetLike for BitSetNot<A>
where
    A: BitSetLike,
{
    type Underlying = A::Underlying;

    #[inline]
    fn layer3(&self) -> A::Underlying {
        !A::Underlying::ZERO
    }
    #[inline]
    fn layer2(&self, _: usize) -> A::Underlying {
        !A::Underlying::ZERO
    }
    #[inline]
    fn layer1(&self, _: usize) -> A::Underlying {
        !A::Underlying::ZERO
    }
    #[inline]
    fn layer0(&self, i: usize) -> A::Underlying {
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
#[derive(Debug, Clone)]
pub struct BitSetXor<A, B>(pub A, pub B)
where
    A: BitSetLike,
    B: BitSetLike<Underlying = A::Underlying>;

impl<A, B> BitSetLike for BitSetXor<A, B>
where
    A: BitSetLike,
    B: BitSetLike<Underlying = A::Underlying>,
{
    type Underlying = A::Underlying;

    #[inline]
    fn layer3(&self) -> Self::Underlying {
        let xor = BitSetAnd(
            BitSetOr(&self.0, &self.1),
            BitSetNot(BitSetAnd(&self.0, &self.1)),
        );
        xor.layer3()
    }
    #[inline]
    fn layer2(&self, id: usize) -> Self::Underlying {
        let xor = BitSetAnd(
            BitSetOr(&self.0, &self.1),
            BitSetNot(BitSetAnd(&self.0, &self.1)),
        );
        xor.layer2(id)
    }
    #[inline]
    fn layer1(&self, id: usize) -> Self::Underlying {
        let xor = BitSetAnd(
            BitSetOr(&self.0, &self.1),
            BitSetNot(BitSetAnd(&self.0, &self.1)),
        );
        xor.layer1(id)
    }
    #[inline]
    fn layer0(&self, id: usize) -> Self::Underlying {
        let xor = BitSetAnd(
            BitSetOr(&self.0, &self.1),
            BitSetNot(BitSetAnd(&self.0, &self.1)),
        );
        xor.layer0(id)
    }
    #[inline]
    fn contains(&self, i: Index) -> bool {
        BitSetAnd(
            BitSetOr(&self.0, &self.1),
            BitSetNot(BitSetAnd(&self.0, &self.1)),
        )
        .contains(i)
    }
}

/// `BitSetAll` is a bitset with all bits set. Essentially the same as
/// `BitSetNot(BitSet::new())` but without any allocation.
#[derive(Debug, Clone)]
pub struct BitSetAll<T: UnsignedInteger> {
    _phantom: PhantomData<T>,
}
impl<T: UnsignedInteger> BitSetLike for BitSetAll<T> {
    type Underlying = T;

    #[inline]
    fn layer3(&self) -> Self::Underlying {
        T::MAX
    }
    #[inline]
    fn layer2(&self, _id: usize) -> Self::Underlying {
        T::MAX
    }
    #[inline]
    fn layer1(&self, _id: usize) -> Self::Underlying {
        T::MAX
    }
    #[inline]
    fn layer0(&self, _id: usize) -> Self::Underlying {
        T::MAX
    }
    #[inline]
    fn contains(&self, _i: Index) -> bool {
        true
    }
}

macro_rules! operator {
    ( impl < ( $( $lifetime:tt )* ) ( $( $arg:ident ),* ) > for $bitset:ty ) => {
        impl<$( $lifetime, )* T, $( $arg ),*> IntoIterator for $bitset
            where
                T: UnsignedInteger,
                $( $arg: BitSetLike<Underlying = T> ),*
        {
            type Item = <BitIter<Self> as Iterator>::Item;
            type IntoIter = BitIter<Self>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }

        impl<$( $lifetime, )* T, $( $arg ),*> Not for $bitset
            where
                T: UnsignedInteger,
                $( $arg: BitSetLike<Underlying = T> ),*
        {
            type Output = BitSetNot<Self>;
            fn not(self) -> Self::Output {
                BitSetNot(self)
            }
        }

        impl<$( $lifetime, )* T, $( $arg, )* OtherBitSetLike> BitAnd<OtherBitSetLike> for $bitset
            where
                T: UnsignedInteger,
                OtherBitSetLike: BitSetLike<Underlying = T>,
                $( $arg: BitSetLike<Underlying = T> ),*
        {
            type Output = BitSetAnd<Self, OtherBitSetLike>;
            fn bitand(self, rhs: OtherBitSetLike) -> Self::Output {
                BitSetAnd(self, rhs)
            }
        }

        impl<$( $lifetime, )* T, $( $arg, )* OtherBitSetLike> BitOr<OtherBitSetLike> for $bitset
            where
                T: UnsignedInteger,
                OtherBitSetLike: BitSetLike<Underlying = T>,
                $( $arg: BitSetLike<Underlying = T> ),*
        {
            type Output = BitSetOr<Self, OtherBitSetLike>;
            fn bitor(self, rhs: OtherBitSetLike) -> Self::Output {
                BitSetOr(self, rhs)
            }
        }

        impl<$( $lifetime, )* T, $( $arg, )* OtherBitSetLike> BitXor<OtherBitSetLike> for $bitset
            where
                T: UnsignedInteger,
                OtherBitSetLike: BitSetLike<Underlying = T>,
                $( $arg: BitSetLike<Underlying = T> ),*
        {
            type Output = BitSetXor<Self, OtherBitSetLike>;
            fn bitxor(self, rhs: OtherBitSetLike) -> Self::Output {
                BitSetXor(self, rhs)
            }
        }

    }
}

operator!(impl<()()> for GenericBitSet<T>);
operator!(impl<('a)()> for &'a GenericBitSet<T>);
operator!(impl<()(A)> for BitSetNot<A>);
operator!(impl<('a)(A)> for &'a BitSetNot<A>);
operator!(impl<()(A, B)> for BitSetAnd<A, B>);
operator!(impl<('a)(A, B)> for &'a BitSetAnd<A, B>);
operator!(impl<()(A, B)> for BitSetOr<A, B>);
operator!(impl<('a)(A, B)> for &'a BitSetOr<A, B>);
operator!(impl<()(A, B)> for BitSetXor<A, B>);
operator!(impl<('a)(A, B)> for &'a BitSetXor<A, B>);
operator!(impl<()()> for BitSetAll<T>);
operator!(impl<('a)()> for &'a BitSetAll<T>);

impl<T: UnsignedInteger> FromIterator<Index> for GenericBitSet<T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Index>,
    {
        let mut bitset = Self::new();
        for item in iter {
            bitset.add(item);
        }
        bitset
    }
}

impl<'a, T: UnsignedInteger> FromIterator<&'a Index> for GenericBitSet<T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = &'a Index>,
    {
        let mut bitset = Self::new();
        for item in iter {
            bitset.add(*item);
        }
        bitset
    }
}

impl<T: UnsignedInteger> Extend<Index> for GenericBitSet<T> {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = Index>,
    {
        for item in iter {
            self.add(item);
        }
    }
}

impl<'a, T: UnsignedInteger> Extend<&'a Index> for GenericBitSet<T> {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = &'a Index>,
    {
        for item in iter {
            self.add(*item);
        }
    }
}

// All specialized implementations for `AtomicBitSet`

impl FromIterator<Index> for AtomicBitSet {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Index>,
    {
        let mut bitset = AtomicBitSet::new();
        for item in iter {
            bitset.add(item);
        }
        bitset
    }
}
impl<'a> FromIterator<&'a Index> for AtomicBitSet {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = &'a Index>,
    {
        let mut bitset = AtomicBitSet::new();
        for item in iter {
            bitset.add(*item);
        }
        bitset
    }
}
impl Extend<Index> for AtomicBitSet {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = Index>,
    {
        for item in iter {
            self.add(item);
        }
    }
}
impl<'a> Extend<&'a Index> for AtomicBitSet {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = &'a Index>,
    {
        for item in iter {
            self.add(*item);
        }
    }
}
impl IntoIterator for AtomicBitSet {
    type Item = <BitIter<Self> as Iterator>::Item;
    type IntoIter = BitIter<Self>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<'a> IntoIterator for &'a AtomicBitSet {
    type Item = <BitIter<Self> as Iterator>::Item;
    type IntoIter = BitIter<Self>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl Not for AtomicBitSet {
    type Output = BitSetNot<Self>;
    fn not(self) -> Self::Output {
        BitSetNot(self)
    }
}
impl<'a> Not for &'a AtomicBitSet {
    type Output = BitSetNot<Self>;
    fn not(self) -> Self::Output {
        BitSetNot(self)
    }
}
impl<OtherBitSetLike> BitAnd<OtherBitSetLike> for AtomicBitSet
where
    OtherBitSetLike: BitSetLike<Underlying = usize>,
{
    type Output = BitSetAnd<Self, OtherBitSetLike>;
    fn bitand(self, rhs: OtherBitSetLike) -> Self::Output {
        BitSetAnd(self, rhs)
    }
}
impl<'a, OtherBitSetLike> BitAnd<OtherBitSetLike> for &'a AtomicBitSet
where
    OtherBitSetLike: BitSetLike<Underlying = usize>,
{
    type Output = BitSetAnd<Self, OtherBitSetLike>;
    fn bitand(self, rhs: OtherBitSetLike) -> Self::Output {
        BitSetAnd(self, rhs)
    }
}
impl<OtherBitSetLike> BitOr<OtherBitSetLike> for AtomicBitSet
where
    OtherBitSetLike: BitSetLike<Underlying = usize>,
{
    type Output = BitSetOr<Self, OtherBitSetLike>;
    fn bitor(self, rhs: OtherBitSetLike) -> Self::Output {
        BitSetOr(self, rhs)
    }
}
impl<'a, OtherBitSetLike> BitOr<OtherBitSetLike> for &'a AtomicBitSet
where
    OtherBitSetLike: BitSetLike<Underlying = usize>,
{
    type Output = BitSetOr<Self, OtherBitSetLike>;
    fn bitor(self, rhs: OtherBitSetLike) -> Self::Output {
        BitSetOr(self, rhs)
    }
}
impl<OtherBitSetLike> BitXor<OtherBitSetLike> for AtomicBitSet
where
    OtherBitSetLike: BitSetLike<Underlying = usize>,
{
    type Output = BitSetXor<Self, OtherBitSetLike>;
    fn bitxor(self, rhs: OtherBitSetLike) -> Self::Output {
        BitSetXor(self, rhs)
    }
}
impl<'a, OtherBitSetLike> BitXor<OtherBitSetLike> for &'a AtomicBitSet
where
    OtherBitSetLike: BitSetLike<Underlying = usize>,
{
    type Output = BitSetXor<Self, OtherBitSetLike>;
    fn bitxor(self, rhs: OtherBitSetLike) -> Self::Output {
        BitSetXor(self, rhs)
    }
}

#[cfg(test)]
mod tests {
    extern crate typed_test_gen;
    use self::typed_test_gen::test_with;

    use {BitSetLike, BitSetXor, GenericBitSet, Index, UnsignedInteger};

    #[test_with(u32, u64, usize)]
    fn or_assign<T: UnsignedInteger>() {
        use std::collections::HashSet;
        use std::mem::size_of;

        let uint_bits = size_of::<T>() as u32 * 8;
        let n = 10_000;
        let f1 = &|i| (7 * uint_bits * i) % T::MAX_EID;
        let f2 = &|i| (13 * uint_bits * i) % T::MAX_EID;

        let mut c1: GenericBitSet<T> = (0..n).map(f1).collect();
        let c2: GenericBitSet<T> = (0..n).map(f2).collect();

        c1 |= &c2;

        let h1: HashSet<_> = (0..n).map(f1).collect();
        let h2: HashSet<_> = (0..n).map(f2).collect();
        assert_eq!(c1.iter().collect::<HashSet<_>>(), &h1 | &h2);
    }

    #[test_with(u32, u64, usize)]
    fn or_assign_random<T: UnsignedInteger>() {
        use rand::prelude::*;

        use std::collections::HashSet;
        let limit = 1_048_576;
        let mut rng = thread_rng();

        let mut set1 = GenericBitSet::<T>::new();
        let mut check_set1 = HashSet::new();
        for _ in 0..(limit / 100) {
            let index = rng.gen_range(0, limit);
            set1.add(index);
            check_set1.insert(index);
        }

        let mut set2 = GenericBitSet::<T>::new();
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

    #[test_with(u32, u64, usize)]
    fn and_assign<T: UnsignedInteger>() {
        use std::collections::HashSet;
        use std::mem::size_of;

        let uint_bits = size_of::<T>() as u32 * 8;
        let n = 10_000;
        let f1 = &|n| (7 * uint_bits * n) % T::MAX_EID;
        let f2 = &|n| (13 * uint_bits * n) % T::MAX_EID;

        let mut c1: GenericBitSet<T> = (0..n).map(f1).collect();
        let c2: GenericBitSet<T> = (0..n).map(f2).collect();

        c1 &= &c2;

        let h1: HashSet<_> = (0..n).map(f1).collect();
        let h2: HashSet<_> = (0..n).map(f2).collect();
        assert_eq!(c1.iter().collect::<HashSet<_>>(), &h1 & &h2);
    }

    #[test_with(u32, u64, usize)]
    fn and_assign_specific<T: UnsignedInteger>() {
        let mut c1 = GenericBitSet::<T>::new();
        c1.add(0);
        let common = ((1 << T::LOG_BITS) << T::LOG_BITS) << T::LOG_BITS;
        c1.add(common);
        c1.add((((1 << T::LOG_BITS) << T::LOG_BITS) + 1) << T::LOG_BITS);

        let mut c2 = GenericBitSet::<T>::new();
        c2.add(common);
        c2.add((((1 << T::LOG_BITS) << T::LOG_BITS) + 2) << T::LOG_BITS);

        c1 &= &c2;

        assert_eq!(c1.iter().collect::<Vec<_>>(), [common]);
    }

    #[test_with(u32, u64, usize)]
    fn and_assign_with_modification<T: UnsignedInteger>() {
        let mut c1 = GenericBitSet::<T>::new();
        c1.add(0);
        c1.add((1 << T::LOG_BITS) << T::LOG_BITS);

        let mut c2 = GenericBitSet::<T>::new();
        c2.add(0);

        c1 &= &c2;

        let added = ((1 << T::LOG_BITS) + 1) << T::LOG_BITS;
        c1.add(added);

        assert_eq!(c1.iter().collect::<Vec<_>>(), [0, added]);
    }

    #[test_with(u32, u64, usize)]
    fn and_assign_random<T: UnsignedInteger>() {
        use rand::prelude::*;

        use std::collections::HashSet;
        let limit = 1_048_576;
        let mut rng = thread_rng();

        let mut set1 = GenericBitSet::<T>::new();
        let mut check_set1 = HashSet::new();
        for _ in 0..(limit / 100) {
            let index = rng.gen_range(0, limit);
            set1.add(index);
            check_set1.insert(index);
        }

        let mut set2 = GenericBitSet::<T>::new();
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

    #[test_with(u32, u64, usize)]
    fn xor_assign<T: UnsignedInteger>() {
        use std::collections::HashSet;
        use std::mem::size_of;

        let uint_bits = size_of::<T>() as u32 * 8;
        let n = 10_000;
        let f1 = &|n| (7 * uint_bits * n) % T::MAX_EID;
        let f2 = &|n| (13 * uint_bits * n) % T::MAX_EID;

        let mut c1: GenericBitSet<T> = (0..n).map(f1).collect();
        let c2: GenericBitSet<T> = (0..n).map(f2).collect();
        c1 ^= &c2;

        let h1: HashSet<_> = (0..n).map(f1).collect();
        let h2: HashSet<_> = (0..n).map(f2).collect();
        assert_eq!(c1.iter().collect::<HashSet<_>>(), &h1 ^ &h2);
    }

    #[test_with(u32, u64, usize)]
    fn xor_assign_specific<T: UnsignedInteger>() {
        let mut c1 = GenericBitSet::<T>::new();
        c1.add(0);
        let common = ((1 << T::LOG_BITS) << T::LOG_BITS) << T::LOG_BITS;
        c1.add(common);
        let a = (((1 << T::LOG_BITS) + 1) << T::LOG_BITS) << T::LOG_BITS;
        c1.add(a);

        let mut c2 = GenericBitSet::<T>::new();
        c2.add(common);
        let b = (((1 << T::LOG_BITS) + 2) << T::LOG_BITS) << T::LOG_BITS;
        c2.add(b);

        c1 ^= &c2;

        assert_eq!(c1.iter().collect::<Vec<_>>(), [0, a, b]);
    }

    #[test_with(u32, u64, usize)]
    fn xor_assign_random<T: UnsignedInteger>() {
        use rand::prelude::*;
        use std::collections::HashSet;
        let limit = 1_048_576;
        let mut rng = thread_rng();

        let mut set1 = GenericBitSet::<T>::new();
        let mut check_set1 = HashSet::new();
        for _ in 0..(limit / 100) {
            let index = rng.gen_range(0, limit);
            set1.add(index);
            check_set1.insert(index);
        }

        let mut set2 = GenericBitSet::<T>::new();
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

    #[test_with(u32, u64, usize)]
    fn operators<T: UnsignedInteger>() {
        let mut bitset = GenericBitSet::<T>::new();
        bitset.add(1);
        bitset.add(3);
        bitset.add(5);
        bitset.add(15);
        bitset.add(200);
        bitset.add(50001);

        let mut other = GenericBitSet::<T>::new();
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

    #[test_with(u32, u64, usize)]
    fn xor<T: UnsignedInteger>() {
        // 0011
        let mut bitset = GenericBitSet::<T>::new();
        bitset.add(2);
        bitset.add(3);
        bitset.add(50000);

        // 0101
        let mut other = GenericBitSet::<T>::new();
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
