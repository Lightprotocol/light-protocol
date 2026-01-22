use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::constants::RENT_SPONSOR_V1 as RENT_SPONSOR;

/// # Create a close ctoken account instruction:
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_token::instruction::{CloseAccount, LIGHT_TOKEN_PROGRAM_ID};
/// # let account = Pubkey::new_unique();
/// # let destination = Pubkey::new_unique();
/// # let owner = Pubkey::new_unique();
/// let instruction =
///     CloseAccount::new(LIGHT_TOKEN_PROGRAM_ID, account, destination, owner)
///     .instruction()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct CloseAccount {
    pub token_program: Pubkey,
    pub account: Pubkey,
    pub destination: Pubkey,
    pub owner: Pubkey,
    pub rent_sponsor: Pubkey,
}

impl CloseAccount {
    pub fn new(token_program: Pubkey, account: Pubkey, destination: Pubkey, owner: Pubkey) -> Self {
        Self {
            token_program,
            account,
            destination,
            owner,
            rent_sponsor: RENT_SPONSOR,
        }
    }

    pub fn custom_rent_sponsor(mut self, rent_sponsor: Pubkey) -> Self {
        self.rent_sponsor = rent_sponsor;
        self
    }

    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        // CloseAccount discriminator is 9 (no additional instruction data)
        let data = vec![9u8];

        let accounts = vec![
            AccountMeta::new(self.account, false),
            AccountMeta::new(self.destination, false),
            AccountMeta::new(self.owner, true), // signer, mutable to receive write_top_up
            AccountMeta::new(self.rent_sponsor, false),
        ];

        Ok(Instruction {
            program_id: self.token_program,
            accounts,
            data,
        })
    }
}

/// # Close a ctoken account via CPI:
/// ```rust,no_run
/// # use light_token::instruction::CloseAccountCpi;
/// # use solana_account_info::AccountInfo;
/// # let token_program: AccountInfo = todo!();
/// # let account: AccountInfo = todo!();
/// # let destination: AccountInfo = todo!();
/// # let owner: AccountInfo = todo!();
/// // Use ctoken::RENT_SPONSOR or ctoken::rent_sponsor_pda() to get the protocol rent sponsor.
/// # let rent_sponsor: AccountInfo = todo!();
/// CloseAccountCpi {
///     token_program,
///     account,
///     destination,
///     owner,
///     rent_sponsor,
/// }
/// .invoke()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct CloseAccountCpi<'info> {
    pub token_program: AccountInfo<'info>,
    pub account: AccountInfo<'info>,
    pub destination: AccountInfo<'info>,
    pub owner: AccountInfo<'info>,
    pub rent_sponsor: AccountInfo<'info>,
}

impl<'info> CloseAccountCpi<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        CloseAccount::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;
        let account_infos = [
            self.account,
            self.destination,
            self.owner,
            self.rent_sponsor,
        ];
        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;
        let account_infos = [
            self.account,
            self.destination,
            self.owner,
            self.rent_sponsor,
        ];
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> From<&CloseAccountCpi<'info>> for CloseAccount {
    fn from(account_infos: &CloseAccountCpi<'info>) -> Self {
        Self {
            token_program: *account_infos.token_program.key,
            account: *account_infos.account.key,
            destination: *account_infos.destination.key,
            owner: *account_infos.owner.key,
            rent_sponsor: *account_infos.rent_sponsor.key,
        }
    }
}
