//! The discrete log implementation for the Baby Jubjub ElGamal decryption.
//!
//! The implementation uses the baby-step giant-step method, which consists of a precomputation
//! step and an online step. The precomputation step involves computing a hash table of a number
//! of Projective points that is independent of a discrete log instance. The online phase computes
//! the final discrete log solution using the discrete log instance and the pre-computed hash
//! table. More details on the baby-step giant-step algorithm and the implementation can be found
//! in the [spl documentation](https://spl.solana.com).
//!
//! The implementation is NOT intended to run in constant-time. There are some measures to prevent
//! straightforward timing attacks. For instance, it does not short-circuit the search when a
//! solution is found. However, the use of hashtables, batching, and threads make the
//! implementation inherently not constant-time. This may theoretically allow an adversary to gain
//! information on a discrete log solution depending on the execution time of the implementation.
//!

pub use ark_ec::twisted_edwards::Projective;
pub use {
    ark_ec::{twisted_edwards::TECurveConfig, CurveGroup},
    ark_jubjub::{
        EdwardsAffine, 
        EdwardsProjective as G, 
        BabyJubConfig,
        Fr as F,
        GENERATOR_X,
        GENERATOR_Y,
    },
    ark_ff::{PrimeField, Field, biginteger::BigInteger256 as BigInt, BigInteger},
    ark_std::{Zero, UniformRand, ops::Mul},
    itertools::Itertools,
};

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, thread};

pub const BASE: EdwardsAffine = EdwardsAffine::new_unchecked(GENERATOR_X, GENERATOR_Y);

#[derive(Debug)]
pub enum DiscreteLogError {
    DiscreteLogThreads,
    DiscreteLogBatchSize,
}

impl std::error::Error for DiscreteLogError {}

impl std::fmt::Display for DiscreteLogError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DiscreteLogError::DiscreteLogThreads => {
                write!(f, "discrete log number of threads not power-of-two")
            }
            DiscreteLogError::DiscreteLogBatchSize => {
                write!(f, "discrete log batch size too large")
            }
        }
    }
}

/// Type that captures a discrete log challenge.
///
/// The goal of discrete log is to find x such that x * generator = target.
#[derive(Clone, Debug)]
pub struct DiscreteLog {
    /// Generator point for discrete log
    pub generator: Projective<BabyJubConfig>,
    /// Target point for discrete log
    pub target: Projective<BabyJubConfig>,
    /// Number of threads used for discrete log computation
    num_threads: usize,
    /// Range bound for discrete log search derived from the max value to search for and
    /// `num_threads`
    range_bound: usize,
    /// Projective point representing each step of the discrete log search
    step_point: Projective<BabyJubConfig>,
    /// Projective point compression batch size
    compression_batch_size: usize,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DecodePrecomputation(pub HashMap<[u8; 32], u32>);

/// Builds a HashMap of 2^size elements
 #[allow(dead_code)]
pub fn decode_u32_precomputation(base: Projective<BabyJubConfig>, size: u32) -> DecodePrecomputation {
    let mut hashmap = HashMap::new();
    let range = 32 - size;
    let two_range_scalar = F::from(2u64.pow(range + 1 as u32));
    let upper_bound = 2u64.pow(size);

    let identity = G::zero();
    let generator = base * &two_range_scalar;

    // iterator for 2^(range + 1)*0G , 2^(range + 1)*1G, 2^(range + 1)*2G, ...
    let edwards_iter = EdwardsIterator::new((identity, 0), (generator, 1));
    for (point, x_hi) in edwards_iter.take(upper_bound as usize) {
        let key = point.into_affine().x.into_bigint().to_bytes_le().try_into().unwrap();
        hashmap.insert(key, x_hi as u32);
    }

    DecodePrecomputation(hashmap)
}

lazy_static::lazy_static! {
    /// Pre-computed HashMap needed for decryption. The HashMap is independent of (works for) any key.
    pub static ref DECODE_PRECOMPUTATION_FOR_G: DecodePrecomputation = {
        static DECODE_PRECOMPUTATION_FOR_G_BINCODE: &[u8] =
            include_bytes!("decode_lookup_table16.bincode");
        bincode::deserialize(DECODE_PRECOMPUTATION_FOR_G_BINCODE).unwrap_or_default()
    };
}

/// Solves the discrete log instance using a 16/16 bit offline/online split
impl DiscreteLog {
    /// Discrete log instance constructor.
    ///
    /// Default number of threads set to 1.
    pub fn new(generator: Projective<BabyJubConfig>, target: Projective<BabyJubConfig>, size: u32) -> Self {
        Self {
            generator,
            target,
            num_threads: 1,
            range_bound: 2u64.pow(32 - size) as usize,
            step_point: G::from(BASE),
            compression_batch_size: 16,
        }
    }

