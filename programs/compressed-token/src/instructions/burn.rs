use account_compression::{program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface};
use light_system_program::{
    account_traits::{InvokeAccounts, SignerAccounts},
    program::LightSystemProgram,
};

use crate::program::LightCompressedToken;

#[derive(Accounts)]
pub struct BurnInstruction<'info> {
    /// UNCHECKED: only pays fees.
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    /// CHECK:
    /// Authority is verified through proof since both owner and delegate
    /// are included in the token data hash, which is a public input to the
    /// validity proof.
    pub authority: Signer<'info>,
    /// CHECK: (seed constraint).
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump,)]
    pub cpi_authority_pda: UncheckedAccount<'info>,
    /// CHECK: is used to burn tokens.
    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,
    /// CHECK: in invoke_token_program_with_multiple_token_pool_accounts.
    #[account(mut)]
    pub token_pool_pda: AccountInfo<'info>,
    pub token_program: Interface<'info, TokenInterface>,
    pub light_system_program: Program<'info, LightSystemProgram>,
    /// CHECK: (account compression program).
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK: (system program) when emitting event.
    pub noop_program: UncheckedAccount<'info>,
    /// CHECK: (system program) to cpi account compression program.
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump, seeds::program = light_system_program::ID,)]
    pub account_compression_authority: UncheckedAccount<'info>,
    pub account_compression_program: Program<'info, AccountCompression>,
    pub self_program: Program<'info, LightCompressedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> InvokeAccounts<'info> for BurnInstruction<'info> {
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

impl<'info> SignerAccounts<'info> for BurnInstruction<'info> {
    fn get_fee_payer(&self) -> &Signer<'info> {
        &self.fee_payer
    }

    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
}
