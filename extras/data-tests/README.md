This crate allows running the test based on files with test cases stored in the
standardized format (credit to Daniel Lemire and Nigel Tao for the test cases).
The test data is sourced from [this](https://github.com/lemire/fast_float_supplemental_tests)
repository which is used for the original fast_float C++ library tests.

This crate is copied from [Alexhuszagh/fast-float-rust](https://github.com/Alexhuszagh/fast-float-rust)
and modified to test `strtod`. Currently only `f64` is supported by `strtod`.

Test data files can be found under `ext/data`.

To run the tests:

```sh
cargo run --release
```
