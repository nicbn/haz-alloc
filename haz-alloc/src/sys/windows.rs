use crate::sys_common::{self, MutexAdapter};
use haz_alloc_core::backend::TlsCallback;
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use winapi::um::memoryapi::*;
use winapi::um::synchapi::*;
use winapi::um::sysinfoapi::*;
use winapi::um::winnt::*;

pub struct Backend;

unsafe impl haz_alloc_core::Backend for Backend {
    type Mutex = MutexAdapter;

    fn mreserve(ptr: *mut u8, size: usize) -> *mut u8 {
        unsafe { VirtualAlloc(ptr as _, size, MEM_RESERVE, PAGE_NOACCESS) as _ }
    }

    #[inline]
    unsafe fn mcommit(ptr: *mut u8, size: usize) -> bool {
        !VirtualAlloc(ptr as _, size, MEM_COMMIT, PAGE_READWRITE).is_null()
    }

    #[inline]
    unsafe fn mdecommit(ptr: *mut u8, size: usize) {
        VirtualFree(ptr as _, size, MEM_DECOMMIT);
    }

    #[inline]
    unsafe fn munreserve(ptr: *mut u8, _: usize) {
        VirtualFree(ptr as _, 0, MEM_RELEASE);
    }

    #[inline]
    fn pagesize() -> usize {
        static PAGESIZE: AtomicU32 = AtomicU32::new(0);

        #[cold]
        fn cold() -> u32 {
            let mut data = MaybeUninit::uninit();
            unsafe { GetSystemInfo(data.as_mut_ptr()) };

            let size = unsafe { data.assume_init().dwPageSize };
            PAGESIZE.store(size, Ordering::Relaxed);

            size
        }

        match PAGESIZE.load(Ordering::Relaxed) {
            0 => cold() as usize,
            pagesize => pagesize as usize,
        }
    }

    unsafe fn tls_attach(callback: *const TlsCallback) {
        sys_common::tls_attach(callback)
    }
}
