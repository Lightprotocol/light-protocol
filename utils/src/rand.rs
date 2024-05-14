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
