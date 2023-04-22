use super::*;

#[test]
fn test_classes() {
    for size in SMALL_CLASSES {
        assert_eq!(SMALL_CLASSES[small_class_of(*size)], *size);
    }
}
