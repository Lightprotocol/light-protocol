use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

/// # Create a transfer instruction:
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_token_sdk::token::Transfer;
/// # let source = Pubkey::new_unique();
/// # let destination = Pubkey::new_unique();
/// # let authority = Pubkey::new_unique();
/// let instruction = Transfer {
///     source,
///     destination,
///     amount: 100,
///     authority,
///     max_top_up: None,
/// }.instruction()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct Transfer {
    pub source: Pubkey,
    pub destination: Pubkey,
    pub amount: u64,
    pub authority: Pubkey,
    /// Maximum lamports for rent and top-up combined. Transaction fails if exceeded. (0 = no limit)
    /// When set to a non-zero value, includes max_top_up in instruction data
    pub max_top_up: Option<u16>,
}

/// # Transfer via CPI:
/// ```rust,no_run
/// # use light_token_sdk::token::TransferCpi;
/// # use solana_account_info::AccountInfo;
/// # let source: AccountInfo = todo!();
/// # let destination: AccountInfo = todo!();
/// # let authority: AccountInfo = todo!();
/// TransferCpi {
///     source,
///     destination,
///     amount: 100,
///     authority,
///     max_top_up: None,
/// }
/// .invoke()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct TransferCpi<'info> {
    pub source: AccountInfo<'info>,
    pub destination: AccountInfo<'info>,
    pub amount: u64,
    pub authority: AccountInfo<'info>,
    /// Maximum lamports for rent and top-up combined. Transaction fails if exceeded. (0 = no limit)
    pub max_top_up: Option<u16>,
}

impl<'info> TransferCpi<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        Transfer::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = Transfer::from(&self).instruction()?;
        let account_infos = [self.source, self.destination, self.authority];
        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = Transfer::from(&self).instruction()?;
        let account_infos = [self.source, self.destination, self.authority];
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> From<&TransferCpi<'info>> for Transfer {
    fn from(account_infos: &TransferCpi<'info>) -> Self {
        Self {
            source: *account_infos.source.key,
            destination: *account_infos.destination.key,
            amount: account_infos.amount,
            authority: *account_infos.authority.key,
            max_top_up: account_infos.max_top_up,
        }
    }
}

impl Transfer {
    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        Ok(Instruction {
            program_id: Pubkey::from(LIGHT_TOKEN_PROGRAM_ID),
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
