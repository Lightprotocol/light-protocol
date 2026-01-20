//! Traits for decompression variant construction.
//!
//! These traits enable ergonomic client-side construction of `RentFreeDecompressAccount`
//! from seeds and compressed account data.

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

/// Trait for CToken account variant types that can construct a full variant with token data.
///
/// Implemented by generated `TokenAccountVariant` enum.
/// The macro generates the impl that wraps variant + token_data into `RentFreeAccountVariant`.
///
/// # Example (generated code)
/// ```ignore
/// impl IntoCTokenVariant<RentFreeAccountVariant> for TokenAccountVariant {
///     fn into_ctoken_variant(self, token_data: TokenData) -> RentFreeAccountVariant {
///         RentFreeAccountVariant::CTokenData(CTokenData {
///             variant: self,
///             token_data,
///         })
///     }
/// }
/// ```
///
/// Type parameter `T` is typically `light_token::compat::TokenData`.
pub trait IntoCTokenVariant<V, T> {
    /// Construct variant from CToken variant and token data.
    ///
    /// # Arguments
    /// * `token_data` - The parsed `TokenData` from compressed account bytes
    ///
    /// # Returns
    /// The constructed variant containing both CToken variant and token data
    fn into_ctoken_variant(self, token_data: T) -> V;
}
