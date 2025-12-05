use light_array_map::ArrayMap;
use light_compressed_account::hash_to_bn254_field_size_be;
use pinocchio::pubkey::{pubkey_eq, Pubkey};

use crate::error::CTokenError;
/// Context for caching hashed values to avoid recomputation
pub struct HashCache {
    /// Cache for mint hashes: (mint_pubkey, hashed_mint)
    pub hashed_mints: ArrayMap<Pubkey, [u8; 32], 5>,
    /// Cache for pubkey hashes: (pubkey, hashed_pubkey)
    pub hashed_pubkeys: Vec<(Pubkey, [u8; 32])>,
}

impl HashCache {
    /// Create a new empty context
    pub fn new() -> Self {
        Self {
            hashed_mints: ArrayMap::new(),
            hashed_pubkeys: Vec::new(),
        }
    }

    /// Get or compute hash for a mint pubkey
    pub fn get_or_hash_mint(&mut self, mint: &Pubkey) -> Result<[u8; 32], CTokenError> {
        if let Some(hash) = self.hashed_mints.get_by_key(mint) {
            return Ok(*hash);
        }

        let hashed_mint = hash_to_bn254_field_size_be(mint);
        self.hashed_mints
            .insert(*mint, hashed_mint, CTokenError::InvalidAccountData)?;
        Ok(hashed_mint)
    }

    /// Get or compute hash for a pubkey (owner, delegate, etc.)
    pub fn get_or_hash_pubkey(&mut self, pubkey: &Pubkey) -> [u8; 32] {
        let hashed_pubkey = self
            .hashed_pubkeys
            .iter()
            .find(|a| pubkey_eq(&a.0, pubkey))
            .map(|a| a.1);
        match hashed_pubkey {
            Some(hashed_pubkey) => hashed_pubkey,
            None => {
                let hashed_pubkey = hash_to_bn254_field_size_be(pubkey);
                self.hashed_pubkeys.push((*pubkey, hashed_pubkey));
                hashed_pubkey
            }
        }
    }
}

impl Default for HashCache {
    fn default() -> Self {
        Self::new()
    }
}
