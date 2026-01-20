use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

/// # Create a transfer ctoken instruction:
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_token::instruction::Transfer;
/// # let source = Pubkey::new_unique();
/// # let destination = Pubkey::new_unique();
/// # let authority = Pubkey::new_unique();
/// let instruction = Transfer {
///     source,
///     destination,
///     amount: 100,
///     authority,
///     max_top_up: None,
///     fee_payer: None,
/// }.instruction()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct Transfer {
    pub source: Pubkey,
    pub destination: Pubkey,
    pub amount: u64,
    pub authority: Pubkey,
    /// Maximum lamports for rent and top-up combined. Transaction fails if exceeded. (0 = no limit)
    /// When set, includes max_top_up in instruction data and adds system program account for compressible top-up
    pub max_top_up: Option<u16>,
    /// Optional fee payer for rent top-ups. If not provided, authority pays.
    /// When set, fee_payer pays for top-ups instead of authority.
    pub fee_payer: Option<Pubkey>,
}

/// # Transfer ctoken via CPI:
/// ```rust,no_run
/// # use light_token::instruction::TransferCpi;
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
///     fee_payer: None,
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
    /// Optional fee payer for rent top-ups. If not provided, authority pays.
    pub fee_payer: Option<AccountInfo<'info>>,
}

impl<'info> TransferCpi<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        Transfer::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = Transfer::from(&self).instruction()?;
        if let Some(fee_payer) = self.fee_payer {
            let account_infos = [self.source, self.destination, self.authority, fee_payer];
            invoke(&instruction, &account_infos)
        } else {
            let account_infos = [self.source, self.destination, self.authority];
            invoke(&instruction, &account_infos)
        }
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = Transfer::from(&self).instruction()?;
        if let Some(fee_payer) = self.fee_payer {
            let account_infos = [self.source, self.destination, self.authority, fee_payer];
            invoke_signed(&instruction, &account_infos, signer_seeds)
        } else {
            let account_infos = [self.source, self.destination, self.authority];
            invoke_signed(&instruction, &account_infos, signer_seeds)
        }
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
            fee_payer: account_infos.fee_payer.as_ref().map(|a| *a.key),
        }
    }
}

impl Transfer {
    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        // Authority is writable only when max_top_up is set AND no fee_payer
        // (authority pays for top-ups only if no separate fee_payer)
        let authority_meta = if self.max_top_up.is_some() && self.fee_payer.is_none() {
            AccountMeta::new(self.authority, true)
        } else {
            AccountMeta::new_readonly(self.authority, true)
        };

        let mut accounts = vec![
            AccountMeta::new(self.source, false),
            AccountMeta::new(self.destination, false),
            authority_meta,
        ];

        // Add fee_payer if provided (must be signer and writable)
        if let Some(fee_payer) = self.fee_payer {
            accounts.push(AccountMeta::new(fee_payer, true));
        }

        Ok(Instruction {
            program_id: Pubkey::from(LIGHT_TOKEN_PROGRAM_ID),
            accounts,
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
