use super::Arena;
use crate::__internal::UsizeExt;
use crate::backend::Mutex;
use crate::{bitset, Backend};
use core::alloc::Layout;
use core::cmp::Ordering;
use core::{mem, ptr};

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
pub(super) unsafe fn alloc<B: Backend>(arena: &Arena<B>, layout: Layout) -> *mut u8 {
    let total_size =
        (mem::size_of::<Page>().align_up(layout.align()) + layout.size()).align_up(B::pagesize());
    let pages = total_size / B::pagesize();

    let index = if let Some(x) = bitset::find_zero_run(&*arena.commited(), pages) {
        x
    } else {
        return ptr::null_mut();
    };

    let p = (arena as *const Arena<B> as *const u8).add(index * B::pagesize()) as *mut Page;

    if !B::mcommit(p as _, total_size) {
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
pub(super) unsafe fn realloc_in_place<B: Backend>(
    page: *mut super::Page,
    arena: &Arena<B>,
    layout: Layout,
) -> bool {
    let page = page as *mut Page;
    let total_size =
        (mem::size_of::<Page>().align_up(layout.align()) + layout.size()).align_up(B::pagesize());
    let pages = total_size / B::pagesize();
    let old_pages = (*page).real_size / B::pagesize();

    match pages.cmp(&old_pages) {
        Ordering::Greater => {
            let _ = arena.lock.lock();
            grow_in_place(page, arena, pages, old_pages, total_size)
        }
        Ordering::Equal => true,
        Ordering::Less => {
            let _ = arena.lock.lock();
            shrink_in_place(page, arena, pages, old_pages, total_size)
        }
    }
}

/// Lock must be locked.
unsafe fn grow_in_place<B: Backend>(
    page: *mut Page,
    arena: &Arena<B>,
    pages: usize,
    old_pages: usize,
    total_size: usize,
) -> bool {
    let index =
        (page as usize - arena as *const Arena<B> as usize) / B::pagesize() + old_pages;
    let len = pages - old_pages;

    if !bitset::is_zero_range(&*arena.commited(), index, len) {
        return false;
    }
    if !B::mcommit(
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
unsafe fn shrink_in_place<B: Backend>(
    page: *mut Page,
    arena: &Arena<B>,
    pages: usize,
    old_pages: usize,
    total_size: usize,
) -> bool {
    let index = (page as usize - arena as *const Arena<B> as usize) / B::pagesize() + pages;
    let len = old_pages - pages;

    bitset::clear_range(&mut *arena.commited(), index, len);
    B::mdecommit(
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
pub(super) unsafe fn dealloc<B: Backend>(page: *mut super::Page, arena: *const Arena<B>) {
    let page = page as *mut Page;

    let index = (page as usize - arena as usize) / B::pagesize();
    let len = (*page).real_size / B::pagesize();

    B::mdecommit(page as _, (*page).real_size);
    let guard = (*arena).lock.lock();
    bitset::clear_range(&mut *(*arena).commited(), index, len);
    Arena::release(arena, guard);
}

/// # Safety
///
/// Pointer must be valid.
pub(super) unsafe fn size(page: *const super::Page, ptr: *mut u8) -> usize {
    (*(page as *const Page)).real_size - (ptr as usize - page as usize)
}
