use haz_alloc_core::backend::TlsCallback;
use std::cell::Cell;
use std::ptr;

thread_local! {
    static ATTACHED: Attached = Attached(Cell::new(ptr::null()));
}

struct Attached(Cell<*const TlsCallback>);

impl Drop for Attached {
    fn drop(&mut self) {
        let mut callback = self.0.get();
        while !callback.is_null() {
            unsafe {
                ((*callback).func)();
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
