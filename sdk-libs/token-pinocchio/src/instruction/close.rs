//! Close CPI for Light Token operations.

use pinocchio::{
    account_info::AccountInfo,
    cpi::{slice_invoke, slice_invoke_signed},
    instruction::{AccountMeta, Instruction, Signer},
    program_error::ProgramError,
    pubkey::Pubkey,
};

/// Close ctoken account via CPI.
///
/// # Example
///
/// ```rust,ignore
/// use light_token_pinocchio::instruction::CloseAccountCpi;
///
/// CloseAccountCpi {
///     token_program: &ctx.accounts.token_program,
///     account: &ctx.accounts.account,
///     destination: &ctx.accounts.destination,
///     owner: &ctx.accounts.owner,
///     rent_sponsor: &ctx.accounts.rent_sponsor,
/// }
/// .invoke()?;
/// ```
pub struct CloseAccountCpi<'info> {
    /// The token program to invoke (Light Token Program)
    pub token_program: &'info AccountInfo,
    /// The token account to close
    pub account: &'info AccountInfo,
    /// The destination to receive the account's lamports
    pub destination: &'info AccountInfo,
    /// The owner of the token account (signer)
    pub owner: &'info AccountInfo,
    /// The rent sponsor account
    pub rent_sponsor: &'info AccountInfo,
}

impl<'info> CloseAccountCpi<'info> {
    pub fn invoke(self) -> Result<(), ProgramError> {
        self.invoke_signed(&[])
    }

    pub fn invoke_signed(self, signers: &[Signer]) -> Result<(), ProgramError> {
        // Build instruction data: discriminator(1) only
        let data = [9u8]; // Close discriminator

        let program_id = Pubkey::from(*self.token_program.key());

        let account_metas = [
            AccountMeta::writable(self.account.key()),
            AccountMeta::writable(self.destination.key()),
            AccountMeta::writable_signer(self.owner.key()),
            AccountMeta::writable(self.rent_sponsor.key()),
        ];

        let instruction = Instruction {
            program_id: &program_id,
            accounts: &account_metas,
            data: &data,
        };

        let account_infos = [
            self.account,
            self.destination,
            self.owner,
            self.rent_sponsor,
        ];

        if signers.is_empty() {
            slice_invoke(&instruction, &account_infos)
        } else {
            slice_invoke_signed(&instruction, &account_infos, signers)
        }
    }
}
