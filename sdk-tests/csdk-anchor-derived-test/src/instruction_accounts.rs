use anchor_lang::prelude::*;

use crate::state::*;

#[derive(Accounts)]
#[instruction(account_data: AccountCreationData)]
pub struct CreateUserRecordAndGameSession<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        init,
        payer = user,
        space = 8 + UserRecord::INIT_SPACE,
        seeds = [b"user_record", user.key().as_ref()],
        bump,
    )]
    pub user_record: Account<'info, UserRecord>,
    #[account(
        init,
        payer = user,
        space = 8 + GameSession::INIT_SPACE,
        seeds = [b"game_session", account_data.session_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub game_session: Account<'info, GameSession>,

    /// The mint signer used for PDA derivation
    pub mint_signer: Signer<'info>,

    /// The mint authority used for PDA derivation
    pub mint_authority: Signer<'info>,

    /// Compressed token program
    /// CHECK: Program ID validated using LIGHT_TOKEN_PROGRAM_ID constant
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

#[derive(Accounts)]
pub struct InitializeCompressionConfig<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: Config PDA is created and validated by the SDK
    #[account(mut)]
    pub config: AccountInfo<'info>,
    /// CHECK: Program data account is validated by the SDK
    pub program_data: AccountInfo<'info>,
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateCompressionConfig<'info> {
    /// CHECK: Checked by SDK
    #[account(mut)]
    pub config: AccountInfo<'info>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct DecompressAccountsIdempotent<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    /// CHECK: checked by SDK
    pub config: AccountInfo<'info>,
    #[account(mut)]
    pub rent_sponsor: AccountInfo<'info>,
    /// CHECK: anyone can pay (optional - only needed if decompressing tokens)
    #[account(mut)]
    pub ctoken_rent_sponsor: Option<AccountInfo<'info>>,
    /// CHECK: checked by SDK (optional - only needed if decompressing tokens)
    pub ctoken_config: Option<AccountInfo<'info>>,
    /// CHECK:
    #[account(address = solana_pubkey::pubkey!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m"))]
    pub ctoken_program: Option<AccountInfo<'info>>,
    /// CHECK:
    #[account(address = solana_pubkey::pubkey!("GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy"))]
    pub ctoken_cpi_authority: Option<AccountInfo<'info>>,
    /// CHECK: checked by SDK
    pub some_mint: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CompressAccountsIdempotent<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    /// CHECK: Checked by SDK
    pub config: AccountInfo<'info>,
    /// CHECK: checked by SDK
    #[account(mut)]
    pub rent_sponsor: AccountInfo<'info>,
    /// CHECK: Checked by SDK
    #[account(mut)]
    pub compression_authority: AccountInfo<'info>,
}
