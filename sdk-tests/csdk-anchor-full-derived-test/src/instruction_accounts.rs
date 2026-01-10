use anchor_lang::prelude::*;
use light_sdk_macros::LightFinalize;

use crate::state::*;

// =============================================================================
// APPROACH 2: Automatic Compression with LightFinalize (RECOMMENDED)
// =============================================================================
// This approach uses macros to auto-generate compression at instruction end.
// Minimal boilerplate - just add attributes and the macro handles the rest.

/// Simple PDA creation params for LightFinalize approach
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct SimplePdaParams {
    pub proof: light_sdk::instruction::ValidityProof,
    pub address_tree_info: light_sdk::instruction::PackedAddressTreeInfo,
    pub output_state_tree_index: u8,
}

/// Demonstrates FULL AUTOMATIC compression using LightFinalize derive.
/// No manual compression code needed - macro generates everything.
#[derive(Accounts, LightFinalize)]
#[instruction(params: SimplePdaParams)]
pub struct CreateSimplePda<'info> {
    /// Fee payer - LightFinalize looks for "fee_payer", "payer", or "creator"
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// The PDA to compress - marked with #[compressible] field attribute
    #[account(
        init,
        payer = fee_payer,
        space = 8 + NewStyleRecord::INIT_SPACE,
        seeds = [b"simple_pda", fee_payer.key().as_ref()],
        bump,
    )]
    #[compressible(
        address_tree_info = params.address_tree_info,
        output_tree = params.output_state_tree_index
    )]
    pub my_pda: Account<'info, NewStyleRecord>,

    /// Compression config - LightFinalize looks for field named "compression_config"
    /// CHECK: Validated by CompressibleConfig::load_checked
    pub compression_config: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

/// Create multiple PDAs with automatic compression
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct MultiPdaParams {
    pub proof: light_sdk::instruction::ValidityProof,
    pub user_address_tree_info: light_sdk::instruction::PackedAddressTreeInfo,
    pub game_address_tree_info: light_sdk::instruction::PackedAddressTreeInfo,
    pub output_state_tree_index: u8,
    // Data for initialization
    pub owner: Pubkey,
    pub category_id: u64,
    pub session_id: u64,
}

/// Multiple PDAs compressed automatically - each field with #[compressible]
#[derive(Accounts, LightFinalize)]
#[instruction(params: MultiPdaParams)]
pub struct CreateMultiplePdas<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// Authority for seeds
    pub authority: Signer<'info>,

    /// Mint authority for seeds
    pub mint_authority: Signer<'info>,

    /// First PDA - UserRecord
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

    /// Second PDA - GameSession
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

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

/// Full auto params: 2 PDAs in one instruction
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct FullAutoParams {
    pub proof: light_sdk::instruction::ValidityProof,
    // PDA compression params
    pub user_address_tree_info: light_sdk::instruction::PackedAddressTreeInfo,
    pub game_address_tree_info: light_sdk::instruction::PackedAddressTreeInfo,
    pub output_state_tree_index: u8,
    // Data for initialization
    pub owner: Pubkey,
    pub category_id: u64,
    pub session_id: u64,
}

/// FULL AUTOMATIC: Creates 2 PDAs in one instruction using LightFinalize.
/// - UserRecord PDA: #[compressible]
/// - GameSession PDA: #[compressible]
/// All batched together with a single proof execution!
///
/// NOTE: #[light_mint] support requires additional CPI context setup.
/// See the manual approach for mint + PDA examples.
#[derive(Accounts, LightFinalize)]
#[instruction(params: FullAutoParams)]
pub struct CreateUserRecordAndGameSessionAuto<'info> {
    /// Fee payer for all operations
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// Authority signer used in PDA seeds
    pub authority: Signer<'info>,

    /// Mint authority used in PDA seeds
    pub mint_authority: Signer<'info>,

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

    /// CHECK: Compression config - required by LightFinalize
    pub compression_config: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

// =============================================================================
// FULL AUTO WITH MINT: Creates PDAs + light_mint in ONE instruction
// =============================================================================
// This demonstrates the FULL macro capability: PDAs + mints batched together

/// Mint signer seed for the LP mint (similar to Raydium's POOL_LP_MINT_SEED)
pub const LP_MINT_SIGNER_SEED: &[u8] = b"lp_mint_signer";

/// Vault seed for program-owned CToken vault (like cp-swap's token vaults)
pub const AUTO_VAULT_SEED: &[u8] = b"auto_vault";

/// Vault authority seed
pub const AUTO_VAULT_AUTHORITY_SEED: &[u8] = b"auto_vault_authority";

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

// =============================================================================
// APPROACH 1: Manual Compression (more control, more code)
// =============================================================================
// Original manual approach - kept for comparison

