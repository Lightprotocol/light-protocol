use crate::{errors::HasherError, Hash, Hasher};

#[derive(Clone, Copy)]
pub struct Poseidon;

impl Hasher for Poseidon {
    fn hash(val: &[u8]) -> Result<Hash, HasherError> {
        Self::hashv(&[val])
    }

    fn hashv(vals: &[&[u8]]) -> Result<Hash, HasherError> {
        // Perform the calculation inline, calling this from within a program is
        // not supported.
        #[cfg(not(target_os = "solana"))]
        {
            use ark_bn254::Fr;
            use light_poseidon::{Poseidon, PoseidonBytesHasher, PoseidonError};

            impl From<PoseidonError> for HasherError {
                fn from(error: PoseidonError) -> Self {
                    match error {
                        PoseidonError::InvalidNumberOfInputs { .. } => {
                            HasherError::PoseidonInvalidNumberOfInputs
                        }
                        PoseidonError::EmptyInput => HasherError::PoseidonEmptyInput,
                        PoseidonError::InvalidInputLength { .. } => {
                            HasherError::PoseidonInvalidInputLength
                        }
                        PoseidonError::BytesToPrimeFieldElement { .. } => {
                            HasherError::PoseidonBytesToPrimeFieldElement
                        }
                        PoseidonError::InputLargerThanModulus => {
                            HasherError::PoseidonInputLargerThanModulus
                        }
                        PoseidonError::VecToArray => HasherError::PoseidonVecToArray,
                        PoseidonError::U64Tou8 => HasherError::PoseidonU64Tou8,
                        PoseidonError::BytesToBigInt => HasherError::PoseidonBytesToBigInt,
                        PoseidonError::InvalidWidthCircom { .. } => {
                            HasherError::PoseidonInvalidWidthCircom
                        }
                    }
                }
            }

            let mut hasher = Poseidon::<Fr>::new_circom(vals.len()).map_err(HasherError::from)?;
            let res = hasher.hash_bytes_be(vals).map_err(HasherError::from)?;

            Ok(res)
        }
        // Call via a system call to perform the calculation.
        #[cfg(target_os = "solana")]
        {
            use crate::hasher::HASH_BYTES;

            impl From<u64> for HasherError {
                fn from(error: u64) -> Self {
                    match error {
                        1 => HasherError::PoseidonInvalidNumberOfInputs,
                        2 => HasherError::PoseidonEmptyInput,
                        3 => HasherError::PoseidonInvalidInputLength,
                        4 => HasherError::PoseidonBytesToPrimeFieldElement,
                        5 => HasherError::PoseidonInputLargerThanModulus,
                        6 => HasherError::PoseidonVecToArray,
                        7 => HasherError::PoseidonU64Tou8,
                        8 => HasherError::PoseidonBytesToBigInt,
                        9 => HasherError::PoseidonInvalidWidthCircom,
                        _ => HasherError::PoseidonUnknown,
                    }
                }
            }

            let mut hash_result = [0; HASH_BYTES];
            let result = unsafe {
                crate::syscalls::sol_poseidon(
                    0, // bn254
                    0, // big-endian
                    vals as *const _ as *const u8,
                    vals.len() as u64,
                    &mut hash_result as *mut _ as *mut u8,
                )
            };

            match result {
                0 => Ok(hash_result),
                e => Err(HasherError::from(e)),
            }
        }
    }
}
