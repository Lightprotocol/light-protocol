use anchor_lang::prelude::*;
use light_sdk_macros::LightFinalize;

use crate::state::*;
/// Full auto params with mint: 2 PDAs + 1 CMint + vault + 2 user ATAs in one instruction
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct FullAutoWithMintParams {
    pub proof: light_sdk::instruction::ValidityProof,
    // PDA compression params
    pub user_address_tree_info: light_sdk::instruction::PackedAddressTreeInfo,
    pub game_address_tree_info: light_sdk::instruction::PackedAddressTreeInfo,
    // Mint compression params
    pub mint_address_tree_info: light_sdk::instruction::PackedAddressTreeInfo,
    pub output_state_tree_index: u8,
    // Data for initialization
    pub owner: Pubkey,
    pub category_id: u64,
    pub session_id: u64,
    // Mint signer bump for PDA signing
    pub mint_signer_bump: u8,
    // CToken vault/ATA params (like cp-swap)
    pub vault_bump: u8,
    pub vault_mint_amount: u64,
    // User 1 ATA params
    pub user1_ata_bump: u8,
    pub user1_ata_mint_amount: u64,
    // User 2 ATA params
    pub user2_ata_bump: u8,
    pub user2_ata_mint_amount: u64,
}
pub const LP_MINT_SIGNER_SEED: &[u8] = b"lp_mint_signer";

/// Vault seed for program-owned CToken vault (like cp-swap's token vaults)
pub const AUTO_VAULT_SEED: &[u8] = b"auto_vault";

/// Vault authority seed
pub const AUTO_VAULT_AUTHORITY_SEED: &[u8] = b"auto_vault_authority";

