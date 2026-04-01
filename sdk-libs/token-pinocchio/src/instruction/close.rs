//! Close CPI for Light Token operations.

use pinocchio::{
    AccountView as AccountInfo,
    cpi::{invoke_with_slice, invoke_signed_with_slice, Signer},
    instruction::{InstructionAccount, InstructionView},
    error::ProgramError,
    address::Address,
};

/// Close light-token account via CPI.
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

        let program_id = Address::from(*self.token_program.address());

        let account_metas = [
            InstructionAccount::writable(self.account.address()),
            InstructionAccount::writable(self.destination.address()),
            InstructionAccount::readonly_signer(self.owner.address()),
            InstructionAccount::writable(self.rent_sponsor.address()),
        ];

        let instruction = InstructionView {
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
            invoke_with_slice(&instruction, &account_infos)
        } else {
            invoke_signed_with_slice(&instruction, &account_infos, signers)
        }
    }
}
