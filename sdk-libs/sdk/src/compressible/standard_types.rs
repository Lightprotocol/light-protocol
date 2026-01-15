//! Standard types for unified decompression.
//!
//! These types are automatically included in `CompressedAccountVariant` by the
//! `#[compressible]` macro - programs don't declare them.

#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};

/// Standard ATA for unified decompression.
///
/// Used for decompressing user-owned Associated Token Accounts.
/// The wallet must sign the transaction (not the program).
///
/// Indices reference packed_accounts (post-system accounts in remaining_accounts).
#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct LightAta {
    /// Index into packed_accounts for wallet (must be signer)
    pub wallet_index: u8,
    /// Index into packed_accounts for mint
    pub mint_index: u8,
    /// Index into packed_accounts for derived ATA address
    pub ata_index: u8,
    /// Token amount to decompress
    pub amount: u64,
    /// Whether the token has a delegate
    pub has_delegate: bool,
    /// Delegate index (only valid if has_delegate is true)
    pub delegate_index: u8,
    /// Whether the token is frozen
    pub is_frozen: bool,
}

/// Standard CMint for unified decompression.
///
/// Used for decompressing Compressed Mints to CMint accounts.
/// The mint authority must sign (or fee_payer if it's the authority).
///
/// Some fields are indices into packed_accounts (post-system accounts),
/// others are raw data (compressed_address, mint metadata).
#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub struct LightMint {
    // === Account indices ===
    /// Index into packed_accounts for mint_seed pubkey
    pub mint_seed_index: u8,
    /// Index into packed_accounts for derived CMint PDA
    pub cmint_pda_index: u8,
    /// Whether the mint has a mint authority
    pub has_mint_authority: bool,
    /// Mint authority index (only valid if has_mint_authority is true)
    pub mint_authority_index: u8,
    /// Whether the mint has a freeze authority
    pub has_freeze_authority: bool,
    /// Freeze authority index (only valid if has_freeze_authority is true)
    pub freeze_authority_index: u8,

    // === Raw data (not indices) ===
    /// Compressed account address (Light protocol address hash)
    pub compressed_address: [u8; 32],
    /// Token decimals
    pub decimals: u8,
    /// Total supply
    pub supply: u64,
    /// Metadata version
    pub version: u8,
    /// Whether mint has been decompressed before
    pub cmint_decompressed: bool,
    /// Rent payment in epochs (must be >= 2)
    pub rent_payment: u8,
    /// Lamports for future write operations
    pub write_top_up: u32,
    /// Extensions data (if any) - serialized ExtensionInstructionData
    pub extensions: Option<Vec<u8>>,
}
