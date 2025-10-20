use crate::{
    errors::HasherError,
    zero_bytes::{keccak::ZERO_BYTES, ZeroBytes},
    zero_indexed_leaf::keccak::ZERO_INDEXED_LEAF,
    Hash, Hasher,
};

#[derive(Clone, Copy)] // To allow using with zero copy Solana accounts.
pub struct Keccak;

impl Hasher for Keccak {
    const ID: u8 = 2;

    fn hash(val: &[u8]) -> Result<Hash, HasherError> {
        Self::hashv(&[val])
    }

    fn hashv(_vals: &[&[u8]]) -> Result<Hash, HasherError> {
        #[cfg(all(not(target_os = "solana"), feature = "keccak"))]
        {
            use sha3::{Digest, Keccak256};

            let mut hasher = Keccak256::default();
            for val in _vals {
                hasher.update(val);
            }
            Ok(hasher.finalize().into())
        }
        #[cfg(all(not(target_os = "solana"), not(feature = "keccak")))]
        {
            Err(HasherError::KeccakFeatureNotEnabled)
        }
        // Call via a system call to perform the calculation
        #[cfg(target_os = "solana")]
        {
            use crate::HASH_BYTES;

            let mut hash_result = [0; HASH_BYTES];
            unsafe {
                crate::syscalls::sol_keccak256(
                    _vals as *const _ as *const u8,
                    _vals.len() as u64,
                    &mut hash_result as *mut _ as *mut u8,
                );
            }
            Ok(hash_result)
        }
    }

    fn zero_bytes() -> ZeroBytes {
        ZERO_BYTES
    }

    fn zero_indexed_leaf() -> [u8; 32] {
        ZERO_INDEXED_LEAF
    }
}
