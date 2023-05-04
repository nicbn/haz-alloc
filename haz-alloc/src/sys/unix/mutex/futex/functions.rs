//! Ripped off from libstd implementation
//!
//! https://github.com/rust-lang/rust/blob/master/library/std/src/sys/unix/futex.rs

use core::sync::atomic::AtomicU32;

#[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
pub fn futex_wait(futex: &AtomicU32, expected: u32) -> bool {
    use core::ptr;
    use core::sync::atomic::Ordering;
    use errno::errno;

    loop {
        if futex.load(Ordering::Relaxed) != expected {
            return true;
        }

        let r = unsafe {
            cfg_if::cfg_if! {
                if #[cfg(target_os = "freebsd")] {
                    let umtx_timeout = timespec.map(|t| libc::_umtx_time {
                        _timeout: t,
                        _flags: libc::UMTX_ABSTIME,
                        _clockid: libc::CLOCK_MONOTONIC as u32,
                    });
                    let umtx_timeout_ptr = umtx_timeout.as_ref().map_or(null(), |t| t as *const _);
                    let umtx_timeout_size = umtx_timeout.as_ref().map_or(0, |t| crate::mem::size_of_val(t));
                    libc::_umtx_op(
                        futex as *const AtomicU32 as *mut _,
                        libc::UMTX_OP_WAIT_UINT_PRIVATE,
                        expected as libc::c_ulong,
                        crate::ptr::invalid_mut(umtx_timeout_size),
                        umtx_timeout_ptr as *mut _,
                    )
                } else if #[cfg(any(target_os = "linux", target_os = "android"))] {
                    libc::syscall(
                        libc::SYS_futex,
                        futex as *const AtomicU32,
                        libc::FUTEX_WAIT_BITSET | libc::FUTEX_PRIVATE_FLAG,
                        expected,
                        ptr::null::<libc::timespec>(),
                        ptr::null::<u32>(), // This argument is unused for FUTEX_WAIT_BITSET.
                        !0u32,              // A full bitmask, to make it behave like a regular FUTEX_WAIT.
                    )
                } else {
                    compile_error!("unknown target_os");
                }
            }
        };

        match (r < 0).then(|| errno().0) {
            Some(libc::ETIMEDOUT) => return false,
            Some(libc::EINTR) => continue,
            _ => return true,
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn futex_wake(futex: &AtomicU32) -> bool {
    let ptr = futex as *const AtomicU32;
    let op = libc::FUTEX_WAKE | libc::FUTEX_PRIVATE_FLAG;
    unsafe { libc::syscall(libc::SYS_futex, ptr, op, 1) > 0 }
}

#[cfg(target_os = "freebsd")]
pub fn futex_wake(futex: &AtomicU32) -> bool {
    use core::ptr::null_mut;
    unsafe {
        libc::_umtx_op(
            futex as *const AtomicU32 as *mut _,
            libc::UMTX_OP_WAKE_PRIVATE,
            1,
            null_mut(),
            null_mut(),
        )
    };
    false
}

#[cfg(target_os = "openbsd")]
pub fn futex_wait(futex: &AtomicU32, expected: u32) -> bool {
    use core::ptr::{null, null_mut};
    use errno::errno;

    let r = unsafe {
        libc::futex(
            futex as *const AtomicU32 as *mut u32,
            libc::FUTEX_WAIT,
            expected as i32,
            null(),
            null_mut(),
        )
    };

    r == 0 || errno().0 != libc::ETIMEDOUT
}

#[cfg(target_os = "openbsd")]
pub fn futex_wake(futex: &AtomicU32) -> bool {
    use core::ptr::{null, null_mut};
    unsafe {
        libc::futex(
            futex as *const AtomicU32 as *mut u32,
            libc::FUTEX_WAKE,
            1,
            null(),
            null_mut(),
        ) > 0
    }
}

#[cfg(target_os = "dragonfly")]
pub fn futex_wait(futex: &AtomicU32, expected: u32) -> bool {
    use errno::errno;

    let r =
        unsafe { libc::umtx_sleep(futex as *const AtomicU32 as *const i32, expected as i32, 0) };

    r == 0 || errno().0 != libc::ETIMEDOUT
}

#[cfg(target_os = "dragonfly")]
pub fn futex_wake(futex: &AtomicU32) -> bool {
    unsafe { libc::umtx_wakeup(futex as *const AtomicU32 as *const i32, 1) };
    false
}

#[cfg(target_os = "emscripten")]
extern "C" {
    fn emscripten_futex_wake(addr: *const AtomicU32, count: libc::c_int) -> libc::c_int;
    fn emscripten_futex_wait(
        addr: *const AtomicU32,
        val: libc::c_uint,
        max_wait_ms: libc::c_double,
    ) -> libc::c_int;
}

#[cfg(target_os = "emscripten")]
pub fn futex_wait(futex: &AtomicU32, expected: u32) -> bool {
    unsafe { emscripten_futex_wait(futex, expected, f64::INFINITY) != -libc::ETIMEDOUT }
}

#[cfg(target_os = "emscripten")]
pub fn futex_wake(futex: &AtomicU32) -> bool {
    unsafe { emscripten_futex_wake(futex, 1) > 0 }
}

#[cfg(target_os = "fuchsia")]
pub mod zircon {
    pub type zx_futex_t = core::sync::atomic::AtomicU32;
    pub type zx_handle_t = u32;
    pub type zx_status_t = i32;
    pub type zx_time_t = i64;

    pub const ZX_HANDLE_INVALID: zx_handle_t = 0;

    pub const ZX_TIME_INFINITE: zx_time_t = zx_time_t::MAX;

    pub const ZX_OK: zx_status_t = 0;
    pub const ZX_ERR_INVALID_ARGS: zx_status_t = -10;
    pub const ZX_ERR_BAD_HANDLE: zx_status_t = -11;
    pub const ZX_ERR_WRONG_TYPE: zx_status_t = -12;
    pub const ZX_ERR_BAD_STATE: zx_status_t = -20;
    pub const ZX_ERR_TIMED_OUT: zx_status_t = -21;

    extern "C" {
        pub fn zx_futex_wait(
            value_ptr: *const zx_futex_t,
            current_value: zx_futex_t,
            new_futex_owner: zx_handle_t,
            deadline: zx_time_t,
        ) -> zx_status_t;
        pub fn zx_futex_wake(value_ptr: *const zx_futex_t, wake_count: u32) -> zx_status_t;
    }
}

#[cfg(target_os = "fuchsia")]
pub fn futex_wait(futex: &AtomicU32, expected: u32) -> bool {
    unsafe {
        zircon::zx_futex_wait(
            futex,
            AtomicU32::new(expected),
            zircon::ZX_HANDLE_INVALID,
            zircon::ZX_TIME_INFINITE,
        ) != zircon::ZX_ERR_TIMED_OUT
    }
}

#[cfg(target_os = "fuchsia")]
pub fn futex_wake(futex: &AtomicU32) -> bool {
    unsafe { zircon::zx_futex_wake(futex, 1) };
    false
}
