#![feature(test)]
#![allow(clippy::all)]

use haz_alloc::Alloc;
use std::alloc::Layout;

static ALLOC: Alloc = Alloc::new();

#[test]
fn test_small_to_huge() {
    unsafe {
        let mut p = ALLOC.alloc_zeroed(Layout::from_size_align(8, 8).unwrap()) as *mut u64;
        assert_eq!(*p, 0);
        assert_eq!(ALLOC.size(p as _), 8);
        *p = 100;
        p = ALLOC.realloc(p as _, Layout::from_size_align(16, 8).unwrap()) as *mut u64;
        assert_eq!(*p, 100);
        assert_eq!(ALLOC.size(p as _), 16);
        p = ALLOC.realloc(p as _, Layout::from_size_align(10240, 8).unwrap()) as *mut u64;
        assert_eq!(*p, 100);
        assert!(ALLOC.size(p as _) >= 10240);
        p = ALLOC.realloc(p as _, Layout::from_size_align(20000, 8).unwrap()) as *mut u64;
        assert_eq!(*p, 100);
        assert!(ALLOC.size(p as _) >= 20000);
        p = ALLOC.realloc(p as _, Layout::from_size_align(327680, 8).unwrap()) as *mut u64;
        assert_eq!(*p, 100);
        assert!(ALLOC.size(p as _) >= 327680);
        p = ALLOC.realloc(p as _, Layout::from_size_align(3276800, 8).unwrap()) as *mut u64;
        assert_eq!(*p, 100);

        ALLOC.dealloc(p as _);
    }
}

#[test]
fn test_huge_to_small() {
    unsafe {
        let mut p = ALLOC.alloc(Layout::from_size_align(3276800, 8).unwrap()) as *mut u64;
        assert!(ALLOC.size(p as _) >= 3276800);
        *p = 100;
        p = ALLOC.realloc(p as _, Layout::from_size_align(10240, 8).unwrap()) as *mut u64;
        assert_eq!(*p, 100);
        assert!(ALLOC.size(p as _) >= 10240);
        p = ALLOC.realloc(p as _, Layout::from_size_align(16, 8).unwrap()) as *mut u64;
        assert_eq!(*p, 100);
        assert!(ALLOC.size(p as _) >= 16);

        ALLOC.dealloc(p as _);
    }
}
