use std::f64::consts::LN_2;

use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum BloomFilterError {
    #[error("Bloom filter is full")]
    Full,
    #[error("Invalid store capacity")]
    InvalidStoreCapacity,
}

impl From<BloomFilterError> for u32 {
    fn from(e: BloomFilterError) -> u32 {
        match e {
            BloomFilterError::Full => 14201,
            BloomFilterError::InvalidStoreCapacity => 14202,
        }
    }
}

#[cfg(feature = "solana")]
impl From<BloomFilterError> for solana_program_error::ProgramError {
    fn from(e: BloomFilterError) -> Self {
        solana_program_error::ProgramError::Custom(e.into())
    }
}

#[cfg(feature = "pinocchio")]
impl From<BloomFilterError> for pinocchio::program_error::ProgramError {
    fn from(e: BloomFilterError) -> Self {
        pinocchio::program_error::ProgramError::Custom(e.into())
    }
}

#[derive(Debug)]
pub struct BloomFilter<'a> {
    pub num_iters: usize,
    pub capacity: u64,
    pub store: &'a mut [u8],
}

impl<'a> BloomFilter<'a> {
    // TODO: find source for this
    pub fn calculate_bloom_filter_size(n: usize, p: f64) -> usize {
        let m = -((n as f64) * p.ln()) / (LN_2 * LN_2);
        let optimal_m = m.ceil() as usize;

        // Ensure optimal hash functions <= 5
        let k = Self::calculate_optimal_hash_functions(n, optimal_m);
        if k <= 10 {
            optimal_m
        } else {
            // If k > 5, we need to find m such that we achieve the target false positive rate p
            // with exactly k=5 hash functions
            // From p = (1 - e^(-kn/m))^k, we solve for m:
            // m = -kn / ln(1 - p^(1/k))
            let k = 10.0;
            let p_root = p.powf(1.0 / k);
            let m = -(k * n as f64) / (1.0 - p_root).ln();
            m.ceil() as usize
        }
    }

    pub fn calculate_optimal_hash_functions(n: usize, m: usize) -> usize {
        let k = (m as f64 / n as f64) * LN_2;
        k.ceil() as usize
    }

    pub fn calculate_false_positive_rate(n: usize, m: usize, k: usize) -> f64 {
        // p = (1 - e^(-kn/m))^k
        let exponent = -(k as f64 * n as f64) / m as f64;
        (1.0 - exponent.exp()).powf(k as f64)
    }

    pub fn new(
        num_iters: usize,
        capacity: u64,
        store: &'a mut [u8],
    ) -> Result<Self, BloomFilterError> {
        // Capacity is in bits while store is in bytes.
        if store.len() * 8 != capacity as usize {
            return Err(BloomFilterError::InvalidStoreCapacity);
        }
        Ok(Self {
            num_iters,
            capacity,
            store,
        })
    }

    pub fn probe_index_fast_murmur(value_bytes: &[u8], iteration: usize, capacity: &u64) -> usize {
        let iter_bytes = iteration.to_le_bytes();
        // let base_hash = fastmurmur3::hash(value_bytes);
        let mut combined_bytes = [0u8; 24];
        combined_bytes[..16].copy_from_slice(&value_bytes[0..16]);
        combined_bytes[16..].copy_from_slice(&iter_bytes);

        let combined_hash = fastmurmur3::hash(&combined_bytes);
        (combined_hash % (*capacity as u128)) as usize
    }

    pub fn insert(&mut self, value: &[u8; 32]) -> Result<(), BloomFilterError> {
        if self._insert(value, true) {
            Ok(())
        } else {
            Err(BloomFilterError::Full)
        }
    }

    // TODO: reconsider &mut self
    pub fn contains(&mut self, value: &[u8; 32]) -> bool {
        !self._insert(value, false)
    }

    fn _insert(&mut self, value: &[u8; 32], insert: bool) -> bool {
        let mut all_bits_set = true;
        use bitvec::prelude::*;

        let bits = BitSlice::<u8, Msb0>::from_slice_mut(self.store);
        for i in 0..self.num_iters {
            let probe_index = Self::probe_index_fast_murmur(value, i, &(self.capacity));
            if bits[probe_index] {
                continue;
            } else if insert {
                all_bits_set = false;
                bits.set(probe_index, true);
            } else if !bits[probe_index] && !insert {
                return true;
            }
        }
        !all_bits_set
    }
}

#[cfg(test)]
mod test {
    use light_hasher::bigint::bigint_to_be_bytes_array;
    use num_bigint::{RandBigInt, ToBigUint};
    use rand::thread_rng;

    use super::*;

