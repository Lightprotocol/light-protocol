use account_compression::program::AccountCompression;
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey, system_program::System};

use crate::{
    account_traits::{InvokeAccounts, SignerAccounts},
    constants::SOL_POOL_PDA_SEED,
    cpi_context_account::CpiContextAccount,
};

#[derive(Accounts)]
pub struct InvokeCpiInstruction<'info> {
    /// Fee payer needs to be mutable to pay rollover and protocol fees.
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    pub authority: Signer<'info>,
    /// CHECK: in account compression program
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK: checked in emit_event.rs.
    pub noop_program: UncheckedAccount<'info>,
    /// CHECK: used to invoke account compression program cpi sign will fail if invalid account is provided seeds = [CPI_AUTHORITY_PDA_SEED].
    pub account_compression_authority: UncheckedAccount<'info>,
    /// CHECK:
    pub account_compression_program: Program<'info, AccountCompression>,
    /// CHECK: checked in cpi_signer_check.
    pub invoking_program: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [SOL_POOL_PDA_SEED], bump
    )]
    pub sol_pool_pda: Option<AccountInfo<'info>>,
    #[account(mut)]
    pub decompression_recipient: Option<AccountInfo<'info>>,
    pub system_program: Program<'info, System>,
    #[account(mut)]
    pub cpi_context_account: Option<Account<'info, CpiContextAccount>>,
}

impl<'info> SignerAccounts<'info> for InvokeCpiInstruction<'info> {
    fn get_fee_payer(&self) -> &Signer<'info> {
        &self.fee_payer
    }

    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
}

impl<'info> InvokeAccounts<'info> for InvokeCpiInstruction<'info> {
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
        self.sol_pool_pda.as_ref()
    }

    fn get_decompression_recipient(&self) -> Option<&AccountInfo<'info>> {
        self.decompression_recipient.as_ref()
    }
}
