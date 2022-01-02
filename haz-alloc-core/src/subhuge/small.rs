use haz_alloc_internal::SMALL_CLASSES;
use haz_alloc_internal::small_class_of;

use super::Arena;
use crate::bitset;
use crate::sys;
use crate::utils;
use crate::utils::page_size;
use core::cell::UnsafeCell;
use core::pin::Pin;
use core::ptr;
use core::sync::atomic::{self, AtomicPtr, AtomicUsize, Ordering};


#[repr(C)]
pub(super) struct Page {
    pub p: super::Page,

    pub next: UnsafeCell<*const Page>,
    pub prev: UnsafeCell<*const Page>,

    pub vacancy: AtomicUsize,
    pub rc: AtomicUsize,
    pub free: AtomicPtr<u8>,
    pub zeroed: UnsafeCell<*mut u8>,
}

impl Page {
    /// # Safety
    ///
    /// Pointer must be valid.
    ///
    /// Lock must be locked.
    unsafe fn alloc_from_free(&self, arena: &Arena, class: usize) -> *mut u8 {
        let mut free = self.free.load(Ordering::Acquire);
        while !free.is_null() {
            let next = *(free as *mut *mut u8);
            match self
                .free
                .compare_exchange_weak(free, next, Ordering::Acquire, Ordering::Acquire)
            {
                Ok(_) => {
                    self.increase_rc(arena, class);
                    return free;
                }
                Err(a) => free = a,
            }
        }

        ptr::null_mut()
    }

    /// # Safety
    ///
    /// Pointer must be valid.
    ///
    /// Lock must be locked.
    #[inline]
    unsafe fn alloc_from_zeroed(&self, arena: &Arena, class: usize) -> *mut u8 {
        let next_page = (self as *const Self as *mut u8).add(page_size());
        let ptr = *self.zeroed.get();
        if ptr < next_page {
            *self.zeroed.get() = ptr.add(SMALL_CLASSES[self.p.class as usize]);
            self.increase_rc(arena, class);
            return ptr;
        }

        ptr::null_mut()
    }

    /// # Safety
    ///
    /// Pointer must be valid.
    ///
    /// Lock must be unlocked.
    #[inline]
    unsafe fn reduce_rc(this: *const Page, arena: *const Arena, class: usize) {
        let add_to_vacant = (*this).vacancy.fetch_add(1, Ordering::Release) == 0;
        if !add_to_vacant {
            let mut rc = (*this).rc.load(Ordering::Relaxed);
            while rc != 1 {
                match (*this).rc.compare_exchange_weak(
                    rc,
                    rc - 1,
                    Ordering::Release,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return,
                    Err(a) => rc = a,
                }
            }
        }

        Pin::new_unchecked(&(*arena).lock).lock();

        if (*this).rc.fetch_sub(1, Ordering::Relaxed) == 1 {
            atomic::fence(Ordering::Acquire);

            if !add_to_vacant {
                (*this).remove_from_vacant(&*arena, class);
            }
            let index = ((this as usize) - (arena as usize)) / utils::page_size();
            bitset::clear(&mut *(*arena).commited(), index);
            sys::__haz_alloc_mdecommit(this as _, utils::page_size());

            Arena::release(arena);
        } else {
            if add_to_vacant {
                (*this).add_to_vacant(&*arena, class);
            }

            Pin::new_unchecked(&(*arena).lock).unlock();
        }
    }

    /// # Safety
    ///
    /// Pointer must be valid.
    ///
    /// Lock must be locked.
    unsafe fn increase_rc(&self, arena: &Arena, class: usize) {
        self.rc.fetch_add(1, Ordering::Relaxed);
        if self.vacancy.fetch_sub(1, Ordering::Release) == 1 {
            atomic::fence(Ordering::Acquire);
            self.remove_from_vacant(arena, class);
        }
    }

    /// # Safety
    ///
    /// Pointer must be valid.
    ///
    /// Lock must be locked.
    unsafe fn add_to_vacant(&self, arena: &Arena, class: usize) {
        let next = *(*arena).vacant[class].get();
        *self.next.get() = next;
        if !next.is_null() {
            *(*next).prev.get() = self as *const _;
        }
        *arena.vacant[class].get() = self as *const _;
    }

    /// # Safety
    ///
    /// Pointer must be valid.
    ///
    /// Lock must be locked.
    unsafe fn remove_from_vacant(&self, arena: &Arena, class: usize) {
        let prev = *self.prev.get();
        let next = *self.next.get();
        if !next.is_null() {
            *(*next).prev.get() = prev;
        }
        if !prev.is_null() {
            *(*prev).next.get() = next;
        } else {
            *arena.vacant[class].get() = next;
        }
        *self.prev.get() = ptr::null_mut();
        *self.next.get() = ptr::null_mut();
    }
}

/// # Safety
///
/// Pointer must be valid.
///
/// Lock must be locked.
pub(super) unsafe fn alloc(arena: &Arena, size: usize, zeroed: bool) -> *mut u8 {
    let class = small_class_of(size);

    let page = *arena.vacant[class].get();
    if !page.is_null() {
        if !zeroed {
            let ptr = (*page).alloc_from_free(arena, class);
            if !ptr.is_null() {
                return ptr;
            }
            return (*page).alloc_from_zeroed(arena, class);
        } else {
            let ptr = (*page).alloc_from_zeroed(arena, class);
            if !ptr.is_null() {
                return ptr;
            }
            let ptr = (*page).alloc_from_free(arena, class);
            if !ptr.is_null() {
                ptr.write_bytes(0, size);
            }
            return ptr;
        }
    }

    let index = if let Some(x) = bitset::find_zero(&*arena.commited()) {
        x
    } else {
        return ptr::null_mut();
    };

    let page = (arena as *const Arena as *const u8).add(index * utils::page_size()) as *mut Page;

    if !sys::__haz_alloc_mcommit(page as _, utils::page_size()) {
        return ptr::null_mut();
    }
    bitset::set(&mut *arena.commited(), index);

    *arena.rc.get() += 1;

    (*page).rc = AtomicUsize::new(1);
    (*page).p.class = class as isize;

    let mut ptr = page.add(1) as *mut u8;
    ptr = ptr.add(ptr.align_offset(size.next_power_of_two() / 2));
    let vacancy = (utils::page_size() - (ptr.add(size) as usize - page as usize)) / size;
    (*page).vacancy = AtomicUsize::new(vacancy);
    (*page).free = AtomicPtr::new(ptr.add(size));

    if vacancy > 0 {
        (*page).add_to_vacant(arena, class);
    }

    ptr
}

pub(super) fn realloc_in_place(class: isize, size: usize) -> bool {
    let class = class as usize;
    SMALL_CLASSES[class] >= size
        && class
            .checked_sub(1)
            .map_or(true, |prev| SMALL_CLASSES[prev] < size)
}

/// # Safety
///
/// Pointer must be valid.
///
/// Lock must be unlocked.
pub(super) unsafe fn dealloc(page: *const Page, arena: *const Arena, x: *mut u8, class: isize) {
    let mut next = (*page).free.load(Ordering::Relaxed);
    loop {
        *(x as *mut *mut u8) = next;
        match (*page)
            .free
            .compare_exchange_weak(next, x, Ordering::Release, Ordering::Relaxed)
        {
            Ok(_) => break,
            Err(e) => next = e,
        }
    }

    Page::reduce_rc(page, arena, class as usize)
}
