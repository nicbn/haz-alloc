use core::cell::Cell;
use core::ptr;

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

/// This trait contains external functions that are provided by the user
/// in order for the library to perform system actions such as
/// page allocation.
///
/// # Safety
///
/// The implementation must make sure the functions in the trait behave
/// properly.
pub unsafe trait Backend {
    type Mutex: RawMutex;
    const MUTEX_INIT: Self::Mutex;

    /// Reserve the block of memory starting at `ptr` if `ptr` is not null and
    /// with `size`.
    ///
    /// If `ptr` is null, the block of memory can start at an offset determined
    /// by the system.
    ///
    /// If the function fails null is returned.
    fn mreserve(ptr: *mut u8, size: usize) -> *mut u8;

    /// Commit memory starting at `ptr` with size `size`.
    ///
    /// If the function fails null is returned.
    ///
    /// # Safety
    ///
    /// The memory must be reserved.
    unsafe fn mcommit(ptr: *mut u8, size: usize) -> bool;

    /// Decommit memory starting at `ptr` with size `size`.
    ///
    /// # Safety
    ///
    /// The memory must be commited.
    unsafe fn mdecommit(ptr: *mut u8, size: usize);

    /// Unreserve memory starting at `ptr` with size `size`.
    ///
    /// # Safety
    ///
    /// The memory must be reserved.
    ///
    /// The size must be equals to the same size used for reserving.
    unsafe fn munreserve(ptr: *mut u8, size: usize);

    /// Returns the page size.
    ///
    /// It is a good idea to cache before returning.
    fn pagesize() -> usize;

    /// Attach the given callback to this thread, running it when the thread
    /// is destroyed.
    ///
    /// # Safety
    ///
    /// The callback must be #[thread_local].
    unsafe fn tls_attach(callback: *const TlsCallback);
}

/// # Safety
///
/// The implementation must make sure the functions in the trait behave
/// properly.
pub unsafe trait RawMutex: 'static + Sync + Send {
    /// Lock a mutex.
    ///
    /// # Safety
    ///
    /// Mutex must not be moved.
    unsafe fn lock(&self);

    /// Unlock a mutex.
    ///
    /// # Safety
    ///
    /// Mutex must not be moved.
    ///
    /// Mutex must be locked.
    unsafe fn unlock(&self);
}
