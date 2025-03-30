use account_compression::program::AccountCompression;
use anchor_lang::prelude::*;

use crate::{
    account_traits::{InvokeAccounts, SignerAccounts},
    constants::SOL_POOL_PDA_SEED,
};

/// These are the base accounts additionally Merkle tree and queue accounts are required.
/// These additional accounts are passed as remaining accounts.
/// 1 Merkle tree for each input compressed account one queue and Merkle tree account each for each output compressed account.
#[derive(Accounts)]
pub struct InvokeInstruction<'info> {
    /// Fee payer needs to be mutable to pay rollover and protocol fees.
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    pub authority: Signer<'info>,
    /// CHECK: this account
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK: is checked when emitting the event.
    pub noop_program: UncheckedAccount<'info>,
    /// CHECK: this account in account compression program.
    /// This pda is used to invoke the account compression program.
    pub account_compression_authority: UncheckedAccount<'info>,
    /// CHECK: Account compression program is used to update state and address
    /// Merkle trees.
    pub account_compression_program: Program<'info, AccountCompression>,
    /// Sol pool pda is used to store the native sol that has been compressed.
    /// It's only required when compressing or decompressing sol.
    #[account(
        mut,
        seeds = [SOL_POOL_PDA_SEED], bump
    )]
    pub sol_pool_pda: Option<AccountInfo<'info>>,
    /// Only needs to be provided for decompression as a recipient for the
    /// decompressed sol.
    /// Compressed sol originate from authority.
    #[account(mut)]
    pub decompression_recipient: Option<AccountInfo<'info>>,
    pub system_program: Program<'info, System>,
}

impl<'info> SignerAccounts<'info> for InvokeInstruction<'info> {
    fn get_fee_payer(&self) -> &Signer<'info> {
        &self.fee_payer
    }

    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
}

impl<'info> InvokeAccounts<'info> for InvokeInstruction<'info> {
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
