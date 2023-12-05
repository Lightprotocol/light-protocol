use crate::{errors::MerkleTreeError, Hash, Hasher};

#[derive(Clone, Copy)]
pub struct Poseidon;

impl Hasher for Poseidon {
    fn hash(val: &[u8]) -> Result<Hash, MerkleTreeError> {
        Self::hashv(&[val])
    }

    fn hashv(vals: &[&[u8]]) -> Result<Hash, MerkleTreeError> {
        // Perform the calculation inline, calling this from within a program is
        // not supported.
        #[cfg(not(target_os = "solana"))]
        {
            use ark_bn254::Fr;
            use light_poseidon::{Poseidon, PoseidonBytesHasher, PoseidonError};

            impl From<PoseidonError> for MerkleTreeError {
                fn from(error: PoseidonError) -> Self {
                    match error {
                        PoseidonError::InvalidNumberOfInputs { .. } => {
                            MerkleTreeError::PoseidonInvalidNumberOfInputs
                        }
                        PoseidonError::EmptyInput => MerkleTreeError::PoseidonEmptyInput,
                        PoseidonError::InvalidInputLength { .. } => {
                            MerkleTreeError::PoseidonInvalidInputLength
                        }
                        PoseidonError::BytesToPrimeFieldElement { .. } => {
                            MerkleTreeError::PoseidonBytesToPrimeFieldElement
                        }
                        PoseidonError::InputLargerThanModulus => {
                            MerkleTreeError::PoseidonInputLargerThanModulus
                        }
                        PoseidonError::VecToArray => MerkleTreeError::PoseidonVecToArray,
                        PoseidonError::U64Tou8 => MerkleTreeError::PoseidonU64Tou8,
                        PoseidonError::BytesToBigInt => MerkleTreeError::PoseidonBytesToBigInt,
                        PoseidonError::InvalidWidthCircom { .. } => {
                            MerkleTreeError::PoseidonInvalidWidthCircom
                        }
                    }
                }
            }

            let mut hasher =
                Poseidon::<Fr>::new_circom(vals.len()).map_err(MerkleTreeError::from)?;
            let res = hasher.hash_bytes_be(vals).map_err(MerkleTreeError::from)?;

            Ok(res)
        }
        // Call via a system call to perform the calculation.
        #[cfg(target_os = "solana")]
        {
            use crate::hasher::HASH_BYTES;

            impl From<u64> for MerkleTreeError {
                fn from(error: u64) -> Self {
                    match error {
                        1 => MerkleTreeError::PoseidonInvalidNumberOfInputs,
                        2 => MerkleTreeError::PoseidonEmptyInput,
                        3 => MerkleTreeError::PoseidonInvalidInputLength,
                        4 => MerkleTreeError::PoseidonBytesToPrimeFieldElement,
                        5 => MerkleTreeError::PoseidonInputLargerThanModulus,
                        6 => MerkleTreeError::PoseidonVecToArray,
                        7 => MerkleTreeError::PoseidonU64Tou8,
                        8 => MerkleTreeError::PoseidonBytesToBigInt,
                        9 => MerkleTreeError::PoseidonInvalidWidthCircom,
                        _ => MerkleTreeError::PoseidonUnknown,
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
                e => Err(MerkleTreeError::from(e)),
            }
        }
    }
}
