use crate::Backend;
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
    size: usize,
    pub ty: ReserveType,
}

pub fn new<B: Backend>(size: usize, ty: ReserveType) -> (usize, *mut ReserveHeader) {
    if size >= usize::MAX - RESERVE_ALIGN {
        return (0, ptr::null_mut());
    }

    let total_size = RESERVE_ALIGN + size;
    let base = B::mreserve(ptr::null_mut(), total_size);
    if base.is_null() {
        return (0, ptr::null_mut());
    }

    let offset = base.align_offset(RESERVE_ALIGN);
    let ptr = unsafe { base.add(offset) as *mut ReserveHeader };
    if unsafe { !B::mcommit(ptr as *mut u8, B::pagesize()) } {
        return (0, ptr::null_mut());
    }
    unsafe {
        ptr.write(ReserveHeader {
            offset: offset as u32,
            size: total_size,
            ty,
        });
    }

    (total_size - offset, ptr)
}

#[inline]
pub unsafe fn delete(munreserve: unsafe fn(ptr: *mut u8, size: usize), ptr: *mut ReserveHeader) {
    let offset = (*ptr).offset;
    munreserve((ptr as *mut u8).sub(offset as usize), (*ptr).size);
}
