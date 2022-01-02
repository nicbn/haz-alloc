use crate::{sys, utils};
use core::ptr;

#[cfg(target_pointer_width = "64")]
pub const RESERVE_ALIGN: usize = 32 * 1024 * 1024;
#[cfg(not(target_pointer_width = "64"))]
pub const RESERVE_ALIGN: usize = 2 * 1024 * 1024;

#[repr(u32)]
pub enum ReserveType {
    SubHuge,
    Huge,
}

pub struct ReserveHeader {
    offset: u32,
    pub ty: ReserveType,
}

pub fn new(size: usize, ty: ReserveType) -> (usize, *mut ReserveHeader) {
    if size >= usize::MAX - RESERVE_ALIGN {
        return (0, ptr::null_mut());
    }

    let total_size = RESERVE_ALIGN + size;
    let base = unsafe { sys::__haz_alloc_mreserve(ptr::null_mut(), total_size) };
    if base.is_null() {
        return (0, ptr::null_mut());
    }

    let offset = base.align_offset(RESERVE_ALIGN);
    let ptr = unsafe { base.add(offset) as *mut ReserveHeader };
    unsafe { sys::__haz_alloc_mcommit(ptr as *mut u8, utils::page_size()) };
    unsafe {
        ptr.write(ReserveHeader {
            offset: offset as u32,
            ty,
        });
    }

    (total_size - offset, ptr)
}

#[inline]
pub unsafe fn delete(ptr: *mut ReserveHeader) {
    let offset = (*ptr).offset;
    sys::__haz_alloc_munreserve((ptr as *mut u8).sub(offset as usize));
}
