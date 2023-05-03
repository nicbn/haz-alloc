use crate::backend::{Mutex, TlsCallback};
use crate::reserve::{self, ReserveHeader, ReserveType, RESERVE_ALIGN};
use crate::Backend;
use crate::__internal::{UsizeExt, SMALL_CLASSES, SMALL_MAX};
use core::alloc::Layout;
use core::cell::{Cell, UnsafeCell};
use core::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use core::{mem, ptr};

mod large;
mod small;

static ARENAS: [AtomicPtr<()>; 16] = [
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

struct ArenaCell(Cell<*mut ()>);

impl ArenaCell {
    const fn new() -> Self {
        Self(Cell::new(ptr::null_mut()))
    }

    #[inline]
    fn get<B: Backend>(&self) -> *mut Arena<B> {
        self.0.get() as *mut Arena<B>
    }

    #[inline]
    fn set<B: Backend>(&self, ptr: *mut Arena<B>) {
        self.0.set(ptr as *mut ())
    }
}

#[inline]
fn tls_arena<B: Backend, T, F>(with: F) -> T
where
    F: FnOnce(&ArenaCell) -> T,
{
    #[thread_local]
    static TLS_ARENA: ArenaCell = ArenaCell::new();
    #[thread_local]
    static CALLBACK: TlsCallback = TlsCallback::new();
    
    #[cold]
    fn slow<B: Backend>() {
        CALLBACK.func.set(Some(|| {
            let arena = TLS_ARENA.get::<B>();
            if !arena.is_null() {
                for x in ARENAS.iter() {
                    if x.compare_exchange(
                        ptr::null_mut(),
                        arena as *mut (),
                        Ordering::Release,
                        Ordering::Relaxed,
                    )
                    .is_ok()
                    {
                        return;
                    }
                }
        
                let guard = unsafe { (*arena).lock.lock() };
                unsafe { Arena::release(arena, guard) };
            }
        }));

        unsafe { B::tls_attach(&CALLBACK) };
    }

    if CALLBACK.func.get().is_none() {
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
struct Arena<B: Backend> {
    page: small::Page,

    rc: UnsafeCell<usize>,
    vacant: [UnsafeCell<*const small::Page>; SMALL_CLASSES.len()],
    lock: B::Mutex,
}

impl<B: Backend> Arena<B> {
    /// # Safety
    ///
    /// Pointer must be valid.
    unsafe fn release(this: *const Self, guard: <B::Mutex as Mutex>::Guard<'_>) {
        *(*this).rc.get() -= 1;
        if *(*this).rc.get() == 0 {
            drop(guard);
            reserve::delete::<B>(ptr::addr_of!((*this).page.p.r) as _);
        }
    }
}

impl<B: Backend> Arena<B> {
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

    unsafe fn alloc(&self, layout: Layout, zeroed: bool) -> *mut u8 {
        let guard = self.lock.lock();
        let rounded_size = layout.size().align_up(layout.align());
        let x = if rounded_size > SMALL_MAX {
            large::alloc(self, layout)
        } else {
            small::alloc(self, rounded_size, zeroed)
        };
        drop(guard);
        x
    }

    #[inline]
    fn commited_len() -> usize {
        RESERVE_ALIGN / B::pagesize() / mem::size_of::<usize>() / 8
    }

    #[inline]
    fn layout() -> (Layout, usize) {
        Layout::new::<Self>()
            .extend(Layout::array::<usize>(Self::commited_len()).unwrap())
            .unwrap()
    }

    fn new() -> *mut Self {
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
            let mut start = (ptr as *mut u8).add(Self::layout().0.size());
            start = start.add(start.align_offset(SMALL_CLASSES[0]));
            (*ptr).page.vacancy = AtomicUsize::new(
                (B::pagesize() - (start as usize - ptr as usize)) / SMALL_CLASSES[0],
            );
            (*ptr).page.zeroed = UnsafeCell::new(start);

            (*ptr).rc = UnsafeCell::new(1);
            ptr::addr_of_mut!((*ptr).lock).write(B::Mutex::INIT);
            (*ptr).vacant[0] = UnsafeCell::new(&(*ptr).page);
            (*(*ptr).commited())[0] = 1;
        }

        ptr
    }
}

/// # Safety
///
/// Pointer must be valid.
pub(super) unsafe fn alloc<B: Backend>(layout: Layout, zeroed: bool) -> *mut u8 {
    tls_arena::<B, _, _>(|tls_arena| {
        let x = tls_arena.get::<B>();
        if !x.is_null() {
            let ptr = (*x).alloc(layout, zeroed);
            if !ptr.is_null() {
                return ptr;
            }
        }

        let arena = Arena::<B>::new();
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
pub(super) unsafe fn realloc_in_place<B: Backend>(
    arena: *const ReserveHeader,
    ptr: *mut u8,
    layout: Layout,
) -> bool {
    let arena = arena as *const Arena<B>;
    let page = (ptr as usize - 1).align_down(B::pagesize()) as *mut Page;
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
pub(super) unsafe fn dealloc<B: Backend>(arena: *const ReserveHeader, ptr: *mut u8) {
    let arena = arena as *const Arena<B>;
    let page = (ptr as usize - 1).align_down(B::pagesize()) as *mut Page;
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
pub(super) unsafe fn size<B: Backend>(ptr: *mut u8) -> usize {
    let page = (ptr as usize - 1).align_down(B::pagesize()) as *mut Page;
    let class = (*page).class;

    if class == -1 {
        large::size(page, ptr)
    } else {
        SMALL_CLASSES[class as usize]
    }
}