/// FULL AUTOMATIC WITH MINT: Creates 2 PDAs + 1 CMint + vault + 2 user ATAs in ONE instruction.
/// - UserRecord PDA: #[compressible]
/// - GameSession PDA: #[compressible]
/// - LP Mint: #[light_mint] (creates + decompresses atomically in pre_init)
/// - Vault: Program-owned CToken account (created in instruction body)
/// - User1 ATA: User1-owned CToken ATA (created in instruction body)
/// - User2 ATA: User2-owned CToken ATA (created in instruction body)
/// - MintTo: Mint tokens to vault, user1_ata, and user2_ata (in instruction body)
///
/// All batched together with a single proof execution!
/// This is the pattern used by protocols like Raydium cp-swap.
#[derive(Accounts, LightFinalize)]
#[instruction(params: FullAutoWithMintParams)]
pub struct CreatePdasAndMintAuto<'info> {
    /// Fee payer for all operations
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// Authority signer used in PDA seeds
    pub authority: Signer<'info>,

    /// Mint authority for the LP mint operations
    #[account(mut)]
    pub mint_authority: Signer<'info>,

    /// User1 wallet - owner of user1_ata
    pub user1: Signer<'info>,

    /// User2 wallet - owner of user2_ata
    pub user2: Signer<'info>,

    /// Mint signer PDA - seeds the CMint address (like Raydium's lp_mint_signer)
    /// CHECK: PDA derived from pool state or authority
    #[account(
        seeds = [LP_MINT_SIGNER_SEED, authority.key().as_ref()],
        bump,
    )]
    pub mint_signer: UncheckedAccount<'info>,

    /// UserRecord PDA - compressed automatically via #[compressible]
    #[account(
        init,
        payer = fee_payer,
        space = 8 + UserRecord::INIT_SPACE,
        seeds = [
            b"user_record",
            authority.key().as_ref(),
            mint_authority.key().as_ref(),
            params.owner.as_ref(),
            params.category_id.to_le_bytes().as_ref()
        ],
        bump,
    )]
    #[compressible(
        address_tree_info = params.user_address_tree_info,
        output_tree = params.output_state_tree_index
    )]
    pub user_record: Account<'info, UserRecord>,

    /// GameSession PDA - compressed automatically via #[compressible]
    #[account(
        init,
        payer = fee_payer,
        space = 8 + GameSession::INIT_SPACE,
        seeds = [
            b"game_session",
            crate::max_key(&fee_payer.key(), &authority.key()).as_ref(),
            params.session_id.to_le_bytes().as_ref()
        ],
        bump,
    )]
    #[compressible(
        address_tree_info = params.game_address_tree_info,
        output_tree = params.output_state_tree_index
    )]
    pub game_session: Account<'info, GameSession>,

    /// CMint - created + decompressed atomically via #[light_mint]
    /// CHECK: Will be initialized by mint_action with DecompressMint in pre_init
    #[account(mut)]
    #[light_mint(
        mint_signer = mint_signer,
        authority = mint_authority,
        decimals = 9,
        address_tree_info = params.mint_address_tree_info,
        signer_seeds = &[LP_MINT_SIGNER_SEED, self.authority.to_account_info().key.as_ref(), &[params.mint_signer_bump]]
    )]
    pub cmint: UncheckedAccount<'info>,

    /// Program-owned CToken vault (like cp-swap's token vaults)
    /// Seeds: ["vault", cmint] - matches variant definition
    /// CHECK: Will be initialized via CreateCTokenAccountCpi in instruction body
    #[account(
        mut,
        seeds = [VAULT_SEED, cmint.key().as_ref()],
        bump,
    )]
    pub vault: UncheckedAccount<'info>,

    /// Vault authority PDA - owns the vault (like cp-swap's authority)
    /// Seeds: ["vault_authority"] - matches variant authority definition
    /// CHECK: PDA used as vault owner
    #[account(
        seeds = [b"vault_authority"],
        bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,

    /// User1's ATA for the CMint
    /// CHECK: Will be initialized via CreateAssociatedCTokenAccountCpi in instruction body
    #[account(mut)]
    pub user1_ata: UncheckedAccount<'info>,

    /// User2's ATA for the CMint
    /// CHECK: Will be initialized via CreateAssociatedCTokenAccountCpi in instruction body
    #[account(mut)]
    pub user2_ata: UncheckedAccount<'info>,

    /// CHECK: Compression config - required by LightFinalize
    pub compression_config: AccountInfo<'info>,

    /// CToken compressible config - required for decompress mint and CToken accounts
    /// CHECK: Validated by SDK
    pub ctoken_compressible_config: AccountInfo<'info>,

    /// CToken rent sponsor - required for decompress mint and CToken accounts
    /// CHECK: Validated by SDK
    #[account(mut)]
    pub ctoken_rent_sponsor: AccountInfo<'info>,

    /// Compressed token program - required for mint_action
    /// CHECK: Program ID validated
    pub ctoken_program: AccountInfo<'info>,

    /// CToken CPI authority PDA - required for mint_action
    /// CHECK: Validated by SDK
    pub ctoken_cpi_authority: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

/// Program-owned vault PDA seed
pub const VAULT_SEED: &[u8] = b"vault";

// ============================================================================
// DecompressCMints - Decompress compressed mints (at most 1, client-validated)
// ============================================================================

use light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;

/// PACKED compressed mint token data.
/// Pubkeys that are actual Solana accounts -> indices into packed_accounts.
/// Compressed address is raw data (not a Solana account), kept as [u8; 32].
///
/// Size comparison (per mint, no extensions):
/// - Unpacked: ~180 bytes (5 pubkeys @ 32 bytes each + fixed fields)
/// - Packed: ~50 bytes (2 pubkey indices + 1 raw address + fixed fields)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PackedMintTokenData {
    /// Index of mint_seed account (used to derive CMint PDA)
    pub mint_seed_index: u8,
    /// Index of CMint PDA in packed_accounts
    pub cmint_pda_index: u8,
    /// Compressed address (Light protocol address) - raw data, NOT an account
    pub compressed_address: [u8; 32],
    /// Merkle tree leaf index
    pub leaf_index: u32,
    /// Whether to prove by index
    pub prove_by_index: bool,
    /// Root index for proof
    pub root_index: u16,
    /// Token supply
    pub supply: u64,
    /// Token decimals
    pub decimals: u8,
    /// Metadata version
    pub version: u8,
    /// Whether mint has been decompressed
    pub cmint_decompressed: bool,
    /// Whether mint authority exists
    pub has_mint_authority: bool,
    /// Index of mint authority (0 if none)
    pub mint_authority_index: u8,
    /// Whether freeze authority exists
    pub has_freeze_authority: bool,
    /// Index of freeze authority (0 if none)
    pub freeze_authority_index: u8,
    /// Rent payment epochs
    pub rent_payment: u8,
    /// Write top up lamports
    pub write_top_up: u32,
    /// Extensions (kept as-is, variable size metadata)
    pub extensions:
        Option<Vec<light_ctoken_interface::instructions::extensions::ExtensionInstructionData>>,
}