#[derive(Accounts)]
#[instruction(account_data: AccountCreationData)]
pub struct CreateUserRecordAndGameSession<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// The mint signer used for PDA derivation
    pub mint_signer: Signer<'info>,

    #[account(
        init,
        payer = user,
        space = 8 + 32 + 4 + 32 + 8 + 8,
        seeds = [
            b"user_record",
            authority.key().as_ref(),
            mint_authority.key().as_ref(),
            account_data.owner.as_ref(),
            account_data.category_id.to_le_bytes().as_ref()
        ],
        bump,
    )]
    pub user_record: Account<'info, UserRecord>,
    #[account(
        init,
        payer = user,
        space = 8 + 8 + 32 + 4 + 32 + 8 + 9 + 8,
        seeds = [
            b"game_session",
            crate::max_key(&user.key(), &authority.key()).as_ref(),
            account_data.session_id.to_le_bytes().as_ref()
        ],
        bump,
    )]
    pub game_session: Account<'info, GameSession>,

    /// Authority signer used in PDA seeds
    pub authority: Signer<'info>,

    /// Mint authority signer used in PDA seeds
    pub mint_authority: Signer<'info>,

    /// Some account used in PlaceholderRecord PDA seeds
    /// CHECK: Used as seed component
    pub some_account: AccountInfo<'info>,

    /// Compressed token program
    /// CHECK: Program ID validated using C_TOKEN_PROGRAM_ID constant
    pub ctoken_program: UncheckedAccount<'info>,

    /// CHECK: CPI authority of the compressed token program
    pub compress_token_program_cpi_authority: UncheckedAccount<'info>,

    /// Needs to be here for the init anchor macro to work.
    pub system_program: Program<'info, System>,

    /// Global compressible config
    /// CHECK: Config is validated by the SDK's load_checked method
    pub config: AccountInfo<'info>,

    /// Rent recipient - must match config
    /// CHECK: Rent recipient is validated against the config
    #[account(mut)]
    pub rent_sponsor: AccountInfo<'info>,
}

/// Program-owned vault PDA seed
pub const VAULT_SEED: &[u8] = b"vault";

/// E2E test: Creates ALL accounts atomically in ONE instruction (like cp-swap):
/// - PlaceholderRecord PDA (compressed)
/// - UserRecord PDA (compressed) - 2nd PDA for multi-PDA coverage
/// - Light mint + decompress to CMint
/// - Program-owned CToken vault (created via CPI, compression_only=false)
/// - User ATA (created via CPI, compression_only=true, like cp-swap LP token ATA)
/// - MintTo both vault and user_ata
#[derive(Accounts)]
#[instruction(data: E2eTestData)]
pub struct E2eCreateMintDecompressAndToken<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The mint signer - used for light mint and CMint PDA derivation
    pub mint_signer: Signer<'info>,

    /// Mint authority for the light mint (mut for CTokenMintToCpi lamport transfers)
    #[account(mut)]
    pub mint_authority: Signer<'info>,

    /// Authority signer used in PDA seeds
    pub authority: Signer<'info>,

    /// Some account used in PlaceholderRecord PDA seeds
    /// CHECK: Used as seed component
    pub some_account: AccountInfo<'info>,

    /// PlaceholderRecord PDA (1st compressed PDA)
    #[account(
        init,
        payer = payer,
        space = 8 + PlaceholderRecord::INIT_SPACE,
        seeds = [
            b"placeholder_record",
            authority.key().as_ref(),
            some_account.key().as_ref(),
            data.placeholder_id.to_le_bytes().as_ref(),
            data.counter.to_le_bytes().as_ref()
        ],
        bump,
    )]
    pub placeholder_record: Account<'info, PlaceholderRecord>,

    /// UserRecord PDA (2nd compressed PDA for multi-PDA coverage)
    /// Seeds MUST match #[light_accounts] variant: UserRecord = ("user_record", ctx.authority, ctx.mint_authority, data.owner, data.category_id.to_le_bytes())
    #[account(
        init,
        payer = payer,
        space = 8 + UserRecord::INIT_SPACE,
        seeds = [
            b"user_record",
            authority.key().as_ref(),
            mint_authority.key().as_ref(),
            data.user_record_owner.as_ref(),
            data.user_record_category_id.to_le_bytes().as_ref()
        ],
        bump,
    )]
    pub user_record: Account<'info, UserRecord>,

    /// CMint account that will be created via decompression
    /// CHECK: Will be initialized by MintAction with DecompressMint
    #[account(mut)]
    pub cmint: UncheckedAccount<'info>,

    /// Program-owned CToken vault account (like cp-swap vaults)
    /// Seeds: ["vault", cmint]
    /// CHECK: Will be initialized via CreateCTokenAccountCpi inside instruction
    #[account(
        mut,
        seeds = [VAULT_SEED, cmint.key().as_ref()],
        bump,
    )]
    pub vault: UncheckedAccount<'info>,

    /// Vault authority (PDA that owns the vault)
    /// CHECK: Used as vault owner for CPI signing
    #[account(
        seeds = [b"vault_authority"],
        bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,

    /// User's ATA for the CMint (like cp-swap's creator_lp_token)
    /// CHECK: Will be initialized via CreateAssociatedCTokenAccountCpi inside instruction
    #[account(mut)]
    pub user_ata: UncheckedAccount<'info>,

    /// Compressed token program
    /// CHECK: Program ID validated
    pub ctoken_program: UncheckedAccount<'info>,

    /// CHECK: CPI authority of the compressed token program
    pub ctoken_cpi_authority: UncheckedAccount<'info>,

    /// CToken compressible config (ctoken-specific)
    /// CHECK: Validated by SDK
    pub ctoken_compressible_config: AccountInfo<'info>,

    /// CToken rent sponsor
    /// CHECK: Validated by SDK
    #[account(mut)]
    pub ctoken_rent_sponsor: AccountInfo<'info>,

    pub system_program: Program<'info, System>,

    /// Program config for compressible accounts
    /// CHECK: Validated by SDK
    pub config: AccountInfo<'info>,

    /// Rent sponsor for program
    /// CHECK: Validated against config
    #[account(mut)]
    pub rent_sponsor: AccountInfo<'info>,
}
