use anchor_lang::solana_program::program_error::ProgramError;
use arrayvec::ArrayVec;
use light_compressed_account::{hash_to_bn254_field_size_be, Pubkey};

/// Context for caching hashed values to avoid recomputation
pub struct TokenContext {
    /// Cache for mint hashes: (mint_pubkey, hashed_mint)
    pub hashed_mints: ArrayVec<(Pubkey, [u8; 32]), 5>,
    /// Cache for pubkey hashes: (pubkey, hashed_pubkey)
    pub hashed_pubkeys: Vec<(Pubkey, [u8; 32])>,
}

impl TokenContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self {
            hashed_mints: ArrayVec::new(),
            hashed_pubkeys: Vec::new(),
        }
    }

    /// Get or compute hash for a mint pubkey
    pub fn get_or_hash_mint(&mut self, mint: Pubkey) -> Result<[u8; 32], ProgramError> {
        let hashed_mint = self.hashed_mints.iter().find(|a| a.0 == mint).map(|a| a.1);
        match hashed_mint {
            Some(hashed_mint) => Ok(hashed_mint),
            None => {
                let hashed_mint = hash_to_bn254_field_size_be(mint.to_bytes().as_slice());
                self.hashed_mints
                    .try_push((mint, hashed_mint))
                    .map_err(|_| ProgramError::InvalidAccountData)?;
                Ok(hashed_mint)
            }
        }
    }

    /// Get or compute hash for a pubkey (owner, delegate, etc.)
    pub fn get_or_hash_pubkey(&mut self, pubkey: &Pubkey) -> [u8; 32] {
        let hashed_pubkey = self
            .hashed_pubkeys
            .iter()
            .find(|a| a.0 == *pubkey)
            .map(|a| a.1);
        match hashed_pubkey {
            Some(hashed_pubkey) => hashed_pubkey,
            None => {
                let hashed_pubkey = hash_to_bn254_field_size_be(pubkey.to_bytes().as_slice());
                self.hashed_pubkeys.push((*pubkey, hashed_pubkey));
                hashed_pubkey
            }
        }
    }
}

impl Default for TokenContext {
    fn default() -> Self {
        Self::new()
    }
}
