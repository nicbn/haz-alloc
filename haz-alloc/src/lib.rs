#![allow(clippy::non_send_fields_in_send_ty)]

use std::alloc::{GlobalAlloc, Layout};

mod sys;
mod sys_common;

#[derive(Clone, Copy)]
pub struct Alloc {
    alloc: haz_alloc_core::Alloc<sys::Backend>,
}

impl Alloc {
    pub const fn new() -> Self {
        Alloc {
            alloc: unsafe { haz_alloc_core::Alloc::new() },
        }
    }
}

impl Alloc {
    /// # Safety
    ///
    /// Layout must be valid.
    #[inline]
    pub unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc.alloc(layout)
    }

    /// # Safety
    ///
    /// Layout must be valid.
    #[inline]
    pub unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        self.alloc.alloc_zeroed(layout)
    }

    /// # Safety
    ///
    /// Layout and pointer must be valid.
    ///
    /// Alignment must match of original allocation.
    #[inline]
    pub unsafe fn realloc(&self, ptr: *mut u8, layout: Layout) -> *mut u8 {
        self.alloc.realloc(ptr, layout)
    }

    /// # Safety
    ///
    /// Pointer must be valid.
    #[inline]
    pub unsafe fn dealloc(&self, ptr: *mut u8) {
        self.alloc.dealloc(ptr)
    }

    /// # Safety
    ///
    /// Pointer must be valid.
    #[inline]
    pub unsafe fn size(&self, ptr: *mut u8) -> usize {
        self.alloc.size(ptr)
    }
}

unsafe impl GlobalAlloc for Alloc {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc(layout)
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, _: Layout) {
        self.dealloc(ptr)
    }

    #[inline]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        self.alloc_zeroed(layout)
    }

    #[inline]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        self.realloc(
            ptr,
            Layout::from_size_align_unchecked(new_size, layout.align()),
        )
    }
}

impl Default for Alloc {
    fn default() -> Self {
        Alloc::new()
    }
}
