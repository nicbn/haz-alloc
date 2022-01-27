use crate::sys_common;
use haz_alloc_core::backend::TlsCallback;
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct Backend;

unsafe impl haz_alloc_core::Backend for Backend {
    type Mutex = Mutex;

    const MUTEX_INIT: Mutex = Mutex(UnsafeCell::new(libc::PTHREAD_MUTEX_INITIALIZER));

    fn mreserve(ptr: *mut u8, size: usize) -> *mut u8 {
        unsafe {
            libc::mmap(
                ptr as _,
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_ANON | libc::MAP_PRIVATE,
                -1,
                0,
            ) as _
        }
    }

    #[inline]
    unsafe fn mcommit(__ptr: *mut u8, __size: usize) -> bool {
        true
    }

    #[inline]
    unsafe fn mdecommit(ptr: *mut u8, size: usize) {
        libc::mmap(
            ptr as _,
            size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_ANON | libc::MAP_PRIVATE | libc::MAP_FIXED,
            -1,
            0,
        );
    }

    #[inline]
    unsafe fn munreserve(ptr: *mut u8, size: usize) {
        libc::munmap(ptr as _, size);
    }

    #[inline]
    fn pagesize() -> usize {
        static PAGESIZE: AtomicUsize = AtomicUsize::new(0);

        #[cold]
        fn cold() -> usize {
            let pagesize = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as _;
            PAGESIZE.store(pagesize, Ordering::Relaxed);
            pagesize
        }

        match PAGESIZE.load(Ordering::Relaxed) {
            0 => cold(),
            pagesize => pagesize,
        }
    }

    unsafe fn tls_attach(callback: *const TlsCallback) {
        sys_common::tls_attach(callback)
    }
}

pub struct Mutex(UnsafeCell<libc::pthread_mutex_t>);

unsafe impl Send for Mutex {}

unsafe impl Sync for Mutex {}

unsafe impl haz_alloc_core::backend::RawMutex for Mutex {
    #[inline]
    unsafe fn lock(&self) {
        libc::pthread_mutex_lock(self.0.get());
    }

    #[inline]
    unsafe fn unlock(&self) {
        libc::pthread_mutex_unlock(self.0.get());
    }
}
