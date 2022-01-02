use crate::sys;
use core::cell::UnsafeCell;
use core::pin::Pin;
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering::*};

macro_rules! thread_local {
    (
        static $x:ident: $xty:ty = $init:expr;
    ) => {
        static $x: $crate::utils::LocalKey<$xty> = unsafe {
            $crate::utils::LocalKey::new(|| {
                use core::cell::{Cell, UnsafeCell};
                use core::mem;
                use core::mem::ManuallyDrop;

                #[thread_local]
                static TLS: (
                    UnsafeCell<ManuallyDrop<$xty>>,
                    Cell<bool>,
                    $crate::sys::TlsCallback,
                ) = (
                    UnsafeCell::new(ManuallyDrop::new($init)),
                    Cell::new(false),
                    $crate::sys::TlsCallback::new(drop),
                );

                fn drop() {
                    unsafe { ManuallyDrop::drop(&mut *TLS.0.get()) }
                }

                if mem::needs_drop::<$xty>() && !TLS.1.get() {
                    TLS.1.set(true);
                    $crate::sys::__haz_alloc_tls_attach(&TLS.2);
                }

                &**TLS.0.get()
            })
        };
    };
}

pub struct LocalKey<T> {
    inner: fn() -> *const T,
}

impl<T> LocalKey<T> {
    #[inline]
    pub const unsafe fn new(f: fn() -> *const T) -> Self {
        Self { inner: f }
    }

    #[inline]
    pub fn with<V>(&self, f: impl FnOnce(&T) -> V) -> V {
        f(unsafe { &*(self.inner)() })
    }
}

pub struct Mutex(UnsafeCell<*mut u8>);

unsafe impl Send for Mutex {}

unsafe impl Sync for Mutex {}

impl Mutex {
    pub const fn new() -> Self {
        Self(UnsafeCell::new(ptr::null_mut()))
    }

    #[inline]
    pub unsafe fn lock(self: Pin<&Self>) {
        sys::__haz_alloc_mutex_lock(self.0.get())
    }

    #[inline]
    pub unsafe fn unlock(self: Pin<&Self>) {
        sys::__haz_alloc_mutex_unlock(self.0.get())
    }
}

pub fn page_size() -> usize {
    static PAGE_SIZE: AtomicUsize = AtomicUsize::new(0);

    match PAGE_SIZE.load(Relaxed) {
        0 => {
            let loaded = unsafe { sys::__haz_alloc_pagesize() };
            PAGE_SIZE.store(loaded, Relaxed);
            loaded
        }
        size => size,
    }
}
