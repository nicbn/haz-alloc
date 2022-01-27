#![no_std]
#![feature(thread_local, const_fn_fn_ptr_basics)]
#![warn(clippy::all)]

mod alloc;
pub mod backend;
mod bitset;
mod huge;
mod reserve;
mod subhuge;

pub use self::alloc::*;
pub use self::backend::Backend;
