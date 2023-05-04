use crate::sys_common;
use haz_alloc_core::backend::TlsCallback;
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use winapi::um::memoryapi::*;
use winapi::um::synchapi::*;
use winapi::um::sysinfoapi::*;
use winapi::um::winnt::*;

pub struct Mutex(UnsafeCell<SRWLOCK>);

unsafe impl Send for Mutex {}

unsafe impl Sync for Mutex {}

unsafe impl haz_alloc_core::backend::Mutex for Mutex {
    type Guard<'a> = MutexGuard<'a>;

    #[inline]
    unsafe fn new(ptr: *mut Self) {
        ptr.write(Self(UnsafeCell::new(SRWLOCK_INIT)));
    }

    #[inline]
    unsafe fn lock(&self) -> Self::Guard<'_> {
        AcquireSRWLockExclusive(self.0.get());
        MutexGuard(self)
    }
}

pub struct MutexGuard<'a>(&'a Mutex);

impl Drop for MutexGuard<'_> {
    #[inline]
    fn drop(&mut self) {
        unsafe { ReleaseSRWLockExclusive(self.0 .0.get()) };
    }
}

pub struct Backend;

unsafe impl haz_alloc_core::Backend for Backend {
    type Mutex = Mutex;

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
