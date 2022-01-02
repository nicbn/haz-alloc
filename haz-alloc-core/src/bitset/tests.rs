use super::*;

#[test]
fn test_find_zero() {
    assert_eq!(find_zero(&[0, 0]), Some(0));
    assert_eq!(find_zero(&[3, 0]), Some(2));
    assert_eq!(find_zero(&[!0, 0]), Some(USIZE_BITS));
}

#[test]
fn test_find_zero_run() {
    assert_eq!(find_zero_run(&[0, 0], 2 * USIZE_BITS), Some(0));
    assert_eq!(find_zero_run(&[1, 0], 2 * USIZE_BITS - 1), Some(1));
}

#[test]
fn test_is_zero_range() {
    assert!(is_zero_range(&[1, 1], 1, USIZE_BITS - 1));
}

#[test]
fn test_set_range() {
    let mut x = [0];
    set_range(&mut x, 0, 2);
    assert_eq!(x[0], 3);
}

#[test]
fn test_clear_range() {
    let mut x = [3];
    clear_range(&mut x, 0, 2);
    assert_eq!(x[0], 0);
}
