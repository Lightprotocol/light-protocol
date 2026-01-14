use anchor_lang::prelude::*;
use light_sdk_macros::LightFinalize;

use crate::state::*;
/// Full auto params with mint: 2 PDAs + 1 CMint + vault + user_ata in one instruction
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
    pub user_ata_bump: u8,
    pub vault_mint_amount: u64,
    pub user_ata_mint_amount: u64,
}
pub const LP_MINT_SIGNER_SEED: &[u8] = b"lp_mint_signer";

/// Vault seed for program-owned CToken vault (like cp-swap's token vaults)
pub const AUTO_VAULT_SEED: &[u8] = b"auto_vault";

/// Vault authority seed
pub const AUTO_VAULT_AUTHORITY_SEED: &[u8] = b"auto_vault_authority";

/// FULL AUTOMATIC WITH MINT: Creates 2 PDAs + 1 CMint + vault + user_ata in ONE instruction.
/// - UserRecord PDA: #[compressible]
/// - GameSession PDA: #[compressible]
/// - LP Mint: #[light_mint] (creates + decompresses atomically in pre_init)
/// - Vault: Program-owned CToken account (created in instruction body)
/// - User ATA: User-owned CToken ATA (created in instruction body)
/// - MintTo: Mint tokens to both vault and user_ata (in instruction body)
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

    /// User's ATA for the CMint (like cp-swap's creator_lp_token)
    /// CHECK: Will be initialized via CreateAssociatedCTokenAccountCpi in instruction body
    #[account(mut)]
    pub user_ata: UncheckedAccount<'info>,

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
