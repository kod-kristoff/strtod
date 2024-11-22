# strtod for Rust

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-informational?style=flat-square)](COPYRIGHT.md)

## Introduction

`strtod` is a floating point parsing implementation for Rust with very
high precision, far better than the built in Rust floating point parser.
https://docs.rs/strtod
The documentation can be found at <https://docs.rs/strtod>.

This is a maintained fork of [`strtod`](https://github.com/pvginkel/strtod).

## Remarks

The quality of the source is not really something to write home about.
The reason for this is that this implementation is a verbatim translation
from <http://mxr.mozilla.org/mozilla-central/source/js/src/dtoa.c>.
That being said, the quality of the parser itself is very high.

The performance of this implementation should be OK. However there is room
for improvement in the BigNum implementation that the parser uses, e.g.
by caching instances or calculations. The original implementation does
this, but this has been removed from this implementation.

## Bugs

Bugs should be reported through github at
[http://github.com/kod-kristoff/strtod/issues](http://github.com/kod-kristoff/strtod/issues).

## Minimum Supported Rust Version

This library supports Rust version `1.16.0`. Every change of the MSRV is done with a minor version bump.

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
