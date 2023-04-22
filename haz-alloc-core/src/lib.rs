#![no_std]
#![feature(thread_local)]
#![warn(clippy::all)]

mod alloc;
pub mod backend;
mod bitset;
mod huge;
mod reserve;
mod subhuge;

#[doc(hidden)]
pub mod __internal;

pub use self::alloc::*;
pub use self::backend::Backend;
