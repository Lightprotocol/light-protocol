use rand::{
    distributions::uniform::{SampleRange, SampleUniform},
    Rng,
};

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

    use super::*;

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
