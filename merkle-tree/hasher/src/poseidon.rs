use crate::{
    errors::HasherError,
    zero_bytes::{poseidon::ZERO_BYTES, ZeroBytes},
    zero_indexed_leaf::poseidon::ZERO_INDEXED_LEAF,
    Hash, Hasher,
};

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
            use light_poseidon::{Poseidon, PoseidonBytesHasher};

            let mut hasher = Poseidon::<Fr>::new_circom(vals.len())?;
            let res = hasher.hash_bytes_be(vals)?;

            Ok(res)
        }
        // Call via a system call to perform the calculation.
        #[cfg(target_os = "solana")]
        {
            use crate::{errors::PoseidonSyscallError, HASH_BYTES};

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
