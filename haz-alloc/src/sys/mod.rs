cfg_if::cfg_if! {
    if #[cfg(windows)] {
        mod windows;
        pub use self::windows::*;
    } else if #[cfg(unix)] {
        mod unix;
        pub use self::unix::*;
    } else {
        compile_error!();
    }
}
