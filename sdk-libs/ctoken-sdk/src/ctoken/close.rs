use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::ctoken::RENT_SPONSOR;

pub struct CloseAccount {
    pub token_program: Pubkey,
    pub account: Pubkey,
    pub destination: Pubkey,
    pub owner: Pubkey,
    pub rent_sponsor: Option<Pubkey>,
}

impl CloseAccount {
    pub fn new(token_program: Pubkey, account: Pubkey, destination: Pubkey, owner: Pubkey) -> Self {
        Self {
            token_program,
            account,
            destination,
            owner,
            rent_sponsor: Some(RENT_SPONSOR),
        }
    }

    pub fn custom_rent_sponsor(mut self, rent_sponsor: Pubkey) -> Self {
        self.rent_sponsor = Some(rent_sponsor);
        self
    }

    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        // CloseAccount discriminator is 9 (no additional instruction data)
        let data = vec![9u8];

        let mut accounts = vec![
            AccountMeta::new(self.account, false),
            AccountMeta::new(self.destination, false),
            AccountMeta::new(self.owner, true), // signer, mutable to receive write_top_up
        ];

        // Add rent sponsor for compressible accounts
        if let Some(rent_sponsor) = self.rent_sponsor {
            accounts.push(AccountMeta::new(rent_sponsor, false));
        }

        Ok(Instruction {
            program_id: self.token_program,
            accounts,
            data,
        })
    }
}

pub struct CloseAccountInfos<'info> {
    pub token_program: AccountInfo<'info>,
    pub account: AccountInfo<'info>,
    pub destination: AccountInfo<'info>,
    pub owner: AccountInfo<'info>,
    pub rent_sponsor: Option<AccountInfo<'info>>,
}

impl<'info> CloseAccountInfos<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        CloseAccount::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;
        if let Some(rent_sponsor) = self.rent_sponsor {
            let account_infos = [self.account, self.destination, self.owner, rent_sponsor];
            invoke(&instruction, &account_infos)
        } else {
            let account_infos = [self.account, self.destination, self.owner];
            invoke(&instruction, &account_infos)
        }
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;
        if let Some(rent_sponsor) = self.rent_sponsor {
            let account_infos = [self.account, self.destination, self.owner, rent_sponsor];
            invoke_signed(&instruction, &account_infos, signer_seeds)
        } else {
            let account_infos = [self.account, self.destination, self.owner];
            invoke_signed(&instruction, &account_infos, signer_seeds)
        }
    }
}

impl<'info> From<&CloseAccountInfos<'info>> for CloseAccount {
    fn from(account_infos: &CloseAccountInfos<'info>) -> Self {
        Self {
            token_program: *account_infos.token_program.key,
            account: *account_infos.account.key,
            destination: *account_infos.destination.key,
            owner: *account_infos.owner.key,
            rent_sponsor: account_infos.rent_sponsor.as_ref().map(|ai| *ai.key),
        }
    }
}
