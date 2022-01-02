#![allow(clippy::non_send_fields_in_send_ty)]

mod sys;

pub use haz_alloc_core::{alloc, alloc_zeroed, dealloc, realloc, size, Alloc};
