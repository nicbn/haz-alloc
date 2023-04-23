# 🚧 Work in progress 🚧

Currently this should not be considered production-ready.

# haz-alloc

[![Crate](https://shields.io/crates/v/haz-alloc?style=for-the-badge)](https://crates.io/crates/haz-alloc)
[![Documentation](https://shields.io/docsrs/haz-alloc?style=for-the-badge)](https://docs.rs/haz-alloc)
[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/nicbn/haz-alloc/rust.yml?style=for-the-badge)](https://github.com/nicbn/haz-alloc/actions)
[![License](https://shields.io/crates/l/haz-alloc?style=for-the-badge)](#license)

haz-alloc is a general-purpose allocator written in Rust, inspired by jemalloc.

**This crate requires the nightly version of Rust.**

## Supported platforms

Supported platforms by `haz-alloc`.

| Platform         | Supported | Tested     |
|------------------|-----------|------------|
| Windows          | ✔️        | ✔️         |
| Linux            | ✔️        | ✔️         |
| Mac OS           | ✔️        | ✔️         |
| Other Unix-like  | ✔️        | ❌         |

If you want to use on some platform that `haz-alloc` does not support, you
can use [`haz-alloc-core`](haz-alloc-core), that implements the allocator, and provide the
system functions it uses.

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