    /// Adjusts number of threads in a discrete log instance.
    pub fn num_threads(&mut self, num_threads: usize, size: u32) -> Result<(), DiscreteLogError> {
        // number of threads must be a positive power-of-two integer
        if num_threads == 0 || (num_threads & (num_threads - 1)) != 0 || num_threads > 65536 {
            return Err(DiscreteLogError::DiscreteLogThreads);
        }

        self.num_threads = num_threads;
        self.range_bound = (2u64.pow(32 - size) as usize).checked_div(num_threads).unwrap();
        self.step_point = G::from(BASE) * F::from(num_threads as u64);//Scalar::from(num_threads as u64) * G;

        Ok(())
    }

    /// Adjusts inversion batch size in a discrete log instance.
    pub fn set_compression_batch_size(
        &mut self,
        compression_batch_size: usize,
    ) -> Result<(), DiscreteLogError> {
        if compression_batch_size >= 65536 as usize {
            return Err(DiscreteLogError::DiscreteLogBatchSize);
        }
        self.compression_batch_size = compression_batch_size;

        Ok(())
    }

    /// Solves the discrete log problem under the assumption that the solution
    /// is a positive 32-bit number.
    pub fn decode_u32(self, size: u32) -> Option<u64> {
        let mut starting_point = self.target;
        let handles = (0..self.num_threads)
            .map(|i| {
                let edwards_iterator = EdwardsIterator::new(
                    (starting_point, i as u64),
                    (-(self.step_point), self.num_threads as u64),
                );

                let handle = thread::spawn(move || {
                    Self::decode_range(
                        edwards_iterator,
                        self.range_bound,
                        self.compression_batch_size,
                        size,
                    )
                });

                starting_point -= G::from(BASE);
                handle
            })
            .collect::<Vec<_>>();

        let mut solution = None;
        for handle in handles {
            let discrete_log = handle.join().unwrap();
            if discrete_log.is_some() {
                solution = discrete_log;
            }
        }
        solution
    }

    fn decode_range(
        edwards_iterator: EdwardsIterator,
        range_bound: usize,
        compression_batch_size: usize,
        size: u32,
    ) -> Option<u64> {
        let hashmap = &DECODE_PRECOMPUTATION_FOR_G;
        let mut decoded = None;
        for batch in &edwards_iterator
            .take(range_bound)
            .chunks(compression_batch_size)
        {
            // batch compression currently errors if any point in the batch is the identity point
            let (batch_points, batch_indices): (Vec<_>, Vec<_>) = batch
                .filter(|(point, index)| {
                    if point.is_zero() {
                        decoded = Some(*index);
                        return false;
                    }
                    true
                })
                .unzip();
            
            for (point, x_lo) in batch_points.iter().zip(batch_indices.iter()) {
                let ppt = point.mul(F::from(2));
                let key: &[u8; 32] = &ppt.into_affine().x.into_bigint().to_bytes_le().try_into().unwrap();
                if hashmap.0.contains_key(key) {
                    let x_hi = hashmap.0[key];
                    decoded = Some(x_lo + 2u64.pow(32 - size) as u64 * x_hi as u64);
                }
            }
        }

        decoded
    }
}

/// Hashable Edwards iterator.
///
/// Given an initial point X and a stepping point P, the iterator iterates through
/// X + 0*P, X + 1*P, X + 2*P, X + 3*P, ...
pub struct EdwardsIterator {
    pub current: (Projective<BabyJubConfig>, u64),
    pub step: (Projective<BabyJubConfig>, u64),
}

impl EdwardsIterator {
    pub fn new(current: (Projective<BabyJubConfig>, u64), step: (Projective<BabyJubConfig>, u64)) -> Self {
        EdwardsIterator { current, step }
    }
}

impl Iterator for EdwardsIterator {
    type Item = (Projective<BabyJubConfig>, u64);

    fn next(&mut self) -> Option<Self::Item> {
        let r = self.current;
        self.current = (self.current.0 + (self.step.0), self.current.1 + self.step.1);
        Some(r)
    }
}
