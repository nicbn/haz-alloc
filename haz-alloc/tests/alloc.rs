use haz_alloc::Alloc;
use haz_alloc_core::__internal::SMALL_CLASSES;
use std::alloc::Layout;

static ALLOC: Alloc = Alloc::new();

#[test]
fn test_small() {
    unsafe {
        // classes
        for size0 in SMALL_CLASSES {
            let p = ALLOC.alloc_zeroed(Layout::from_size_align(*size0, 8).unwrap()) as *mut u64;
            assert_eq!(*p, 0);
            assert_eq!(ALLOC.size(p as _), *size0);
            ALLOC.dealloc(p as _);
        }
    }
}
