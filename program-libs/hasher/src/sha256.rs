use crate::{
    errors::HasherError,
    zero_bytes::{sha256::ZERO_BYTES, ZeroBytes},
    zero_indexed_leaf::sha256::ZERO_INDEXED_LEAF,
    Hash, Hasher,
};

#[derive(Clone, Copy)] // To allow using with zero copy Solana accounts.
pub struct Sha256;

impl Hasher for Sha256 {
    fn hash(val: &[u8]) -> Result<Hash, HasherError> {
        Self::hashv(&[val])
    }

    fn hashv(vals: &[&[u8]]) -> Result<Hash, HasherError> {
        #[cfg(not(target_os = "solana"))]
        {
            use sha2::{Digest, Sha256};

            let mut hasher = Sha256::default();
            for val in vals {
                hasher.update(val);
            }
            Ok(hasher.finalize().into())
        }
        // Call via a system call to perform the calculation
        #[cfg(target_os = "solana")]
        {
            use crate::HASH_BYTES;

            let mut hash_result = [0; HASH_BYTES];
            unsafe {
                crate::syscalls::sol_sha256(
                    vals as *const _ as *const u8,
                    vals.len() as u64,
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
