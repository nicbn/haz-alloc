use core::cell::UnsafeCell;
use core::ptr::{self, addr_of};

pub struct Mutex {
    mutex: UnsafeCell<libc::pthread_mutex_t>,
}

unsafe impl Send for Mutex {}

unsafe impl Sync for Mutex {}

unsafe impl haz_alloc_core::backend::Mutex for Mutex {
    type Guard<'a> = ();

    #[inline]
    unsafe fn new(ptr: *mut Self) {
        libc::pthread_mutex_init(UnsafeCell::raw_get(addr_of!((*ptr).mutex)), ptr::null_mut());
    }

    #[inline]
    unsafe fn lock(&self) -> Self::Guard<'_> {
        libc::pthread_mutex_lock(self.mutex.get());
    }
}

impl Drop for Mutex {
    #[inline]
    fn drop(&mut self) {
        unsafe { libc::pthread_mutex_destroy(self.mutex.get()) };
    }
}

pub struct MutexGuard<'a>(&'a Mutex);

impl Drop for MutexGuard<'_> {
    #[inline]
    fn drop(&mut self) {
        unsafe { libc::pthread_mutex_unlock(self.0.mutex.get()) };
    }
}
