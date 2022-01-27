use crate::backend::Backend;
use crate::reserve::{ReserveHeader, ReserveType, RESERVE_ALIGN};
use crate::{huge, subhuge};
use core::alloc::{GlobalAlloc, Layout};
use core::marker::PhantomData;
use core::{cmp, ptr};
use haz_alloc_internal::UsizeExt;

pub struct Alloc<B> {
    _backend: PhantomData<B>,
}

impl<B> Copy for Alloc<B> {}

impl<B> Clone for Alloc<B> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            _backend: PhantomData,
        }
    }
}

impl<B> Alloc<B> {
    /// Create a new `Alloc`.
    ///
    /// # Safety
    ///
    /// All `Alloc::new` must be called with the same backend.
    pub const unsafe fn new() -> Self {
        Self {
            _backend: PhantomData,
        }
    }
}

impl<B: Backend> Alloc<B> {
    /// # Safety
    ///
    /// Layout must be valid.
    #[inline]
    pub unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.align() > RESERVE_ALIGN {
            ptr::null_mut()
        } else if layout.size() > subhuge::MAX || layout.align() > B::pagesize() {
            huge::alloc::<B>(layout)
        } else {
            subhuge::alloc::<B>(layout, false)
        }
    }

    /// # Safety
    ///
    /// Layout must be valid.
    #[inline]
    pub unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if layout.align() > RESERVE_ALIGN {
            ptr::null_mut()
        } else if layout.size() > subhuge::MAX || layout.align() > B::pagesize() {
            huge::alloc::<B>(layout)
        } else {
            subhuge::alloc::<B>(layout, true)
        }
    }

    /// # Safety
    ///
    /// Layout and pointer must be valid.
    ///
    /// Alignment must match of original allocation.
    pub unsafe fn realloc(&self, ptr: *mut u8, layout: Layout) -> *mut u8 {
        let header = (ptr as usize - 1).align_down(RESERVE_ALIGN) as *mut ReserveHeader;
        match (*header).ty {
            ReserveType::Huge => {
                if huge::realloc_in_place::<B>(header, ptr, layout) {
                    return ptr;
                }
            }
            ReserveType::SubHuge => {
                if subhuge::realloc_in_place::<B>(header, ptr, layout) {
                    return ptr;
                }
            }
        }

        let new = self.alloc(layout);
        if new.is_null() {
            return ptr::null_mut();
        }
        new.copy_from_nonoverlapping(ptr, cmp::min(layout.size(), self.size(ptr)));
        self.dealloc(ptr);
        new
    }

    /// # Safety
    ///
    /// Pointer must be valid.
    #[inline]
    pub unsafe fn dealloc(&self, ptr: *mut u8) {
        let header = (ptr as usize - 1).align_down(RESERVE_ALIGN) as *mut ReserveHeader;
        match (*header).ty {
            ReserveType::Huge => huge::dealloc::<B>(header),
            ReserveType::SubHuge => subhuge::dealloc::<B>(header, ptr),
        }
    }

    /// # Safety
    ///
    /// Pointer must be valid.
    #[inline]
    pub unsafe fn size(&self, ptr: *mut u8) -> usize {
        let header = (ptr as usize - 1).align_down(RESERVE_ALIGN) as *mut ReserveHeader;
        match (*header).ty {
            ReserveType::Huge => huge::size(header, ptr),
            ReserveType::SubHuge => subhuge::size::<B>(ptr),
        }
    }
}

unsafe impl<B: Backend> GlobalAlloc for Alloc<B> {
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
