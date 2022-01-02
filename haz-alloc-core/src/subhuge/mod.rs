use crate::reserve::{self, ReserveHeader, ReserveType, RESERVE_ALIGN};
use crate::utils::{self, Mutex};
use core::alloc::Layout;
use core::cell::{Cell, UnsafeCell};
use core::pin::Pin;
use core::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use core::{mem, ptr};
use haz_alloc_internal::{UsizeExt, SMALL_CLASSES, SMALL_MAX};

mod large;
mod small;

static ARENAS: [AtomicPtr<Arena>; 16] = [
    AtomicPtr::new(ptr::null_mut()),
    AtomicPtr::new(ptr::null_mut()),
    AtomicPtr::new(ptr::null_mut()),
    AtomicPtr::new(ptr::null_mut()),
    AtomicPtr::new(ptr::null_mut()),
    AtomicPtr::new(ptr::null_mut()),
    AtomicPtr::new(ptr::null_mut()),
    AtomicPtr::new(ptr::null_mut()),
    AtomicPtr::new(ptr::null_mut()),
    AtomicPtr::new(ptr::null_mut()),
    AtomicPtr::new(ptr::null_mut()),
    AtomicPtr::new(ptr::null_mut()),
    AtomicPtr::new(ptr::null_mut()),
    AtomicPtr::new(ptr::null_mut()),
    AtomicPtr::new(ptr::null_mut()),
    AtomicPtr::new(ptr::null_mut()),
];

thread_local! {
    static TLS_ARENA: TlsArena = TlsArena(Cell::new(ptr::null()));
}

pub struct TlsArena(Cell<*const Arena>);
impl Drop for TlsArena {
    fn drop(&mut self) {
        let this = self.0.get() as *mut Arena;
        for x in ARENAS.iter() {
            if x.compare_exchange(ptr::null_mut(), this, Ordering::Release, Ordering::Relaxed)
                .is_ok()
            {
                return;
            }
        }

        unsafe { Arena::release(this) };
    }
}

pub const MAX: usize = 256 * 1024;

#[repr(C)]
struct Page {
    r: ReserveHeader,
    class: isize,
}

#[repr(C)]
struct Arena {
    page: small::Page,

    rc: UnsafeCell<usize>,
    lock: Mutex,
    vacant: [UnsafeCell<*const small::Page>; SMALL_CLASSES.len()],
}
impl Arena {
    #[inline]
    fn commited_len() -> usize {
        RESERVE_ALIGN / utils::page_size() / mem::size_of::<usize>() / 8
    }

    #[inline]
    fn layout() -> (Layout, usize) {
        Layout::new::<Self>()
            .extend(Layout::array::<usize>(Self::commited_len()).unwrap())
            .unwrap()
    }

    fn new() -> *const Arena {
        for x in ARENAS.iter() {
            let x = x.swap(ptr::null_mut(), Ordering::Acquire);
            if !x.is_null() {
                return x;
            }
        }

        let (_, ptr) = reserve::new(RESERVE_ALIGN, ReserveType::SubHuge);
        let ptr = ptr as *mut Arena;
        unsafe {
            (*ptr).page.rc = AtomicUsize::new(1);
            let mut start = (ptr as *mut u8).add(Self::layout().0.size());
            start = start.add(start.align_offset(SMALL_CLASSES[0]));
            (*ptr).page.vacancy = AtomicUsize::new(
                (utils::page_size() - (start as usize - ptr as usize)) / SMALL_CLASSES[0],
            );
            (*ptr).page.zeroed = UnsafeCell::new(start);

            (*ptr).rc = UnsafeCell::new(1);
            ptr::addr_of_mut!((*ptr).lock).write(Mutex::new());
            (*ptr).vacant[0] = UnsafeCell::new(&(*ptr).page);
            (*(*ptr).commited())[0] = 1;
        }

        ptr
    }

    /// # Safety
    ///
    /// Pointer must be valid.
    ///
    /// Lock must be locked.
    #[inline]
    unsafe fn commited(&self) -> *mut [usize] {
        let (_, offset) = Self::layout();
        ptr::slice_from_raw_parts_mut(
            (self as *const Self as *const u8).add(offset) as *mut usize,
            Self::commited_len(),
        )
    }

    /// # Safety
    ///
    /// Pointer must be valid.
    ///
    /// Lock must be locked.
    ///
    /// Lock will be unlocked.
    unsafe fn release(this: *const Self) {
        *(*this).rc.get() -= 1;
        if *(*this).rc.get() == 0 {
            reserve::delete(ptr::addr_of!((*this).page.p.r) as _);
        } else {
            Pin::new_unchecked(&(*this).lock).unlock();
        }
    }

    unsafe fn alloc(&self, layout: Layout, zeroed: bool) -> *mut u8 {
        Pin::new_unchecked(&self.lock).lock();
        let rounded_size = layout.size().align_up(layout.align());
        let x = if rounded_size > SMALL_MAX {
            large::alloc(self, layout)
        } else {
            small::alloc(self, rounded_size, zeroed)
        };
        Pin::new_unchecked(&self.lock).unlock();
        x
    }
}

/// # Safety
///
/// Pointer must be valid.
pub(super) unsafe fn alloc(layout: Layout, zeroed: bool) -> *mut u8 {
    TLS_ARENA.with(|TlsArena(tls_arena)| {
        let mut arena = tls_arena.get();
        if !arena.is_null() {
            let ptr = (*arena).alloc(layout, zeroed);
            if !ptr.is_null() {
                return ptr;
            }
        }

        arena = Arena::new();
        if !arena.is_null() {
            tls_arena.set(arena);
            let ptr = (*arena).alloc(layout, zeroed);
            if !ptr.is_null() {
                return ptr;
            }
        }
        ptr::null_mut()
    })
}

/// # Safety
///
/// Pointer must be valid.
pub(super) unsafe fn realloc_in_place(
    arena: *const ReserveHeader,
    ptr: *mut u8,
    layout: Layout,
) -> bool {
    let arena = arena as *const Arena;
    let page = (ptr as usize - 1).align_down(utils::page_size()) as *mut Page;
    let class = (*page).class;
    let rounded_size = layout.size().align_up(layout.align());

    if class == -1 {
        if rounded_size > SMALL_MAX {
            return large::realloc_in_place(page, &*arena, layout);
        }
    } else if rounded_size <= SMALL_MAX {
        return small::realloc_in_place(class, rounded_size);
    }

    false
}

/// # Safety
///
/// Pointer must be valid.
pub(super) unsafe fn dealloc(arena: *const ReserveHeader, ptr: *mut u8) {
    let arena = arena as *const Arena;
    let page = (ptr as usize - 1).align_down(utils::page_size()) as *mut Page;
    let class = (*page).class;

    if class == -1 {
        large::dealloc(page, arena)
    } else {
        small::dealloc(page as _, arena, ptr, class)
    }
}

/// # Safety
///
/// Pointer must be valid.
pub(super) unsafe fn size(ptr: *mut u8) -> usize {
    let page = (ptr as usize - 1).align_down(utils::page_size()) as *mut Page;
    let class = (*page).class;

    if class == -1 {
        large::size(page, ptr)
    } else {
        SMALL_CLASSES[class as usize]
    }
}
