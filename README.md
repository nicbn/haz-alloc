# 🚧 Work in progress 🚧

Currently this should not be considered production-ready.

# haz-alloc

![License](https://shields.io/crates/l/haz-alloc)

|                  |                |
|------------------|----------------|
| `haz-alloc`      | [![Crate](https://shields.io/crates/v/haz-alloc)](https://crates.io/crates/haz-alloc) [![Documentation](https://shields.io/docsrs/haz-alloc)](https://docs.rs/haz-alloc) |
| `haz-alloc-core` | [![Crate](https://shields.io/crates/v/haz-alloc-core)](https://crates.io/crates/haz-alloc-core) [![Documentation](https://shields.io/docsrs/haz-alloc-core)](https://docs.rs/haz-alloc-core) |

haz-alloc is a general-purpose allocator written in Rust, inspired by jemalloc.


You probably want is the `haz-alloc` crate. It provides everything
out-of-the-box, ready to use.

If you want to use on some platform that `haz-alloc` does not support, you
can use `haz-alloc-core`, that implements the allocator, and provide the
system functions it uses.

## Supported platforms

Supported platforms by `haz-alloc`. `haz-alloc-core` is independent of platform
and should work on pretty much anything.

| Platform         | Supported | Tested     |
|------------------|-----------|------------|
| Windows          | ✔️        | ❌         |
| Linux            | ✔️        | ✔️         |
| Other Unix-like  | ✔️        | ❌         |

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
