use account_compression::{config_accounts::GroupAuthority, program::AccountCompression};
use anchor_lang::prelude::*;

declare_id!("6UqiSPd2mRCTTwkzhcs1M6DGYsqHWd5jiPueX3LwDMXQ");

#[error_code]
pub enum ErrorCode {
    #[msg("Sum check failed")]
    SumCheckFailed,
}

#[constant]
pub const AUTHORITY_PDA_SEED: &[u8] = b"authority";

#[program]
pub mod light {

    use anchor_lang::solana_program::pubkey::Pubkey;

    use super::*;

    pub fn update_authority(
        ctx: Context<UpdateAuthority>,
        bump: u8,
        new_authority: Pubkey,
    ) -> Result<()> {
        let bump = &[bump];
        let seeds = [AUTHORITY_PDA_SEED, bump];
        let signer_seeds = &[&seeds[..]];

        let accounts = account_compression::cpi::accounts::UpdateGroupAuthority {
            authority: ctx.accounts.authority_pda.to_account_info(),
            group_authority: ctx.accounts.group_pda.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.account_compression_program.to_account_info(),
            accounts,
            signer_seeds,
        );

        account_compression::cpi::update_group_authority(cpi_ctx, new_authority)
    }

    pub fn register_system_program(
        ctx: Context<RegisteredProgram>,
        bump: u8,
        program_id: Pubkey,
    ) -> Result<()> {
        let bump = &[bump];
        let seeds = [AUTHORITY_PDA_SEED, bump];
        let signer_seeds = &[&seeds[..]];

        let accounts = account_compression::cpi::accounts::RegisterProgramToGroup {
            authority: ctx.accounts.authority_pda.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
            group_authority_pda: ctx.accounts.group_pda.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.account_compression_program.to_account_info(),
            accounts,
            signer_seeds,
        );

        account_compression::cpi::register_program_to_group(cpi_ctx, program_id)
    }

    // TODO: add register relayer

    // TODO: add rollover Merkle tree with rewards

    // TODO: add rollover lookup table with rewards

    // TODO: add nullify utxo with rewards
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct UpdateAuthority<'info> {
    #[account(mut)]
    authority: Signer<'info>,
    #[account(mut, seeds = [AUTHORITY_PDA_SEED, &__program_id.to_bytes().as_slice()], bump)]
    authority_pda: AccountInfo<'info>,
    #[account(mut)]
    group_pda: Account<'info, GroupAuthority>,
    account_compression_program: Program<'info, AccountCompression>,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct RegisteredProgram<'info> {
    #[account(mut)]
    authority: Signer<'info>,
    #[account(mut, seeds = [AUTHORITY_PDA_SEED, &__program_id.to_bytes().as_slice()], bump)]
    authority_pda: AccountInfo<'info>,
    #[account(mut)]
    group_pda: Account<'info, GroupAuthority>,
    account_compression_program: Program<'info, AccountCompression>,
    system_program: Program<'info, System>,
    registered_program_pda:
        Account<'info, account_compression::register_program::RegisteredProgram>,
}
