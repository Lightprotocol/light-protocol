use anchor_lang::prelude::*;

use crate::state::UserRecord;

// In a standalone file to test macro support.
#[derive(Accounts)]
pub struct CreateRecord<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        init,
        payer = user,
        // Manually add 10 bytes! Discriminator + owner + string len + name +
        // score + option<compression_info>
        space = 8 + 32 + 4 + 32 + 8 + 10,
        seeds = [b"user_record", user.key().as_ref()],
        bump,
    )]
    pub user_record: Account<'info, UserRecord>,
    /// UNCHECKED: checked via config.
    #[account(mut)]
    pub rent_recipient: AccountInfo<'info>,
    /// The global config account
    /// UNCHECKED: checked via load_checked.
    pub config: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}
