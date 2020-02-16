# Changelog

## 0.6.3 (2020-02-17)

* `BitSetAnd`, `BitSetOr`, `BitSetNot`, `BitSetXor`, `BitSetAll` now implement `Clone`. ([#52])
* Bitset layers can be read though `BitSet::layer{0,1,2}_as_slice`. ([#53])
* `rayon` is updated to `1.3`. ([#56])

[#52]: https://github.com/amethyst/hibitset/pull/52
[#53]: https://github.com/amethyst/hibitset/pull/53
[#56]: https://github.com/amethyst/hibitset/pull/56

## 0.6.2 (2019-07-27)

* `BitIter` now implements `Clone`. ([#49])

[#49]: https://github.com/amethyst/hibitset/pull/49
