use account_compression::program::AccountCompression;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{TokenAccount, TokenInterface};
use light_system_program::{
    self,
    account_traits::{InvokeAccounts, SignerAccounts},
    program::LightSystemProgram,
};

use crate::program::LightCompressedToken;

#[derive(Accounts)]
pub struct TransferInstruction<'info> {
    /// UNCHECKED: only pays fees.
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    /// CHECK:
    /// Authority is verified through proof since both owner and delegate
    /// are included in the token data hash, which is a public input to the
    /// validity proof.
    pub authority: Signer<'info>,
    /// CHECK: (seed constraint in cpi).
    pub cpi_authority_pda: UncheckedAccount<'info>,
    pub light_system_program: Program<'info, LightSystemProgram>,
    /// CHECK: (account compression program).
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK: (account compression program) when emitting event.
    pub noop_program: UncheckedAccount<'info>,
    /// CHECK: (different program) is used to cpi account compression program from light system program.
    pub account_compression_authority: UncheckedAccount<'info>,
    pub account_compression_program: Program<'info, AccountCompression>,
    /// CHECK:(system program) used to derive cpi_authority_pda and check that
    /// this program is the signer of the cpi.
    pub self_program: Program<'info, LightCompressedToken>,
    #[account(mut)]
    pub token_pool_pda: Option<InterfaceAccount<'info, TokenAccount>>,
    #[account(mut, constraint= if token_pool_pda.is_some() {Ok(token_pool_pda.as_ref().unwrap().key() != compress_or_decompress_token_account.key())}else {err!(crate::ErrorCode::TokenPoolPdaUndefined)}? @crate::ErrorCode::IsTokenPoolPda)]
    pub compress_or_decompress_token_account: Option<InterfaceAccount<'info, TokenAccount>>,
    pub token_program: Option<Interface<'info, TokenInterface>>,
    pub system_program: Program<'info, System>,
}

// TODO: transform all to account info
impl<'info> InvokeAccounts<'info> for TransferInstruction<'info> {
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

impl<'info> SignerAccounts<'info> for TransferInstruction<'info> {
    fn get_fee_payer(&self) -> &Signer<'info> {
        &self.fee_payer
    }

    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
}