    #[test]
    fn test_bloom_filter_size() {
        let n = 15_000;
        let desired_p: f64 = 1.0 / 1_000_000_000.0; // 1e-9

        // Calculate unconstrained optimal values
        let unconstrained_m = -((n as f64) * desired_p.ln()) / (LN_2 * LN_2);
        let unconstrained_m = unconstrained_m.ceil() as usize;
        let unconstrained_k = BloomFilter::calculate_optimal_hash_functions(n, unconstrained_m);
        let unconstrained_p =
            BloomFilter::calculate_false_positive_rate(n, unconstrained_m, unconstrained_k);

        // Calculate constrained values
        let bloom_filter_size = BloomFilter::calculate_bloom_filter_size(n, desired_p);
        // Check if we need to use constrained k=5
        let optimal_hash_functions = if unconstrained_k > 10 {
            10
        } else {
            BloomFilter::calculate_optimal_hash_functions(n, bloom_filter_size)
        };
        let actual_p = BloomFilter::calculate_false_positive_rate(
            n,
            bloom_filter_size,
            optimal_hash_functions,
        );

        println!("=== Bloom Filter Analysis ===");
        println!("Number of elements (n): {}", n);
        println!("Desired false positive rate: {:.2e}", desired_p);
        println!();
        println!("Unconstrained optimal:");
        println!(
            "  - Bloom filter size (m): {} bits ({} KB)",
            unconstrained_m,
            unconstrained_m / 8 / 1024
        );
        println!("  - Hash functions (k): {}", unconstrained_k);
        println!("  - Actual false positive rate: {:.2e}", unconstrained_p);
        println!();
        println!("Constrained (k <= 10):");
        println!(
            "  - Bloom filter size (m): {} bits ({} KB)",
            bloom_filter_size,
            bloom_filter_size / 8 / 1024
        );
        println!("  - Hash functions (k): {}", optimal_hash_functions);
        println!("  - Actual false positive rate: {:.2e}", actual_p);
        println!(
            "  - False positive rate increase: {:.2}x",
            actual_p / desired_p
        );
    }

    #[test]
    fn test_equilibrium_analysis() {
        println!("=== Bloom Filter Equilibrium Analysis ===");
        println!("Constraint: k <= 10 (max 10 hash functions)");
        println!();

        let n_values = vec![1_000, 10_000, 15_000, 100_000, 1_000_000];
        let p_values: Vec<f64> = vec![1e-3, 1e-6, 1e-9, 1e-12];

        for &n in &n_values {
            println!("Number of elements: n = {}", n);
            println!(
                "{:<15} | {:<20} | {:<15} | {:<20} | {:<15} | {:<15}",
                "Target FP Rate",
                "Unconstrained Size",
                "Unconstrained k",
                "Constrained Size",
                "Constrained k",
                "Size Increase"
            );
            println!(
                "{:-<15}-+-{:-<20}-+-{:-<15}-+-{:-<20}-+-{:-<15}-+-{:-<15}",
                "", "", "", "", "", ""
            );

            for &p in &p_values {
                // Calculate unconstrained optimal values
                let unconstrained_m = -((n as f64) * p.ln()) / (LN_2 * LN_2);
                let unconstrained_m = unconstrained_m.ceil() as usize;
                let unconstrained_k =
                    BloomFilter::calculate_optimal_hash_functions(n, unconstrained_m);

                // Calculate constrained values
                let constrained_m = BloomFilter::calculate_bloom_filter_size(n, p);
                let constrained_k = if unconstrained_k > 10 {
                    10
                } else {
                    unconstrained_k
                };

                // Calculate actual false positive rates
                let _unconstrained_p_actual =
                    BloomFilter::calculate_false_positive_rate(n, unconstrained_m, unconstrained_k);
                let _constrained_p_actual =
                    BloomFilter::calculate_false_positive_rate(n, constrained_m, constrained_k);

                let size_increase = constrained_m as f64 / unconstrained_m as f64;

                println!(
                    "{:<15.2e} | {:<20} | {:<15} | {:<20} | {:<15} | {:<15.2}x",
                    p,
                    format!("{} KB", unconstrained_m / 8 / 1024),
                    unconstrained_k,
                    format!("{} KB", constrained_m / 8 / 1024),
                    constrained_k,
                    size_increase
                );
            }
            println!();
        }

        // Special case analysis for n=15,000 and p=1e-9
        println!("=== Detailed Analysis: n=15,000, p=1e-9 ===");
        let n = 15_000;
        let p = 1e-12;
        let m_constrained = BloomFilter::calculate_bloom_filter_size(n, p);
        let k_constrained = 3;
        let p_actual = BloomFilter::calculate_false_positive_rate(n, m_constrained, k_constrained);

        println!("Bloom filter size: {}", m_constrained);
        println!("Memory usage: {} KB", m_constrained / 8 / 1024);
        println!("Hash functions: {}", k_constrained);
        println!("Target false positive rate: {:.2e}", p);
        println!("Actual false positive rate: {:.2e}", p_actual);
        println!(
            "Achieving target: {}",
            if p_actual <= p { "YES ✓" } else { "NO ✗" }
        );
    }