/// Enum wrapper for packed mint variants (future extensibility).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum PackedMintVariant {
    /// Standard packed compressed mint
    Standard(PackedMintTokenData),
}

/// Per-mint compressed account data with PACKED token data.
/// All pubkeys represented as indices into packed_accounts.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PackedMintAccountData {
    /// Merkle tree metadata (tree indices, leaf index, etc.)
    pub meta: CompressedAccountMetaNoLamportsNoAddress,
    /// The packed mint data (indices only)
    pub data: PackedMintVariant,
}

/// Parameters for decompressing compressed mints.
///
/// Client-side validation: at most 1 mint allowed (error otherwise).
/// Works for both prove_by_index=true and prove_by_index=false.
///
/// remaining_accounts layout:
/// [0..N] packed_accounts (de-duplicated pubkeys, client uses PackedAccounts)
///
/// System accounts must be at indices 0-5:
/// [0] ctoken_program
/// [1] light_system_program
/// [2] cpi_authority_pda
/// [3] registered_program_pda
/// [4] account_compression_authority
/// [5] account_compression_program
/// Then: state_tree, input_queue, output_queue, mint_seed, cmint_pda, etc.
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct DecompressCMintsParams {
    /// Validity proof covering all input mints
    pub proof: light_sdk::instruction::ValidityProof,
    /// Vec of packed mint account data (at most 1, validated client-side)
    pub compressed_accounts: Vec<PackedMintAccountData>,
    /// Offset where system accounts start in remaining_accounts (typically 0)
    pub system_accounts_offset: u8,
}

/// Accounts for decompressing compressed mints.
///
/// remaining_accounts contains all packed accounts (indices reference into this).
#[derive(Accounts)]
pub struct DecompressCMints<'info> {
    /// Fee payer for all operations
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// Authority for the mints (must sign)
    pub authority: Signer<'info>,

    /// Ctoken compressible config
    /// CHECK: Validated by ctoken program
    pub ctoken_compressible_config: AccountInfo<'info>,

    /// Ctoken rent sponsor
    /// CHECK: Validated by ctoken program
    #[account(mut)]
    pub ctoken_rent_sponsor: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// DecompressAtas - Decompress compressed ATAs (1 or more, batched in ONE CPI)
// ============================================================================

/// PACKED ATA token data for decompression (14 bytes total).
/// Uses indices instead of full Pubkeys - client packs, on-chain unpacks.
/// Mirrors ctoken's MultiTokenTransferOutputData pattern.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PackedAtaTokenData {
    /// Index of wallet account in packed_accounts (signer, used to derive ATA)
    pub wallet_index: u8,
    /// Index of mint account in packed_accounts
    pub mint_index: u8,
    /// Index of ATA account in packed_accounts (derived from wallet + mint)
    pub ata_index: u8,
    /// Amount in the compressed account
    pub amount: u64,
    /// Whether delegate is set
    pub has_delegate: bool,
    /// Index of delegate in packed_accounts (0 if none)
    pub delegate_index: u8,
    /// Whether account is frozen
    pub is_frozen: bool,
}

