use anchor_lang::prelude::*;

use crate::state::*;

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
        // Space: discriminator(8) + owner(32) + name_len(4) + name(32) + score(8) + category_id(8) = 92 bytes
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
        // Space: discriminator(8) + session_id(8) + player(32) + game_type_len(4) +
        //        game_type(32) + start_time(8) + end_time(1+8) + score(8) = 109 bytes
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
