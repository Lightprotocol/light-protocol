use anchor_lang::prelude::*;

declare_id!("6UqiSPd2mRCTTwkzhcs1M6DGYsqHWd5jiPueX3LwDMXQ");

const USER_ENTRY_SEED: &[u8] = b"user-entry";

#[program]
pub mod user_registry {
    use super::*;

    pub fn initialize_user_entry(
        ctx: Context<InitializeUserEntry>,
        light_pubkey: [u8; 32],
        light_encryption_pubkey: [u8; 32],
    ) -> Result<()> {
        let user_entry = &mut ctx.accounts.user_entry;
        user_entry.solana_pubkey = ctx.accounts.signer.key().to_bytes();
        user_entry.light_pubkey = light_pubkey;
        user_entry.light_encryption_pubkey = light_encryption_pubkey;

        Ok(())
    }
}

#[account]
pub struct UserEntry {
    pub solana_pubkey: [u8; 32],
    pub light_pubkey: [u8; 32],
    pub light_encryption_pubkey: [u8; 32],
}

#[derive(Accounts)]
pub struct InitializeUserEntry<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(
        init,
        space = 8 + 32 + 32 + 32,
        seeds = [USER_ENTRY_SEED, signer.key().to_bytes().as_ref()],
        bump,
        payer = signer,
    )]
    pub user_entry: Account<'info, UserEntry>,
}
