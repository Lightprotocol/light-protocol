// --- cpi-context-gated traits (from decompress_runtime.rs) ---

#[cfg(feature = "cpi-context")]
use solana_program_error::ProgramError;
#[cfg(feature = "cpi-context")]
use solana_pubkey::Pubkey;

/// Trait for account variants that can be checked for token or PDA type.
#[cfg(feature = "cpi-context")]
pub trait HasTokenVariant {
    /// Returns true if this variant represents a token account (PackedTokenData).
    fn is_packed_token(&self) -> bool;
}

/// Trait for PDA types that can derive seeds with full account context access.
#[cfg(feature = "cpi-context")]
pub trait PdaSeedDerivation<A, S> {
    fn derive_pda_seeds_with_accounts(
        &self,
        program_id: &Pubkey,
        accounts: &A,
        seed_params: &S,
    ) -> Result<(Vec<Vec<u8>>, Pubkey), ProgramError>;
}
