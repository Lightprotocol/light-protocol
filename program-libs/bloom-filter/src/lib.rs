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
        m.ceil() as usize
    }

    pub fn calculate_optimal_hash_functions(n: usize, m: usize) -> usize {
        let k = (m as f64 / n as f64) * LN_2;
        k.ceil() as usize
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

    pub fn probe_index_keccak(value_bytes: &[u8; 32], iteration: usize, capacity: &u64) -> usize {
        let iter_bytes: [u8; 8] = iteration.to_le_bytes();
        let mut combined_bytes = [0u8; 40];
        combined_bytes[..32].copy_from_slice(value_bytes);
        combined_bytes[32..].copy_from_slice(&iter_bytes);

        let hash = solana_nostd_keccak::hash(&combined_bytes);

        let mut index = 0u64;
        for chunk in hash.chunks(8) {
            let value = u64::from_le_bytes(chunk.try_into().unwrap());
            index = value.wrapping_add(index) % *capacity;
        }
        index as usize
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
            let probe_index = Self::probe_index_keccak(value, i, &(self.capacity));
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
        let optimal_hash_functions = 3;
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
    #[ignore = "bench"]
    #[test]
    fn bench_bloom_filter() {
        let capacity = 5000;
        let bloom_filter_capacity =
            BloomFilter::calculate_bloom_filter_size(capacity, 0.000_000_000_1);
        let optimal_hash_functions = 15;
        let iterations = 1_000_000;
        rnd_test(
            iterations,
            capacity,
            bloom_filter_capacity,
            optimal_hash_functions,
            true,
        );
    }

    fn rnd_test(
        num_iters: usize,
        capacity: usize,
        bloom_filter_capacity: usize,
        optimal_hash_functions: usize,
        bench: bool,
    ) {
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
    }
}
