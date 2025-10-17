use crate::{
    errors::HasherError,
    zero_bytes::{sha256::ZERO_BYTES, ZeroBytes},
    zero_indexed_leaf::sha256::ZERO_INDEXED_LEAF,
    Hash, Hasher,
};

/// Compile-time assertion trait that ensures a generic Hasher type is SHA256.
/// Used by LightHasherSha macro to enforce SHA256-only implementation at compile time.
pub trait RequireSha256: Hasher {
    const ASSERT: () = assert!(
        Self::ID == 1,
        "DataHasher for LightHasherSha only works with SHA256 (ID=1). Example: your_struct.hash::<Sha256>()?"
    );
}

impl<T: Hasher> RequireSha256 for T {}

#[derive(Clone, Copy)] // To allow using with zero copy Solana accounts.
pub struct Sha256;

impl Hasher for Sha256 {
    const ID: u8 = 1;
    fn hash(val: &[u8]) -> Result<Hash, HasherError> {
        Self::hashv(&[val])
    }

    fn hashv(_vals: &[&[u8]]) -> Result<Hash, HasherError> {
        #[cfg(all(not(target_os = "solana"), feature = "sha256"))]
        {
            use sha2::{Digest, Sha256};

            let mut hasher = Sha256::default();
            for val in _vals {
                hasher.update(val);
            }
            Ok(hasher.finalize().into())
        }
        #[cfg(all(not(target_os = "solana"), not(feature = "sha256")))]
        {
            Err(HasherError::Sha256FeatureNotEnabled)
        }
        // Call via a system call to perform the calculation
        #[cfg(target_os = "solana")]
        {
            use crate::HASH_BYTES;

            let mut hash_result = [0; HASH_BYTES];
            unsafe {
                crate::syscalls::sol_sha256(
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

/// SHA256 hasher that sets byte 0 to zero after hashing.
/// Used for big-endian compatibility with BN254 field size.
#[derive(Clone, Copy)]
pub struct Sha256BE;

impl Hasher for Sha256BE {
    const ID: u8 = 3;

    fn hash(val: &[u8]) -> Result<Hash, HasherError> {
        let mut result = Sha256::hash(val)?;
        result[0] = 0;
        Ok(result)
    }

    fn hashv(vals: &[&[u8]]) -> Result<Hash, HasherError> {
        let mut result = Sha256::hashv(vals)?;
        result[0] = 0;
        Ok(result)
    }

    fn zero_bytes() -> ZeroBytes {
        ZERO_BYTES
    }

    fn zero_indexed_leaf() -> [u8; 32] {
        ZERO_INDEXED_LEAF
    }
}
