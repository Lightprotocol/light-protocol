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

/// Compressed mint data for decompression - enum variant wrapper.
/// Packed = Unpacked for now (noop), allowing future extensions.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum CompressedMintVariant {
    /// Standard compressed mint (packed = unpacked for now)
    Standard(CompressedMintTokenData),
}

/// The actual compressed mint token data.
/// Similar to light_ctoken_sdk::compat::CompressedMintData but with proper serialization.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct CompressedMintTokenData {
    /// Mint seed pubkey (used to derive CMint PDA)
    pub mint_seed_pubkey: Pubkey,
    /// Compressed mint with context (from indexer)
    pub compressed_mint_with_context: light_ctoken_sdk::ctoken::CompressedMintWithContext,
    /// Rent payment in epochs (0 or >= 2)
    pub rent_payment: u8,
    /// Lamports for future write operations
    pub write_top_up: u32,
}

/// Compressed account data for mint decompression.
/// Mirrors `CompressedAccountData` pattern from decompress_accounts_idempotent.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct CompressedMintAccountData {
    /// Merkle tree metadata (tree indices, leaf index, etc.)
    pub meta: CompressedAccountMetaNoLamportsNoAddress,
    /// The compressed mint data (with enum for future extensibility)
    pub data: CompressedMintVariant,
}

/// Parameters for decompressing compressed mints.
/// Mirrors `DecompressMultipleAccountsIdempotentData` structure.
///
/// Client-side validation: at most 1 mint allowed (error otherwise).
/// Works for both prove_by_index=true and prove_by_index=false.
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct DecompressCMintsParams {
    /// Validity proof covering all input mints
    pub proof: light_sdk::instruction::ValidityProof,
    /// Vec of compressed mint account data (at most 1, validated client-side)
    pub compressed_accounts: Vec<CompressedMintAccountData>,
    /// Offset where system accounts start in remaining_accounts
    pub system_accounts_offset: u8,
}

/// Accounts for decompressing compressed mints.
///
/// Remaining accounts (in order):
/// - ctoken_program (required for CPI)
/// - light_system_program
/// - cpi_authority_pda (ctoken's CPI authority)
/// - registered_program_pda
/// - account_compression_authority
/// - account_compression_program
/// - state_tree
/// - input_queue
/// - output_queue
/// - For each mint: [mint_signer_pda, cmint_pda]
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

/// ATA-specific token data for decompression.
/// For compression_only ATAs, the compressed account's owner = ATA pubkey (not wallet).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct CompressedAtaTokenData {
    /// Wallet owner (signs the transaction, used to derive ATA)
    pub wallet: Pubkey,
    /// Mint for this ATA
    pub mint: Pubkey,
    /// Amount in the compressed account
    pub amount: u64,
    /// Delegate (if any)
    pub delegate: Option<Pubkey>,
    /// Whether account is frozen
    pub is_frozen: bool,
}

/// Enum wrapper for ATA variants (future extensibility).
/// Packed = Unpacked for now (noop).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum CompressedAtaVariant {
    /// Standard compression_only ATA
    Standard(CompressedAtaTokenData),
}

/// Per-ATA compressed account data.
/// Mirrors `CompressedAccountData` pattern from decompress_accounts_idempotent.
/// Compressed ATA account data with explicit indices.
/// Indices allow arbitrary de-duplication (shared mints, shared wallets, etc.)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct CompressedAtaAccountData {
    /// Merkle tree metadata (tree indices, leaf index, etc.)
    pub meta: CompressedAccountMetaNoLamportsNoAddress,
    /// The compressed ATA data
    pub data: CompressedAtaVariant,
    /// Index of wallet account in packed_accounts (relative to packed_accounts start)
    pub wallet_index: u8,
    /// Index of mint account in packed_accounts
    pub mint_index: u8,
    /// Index of ATA account in packed_accounts
    pub ata_index: u8,
}

/// Parameters for decompressing compressed ATAs.
///
/// Key difference from CMints: ATAs CAN be batched in ONE CPI call.
/// Works for both prove_by_index=true and prove_by_index=false.
///
/// Packed accounts can be de-duplicated arbitrarily:
/// - Shared mint: use same mint_index for multiple ATAs
/// - Shared wallet: use same wallet_index
/// - Unique everything: each ATA has its own indices
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct DecompressAtasParams {
    /// Validity proof covering all input ATAs
    pub proof: light_sdk::instruction::ValidityProof,
    /// Vec of compressed ATA data (1 or more allowed)
    pub compressed_accounts: Vec<CompressedAtaAccountData>,
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