    #[test]
    fn test_insert_and_contains() -> Result<(), BloomFilterError> {
        let capacity = 128_000 * 8;
        let mut store = [0u8; 128_000];
        let mut bf = BloomFilter {
            num_iters: 3,
            capacity,
            store: &mut store,
        };

        let value1 = [1u8; 32];
        let value2 = [2u8; 32];

        bf.insert(&value1)?;
        assert!(bf.contains(&value1));
        assert!(!bf.contains(&value2));

        Ok(())
    }

    #[test]
    fn short_rnd_test() {
        let capacity = 500;
        let bloom_filter_capacity = 20_000 * 8;
        let optimal_hash_functions = 4;
        rnd_test(
            1000,
            capacity,
            bloom_filter_capacity,
            optimal_hash_functions,
            false,
        );
    }

    /// Bench results:
    /// - 15310 CU for 10 insertions with 3 hash functions
    /// - capacity 5000 0.000_000_000_1 with 15 hash functions seems to not
    ///   produce any collisions
    //#[ignore = "bench"]
    #[test]
    fn bench_bloom_filter() {
        let capacity = 15000;

        let optimal_hash_functions = 4;
        let iterations = 1000;
        let mut bloom_filter_capacity = 2_160_000;

        while !rnd_test(
            iterations,
            capacity,
            bloom_filter_capacity,
            optimal_hash_functions,
            true,
        ) {
            println!("bloom_filter_capacity: {}", bloom_filter_capacity);
            bloom_filter_capacity += 100_000;
        }
    }

    fn rnd_test(
        num_iters: usize,
        capacity: usize,
        bloom_filter_capacity: usize,
        optimal_hash_functions: usize,
        bench: bool,
    ) -> bool {
        println!("Optimal hash functions: {}", optimal_hash_functions);
        println!(
            "Bloom filter capacity (kb): {}",
            bloom_filter_capacity / 8 / 1_000
        );
        let mut num_total_txs = 0;
        let mut rng = thread_rng();
        let mut failed_vec = Vec::new();
        for j in 0..num_iters {
            let mut inserted_values = Vec::new();
            let mut store = vec![0; bloom_filter_capacity];
            let mut bf = BloomFilter {
                num_iters: optimal_hash_functions,
                capacity: bloom_filter_capacity as u64,
                store: &mut store,
            };
            if j == 0 {
                println!("Bloom filter capacity: {}", bf.capacity);
                println!("Bloom filter size: {}", bf.store.len());
                println!("Bloom filter size (kb): {}", bf.store.len() / 8 / 1_000);
                println!("num iters: {}", bf.num_iters);
            }
            for i in 0..capacity {
                num_total_txs += 1;
                let value = {
                    let mut _value = 0u64.to_biguint().unwrap();
                    while inserted_values.contains(&_value.clone()) {
                        _value = rng.gen_biguint(254);
                    }
                    inserted_values.push(_value.clone());

                    _value
                };
                let value: [u8; 32] = bigint_to_be_bytes_array(&value).unwrap();
                match bf.insert(&value) {
                    Ok(_) => {
                        assert!(bf.contains(&value));
                    }
                    Err(_) => {
                        println!("Failed to insert iter: {}", i);
                        println!("total iter {}", j);
                        println!("num_total_txs {}", num_total_txs);
                        failed_vec.push(i);
                        return false;
                    }
                };
                assert!(bf.contains(&value));
                assert!(bf.insert(&value).is_err());
            }
        }
        if bench {
            println!("total num tx {}", num_total_txs);
            let average = failed_vec.iter().sum::<usize>() as f64 / failed_vec.len() as f64;
            println!("average failed insertions: {}", average);
            println!(
                "max failed insertions: {}",
                failed_vec.iter().max().unwrap()
            );
            println!(
                "min failed insertions: {}",
                failed_vec.iter().min().unwrap()
            );

            let num_chunks = 10;
            let chunk_size = num_iters / num_chunks;
            failed_vec.sort();
            for (i, chunk) in failed_vec.chunks(chunk_size).enumerate() {
                let average = chunk.iter().sum::<usize>() as f64 / chunk.len() as f64;
                println!("chunk: {} average failed insertions: {}", i, average);
                println!(
                    "chunk: {} max failed insertions: {}",
                    i,
                    chunk.iter().max().unwrap()
                );
                println!(
                    "chunk: {} min failed insertions: {}",
                    i,
                    chunk.iter().min().unwrap()
                );
            }
        }

        true
    }
}
