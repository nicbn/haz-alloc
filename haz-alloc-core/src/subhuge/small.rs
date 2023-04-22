use super::Arena;
use crate::__internal::{small_class_of, SMALL_CLASSES};
use crate::backend::RawMutex;
use crate::bitset;
use crate::Backend;
use core::cell::UnsafeCell;
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
    unsafe fn alloc_from_free<B: Backend>(&self, arena: &Arena<B::Mutex>, class: usize) -> *mut u8 {
        let mut free = self.free.load(Ordering::Acquire);
        while !free.is_null() {
            let next = *(free as *mut *mut u8);
            match self
                .free
                .compare_exchange_weak(free, next, Ordering::Acquire, Ordering::Acquire)
            {
                Ok(_) => {
                    self.increase_rc::<B>(arena, class);
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
    unsafe fn alloc_from_zeroed<B: Backend>(
        &self,
        arena: &Arena<B::Mutex>,
        class: usize,
    ) -> *mut u8 {
        let next_page = (self as *const Self as *mut u8).add(B::pagesize());
        let ptr = *self.zeroed.get();
        if ptr < next_page {
            *self.zeroed.get() = ptr.add(SMALL_CLASSES[self.p.class as usize]);
            self.increase_rc::<B>(arena, class);
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
    unsafe fn reduce_rc<B: Backend>(
        this: *const Page,
        arena: *const Arena<B::Mutex>,
        class: usize,
    ) {
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

        (*arena).lock.lock();

        if (*this).rc.fetch_sub(1, Ordering::Relaxed) == 1 {
            atomic::fence(Ordering::Acquire);

            if !add_to_vacant {
                (*this).remove_from_vacant::<B>(&*arena, class);
            }
            let index = ((this as usize) - (arena as usize)) / B::pagesize();
            bitset::clear(&mut *(*arena).commited::<B>(), index);
            B::mdecommit(this as _, B::pagesize());

            Arena::release(arena);
        } else {
            if add_to_vacant {
                (*this).add_to_vacant::<B>(&*arena, class);
            }

            (*arena).lock.unlock();
        }
    }

    /// # Safety
    ///
    /// Pointer must be valid.
    ///
    /// Lock must be locked.
    unsafe fn increase_rc<B: Backend>(&self, arena: &Arena<B::Mutex>, class: usize) {
        self.rc.fetch_add(1, Ordering::Relaxed);
        if self.vacancy.fetch_sub(1, Ordering::Release) == 1 {
            atomic::fence(Ordering::Acquire);
            self.remove_from_vacant::<B>(arena, class);
        }
    }

    /// # Safety
    ///
    /// Pointer must be valid.
    ///
    /// Lock must be locked.
    unsafe fn add_to_vacant<B: Backend>(&self, arena: &Arena<B::Mutex>, class: usize) {
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
    unsafe fn remove_from_vacant<B: Backend>(&self, arena: &Arena<B::Mutex>, class: usize) {
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
pub(super) unsafe fn alloc<B: Backend>(
    arena: &Arena<B::Mutex>,
    size: usize,
    zeroed: bool,
) -> *mut u8 {
    let class = small_class_of(size);

    let page = *arena.vacant[class].get();
    if !page.is_null() {
        if !zeroed {
            let ptr = (*page).alloc_from_free::<B>(arena, class);
            if !ptr.is_null() {
                return ptr;
            }
            return (*page).alloc_from_zeroed::<B>(arena, class);
        } else {
            let ptr = (*page).alloc_from_zeroed::<B>(arena, class);
            if !ptr.is_null() {
                return ptr;
            }
            let ptr = (*page).alloc_from_free::<B>(arena, class);
            if !ptr.is_null() {
                ptr.write_bytes(0, size);
            }
            return ptr;
        }
    }

    let index = if let Some(x) = bitset::find_zero(&*arena.commited::<B>()) {
        x
    } else {
        return ptr::null_mut();
    };

    let pagesize = B::pagesize();

    let page = (arena as *const Arena<B::Mutex> as *const u8).add(index * pagesize) as *mut Page;

    if !B::mcommit(page as _, pagesize) {
        return ptr::null_mut();
    }
    bitset::set(&mut *arena.commited::<B>(), index);

    *arena.rc.get() += 1;

    (*page).rc = AtomicUsize::new(1);
    (*page).p.class = class as isize;

    let mut ptr = page.add(1) as *mut u8;
    ptr = ptr.add(ptr.align_offset(size.next_power_of_two() / 2));
    let vacancy = (pagesize - (ptr.add(size) as usize - page as usize)) / size;
    (*page).vacancy = AtomicUsize::new(vacancy);
    (*page).free = AtomicPtr::new(ptr.add(size));

    if vacancy > 0 {
        (*page).add_to_vacant::<B>(arena, class);
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
pub(super) unsafe fn dealloc<B: Backend>(
    page: *const Page,
    arena: *const Arena<B::Mutex>,
    x: *mut u8,
    class: isize,
) {
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

    Page::reduce_rc::<B>(page, arena, class as usize)
}
