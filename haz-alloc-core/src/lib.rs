#![no_std]
#![feature(thread_local, const_fn_fn_ptr_basics)]

#[macro_use]
mod utils;
mod alloc;
mod bitset;
mod huge;
mod reserve;
mod subhuge;
pub mod sys;
pub use self::alloc::*;
