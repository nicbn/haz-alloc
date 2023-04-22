use crate::backend::{RawMutex, TlsCallback};
use crate::reserve::{self, ReserveHeader, ReserveType, RESERVE_ALIGN};
use crate::Backend;
use crate::__internal::{UsizeExt, SMALL_CLASSES, SMALL_MAX};
use core::alloc::Layout;
use core::cell::{Cell, UnsafeCell};
use core::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use core::{mem, ptr};

mod large;
mod small;

static ARENAS: [AtomicPtr<u8>; 16] = [
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

#[inline]
fn tls_arena<B: Backend, T, F>(with: F) -> T
where
    F: FnOnce(&Cell<Option<ptr::NonNull<Arena<dyn RawMutex>>>>) -> T,
{
    #[thread_local]
    static TLS_ARENA: Cell<Option<ptr::NonNull<Arena<dyn RawMutex>>>> = Cell::new(None);
    #[thread_local]
    static INIT: Cell<bool> = Cell::new(false);
    #[thread_local]
    static CALLBACK: TlsCallback = TlsCallback::new(|| {
        if let Some(ptr) = TLS_ARENA.get() {
            for x in ARENAS.iter() {
                if x.compare_exchange(
                    ptr::null_mut(),
                    ptr.as_ptr() as _,
                    Ordering::Release,
                    Ordering::Relaxed,
                )
                .is_ok()
                {
                    return;
                }
            }

            unsafe { Arena::release(ptr.as_ptr()) };
        }
    });

    #[cold]
    fn slow<B: Backend>() {
        INIT.set(true);

        unsafe { B::tls_attach(&CALLBACK) };
    }

    if !INIT.get() {
        slow::<B>();
    }

    with(&TLS_ARENA)
}

pub const MAX: usize = 256 * 1024;

#[repr(C)]
struct Page {
    r: ReserveHeader,
    class: isize,
}

#[repr(C)]
struct Arena<M: ?Sized> {
    page: small::Page,

    munreserve: unsafe fn(ptr: *mut u8, size: usize),
    rc: UnsafeCell<usize>,
    vacant: [UnsafeCell<*const small::Page>; SMALL_CLASSES.len()],
    lock: M,
}

impl<M: RawMutex + ?Sized> Arena<M> {
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
            reserve::delete((*this).munreserve, ptr::addr_of!((*this).page.p.r) as _);
        } else {
            (*this).lock.unlock();
        }
    }
}

impl<M: RawMutex> Arena<M> {
    /// # Safety
    ///
    /// Pointer must be valid.
    ///
    /// Lock must be locked.
    #[inline]
    unsafe fn commited<B: Backend<Mutex = M>>(&self) -> *mut [usize] {
        let (_, offset) = Self::layout::<B>();
        ptr::slice_from_raw_parts_mut(
            (self as *const Self as *const u8).add(offset) as *mut usize,
            Self::commited_len::<B>(),
        )
    }

    unsafe fn alloc<B: Backend<Mutex = M>>(&self, layout: Layout, zeroed: bool) -> *mut u8 {
        self.lock.lock();
        let rounded_size = layout.size().align_up(layout.align());
        let x = if rounded_size > SMALL_MAX {
            large::alloc::<B>(self, layout)
        } else {
            small::alloc::<B>(self, rounded_size, zeroed)
        };
        self.lock.unlock();
        x
    }

    #[inline]
    fn commited_len<B: Backend<Mutex = M>>() -> usize {
        RESERVE_ALIGN / B::pagesize() / mem::size_of::<usize>() / 8
    }

    #[inline]
    fn layout<B: Backend<Mutex = M>>() -> (Layout, usize) {
        Layout::new::<Self>()
            .extend(Layout::array::<usize>(Self::commited_len::<B>()).unwrap())
            .unwrap()
    }

    fn new<B: Backend<Mutex = M>>() -> *mut Self {
        for x in ARENAS.iter() {
            let x = x.swap(ptr::null_mut(), Ordering::Acquire);
            if !x.is_null() {
                return x as _;
            }
        }

        let (_, ptr) = reserve::new::<B>(RESERVE_ALIGN, ReserveType::SubHuge);
        let ptr = ptr as *mut Self;
        unsafe {
            (*ptr).page.rc = AtomicUsize::new(1);
            let mut start = (ptr as *mut u8).add(Self::layout::<B>().0.size());
            start = start.add(start.align_offset(SMALL_CLASSES[0]));
            (*ptr).page.vacancy = AtomicUsize::new(
                (B::pagesize() - (start as usize - ptr as usize)) / SMALL_CLASSES[0],
            );
            (*ptr).page.zeroed = UnsafeCell::new(start);

            (*ptr).munreserve = B::munreserve;

            (*ptr).rc = UnsafeCell::new(1);
            ptr::addr_of_mut!((*ptr).lock).write(B::MUTEX_INIT);
            (*ptr).vacant[0] = UnsafeCell::new(&(*ptr).page);
            (*(*ptr).commited::<B>())[0] = 1;
        }

        ptr
    }
}

/// # Safety
///
/// Pointer must be valid.
pub(super) unsafe fn alloc<B: Backend>(layout: Layout, zeroed: bool) -> *mut u8 {
    tls_arena::<B, _, _>(|tls_arena| {
        if let Some(x) = tls_arena.get() {
            let ptr = (*(x.as_ptr() as *mut Arena<B::Mutex>)).alloc::<B>(layout, zeroed);
            if !ptr.is_null() {
                return ptr;
            }
        }

        let arena = Arena::new::<B>();
        if !arena.is_null() {
            tls_arena.set(ptr::NonNull::new(arena as _));
            let ptr = (*arena).alloc::<B>(layout, zeroed);
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
pub(super) unsafe fn realloc_in_place<B: Backend>(
    arena: *const ReserveHeader,
    ptr: *mut u8,
    layout: Layout,
) -> bool {
    let arena = arena as *const Arena<B::Mutex>;
    let page = (ptr as usize - 1).align_down(B::pagesize()) as *mut Page;
    let class = (*page).class;
    let rounded_size = layout.size().align_up(layout.align());

    if class == -1 {
        if rounded_size > SMALL_MAX {
            return large::realloc_in_place::<B>(page, &*arena, layout);
        }
    } else if rounded_size <= SMALL_MAX {
        return small::realloc_in_place(class, rounded_size);
    }

    false
}

/// # Safety
///
/// Pointer must be valid.
pub(super) unsafe fn dealloc<B: Backend>(arena: *const ReserveHeader, ptr: *mut u8) {
    let arena = arena as *const Arena<B::Mutex>;
    let page = (ptr as usize - 1).align_down(B::pagesize()) as *mut Page;
    let class = (*page).class;

    if class == -1 {
        large::dealloc::<B>(page, arena)
    } else {
        small::dealloc::<B>(page as _, arena, ptr, class)
    }
}

/// # Safety
///
/// Pointer must be valid.
pub(super) unsafe fn size<B: Backend>(ptr: *mut u8) -> usize {
    let page = (ptr as usize - 1).align_down(B::pagesize()) as *mut Page;
    let class = (*page).class;

    if class == -1 {
        large::size(page, ptr)
    } else {
        SMALL_CLASSES[class as usize]
    }
}
