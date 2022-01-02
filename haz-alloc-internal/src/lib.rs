#![no_std]

#[cfg(test)]
mod tests;

use core::mem;

pub const SMALL_CLASSES: &[usize] = &[
    mem::size_of::<usize>(),
    8,
    // Incremented by 16:
    16,
    16 * 2,
    16 * 3,
    16 * 4,
    16 * 5,
    16 * 6,
    // Incremented by 64:
    64 * 2,
    64 * 3,
    64 * 4,
    64 * 5,
    64 * 6,
    64 * 7,
    64 * 8,
    // Incremented by 256:
    256 * 3,
    256 * 4,
    256 * 5,
    256 * 6,
    256 * 7,
];

pub const SMALL_MAX: usize = 256 * 7;

pub fn small_class_of(size: usize) -> usize {
    if size <= mem::size_of::<usize>() {
        0
    } else if size <= 8 {
        1
    } else if size <= 16 * 6 {
        2 + size.align_up(16) / 16 - 1
    } else if size <= 64 * 8 {
        8 + size.align_up(64) / 64 - 2
    } else {
        15 + size.align_up(256) / 256 - 3
    }
}

pub trait UsizeExt {
    fn align_up(self, multiple: usize) -> Self;
    fn align_down(self, multiple: usize) -> Self;
}

impl UsizeExt for usize {
    #[inline]
    fn align_up(self, multiple: usize) -> Self {
        (self + multiple - 1) / multiple * multiple
    }

    #[inline]
    fn align_down(self, multiple: usize) -> Self {
        self / multiple * multiple
    }
}
