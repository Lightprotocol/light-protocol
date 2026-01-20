use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

/// # Create a transfer ctoken checked instruction:
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_token::token::TransferChecked;
/// # let source = Pubkey::new_unique();
/// # let mint = Pubkey::new_unique();
/// # let destination = Pubkey::new_unique();
/// # let authority = Pubkey::new_unique();
/// let instruction = TransferChecked {
///     source,
///     mint,
///     destination,
///     amount: 100,
///     decimals: 9,
///     authority,
///     max_top_up: None,
/// }.instruction()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct TransferChecked {
    pub source: Pubkey,
    pub mint: Pubkey,
    pub destination: Pubkey,
    pub amount: u64,
    pub decimals: u8,
    pub authority: Pubkey,
    /// Maximum lamports for rent and top-up combined. Transaction fails if exceeded. (0 = no limit)
    /// When set to a non-zero value, includes max_top_up in instruction data
    pub max_top_up: Option<u16>,
}

/// # Transfer ctoken checked via CPI:
/// ```rust,no_run
/// # use light_token::token::TransferCheckedCpi;
/// # use solana_account_info::AccountInfo;
/// # let source: AccountInfo = todo!();
/// # let mint: AccountInfo = todo!();
/// # let destination: AccountInfo = todo!();
/// # let authority: AccountInfo = todo!();
/// TransferCheckedCpi {
///     source,
///     mint,
///     destination,
///     amount: 100,
///     decimals: 9,
///     authority,
///     max_top_up: None,
/// }
/// .invoke()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct TransferCheckedCpi<'info> {
    pub source: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub destination: AccountInfo<'info>,
    pub amount: u64,
    pub decimals: u8,
    pub authority: AccountInfo<'info>,
    /// Maximum lamports for rent and top-up combined. Transaction fails if exceeded. (0 = no limit)
    pub max_top_up: Option<u16>,
}

impl<'info> TransferCheckedCpi<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        TransferChecked::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = TransferChecked::from(&self).instruction()?;
        let account_infos = [self.source, self.mint, self.destination, self.authority];
        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = TransferChecked::from(&self).instruction()?;
        let account_infos = [self.source, self.mint, self.destination, self.authority];
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> From<&TransferCheckedCpi<'info>> for TransferChecked {
    fn from(account_infos: &TransferCheckedCpi<'info>) -> Self {
        Self {
            source: *account_infos.source.key,
            mint: *account_infos.mint.key,
            destination: *account_infos.destination.key,
            amount: account_infos.amount,
            decimals: account_infos.decimals,
            authority: *account_infos.authority.key,
            max_top_up: account_infos.max_top_up,
        }
    }
}

impl TransferChecked {
    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        // Authority is writable only when max_top_up is set (for compressible top-up lamport transfer)
        let authority_meta = if self.max_top_up.is_some() {
            AccountMeta::new(self.authority, true)
        } else {
            AccountMeta::new_readonly(self.authority, true)
        };

        let mut accounts = vec![
            AccountMeta::new(self.source, false),
            AccountMeta::new_readonly(self.mint, false),
            AccountMeta::new(self.destination, false),
            authority_meta,
        ];

        // Include system program for compressible top-up when max_top_up is set
        if self.max_top_up.is_some() {
            accounts.push(AccountMeta::new_readonly(
                solana_pubkey::pubkey!("11111111111111111111111111111111"),
                false,
            ));
        }

        Ok(Instruction {
            program_id: Pubkey::from(LIGHT_TOKEN_PROGRAM_ID),
            accounts,
            data: {
                // Discriminator (1) + amount (8) + decimals (1) + optional max_top_up (2)
                let mut data = vec![12u8]; // TransferChecked discriminator (SPL compatible)
                data.extend_from_slice(&self.amount.to_le_bytes());
                data.push(self.decimals);
                // Include max_top_up if set (11-byte format)
                if let Some(max_top_up) = self.max_top_up {
                    data.extend_from_slice(&max_top_up.to_le_bytes());
                }
                data
            },
        })
    }
}
