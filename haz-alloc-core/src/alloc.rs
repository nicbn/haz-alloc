use crate::reserve::{ReserveHeader, ReserveType, RESERVE_ALIGN};
use crate::{huge, subhuge, utils};
use core::alloc::{GlobalAlloc, Layout};
use core::{cmp, ptr};
use haz_alloc_internal::UsizeExt;

/// # Safety
///
/// Layout must be valid.
#[inline]
pub unsafe fn alloc(layout: Layout) -> *mut u8 {
    if layout.align() > RESERVE_ALIGN {
        ptr::null_mut()
    } else if layout.size() > subhuge::MAX || layout.align() > utils::page_size() {
        huge::alloc(layout)
    } else {
        subhuge::alloc(layout, false)
    }
}

/// # Safety
///
/// Layout must be valid.
#[inline]
pub unsafe fn alloc_zeroed(layout: Layout) -> *mut u8 {
    if layout.align() > RESERVE_ALIGN {
        ptr::null_mut()
    } else if layout.size() > subhuge::MAX || layout.align() > utils::page_size() {
        huge::alloc(layout)
    } else {
        subhuge::alloc(layout, true)
    }
}

/// # Safety
///
/// Layout and pointer must be valid.
///
/// Alignment must match of original allocation.
pub unsafe fn realloc(ptr: *mut u8, layout: Layout) -> *mut u8 {
    let header = (ptr as usize - 1).align_down(RESERVE_ALIGN) as *mut ReserveHeader;
    match (*header).ty {
        ReserveType::Huge => {
            if huge::realloc_in_place(header, ptr, layout) {
                return ptr;
            }
        }
        ReserveType::SubHuge => {
            if subhuge::realloc_in_place(header, ptr, layout) {
                return ptr;
            }
        }
    }

    let new = alloc(layout);
    if new.is_null() {
        return ptr::null_mut();
    }
    new.copy_from_nonoverlapping(ptr, cmp::min(layout.size(), size(ptr)));
    dealloc(ptr);
    new
}

/// # Safety
///
/// Pointer must be valid.
#[inline]
pub unsafe fn dealloc(ptr: *mut u8) {
    let header = (ptr as usize - 1).align_down(RESERVE_ALIGN) as *mut ReserveHeader;
    match (*header).ty {
        ReserveType::Huge => huge::dealloc(header),
        ReserveType::SubHuge => subhuge::dealloc(header, ptr),
    }
}

/// # Safety
///
/// Pointer must be valid.
#[inline]
pub unsafe fn size(ptr: *mut u8) -> usize {
    let header = (ptr as usize - 1).align_down(RESERVE_ALIGN) as *mut ReserveHeader;
    match (*header).ty {
        ReserveType::Huge => huge::size(header, ptr),
        ReserveType::SubHuge => subhuge::size(ptr),
    }
}

pub struct Alloc;
unsafe impl GlobalAlloc for Alloc {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        alloc(layout)
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, _: Layout) {
        dealloc(ptr)
    }

    #[inline]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        alloc_zeroed(layout)
    }

    #[inline]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        realloc(
            ptr,
            Layout::from_size_align_unchecked(new_size, layout.align()),
        )
    }
}
