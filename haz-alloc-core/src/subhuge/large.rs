use super::Arena;
use crate::utils;
use crate::{bitset, sys};
use core::alloc::Layout;
use core::cmp::Ordering;
use core::pin::Pin;
use core::{mem, ptr};
use haz_alloc_internal::UsizeExt;

#[repr(C)]
struct Page {
    p: super::Page,

    // Includes header size
    real_size: usize,
}

/// # Safety
///
/// Pointer must be valid.
///
/// Lock must be locked.
pub(super) unsafe fn alloc(arena: &Arena, layout: Layout) -> *mut u8 {
    let total_size = (mem::size_of::<Page>().align_up(layout.align()) + layout.size())
        .align_up(utils::page_size());
    let pages = total_size / utils::page_size();

    let index = if let Some(x) = bitset::find_zero_run(&*arena.commited(), pages) {
        x
    } else {
        return ptr::null_mut();
    };

    let p = (arena as *const Arena as *const u8).add(index * utils::page_size()) as *mut Page;

    if !sys::__haz_alloc_mcommit(p as _, total_size) {
        return ptr::null_mut();
    }

    bitset::set_range(&mut *arena.commited(), index, pages);
    *arena.rc.get() += 1;

    ptr::addr_of_mut!((*p).p.class).write(-1);
    ptr::addr_of_mut!((*p).real_size).write(total_size);

    let p = p.add(1) as *mut u8;
    p.add(p.align_offset(layout.align()))
}

/// # Safety
///
/// Pointers must be valid.
///
/// Lock must be unlocked.
pub(super) unsafe fn realloc_in_place(
    page: *mut super::Page,
    arena: &Arena,
    layout: Layout,
) -> bool {
    let page = page as *mut Page;
    let total_size = (mem::size_of::<Page>().align_up(layout.align()) + layout.size())
        .align_up(utils::page_size());
    let pages = total_size / utils::page_size();
    let old_pages = (*page).real_size / utils::page_size();

    match pages.cmp(&old_pages) {
        Ordering::Greater => {
            Pin::new_unchecked(&arena.lock).lock();
            let x = grow_in_place(page, arena, pages, old_pages, total_size);
            Pin::new_unchecked(&arena.lock).unlock();
            x
        }
        Ordering::Equal => true,
        Ordering::Less => {
            Pin::new_unchecked(&arena.lock).lock();
            let x = shrink_in_place(page, arena, pages, old_pages, total_size);
            Pin::new_unchecked(&arena.lock).unlock();
            x
        }
    }
}

/// Lock must be locked.
unsafe fn grow_in_place(
    page: *mut Page,
    arena: &Arena,
    pages: usize,
    old_pages: usize,
    total_size: usize,
) -> bool {
    let index = (page as usize - arena as *const Arena as usize) / utils::page_size() + old_pages;
    let len = pages - old_pages;

    if !bitset::is_zero_range(&*arena.commited(), index, len) {
        return false;
    }
    if !sys::__haz_alloc_mcommit(
        (page as *mut u8).add((*page).real_size),
        total_size - (*page).real_size,
    ) {
        return false;
    }
    bitset::set_range(&mut *arena.commited(), index, len);
    (*page).real_size = total_size;

    true
}

/// Lock must be locked.
unsafe fn shrink_in_place(
    page: *mut Page,
    arena: &Arena,
    pages: usize,
    old_pages: usize,
    total_size: usize,
) -> bool {
    let index = (page as usize - arena as *const Arena as usize) / utils::page_size() + pages;
    let len = old_pages - pages;

    bitset::clear_range(&mut *arena.commited(), index, len);
    sys::__haz_alloc_mdecommit(
        (page as *mut u8).add(total_size),
        (*page).real_size - total_size,
    );
    (*page).real_size = total_size;

    true
}

/// # Safety
///
/// Pointers must be valid.
///
/// Lock must be unlocked.
pub(super) unsafe fn dealloc(page: *mut super::Page, arena: *const Arena) {
    let page = page as *mut Page;

    let index = (page as usize - arena as *const Arena as usize) / utils::page_size();
    let len = (*page).real_size / utils::page_size();

    sys::__haz_alloc_mdecommit(page as _, (*page).real_size);
    Pin::new_unchecked(&(*arena).lock).lock();
    bitset::clear_range(&mut *(*arena).commited(), index, len);
    Arena::release(arena);
}

/// # Safety
///
/// Pointer must be valid.
pub(super) unsafe fn size(page: *const super::Page, ptr: *mut u8) -> usize {
    (*(page as *const Page)).real_size - (ptr as usize - page as usize)
}