/// Enum wrapper for packed ATA variants (future extensibility).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum PackedAtaVariant {
    /// Standard compression_only ATA
    Standard(PackedAtaTokenData),
}

/// Per-ATA compressed account data with PACKED token data.
/// All pubkeys are represented as indices into packed_accounts.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PackedAtaAccountData {
    /// Merkle tree metadata (tree indices, leaf index, etc.)
    pub meta: CompressedAccountMetaNoLamportsNoAddress,
    /// The packed ATA data (indices only, ~14 bytes)
    pub data: PackedAtaVariant,
}

/// Parameters for decompressing compressed ATAs.
///
/// Key difference from CMints: ATAs CAN be batched in ONE CPI call.
/// Works for both prove_by_index=true and prove_by_index=false.
///
/// Uses PACKED data - indices only, no full Pubkeys. ~14 bytes per ATA vs ~77 bytes.
/// Packed accounts can be de-duplicated arbitrarily via shared indices.
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct DecompressAtasParams {
    /// Validity proof covering all input ATAs
    pub proof: light_sdk::instruction::ValidityProof,
    /// Vec of PACKED ATA data (indices only)
    pub compressed_accounts: Vec<PackedAtaAccountData>,
    /// Offset where system accounts start in remaining_accounts
    pub system_accounts_offset: u8,
}

/// Accounts for decompressing compressed ATAs.
///
/// Remaining accounts layout:
/// [0] ctoken_program (required for CPI)
/// [1-5] system accounts (light_system, cpi_auth, registered, compression_auth, compression_prog)
/// [6+] packed_accounts (arbitrary order, referenced by indices in params)
#[derive(Accounts)]
pub struct DecompressAtas<'info> {
    /// Fee payer for all operations
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// Ctoken compressible config
    /// CHECK: Validated by ctoken program
    pub ctoken_compressible_config: AccountInfo<'info>,

    /// Ctoken rent sponsor
    /// CHECK: Validated by ctoken program
    #[account(mut)]
    pub ctoken_rent_sponsor: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// DecompressUnified - Unified decompression for ATAs and CMints
// ============================================================================

/// Unified enum for decompression variants.
/// Allows mixing ATAs and CMints in a single instruction.

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum DecompressVariant {
    /// Compressed ATA - packed token data
    Ata(PackedAtaTokenData),
    /// Compressed Mint - packed mint data
    Mint(PackedMintTokenData),
}

/// Unified account data for decompression.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct DecompressUnifiedAccountData {
    /// Merkle tree metadata
    pub meta: CompressedAccountMetaNoLamportsNoAddress,
    /// The account data (ATA or Mint variant)
    pub data: DecompressVariant,
}

/// Parameters for unified decompression.
/// - Any number of ATAs allowed
/// - At most 1 CMint allowed (error on-chain if >1)
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct DecompressUnifiedParams {
    /// Validity proof covering ALL inputs
    pub proof: light_sdk::instruction::ValidityProof,
    /// Accounts to decompress (any mix of ATAs and CMints)
    pub compressed_accounts: Vec<DecompressUnifiedAccountData>,
    /// Offset where system accounts start in remaining_accounts
    pub system_accounts_offset: u8,
}

/// Accounts for unified decompression.
///
/// remaining_accounts layout (via PackedAccounts):
/// [0] ctoken_program, [1] light_system_program, [2] cpi_authority,
/// [3] registered_program, [4] acc_compression_authority, [5] acc_compression_program,
/// [6] cpi_context (if mint+atas), then trees, mints, wallets, atas, cmint_pda, mint_seed...
#[derive(Accounts)]
#[instruction(params: DecompressUnifiedParams)]
pub struct DecompressUnified<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// Authority for mints (must sign if decompressing mint)
    pub authority: Signer<'info>,

    /// CToken compressible config
    /// CHECK: Validated by ctoken program
    pub ctoken_compressible_config: AccountInfo<'info>,

    /// CToken rent sponsor
    /// CHECK: Validated by ctoken program
    #[account(mut)]
    pub ctoken_rent_sponsor: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}
