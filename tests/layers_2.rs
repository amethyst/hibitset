extern crate hibitset;
extern crate typenum;
extern crate rand;
#[cfg(feature="parallel")]
extern crate rayon;

use hibitset::{BitSet, BitSetAnd, BitSetNot, BitSetLike};

use typenum::Unsigned;

use std::mem::size_of;

type Layers = typenum::U2;

#[test]
fn insert() {
    let bits = (size_of::<usize>() as f32).log2() as usize;
    let limit = bits * (Layers::to_usize() + 1);
    let max_size = 2 << (limit - 1);

    let step = 1000;
    let tests = max_size / step;

    let mut c = BitSet::<Layers>::new();
    for n in 0..tests {
        let n = n * step;
        assert!(!c.add(n));
        assert!(c.add(n));
    }

    for n in 0..tests {
        let n = n * step;
        assert!(c.contains(n));
    }
}

#[test]
fn insert_large() {
    let bits = (size_of::<usize>() as f32).log2() as usize;
    let limit = bits * (Layers::to_usize() + 1);
    let max_size = 2 << (limit - 1);

    let step = 10;
    let tests = max_size / step;

    let mut c = BitSet::<Layers>::new();
    for n in 0..tests {
        let n = n * step;
        assert!(!c.add(n));
        assert!(c.add(n));
    }

    for n in 0..tests {
        let n = n * step;
        assert!(c.contains(n));
    }
}
#[test]
fn remove() {
    let bits = (size_of::<usize>() as f32).log2() as usize;
    let limit = bits * (Layers::to_usize() + 1);
    let max_size = 2 << (limit - 1);

    let step = 1000;
    let tests = max_size / step;

    let mut c = BitSet::<Layers>::new();
    for n in 0..tests {
        let n = n * step;
        assert!(!c.add(n));
    }

    for n in 0..tests {
        let n = n * step;
        assert!(c.contains(n));
        assert!(c.remove(n));
        assert!(!c.contains(n));
        assert!(!c.remove(n));
    }
}

#[test]
fn iter() {
    let mut c = BitSet::<Layers>::new();
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
    let mut odd = BitSet::<Layers>::new();
    let mut even = BitSet::<Layers>::new();
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
    let mut set = BitSet::<Layers>::new();
    let mut rng = weak_rng();
    let max_added = 1_048_576 / 10;
    let mut added = 0;
    for _ in 0..max_added {
        let index = rng.gen_range(0, max_added);
        if !set.add(index) {
            added += 1;
        }
    }
    assert_eq!(set.iter().count(), added as usize);
}

#[test]
fn iter_clusters() {
    use std::mem::size_of;
    let bits = (size_of::<usize>() as f32).log2() as usize;
    let mut set = BitSet::<Layers>::new();
    for x in 0..8 {
        let x = (x * 3) << (bits * 2); // scale to the last slot
        for y in 0..8 {
            let y = (y * 3) << bits;
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
    let mut c = BitSet::<Layers>::new();
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

#[cfg(feature="parallel")]
mod parallel {
    use super::{BitSet, BitSetAnd, BitSetLike, Layers};
    
    use rayon::iter::ParallelIterator;

    use typenum::Unsigned;

    #[test]
    fn par_iter_one() {
        use std::mem::size_of;
    
        let bits = (size_of::<usize>() as f32).log2() as usize;
        let limit = bits * (Layers::to_usize() + 1);
        let max_size = 2 << (limit - 1);

        let step = 5000;
        let tests = max_size / step;
        for n in 0..tests {
            let n = n * step;
            let mut set = BitSet::<Layers>::new();
            set.add(n);
            assert_eq!(set.par_iter().count(), 1);
        }
        let mut set = BitSet::<Layers>::new();
        set.add(max_size - 1);
        assert_eq!(set.par_iter().count(), 1);
    }

    #[test]
    fn par_iter_random_add() {
        use rand::{Rng, weak_rng};
        use std::collections::HashSet;
        use std::sync::{Arc, Mutex};
        use std::mem::size_of;
    
        let bits = (size_of::<usize>() as f32).log2() as usize;
        let limit = bits * (Layers::to_usize() + 1);
        let max_size = 2 << (limit - 1);

        let mut set = BitSet::<Layers>::new();
        let mut check_set = HashSet::new();
        let mut rng = weak_rng();
        let max_added = max_size / 10;
        for _ in 0..max_added {
            let index = rng.gen_range(0, max_added);
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
        let mut odd = BitSet::<Layers>::new();
        let mut even = BitSet::<Layers>::new();
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
        use std::mem::size_of;
    
        let bits = (size_of::<usize>() as f32).log2() as usize;
        let mut set = BitSet::<Layers>::new();
        let mut check_set = HashSet::new();
        for x in 0..8 {
            let x = (x * 3) << (bits * 2); // scale to the last slot
            for y in 0..8 {
                let y = (y * 3) << (bits);
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
