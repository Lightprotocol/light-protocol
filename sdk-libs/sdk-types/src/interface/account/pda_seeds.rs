//! PDA seed derivation traits.

use alloc::vec::Vec;

use crate::error::LightSdkTypesError;

/// Trait for account variants that can be checked for token or PDA type.
pub trait HasTokenVariant {
    /// Returns true if this variant represents a token account (PackedTokenData).
    fn is_packed_token(&self) -> bool;
}

/// Trait for PDA types that can derive seeds with full account context access.
pub trait PdaSeedDerivation<A, S> {
    fn derive_pda_seeds_with_accounts(
        &self,
        program_id: &[u8; 32],
        accounts: &A,
        seed_params: &S,
    ) -> Result<(Vec<Vec<u8>>, [u8; 32]), LightSdkTypesError>;
}
