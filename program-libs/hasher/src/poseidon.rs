use thiserror::{self, Error};

use crate::{
    errors::HasherError,
    zero_bytes::{poseidon::ZERO_BYTES, ZeroBytes},
    zero_indexed_leaf::poseidon::ZERO_INDEXED_LEAF,
    Hash, Hasher,
};

#[derive(Debug, Error, PartialEq)]
pub enum PoseidonSyscallError {
    #[error("Invalid parameters.")]
    InvalidParameters,
    #[error("Invalid endianness.")]
    InvalidEndianness,
    #[error("Invalid number of inputs. Maximum allowed is 12.")]
    InvalidNumberOfInputs,
    #[error("Input is an empty slice.")]
    EmptyInput,
    #[error(
        "Invalid length of the input. The length matching the modulus of the prime field is 32."
    )]
    InvalidInputLength,
    #[error("Failed to convert bytest into a prime field element.")]
    BytesToPrimeFieldElement,
    #[error("Input is larger than the modulus of the prime field.")]
    InputLargerThanModulus,
    #[error("Failed to convert a vector of bytes into an array.")]
    VecToArray,
    #[error("Failed to convert the number of inputs from u64 to u8.")]
    U64Tou8,
    #[error("Failed to convert bytes to BigInt")]
    BytesToBigInt,
    #[error("Invalid width. Choose a width between 2 and 16 for 1 to 15 inputs.")]
    InvalidWidthCircom,
    #[error("Unexpected error")]
    Unexpected,
}

impl From<u64> for PoseidonSyscallError {
    fn from(error: u64) -> Self {
        match error {
            1 => PoseidonSyscallError::InvalidParameters,
            2 => PoseidonSyscallError::InvalidEndianness,
            3 => PoseidonSyscallError::InvalidNumberOfInputs,
            4 => PoseidonSyscallError::EmptyInput,
            5 => PoseidonSyscallError::InvalidInputLength,
            6 => PoseidonSyscallError::BytesToPrimeFieldElement,
            7 => PoseidonSyscallError::InputLargerThanModulus,
            8 => PoseidonSyscallError::VecToArray,
            9 => PoseidonSyscallError::U64Tou8,
            10 => PoseidonSyscallError::BytesToBigInt,
            11 => PoseidonSyscallError::InvalidWidthCircom,
            _ => PoseidonSyscallError::Unexpected,
        }
    }
}

impl From<PoseidonSyscallError> for u64 {
    fn from(error: PoseidonSyscallError) -> Self {
        match error {
            PoseidonSyscallError::InvalidParameters => 1,
            PoseidonSyscallError::InvalidEndianness => 2,
            PoseidonSyscallError::InvalidNumberOfInputs => 3,
            PoseidonSyscallError::EmptyInput => 4,
            PoseidonSyscallError::InvalidInputLength => 5,
            PoseidonSyscallError::BytesToPrimeFieldElement => 6,
            PoseidonSyscallError::InputLargerThanModulus => 7,
            PoseidonSyscallError::VecToArray => 8,
            PoseidonSyscallError::U64Tou8 => 9,
            PoseidonSyscallError::BytesToBigInt => 10,
            PoseidonSyscallError::InvalidWidthCircom => 11,
            PoseidonSyscallError::Unexpected => 12,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Poseidon;

impl Hasher for Poseidon {
    const ID: u8 = 0;

    fn hash(val: &[u8]) -> Result<Hash, HasherError> {
        Self::hashv(&[val])
    }

    fn hashv(_vals: &[&[u8]]) -> Result<Hash, HasherError> {
        // Perform the calculation inline, calling this from within a program is
        // not supported.
        #[cfg(all(not(target_os = "solana"), feature = "poseidon"))]
        {
            use ark_bn254::Fr;
            use light_poseidon::{Poseidon, PoseidonBytesHasher};

            let mut hasher = Poseidon::<Fr>::new_circom(_vals.len())?;
            let res = hasher.hash_bytes_be(_vals)?;

            Ok(res)
        }
        #[cfg(all(not(target_os = "solana"), not(feature = "poseidon")))]
        {
            Err(HasherError::PoseidonFeatureNotEnabled)
        }
        // Call via a system call to perform the calculation.
        #[cfg(target_os = "solana")]
        {
            use crate::HASH_BYTES;
            // TODO: reenable once LightHasher refactor is merged
            // solana_program::msg!("remove len check onchain.");
            // for val in vals {
            //     if val.len() != 32 {
            //         return Err(HasherError::InvalidInputLength(val.len()));
            //     }
            // }
            let mut hash_result = [0; HASH_BYTES];
            let result = unsafe {
                crate::syscalls::sol_poseidon(
                    0, // bn254
                    0, // big-endian
                    _vals as *const _ as *const u8,
                    _vals.len() as u64,
                    &mut hash_result as *mut _ as *mut u8,
                )
            };

            match result {
                0 => Ok(hash_result),
                e => Err(HasherError::from(PoseidonSyscallError::from(e))),
            }
        }
    }

    fn zero_bytes() -> ZeroBytes {
        ZERO_BYTES
    }

    fn zero_indexed_leaf() -> [u8; 32] {
        ZERO_INDEXED_LEAF
    }
}
