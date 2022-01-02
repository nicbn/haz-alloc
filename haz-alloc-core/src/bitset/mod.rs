use core::{iter, mem};

#[cfg(test)]
mod tests;

const USIZE_BITS: usize = 8 * mem::size_of::<usize>();

#[inline]
pub fn find_zero(set: &[usize]) -> Option<usize> {
    set.iter()
        .copied()
        .enumerate()
        .find(|(_, v)| *v != !0)
        .map(|(i, v)| i * USIZE_BITS + v.trailing_ones() as usize)
}

pub fn find_zero_run(set: &[usize], len: usize) -> Option<usize> {
    let mut run_start = None;
    for (i, x) in set.iter().enumerate() {
        for j in 0..USIZE_BITS {
            if x & (1 << j) != 0 {
                run_start = None;
                continue;
            }

            let (i0, j0) = *run_start.get_or_insert((i, j));
            if (i - i0) * USIZE_BITS + j - j0 + 1 >= len {
                return Some(i0 * USIZE_BITS + j0);
            }
        }
    }

    None
}

pub fn is_zero_range(set: &[usize], index: usize, len: usize) -> bool {
    set.iter()
        .copied()
        .zip(mask(index, len))
        .skip(index / USIZE_BITS)
        .take((index + len) / USIZE_BITS + 1)
        .all(|(x, y)| x & y == 0)
}

#[inline]
pub fn set(set: &mut [usize], index: usize) {
    set[index / USIZE_BITS] |= 1 << (index % USIZE_BITS);
}

#[inline]
pub fn clear(set: &mut [usize], index: usize) {
    set[index / USIZE_BITS] &= !(1 << (index % USIZE_BITS));
}

pub fn set_range(set: &mut [usize], index: usize, len: usize) {
    for (x, mask) in set
        .iter_mut()
        .zip(mask(index, len))
        .skip(index / USIZE_BITS)
        .take((index + len) / USIZE_BITS + 1)
    {
        *x |= mask;
    }
}

pub fn clear_range(set: &mut [usize], index: usize, len: usize) {
    for (x, mask) in set
        .iter_mut()
        .zip(mask(index, len))
        .skip(index / USIZE_BITS)
        .take((index + len) / USIZE_BITS + 1)
    {
        *x &= !mask;
    }
}

#[inline]
fn mask(index: usize, len: usize) -> impl Iterator<Item = usize> {
    iter::repeat(0)
        .take(index / USIZE_BITS)
        .chain(
            ((index / USIZE_BITS)..=((index + len) / USIZE_BITS)).map(move |i| {
                let mut mask: usize = !0;
                if i == index / USIZE_BITS {
                    mask &= !0 << (index % USIZE_BITS);
                }
                if i == (index + len) / USIZE_BITS {
                    mask &= !(!0 << ((index + len) % USIZE_BITS));
                }
                mask
            }),
        )
        .chain(iter::repeat(0))
}
