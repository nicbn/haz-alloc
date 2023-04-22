#![feature(test)]

use haz_alloc::Alloc;

#[global_allocator]
static ALLOC: Alloc = Alloc::new();

#[test]
fn stress_test() {}
