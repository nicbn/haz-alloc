use self::mutex::Mutex;
use crate::sys_common;
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};
use haz_alloc_core::backend::TlsCallback;

mod mutex;

pub struct Backend;

unsafe impl haz_alloc_core::Backend for Backend {
    type Mutex = Mutex;

    fn mreserve(ptr: *mut u8, size: usize) -> *mut u8 {
        let ptr = unsafe {
            libc::mmap(
                ptr as _,
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_ANON | libc::MAP_PRIVATE,
                -1,
                0,
            )
        };

        if ptr == libc::MAP_FAILED {
            ptr::null_mut()
        } else {
            ptr as _
        }
    }

    #[inline]
    unsafe fn mcommit(__ptr: *mut u8, __size: usize) -> bool {
        true
    }

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
