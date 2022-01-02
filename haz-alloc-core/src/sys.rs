//! This module contains external functions that are provided by the user
//! in order for the library to perform system actions such as
//! page allocation.

use core::ptr;
use core::cell::Cell;

pub struct TlsCallback {
    pub func: fn(),
    pub next: Cell<*const TlsCallback>,
}

impl TlsCallback {
    pub const fn new(func: fn()) -> Self {
        Self {
            func,
            next: Cell::new(ptr::null()),
        }
    }
}

extern "Rust" {
    /// Commit some memory.
    pub fn __haz_alloc_mcommit(ptr: *mut u8, size: usize) -> bool;
    
    /// Reserve some memory.
    pub fn __haz_alloc_mreserve(ptr: *mut u8, size: usize) -> *mut u8;
    
    /// Decommit some memory.
    pub fn __haz_alloc_mdecommit(ptr: *mut u8, size: usize);

    /// Unreserve some memory.
    pub fn __haz_alloc_munreserve(ptr: *mut u8);

    /// Returns the page size. Does not need to be cached.
    pub fn __haz_alloc_pagesize() -> usize;

    /// Lock the given mutex. Mutexes are zero-initialized.
    pub fn __haz_alloc_mutex_lock(mutex: *mut *mut u8);

    /// Unlock the given mutex.
    pub fn __haz_alloc_mutex_unlock(mutex: *mut *mut u8);

    /// Attach the given callback to this thread, running it when the thread
    /// is destroyed.
    pub fn __haz_alloc_tls_attach(callback: *const TlsCallback);
}
