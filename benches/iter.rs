#![feature(test)]
extern crate hibitset;
extern crate test;
extern crate rand;
extern crate rayon;

use hibitset::{BitSet, BitSetLike};

use test::{Bencher, black_box};

use rand::{Rng, XorShiftRng};

use rayon::iter::ParallelIterator;

#[bench]
fn iter_100(b: &mut Bencher) {
    let n = 100;
    let mut rng = XorShiftRng::new_unseeded();
    let mut bitset = BitSet::with_capacity(1048576);
    for _ in 0..n {
        let index = rng.gen_range(0, 1048576);
        bitset.add(index);
    }
    b.iter(|| black_box((&bitset).iter().map(|n| black_box(n)).count()));
}

#[bench]
fn iter_1000(b: &mut Bencher) {
    let n = 1000;
    let mut rng = XorShiftRng::new_unseeded();
    let mut bitset = BitSet::with_capacity(1048576);
    for _ in 0..n {
        let index = rng.gen_range(0, 1048576);
        bitset.add(index);
    }
    b.iter(|| black_box((&bitset).iter().map(|n| black_box(n)).count()));
}

#[bench]
fn iter_10000(b: &mut Bencher) {
    let n = 10_000;
    let mut rng = XorShiftRng::new_unseeded();
    let mut bitset = BitSet::with_capacity(1048576);
    for _ in 0..n {
        let index = rng.gen_range(0, 1048576);
        bitset.add(index);
    }
    b.iter(|| black_box((&bitset).iter().map(|n| black_box(n)).count()));
}

#[bench]
fn iter_100000(b: &mut Bencher) {
    let n = 100_000;
    let mut rng = XorShiftRng::new_unseeded();
    let mut bitset = BitSet::with_capacity(1048576);
    for _ in 0..n {
        let index = rng.gen_range(0, 1048576);
        bitset.add(index);
    }
    b.iter(|| black_box((&bitset).iter().map(|n| black_box(n)).count()));
}

#[bench]
fn iter_1000000(b: &mut Bencher) {
    let n = 1_000_000;
    let mut rng = XorShiftRng::new_unseeded();
    let mut bitset = BitSet::with_capacity(1048576);
    for _ in 0..n {
        let index = rng.gen_range(0, 1048576);
        bitset.add(index);
    }
    b.iter(|| black_box((&bitset).iter().map(|n| black_box(n)).count()));
}

#[bench]
fn par_iter_100(b: &mut Bencher) {
    let n = 100;
    let mut rng = XorShiftRng::new_unseeded();
    let mut bitset = BitSet::with_capacity(1048576);
    for _ in 0..n {
        let index = rng.gen_range(0, 1048576);
        bitset.add(index);
    }
    b.iter(|| black_box((&bitset).par_iter().map(|n| black_box(n)).count()));
}

#[bench]
fn par_iter_1000(b: &mut Bencher) {
    let n = 1000;
    let mut rng = XorShiftRng::new_unseeded();
    let mut bitset = BitSet::with_capacity(1048576);
    for _ in 0..n {
        let index = rng.gen_range(0, 1048576);
        bitset.add(index);
    }
    b.iter(|| black_box((&bitset).par_iter().map(|n| black_box(n)).count()));
}

#[bench]
fn par_iter_10000(b: &mut Bencher) {
    let n = 10_000;
    let mut rng = XorShiftRng::new_unseeded();
    let mut bitset = BitSet::with_capacity(1048576);
    for _ in 0..n {
        let index = rng.gen_range(0, 1048576);
        bitset.add(index);
    }
    b.iter(|| black_box((&bitset).par_iter().map(|n| black_box(n)).count()));
}

#[bench]
fn par_iter_100000(b: &mut Bencher) {
    let n = 100_000;
    let mut rng = XorShiftRng::new_unseeded();
    let mut bitset = BitSet::with_capacity(1048576);
    for _ in 0..n {
        let index = rng.gen_range(0, 1048576);
        bitset.add(index);
    }
    b.iter(|| black_box((&bitset).par_iter().map(|n| black_box(n)).count()));
}

#[bench]
fn par_iter_1000000(b: &mut Bencher) {
    let n = 1_000_000;
    let mut rng = XorShiftRng::new_unseeded();
    let mut bitset = BitSet::with_capacity(1048576);
    for _ in 0..n {
        let index = rng.gen_range(0, 1048576);
        bitset.add(index);
    }
    b.iter(|| black_box((&bitset).par_iter().map(|n| black_box(n)).count()));
}