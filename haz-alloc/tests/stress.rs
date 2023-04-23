#![feature(test)]

use haz_alloc::Alloc;

#[global_allocator]
static ALLOC: Alloc = Alloc::new();

#[test]
fn stress_test() {
    let collection: Vec<Vec<usize>> = (0..256)
        .map(|_| {
            let mut vec = Vec::new();
            for i in 0..4096 {
                vec.push(i);
            }
            vec.reserve(10);
            vec.reserve(20);
            vec.shrink_to_fit();

            for i in 0..4096 {
                assert_eq!(vec[i], i);
            }

            vec
        })
        .collect();
    drop(collection);
}
