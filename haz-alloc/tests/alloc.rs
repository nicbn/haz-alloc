use haz_alloc_internal::SMALL_CLASSES;
use std::alloc::Layout;

#[test]
fn test_small() {
    unsafe {
        // classes
        for size0 in SMALL_CLASSES {
            let p =
                haz_alloc::alloc_zeroed(Layout::from_size_align(*size0, 8).unwrap()) as *mut u64;
            assert_eq!(*p, 0);
            assert_eq!(haz_alloc::size(p as _), *size0);
            haz_alloc::dealloc(p as _);
        }
    }
}
