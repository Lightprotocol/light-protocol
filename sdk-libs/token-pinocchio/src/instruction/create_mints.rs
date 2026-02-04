//! Batch create Light Mints CPI for pinocchio.
//!
//! Provides type aliases with pinocchio's `AccountInfo` already set,
//! wrapping the generic types from `light_sdk_types`.
//!
//! # Flow
//!
//! - N=1 (no CPI context offset): Single CPI (create + decompress)
//! - N>1 or offset>0: 2N-1 CPIs (N-1 writes + 1 execute with decompress + N-1 decompress)
//!
//! # Example
//!
//! ```rust,ignore
//! use light_token_pinocchio::instruction::{
//!     CreateMintsCpi, CreateMintsParams, SingleMintParams,
//!     get_output_queue_next_index,
//! };
//!
//! // Get base leaf index before any CPIs
//! let base_leaf_index = get_output_queue_next_index(&output_queue)?;
//!
//! // mint and compression_address are derived internally from mint_seed_pubkey
//! let mint_params = [SingleMintParams {
//!     decimals: 9,
//!     address_merkle_tree_root_index: root_index,
//!     mint_authority: authority_key,
//!     mint_bump: None, // derived from mint_seed_pubkey if None
//!     freeze_authority: None,
//!     mint_seed_pubkey: mint_seed_key,
//!     authority_seeds: None,
//!     mint_signer_seeds: Some(&mint_signer_seeds),
//!     token_metadata: None,
//! }];
//!
//! let params = CreateMintsParams {
//!     mints: &mint_params,
//!     proof,
//!     rent_payment: 16,
//!     write_top_up: 766,
//!     cpi_context_offset: 0,
//!     output_queue_index: 0,
//!     address_tree_index: 1,
//!     state_tree_index: 2,
//!     base_leaf_index,
//! };
//!
//! CreateMintsCpi {
//!     mint_seed_accounts: &[&mint_seed_account],
//!     payer: &payer,
//!     address_tree: &address_tree,
//!     output_queue: &output_queue,
//!     state_merkle_tree: &state_merkle_tree,
//!     compressible_config: &config,
//!     mints: &[&mint_pda_account],
//!     rent_sponsor: &rent_sponsor,
//!     light_system_program: &light_system,
//!     cpi_authority_pda: &cpi_authority,
//!     registered_program_pda: &registered_program,
//!     account_compression_authority: &compression_authority,
//!     account_compression_program: &compression_program,
//!     system_program: &system_program,
//!     cpi_context_account: &cpi_context,
//!     params,
//! }.invoke()?;
//! ```

// Re-export non-generic types and functions directly
pub use light_sdk_types::interface::cpi::create_mints::{
    get_output_queue_next_index, CreateMintsParams, SingleMintParams, DEFAULT_RENT_PAYMENT,
    DEFAULT_WRITE_TOP_UP,
};
use pinocchio::account_info::AccountInfo;

/// High-level struct for creating compressed mints (pinocchio).
///
/// Type alias with pinocchio's `AccountInfo` already set.
/// Consolidates proof parsing, tree account resolution, and CPI invocation.
///
/// # Example
///
/// ```rust,ignore
/// CreateMints {
///     mints: &sdk_mints,
///     proof_data: &params.create_accounts_proof,
///     mint_seed_accounts,
///     mint_accounts,
///     infra,
/// }
/// .invoke(&cpi_accounts)?;
/// ```
pub type CreateMints<'a> =
    light_sdk_types::interface::cpi::create_mints::CreateMints<'a, AccountInfo>;

/// CPI struct for creating multiple Light Mints (pinocchio).
///
/// Type alias with pinocchio's `AccountInfo` already set.
pub type CreateMintsCpi<'a> =
    light_sdk_types::interface::cpi::create_mints::CreateMintsCpi<'a, AccountInfo>;

/// Infrastructure accounts for mint creation CPI (pinocchio).
///
/// Type alias with pinocchio's `AccountInfo` already set.
pub type CreateMintsStaticAccounts<'a> =
    light_sdk_types::interface::cpi::create_mints::CreateMintsStaticAccounts<'a, AccountInfo>;

/// Find the mint PDA address for a given mint seed (pinocchio).
///
/// Returns `([u8; 32], u8)` -- the PDA address bytes and bump.
pub fn find_mint_address(mint_seed: &[u8; 32]) -> ([u8; 32], u8) {
    light_sdk_types::interface::cpi::create_mints::find_mint_address::<AccountInfo>(mint_seed)
}

/// Derive the Light Mint address from a mint seed and address tree pubkey (pinocchio).
///
/// This computes `derive_address(find_mint_address(mint_seed).0, address_tree, LIGHT_TOKEN_PROGRAM_ID)`.
pub fn derive_mint_compressed_address(
    mint_seed: &[u8; 32],
    address_tree_pubkey: &[u8; 32],
) -> [u8; 32] {
    light_sdk_types::interface::cpi::create_mints::derive_mint_compressed_address::<AccountInfo>(
        mint_seed,
        address_tree_pubkey,
    )
}
