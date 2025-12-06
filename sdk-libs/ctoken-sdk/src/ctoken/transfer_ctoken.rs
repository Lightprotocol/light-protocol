use light_sdk_types::C_TOKEN_PROGRAM_ID;
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

pub struct TransferCtoken {
    pub source: Pubkey,
    pub destination: Pubkey,
    pub amount: u64,
    pub authority: Pubkey,
    /// Maximum lamports for rent and top-up combined. Transaction fails if exceeded. (0 = no limit)
    /// When set to a non-zero value, includes max_top_up in instruction data
    pub max_top_up: Option<u16>,
}

pub struct TransferCtokenAccountInfos<'info> {
    pub source: AccountInfo<'info>,
    pub destination: AccountInfo<'info>,
    pub amount: u64,
    pub authority: AccountInfo<'info>,
    /// Maximum lamports for rent and top-up combined. Transaction fails if exceeded. (0 = no limit)
    pub max_top_up: Option<u16>,
}

impl<'info> TransferCtokenAccountInfos<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        TransferCtoken::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = TransferCtoken::from(&self).instruction()?;
        let account_infos = [self.source, self.destination, self.authority];
        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = TransferCtoken::from(&self).instruction()?;
        let account_infos = [self.source, self.destination, self.authority];
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> From<&TransferCtokenAccountInfos<'info>> for TransferCtoken {
    fn from(account_infos: &TransferCtokenAccountInfos<'info>) -> Self {
        Self {
            source: *account_infos.source.key,
            destination: *account_infos.destination.key,
            amount: account_infos.amount,
            authority: *account_infos.authority.key,
            max_top_up: account_infos.max_top_up,
        }
    }
}

impl TransferCtoken {
    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        Ok(Instruction {
            program_id: Pubkey::from(C_TOKEN_PROGRAM_ID),
            accounts: vec![
                AccountMeta::new(self.source, false),
                AccountMeta::new(self.destination, false),
                AccountMeta::new_readonly(self.authority, true),
            ],
            data: {
                let mut data = vec![3u8];
                data.extend_from_slice(&self.amount.to_le_bytes());
                // Include max_top_up if set (10-byte format)
                if let Some(max_top_up) = self.max_top_up {
                    data.extend_from_slice(&max_top_up.to_le_bytes());
                }
                data
            },
        })
    }
}
