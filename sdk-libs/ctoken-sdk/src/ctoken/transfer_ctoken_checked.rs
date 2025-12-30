use light_sdk_types::C_TOKEN_PROGRAM_ID;
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

/// # Create a transfer ctoken checked instruction:
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_ctoken_sdk::ctoken::TransferCTokenChecked;
/// # let source = Pubkey::new_unique();
/// # let mint = Pubkey::new_unique();
/// # let destination = Pubkey::new_unique();
/// # let authority = Pubkey::new_unique();
/// let instruction = TransferCTokenChecked {
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
pub struct TransferCTokenChecked {
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
/// # use light_ctoken_sdk::ctoken::TransferCTokenCheckedCpi;
/// # use solana_account_info::AccountInfo;
/// # let source: AccountInfo = todo!();
/// # let mint: AccountInfo = todo!();
/// # let destination: AccountInfo = todo!();
/// # let authority: AccountInfo = todo!();
/// TransferCTokenCheckedCpi {
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
pub struct TransferCTokenCheckedCpi<'info> {
    pub source: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub destination: AccountInfo<'info>,
    pub amount: u64,
    pub decimals: u8,
    pub authority: AccountInfo<'info>,
    /// Maximum lamports for rent and top-up combined. Transaction fails if exceeded. (0 = no limit)
    pub max_top_up: Option<u16>,
}

impl<'info> TransferCTokenCheckedCpi<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        TransferCTokenChecked::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = TransferCTokenChecked::from(&self).instruction()?;
        let account_infos = [self.source, self.mint, self.destination, self.authority];
        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = TransferCTokenChecked::from(&self).instruction()?;
        let account_infos = [self.source, self.mint, self.destination, self.authority];
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> From<&TransferCTokenCheckedCpi<'info>> for TransferCTokenChecked {
    fn from(account_infos: &TransferCTokenCheckedCpi<'info>) -> Self {
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

impl TransferCTokenChecked {
    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        Ok(Instruction {
            program_id: Pubkey::from(C_TOKEN_PROGRAM_ID),
            accounts: vec![
                AccountMeta::new(self.source, false),
                AccountMeta::new_readonly(self.mint, false),
                AccountMeta::new(self.destination, false),
                AccountMeta::new_readonly(self.authority, true),
            ],
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
