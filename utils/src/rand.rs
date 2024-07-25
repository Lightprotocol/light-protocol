use std::ops::{Bound, RangeBounds};

use rand::{
    distributions::uniform::{SampleRange, SampleUniform},
    Rng,
};

use crate::prime::find_next_prime;

const PRIME_RETRIES: usize = 10;

/// Generates a random prime number in the given range. It returns `None` when
/// generating such number was not possible.
pub fn gen_prime<N, R, T>(rng: &mut N, range: R) -> Option<T>
where
    N: Rng,
    R: Clone + RangeBounds<T> + SampleRange<T>,
    T: Into<u32> + From<u32> + Copy + PartialOrd + SampleUniform,
{
    for _ in 0..PRIME_RETRIES {
        let sample: T = rng.gen_range(range.clone());
        let next_prime = find_next_prime(sample.into());

        match range.end_bound() {
            Bound::Included(end) => {
                if next_prime > (*end).into() {
                    continue;
                }
            }
            Bound::Excluded(end) => {
                if next_prime >= (*end).into() {
                    continue;
                }
            }
            _ => {}
        };

        return Some(T::from(next_prime));
    }

    None
}

/// Generates a random value in the given range, excluding the values provided
/// in `exclude`.
pub fn gen_range_exclude<N, R, T>(rng: &mut N, range: R, exclude: &[T]) -> T
where
    N: Rng,
    R: Clone + SampleRange<T>,
    T: PartialEq + SampleUniform,
{
    loop {
        // This utility is supposed to be used only in unit tests. This `clone`
        // is harmless and necessary (can't pass a reference to range, it has
        // to be moved).
        let sample = rng.gen_range(range.clone());
        if !exclude.contains(&sample) {
            return sample;
        }
    }
}

#[cfg(test)]
mod test {
    use rand::Rng;

    use crate::prime::is_prime;

    use super::*;

    #[test]
    fn test_gen_prime() {
        let mut rng = rand::thread_rng();

        let mut successful_gens = 0;
        for i in 0..10_000 {
            let sample: Option<u32> = gen_prime(&mut rng, 1..10_000);
            println!("sample {i}: {sample:?}");
            if let Some(sample) = sample {
                successful_gens += 1;
                assert!(is_prime(sample));
            }
        }

        println!("generated {successful_gens} prime numbers out of 10000 iterations");
    }

    #[test]
    fn test_gen_range_exclude() {
        let mut rng = rand::thread_rng();

        for n_excluded in 1..100 {
            let excluded: Vec<u64> = (0..n_excluded).map(|_| rng.gen_range(0..100)).collect();

            for _ in 0..10_000 {
                let sample = gen_range_exclude(&mut rng, 0..100, excluded.as_slice());
                for excluded in excluded.iter() {
                    assert_ne!(&sample, excluded);
                }
            }
        }
    }
}
