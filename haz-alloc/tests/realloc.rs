#![feature(test)]
#![allow(clippy::all)]

use haz_alloc::{alloc, alloc_zeroed, dealloc, realloc, size};
use std::alloc::Layout;

#[test]
fn test_small_to_huge() {
    unsafe {
        let mut p = alloc_zeroed(Layout::from_size_align(8, 8).unwrap()) as *mut u64;
        assert_eq!(*p, 0);
        assert_eq!(size(p as _), 8);
        *p = 100;
        p = realloc(p as _, Layout::from_size_align(16, 8).unwrap()) as *mut u64;
        assert_eq!(*p, 100);
        assert_eq!(size(p as _), 16);
        p = realloc(p as _, Layout::from_size_align(10240, 8).unwrap()) as *mut u64;
        assert_eq!(*p, 100);
        assert!(size(p as _) >= 10240);
        p = realloc(p as _, Layout::from_size_align(20000, 8).unwrap()) as *mut u64;
        assert_eq!(*p, 100);
        assert!(size(p as _) >= 20000);
        p = realloc(p as _, Layout::from_size_align(327680, 8).unwrap()) as *mut u64;
        assert_eq!(*p, 100);
        assert!(size(p as _) >= 327680);
        p = realloc(p as _, Layout::from_size_align(3276800, 8).unwrap()) as *mut u64;
        assert_eq!(*p, 100);

        dealloc(p as _);
    }
}

#[test]
fn test_huge_to_small() {
    unsafe {
        let mut p = alloc(Layout::from_size_align(3276800, 8).unwrap()) as *mut u64;
        assert!(size(p as _) >= 3276800);
        *p = 100;
        p = realloc(p as _, Layout::from_size_align(10240, 8).unwrap()) as *mut u64;
        assert_eq!(*p, 100);
        assert!(size(p as _) >= 10240);
        p = realloc(p as _, Layout::from_size_align(16, 8).unwrap()) as *mut u64;
        assert_eq!(*p, 100);
        assert!(size(p as _) >= 16);

        dealloc(p as _);
    }
}
