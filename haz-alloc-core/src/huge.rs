use crate::reserve::{self, ReserveHeader, ReserveType, RESERVE_ALIGN};
use crate::{sys, utils};
use core::alloc::Layout;
use core::{mem, ptr};
use haz_alloc_internal::UsizeExt;

#[repr(C)]
struct Header {
    r: ReserveHeader,

    // These sizes include the header
    real_size: usize,
    reserve_size: usize,
}

pub unsafe fn alloc(layout: Layout) -> *mut u8 {
    let (total_layout, offset) =
        match Layout::from_size_align_unchecked(mem::size_of::<Header>(), RESERVE_ALIGN)
            .extend(layout)
        {
            Ok(r) => r,
            Err(_) => return ptr::null_mut(),
        };
    let total_size = total_layout.size().align_up(utils::page_size());

    if total_layout.align() > RESERVE_ALIGN {
        return ptr::null_mut();
    }

    let (reserve_size, header) = reserve::new(total_size, ReserveType::Huge);
    if header.is_null() {
        return ptr::null_mut();
    }

    let header = header as *mut Header;
    sys::__haz_alloc_mcommit(header as *mut u8, total_size);

    ptr::addr_of_mut!((*header).real_size).write(total_size);
    ptr::addr_of_mut!((*header).reserve_size).write(reserve_size);

    (header as *mut u8).add(offset)
}

pub unsafe fn realloc_in_place(header: *mut ReserveHeader, ptr: *mut u8, layout: Layout) -> bool {
    let (total_layout, _) =
        match Layout::from_size_align_unchecked(mem::size_of::<Header>(), RESERVE_ALIGN)
            .extend(layout)
        {
            Ok(r) => r,
            Err(_) => return false,
        };
    let total_size = total_layout.size().align_up(utils::page_size());

    let header = header as *mut Header;
    if total_size <= (*header).real_size {
        let decommit = ptr.add(total_size);
        sys::__haz_alloc_mdecommit(decommit, (*header).real_size - total_size);
        (*header).real_size = total_size;
        true
    } else if total_size <= (*header).reserve_size {
        sys::__haz_alloc_mcommit(ptr, total_size);
        (*header).real_size = total_size;
        true
    } else {
        false
    }
}

pub unsafe fn dealloc(header: *mut ReserveHeader) {
    let header = header as *mut Header;
    reserve::delete(ptr::addr_of_mut!((*header).r));
}

pub unsafe fn size(header: *mut ReserveHeader, ptr: *mut u8) -> usize {
    let header = header as *mut Header;
    let total_size = (*header).real_size;
    total_size - (ptr as usize - header as usize)
}
