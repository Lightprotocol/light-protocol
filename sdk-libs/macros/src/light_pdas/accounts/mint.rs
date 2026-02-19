//! Light mint code generation.
//!
//! This module handles code generation for mint_action CPI invocations.
//! Parsing is handled by `light_account.rs`.
//!
//! ## Code Generation
//!
//! Two cases for mint_action CPI:
//! - **With CPI context**: Batching mint creation with PDA compression
//! - **Without CPI context**: Mint-only instructions
//!
//! See `CpiContextParts` for what differs between these cases.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, Ident};

use super::parse::InfraFields;

// ============================================================================
// Field Types
// ============================================================================

/// A field marked with #[light_account(init, mint, ...)]
#[derive(Debug)]
pub(crate) struct LightMintField {
    /// The field name where #[light_account(init, mint, ...)] is attached (Mint account)
    pub field_ident: Ident,
    /// The mint_signer field (AccountInfo that seeds the mint PDA)
    pub mint_signer: Expr,
    /// The authority for mint operations
    pub authority: Expr,
    /// Decimals for the mint
    pub decimals: Expr,
    /// Optional freeze authority
    pub freeze_authority: Option<Ident>,
    /// Signer seeds for the mint_signer PDA (required, WITHOUT bump - bump is auto-derived or provided via mint_bump)
    pub mint_seeds: Expr,
    /// Optional bump for mint_seeds. If None, auto-derived using find_program_address.
    pub mint_bump: Option<Expr>,
    /// Signer seeds for the authority PDA (optional - if not provided, authority must be a tx signer, WITHOUT bump)
    pub authority_seeds: Option<Expr>,
    /// Optional bump for authority_seeds. If None, auto-derived using find_program_address.
    pub authority_bump: Option<Expr>,
    // Metadata extension fields
    /// Token name for TokenMetadata extension
    pub name: Option<Expr>,
    /// Token symbol for TokenMetadata extension
    pub symbol: Option<Expr>,
    /// Token URI for TokenMetadata extension
    pub uri: Option<Expr>,
    /// Update authority field reference for TokenMetadata extension
    pub update_authority: Option<Ident>,
    /// Additional metadata key-value pairs for TokenMetadata extension
    pub additional_metadata: Option<Expr>,
}

// ============================================================================
// Code Generation
// ============================================================================

/// Resolve optional field name to TokenStream, using default if None.
fn resolve_field_name(field: &Option<syn::Ident>, default: &str) -> TokenStream {
    field.as_ref().map(|f| quote! { #f }).unwrap_or_else(|| {
        let ident = format_ident!("{}", default);
        quote! { #ident }
    })
}

/// Resolved infrastructure field names as TokenStreams.
///
/// Single source of truth for infrastructure fields used across code generation.
pub(super) struct InfraRefs {
    pub fee_payer: TokenStream,
    pub compression_config: TokenStream,
    pub pda_rent_sponsor: TokenStream,
    pub light_token_config: TokenStream,
    pub light_token_rent_sponsor: TokenStream,
    pub light_token_cpi_authority: TokenStream,
}

impl InfraRefs {
    /// Construct from parsed InfraFields, applying defaults for missing fields.
    pub fn from_parsed(infra: &InfraFields) -> Self {
        Self {
            fee_payer: resolve_field_name(&infra.fee_payer, "fee_payer"),
            compression_config: resolve_field_name(&infra.compression_config, "compression_config"),
            pda_rent_sponsor: resolve_field_name(&infra.pda_rent_sponsor, "pda_rent_sponsor"),
            light_token_config: resolve_field_name(&infra.light_token_config, "light_token_config"),
            light_token_rent_sponsor: resolve_field_name(
                &infra.light_token_rent_sponsor,
                "light_token_rent_sponsor",
            ),
            light_token_cpi_authority: resolve_field_name(
                &infra.light_token_cpi_authority,
                "light_token_cpi_authority",
            ),
        }
    }
}
