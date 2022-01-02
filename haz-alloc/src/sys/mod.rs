cfg_if::cfg_if! {
    if #[cfg(windows)] {
        mod windows;
    } else {
        compile_error!();
    }
}
