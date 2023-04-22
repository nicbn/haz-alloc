use haz_alloc_core::backend::TlsCallback;
use std::cell::Cell;
use std::ptr;
use std::sync::{Mutex, MutexGuard};

pub struct MutexAdapter(pub Mutex<()>);

unsafe impl haz_alloc_core::backend::Mutex for MutexAdapter {
    type Guard<'a> = MutexGuard<'a, ()>;

    #[allow(clippy::declare_interior_mutable_const)]
    const INIT: Self = MutexAdapter(Mutex::new(()));

    #[inline]
    fn lock(&self) -> Self::Guard<'_> {
        match self.0.lock() {
            Ok(x) => x,
            Err(x) => x.into_inner(),
        }
    }
}

thread_local! {
    static ATTACHED: Attached = Attached(Cell::new(ptr::null()));
}

struct Attached(Cell<*const TlsCallback>);

impl Drop for Attached {
    fn drop(&mut self) {
        let mut callback = self.0.get();
        while !callback.is_null() {
            unsafe {
                if let Some(func) = (*callback).func.get() {
                    func();
                }

                callback = (*callback).next.get();
            }
        }
    }
}

pub unsafe fn tls_attach(callback: *const TlsCallback) {
    ATTACHED.with(|attached| {
        attached.0.set(callback);
    });
}
