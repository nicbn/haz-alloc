use self::functions::{futex_wait, futex_wake};
use core::hint;
use core::sync::atomic::{AtomicU32, Ordering};

mod functions;

/// Ripped off from libstd implementation
///
/// https://github.com/rust-lang/rust/blob/master/library/std/src/sys/unix/locks/futex_mutex.rs
pub struct Mutex {
    /// 0: unlocked
    /// 1: locked, no other threads waiting
    /// 2: locked, and other threads waiting (contended)
    futex: AtomicU32,
}

impl Mutex {
    #[inline]
    fn raw_lock(&self) {
        if self
            .futex
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            self.raw_lock_contended();
        }
    }

    #[cold]
    fn raw_lock_contended(&self) {
        let mut state = self.spin();

        if state == 0 {
            match self
                .futex
                .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            {
                Ok(_) => return, // Locked!
                Err(s) => state = s,
            }
        }

        loop {
            if state != 2 && self.futex.swap(2, Ordering::Acquire) == 0 {
                return;
            }
            futex_wait(&self.futex, 2);
            state = self.spin();
        }
    }

    fn spin(&self) -> u32 {
        let mut spin = 100;
        loop {
            let state = self.futex.load(Ordering::Relaxed);

            if state != 1 || spin == 0 {
                return state;
            }

            hint::spin_loop();
            spin -= 1;
        }
    }

    #[inline]
    unsafe fn raw_unlock(&self) {
        if self.futex.swap(0, Ordering::Release) == 2 {
            // We only wake up one thread. When that thread locks the mutex, it
            // will mark the mutex as contended (2) (see lock_contended above),
            // which makes sure that any other waiting threads will also be
            // woken up eventually.
            self.wake();
        }
    }

    #[cold]
    fn wake(&self) {
        futex_wake(&self.futex);
    }
}

unsafe impl haz_alloc_core::backend::Mutex for Mutex {
    type Guard<'a> = Guard<'a>;

    #[inline]
    unsafe fn new(ptr: *mut Self) {
        ptr.write(Self {
            futex: AtomicU32::new(0),
        });
    }

    #[inline]
    unsafe fn lock(&self) -> Self::Guard<'_> {
        self.raw_lock();
        Guard(self)
    }
}

pub struct Guard<'a>(&'a Mutex);

impl Drop for Guard<'_> {
    #[inline]
    fn drop(&mut self) {
        unsafe { self.0.raw_unlock() };
    }
}
