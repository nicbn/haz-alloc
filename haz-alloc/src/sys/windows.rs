use haz_alloc_core::sys::TlsCallback;
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicU32, Ordering::*};
use winapi::shared::minwindef::*;
use winapi::um::fibersapi::*;
use winapi::um::memoryapi::*;
use winapi::um::processthreadsapi::*;
use winapi::um::synchapi::*;
use winapi::um::sysinfoapi::*;
use winapi::um::winnt::*;

struct Mutex(UnsafeCell<SRWLOCK>);

unsafe impl Send for Mutex {}

unsafe impl Sync for Mutex {}

#[no_mangle]
pub unsafe fn __haz_alloc_mcommit(ptr: *mut u8, size: usize) -> bool {
    !VirtualAlloc(ptr as _, size, MEM_RESERVE, PAGE_NOACCESS).is_null()
}

#[no_mangle]
pub unsafe fn __haz_alloc_mreserve(ptr: *mut u8, size: usize) -> *mut u8 {
    VirtualAlloc(ptr as _, size, MEM_RESERVE, PAGE_NOACCESS) as _
}

#[no_mangle]
pub unsafe fn __haz_alloc_mdecommit(ptr: *mut u8, size: usize) {
    VirtualFree(ptr as _, size, MEM_DECOMMIT);
}

#[no_mangle]
pub unsafe fn __haz_alloc_munreserve(ptr: *mut u8) {
    VirtualFree(ptr as _, 0, MEM_RELEASE);
}

#[no_mangle]
pub unsafe fn __haz_alloc_pagesize() -> usize {
    let mut data = MaybeUninit::uninit();
    GetSystemInfo(data.as_mut_ptr());
    data.assume_init().dwPageSize as usize
}

#[no_mangle]
pub unsafe fn __haz_alloc_mutex_lock(mutex: *mut *mut u8) {
    AcquireSRWLockExclusive(mutex as _)
}

#[no_mangle]
pub unsafe fn __haz_alloc_mutex_unlock(mutex: *mut *mut u8) {
    ReleaseSRWLockExclusive(mutex as _)
}

#[no_mangle]
pub unsafe fn __haz_alloc_tls_attach(callback: *const TlsCallback) {
    static KEY: AtomicU32 = AtomicU32::new(TLS_OUT_OF_INDEXES);
    static MUTEX: Mutex = Mutex(UnsafeCell::new(SRWLOCK_INIT));

    extern "system" fn destructor(p: LPVOID) {
        let mut callback = p as *const TlsCallback;
        while !callback.is_null() {
            unsafe {
                ((*callback).func)();
                callback = (*callback).next.get();
            }
        }
    }

    let mut key = KEY.load(Acquire);
    if key == TLS_OUT_OF_INDEXES {
        AcquireSRWLockExclusive(MUTEX.0.get());

        key = KEY.load(Relaxed);
        if key == TLS_OUT_OF_INDEXES {
            key = FlsAlloc(Some(destructor));
            KEY.store(key, Release);
        }

        ReleaseSRWLockExclusive(MUTEX.0.get());
    }

    (*callback).next.set(FlsGetValue(key) as _);
    FlsSetValue(key, callback as _);
}
