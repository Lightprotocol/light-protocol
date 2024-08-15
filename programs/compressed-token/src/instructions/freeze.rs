use account_compression::{program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED};
use anchor_lang::prelude::*;
use anchor_spl::token::Mint;
use light_system_program::{
    program::LightSystemProgram,
    sdk::accounts::{InvokeAccounts, SignerAccounts},
};

use crate::program::LightCompressedToken;

#[derive(Accounts)]
pub struct FreezeInstruction<'info> {
    /// UNCHECKED: only pays fees.
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    #[account(constraint= authority.key() == mint.freeze_authority.unwrap()
        @ crate::ErrorCode::InvalidFreezeAuthority)]
    pub authority: Signer<'info>,
    /// CHECK: (seed constraint).
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump,)]
    pub cpi_authority_pda: UncheckedAccount<'info>,
    pub light_system_program: Program<'info, LightSystemProgram>,
    /// CHECK: (account compression program).
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK: (account compression program) when emitting event.
    pub noop_program: UncheckedAccount<'info>,
    /// CHECK: (different program) is used to cpi account compression program from light system program.
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump, seeds::program = light_system_program::ID,)]
    pub account_compression_authority: UncheckedAccount<'info>,
    pub account_compression_program: Program<'info, AccountCompression>,
    /// CHECK: (different system program) to derive cpi_authority_pda and check
    /// that this program is the signer of the cpi.
    pub self_program: Program<'info, LightCompressedToken>,
    pub system_program: Program<'info, System>,
    pub mint: Account<'info, Mint>,
}

impl<'info> InvokeAccounts<'info> for FreezeInstruction<'info> {
    fn get_registered_program_pda(&self) -> &AccountInfo<'info> {
        &self.registered_program_pda
    }

    fn get_noop_program(&self) -> &UncheckedAccount<'info> {
        &self.noop_program
    }

    fn get_account_compression_authority(&self) -> &UncheckedAccount<'info> {
        &self.account_compression_authority
    }

    fn get_account_compression_program(&self) -> &Program<'info, AccountCompression> {
        &self.account_compression_program
    }

    fn get_system_program(&self) -> &Program<'info, System> {
        &self.system_program
    }

    fn get_sol_pool_pda(&self) -> Option<&AccountInfo<'info>> {
        None
    }

    fn get_decompression_recipient(&self) -> Option<&AccountInfo<'info>> {
        None
    }
}

impl<'info> SignerAccounts<'info> for FreezeInstruction<'info> {
    fn get_fee_payer(&self) -> &Signer<'info> {
        &self.fee_payer
    }

    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
}
