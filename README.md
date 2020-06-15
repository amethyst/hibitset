# hibitset

[![Build Status](https://travis-ci.org/slide-rs/hibitset.svg)](https://travis-ci.org/slide-rs/hibitset)
[![Crates.io](https://img.shields.io/crates/v/hibitset.svg?maxAge=2592000)](https://crates.io/crates/hibitset)

Provides hierarchical bit sets, which allow very fast iteration on
sparse data structures.

## Usage

Just add this to your `Cargo.toml`:

```toml
[dependencies]
hibitset = "0.6"
```

## Using the `serde` feature

There is an optional feature to use `serde` for serialization/deserializtion. To enable this feature add the following to your `Cargo.toml`:

```
[dependencies]
hibitset = { version = "0.6.3", features = ["serde"]}
serde_derive = "1.0.111"
serde = "1.0.111"
bincode = "1.2.1"
```

Using `bincode` here is an example of how you can serialize/deserialize a `BitSet`:

```
#[macro_use]
extern crate serde_derive;

use hibitset::{BitSet, BitSetAnd, BitSetLike, BitSetNot};
use bincode;

fn main() {
    let mut set1 = BitSet::new();

    for i in 0..10 {
        set1.add(i * 2);
    }

    let serde_len = bincode::serialized_size(&set1).unwrap();
    let set1_buf = bincode::serialize(&set1).unwrap();

    let result_set1: BitSet = bincode::deserialize(&set1_buf).unwrap();

}
```

## License

This library is licensed under the Apache License 2.0,
see [the LICENSE file][li] for more information.

[li]: LICENSE
