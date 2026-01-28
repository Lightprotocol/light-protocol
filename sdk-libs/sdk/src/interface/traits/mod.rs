//! Traits for decompression variant construction and manual Light Protocol implementation.

// --- v1 trait definitions (always available) ---

#[cfg(feature = "anchor")]
use anchor_lang::error::Error;
#[cfg(not(feature = "anchor"))]
use solana_program_error::ProgramError as Error;

/// Trait for seeds that can construct a compressed account variant.
///
/// Implemented by generated `XxxSeeds` structs (e.g., `UserRecordSeeds`).
/// The macro generates impls that deserialize account data and verify seeds match.
///
/// # Example (generated code)
/// ```ignore
/// impl IntoVariant<RentFreeAccountVariant> for UserRecordSeeds {
///     fn into_variant(self, data: &[u8]) -> Result<RentFreeAccountVariant, Error> {
///         RentFreeAccountVariant::user_record(data, self)
///     }
/// }
/// ```
pub trait IntoVariant<V> {
    /// Construct variant from compressed account data bytes and these seeds.
    ///
    /// # Arguments
    /// * `data` - Raw compressed account data bytes
    ///
    /// # Returns
    /// The constructed variant on success, or an error if:
    /// - Deserialization fails
    /// - Seed verification fails (data.* seeds don't match account data)
    fn into_variant(self, data: &[u8]) -> Result<V, Error>;
}

pub trait PdaSeeds<Accounts, const N: usize> {
    fn seeds<'a>(&'a self, accounts: &'a Accounts) -> [&'a [u8]; N];
}

// --- v2 trait submodules (anchor-gated) ---

#[cfg(feature = "anchor")]
pub mod light_account;
#[cfg(feature = "anchor")]
pub mod variant;

#[cfg(feature = "anchor")]
pub use light_account::{AccountType, LightAccount};
#[cfg(feature = "anchor")]
pub use variant::{LightAccountVariantTrait, PackedLightAccountVariantTrait};
