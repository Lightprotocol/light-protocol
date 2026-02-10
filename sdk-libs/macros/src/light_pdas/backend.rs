//! Code generation backend abstraction.
//!
//! This module provides a trait-based abstraction for generating code that differs
//! between Anchor and Pinocchio frameworks. Instead of duplicating methods with
//! `_pinocchio` suffixes, code generators accept a `CodegenBackend` implementation
//! that provides framework-specific types, paths, and conversions.
//!
//! # Example
//!
//! ```ignore
//! // Instead of:
//! fn generate_foo(&self) -> TokenStream { ... }
//! fn generate_foo_pinocchio(&self) -> TokenStream { ... }
//!
//! // Use:
//! fn generate_foo_with_backend(&self, backend: &dyn CodegenBackend) -> TokenStream { ... }
//! ```

use proc_macro2::TokenStream;
use quote::quote;

/// Trait for code generation backends (Anchor vs Pinocchio).
///
/// This trait encapsulates all differences between Anchor and Pinocchio code generation:
/// - Serialization derives (Anchor vs Borsh)
/// - Crate paths (light_account vs light_account_pinocchio)
/// - Type representations (Pubkey vs [u8; 32])
/// - Account info types
pub trait CodegenBackend {
    // -------------------------------------------------------------------------
    // Serialization Derives
    // -------------------------------------------------------------------------

    /// Returns the serialize derive attribute (e.g., `anchor_lang::AnchorSerialize` or `borsh::BorshSerialize`).
    fn serialize_derive(&self) -> TokenStream;

    /// Returns the deserialize derive attribute (e.g., `anchor_lang::AnchorDeserialize` or `borsh::BorshDeserialize`).
    fn deserialize_derive(&self) -> TokenStream;

    // -------------------------------------------------------------------------
    // Crate Paths
    // -------------------------------------------------------------------------

    /// Returns the account crate path (`light_account` or `light_account_pinocchio`).
    fn account_crate(&self) -> TokenStream;

    /// Returns the account info trait path.
    fn account_info_trait(&self) -> TokenStream;

    // -------------------------------------------------------------------------
    // Types
    // -------------------------------------------------------------------------

    /// Returns the account info type for function signatures.
    fn account_info_type(&self) -> TokenStream;

    /// Returns the packed accounts type for Pack trait implementations.
    fn packed_accounts_type(&self) -> TokenStream;

    /// Returns the account meta type for Pack trait bounds.
    fn account_meta_type(&self) -> TokenStream;

    // -------------------------------------------------------------------------
    // Flags
    // -------------------------------------------------------------------------

    /// Returns true if this is the Pinocchio backend.
    fn is_pinocchio(&self) -> bool;

    // -------------------------------------------------------------------------
    // Error Handling
    // -------------------------------------------------------------------------

    /// Returns the error type for SDK errors.
    fn sdk_error_type(&self) -> TokenStream;

    /// Returns the program error type.
    fn program_error_type(&self) -> TokenStream;

    /// Returns the borrow error conversion for account data borrowing.
    fn borrow_error(&self) -> TokenStream;
}

/// Anchor backend implementation.
///
/// Uses:
/// - `anchor_lang::AnchorSerialize/AnchorDeserialize` for serialization
/// - `light_account::` crate paths
/// - `Pubkey` type for public keys
/// - `anchor_lang::prelude::AccountInfo` for account info
pub struct AnchorBackend;

impl CodegenBackend for AnchorBackend {
    fn serialize_derive(&self) -> TokenStream {
        quote! { anchor_lang::AnchorSerialize }
    }

    fn deserialize_derive(&self) -> TokenStream {
        quote! { anchor_lang::AnchorDeserialize }
    }

    fn account_crate(&self) -> TokenStream {
        quote! { light_account }
    }

    fn account_info_trait(&self) -> TokenStream {
        quote! { light_account::AccountInfoTrait }
    }

    fn account_info_type(&self) -> TokenStream {
        quote! { anchor_lang::prelude::AccountInfo }
    }

    fn packed_accounts_type(&self) -> TokenStream {
        quote! { light_account::interface::instruction::PackedAccounts<AM> }
    }

    fn account_meta_type(&self) -> TokenStream {
        quote! { light_account::AccountMetaTrait }
    }

    fn is_pinocchio(&self) -> bool {
        false
    }

    fn sdk_error_type(&self) -> TokenStream {
        quote! { light_account::LightSdkTypesError }
    }

    fn program_error_type(&self) -> TokenStream {
        quote! { anchor_lang::error::Error }
    }

    fn borrow_error(&self) -> TokenStream {
        quote! { ? }
    }
}

/// Pinocchio backend implementation.
///
/// Uses:
/// - `borsh::BorshSerialize/BorshDeserialize` for serialization
/// - `light_account_pinocchio::` crate paths
/// - `[u8; 32]` type for public keys
/// - `pinocchio::account_info::AccountInfo` for account info
pub struct PinocchioBackend;

impl CodegenBackend for PinocchioBackend {
    fn serialize_derive(&self) -> TokenStream {
        quote! { borsh::BorshSerialize }
    }

    fn deserialize_derive(&self) -> TokenStream {
        quote! { borsh::BorshDeserialize }
    }

    fn account_crate(&self) -> TokenStream {
        quote! { light_account_pinocchio }
    }

    fn account_info_trait(&self) -> TokenStream {
        quote! { light_account_pinocchio::light_account_checks::AccountInfoTrait }
    }

    fn account_info_type(&self) -> TokenStream {
        quote! { pinocchio::account_info::AccountInfo }
    }

    fn packed_accounts_type(&self) -> TokenStream {
        quote! { light_account_pinocchio::PackedAccounts }
    }

    fn account_meta_type(&self) -> TokenStream {
        quote! { light_account_pinocchio::solana_instruction::AccountMeta }
    }

    fn is_pinocchio(&self) -> bool {
        true
    }

    fn sdk_error_type(&self) -> TokenStream {
        quote! { light_account_pinocchio::LightSdkTypesError }
    }

    fn program_error_type(&self) -> TokenStream {
        quote! { pinocchio::program_error::ProgramError }
    }

    fn borrow_error(&self) -> TokenStream {
        quote! { .map_err(|_| light_account_pinocchio::LightSdkTypesError::Borsh)? }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anchor_backend_types() {
        let backend = AnchorBackend;
        assert!(!backend.is_pinocchio());
    }

    #[test]
    fn test_pinocchio_backend_types() {
        let backend = PinocchioBackend;
        assert!(backend.is_pinocchio());
    }
}
