# haz-alloc

![License](https://shields.io/crates/l/haz-alloc)

haz-alloc is a general-purpose allocator written in Rust, inspired by jemalloc.

Currently this should not be considered production-ready.

## Crates

### `haz-alloc`

[![Crate](https://shields.io/crates/v/haz-alloc)](https://crates.io/crates/haz-alloc)
[![Documentation](https://shields.io/docsrs/haz-alloc)](https://docs.rs/haz-alloc)

Provides out-of-the-box allocation. Available for Windows and Unix (only Linux tested so far).

### `haz-alloc-core`

[![Crate](https://shields.io/crates/v/haz-alloc-core)](https://crates.io/crates/haz-alloc-core)
[![Documentation](https://shields.io/docsrs/haz-alloc-core)](https://docs.rs/haz-alloc-core)

Implementation of the allocator, needs some symbols to be provided in order to
work.

### `haz-alloc-internal`

Contains some internal code, not meant for user use.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

Besides the dual MIT/Apache-2.0 license, another common licensing approach used
