use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

/// # Mint tokens to a ctoken account (simple 3-account instruction):
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_token::instruction::MintTo;
/// # let mint = Pubkey::new_unique();
/// # let destination = Pubkey::new_unique();
/// # let authority = Pubkey::new_unique();
/// let instruction = MintTo {
///     mint,
///     destination,
///     amount: 100,
///     authority,
///     max_top_up: None,
///     fee_payer: None,
/// }.instruction()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct MintTo {
    /// Mint account (supply tracking)
    pub mint: Pubkey,
    /// Destination Light Token account to mint to
    pub destination: Pubkey,
    /// Amount of tokens to mint
    pub amount: u64,
    /// Mint authority
    pub authority: Pubkey,
    /// Maximum lamports for rent and top-up combined. Transaction fails if exceeded. (0 = no limit)
    /// When set to a non-zero value, includes max_top_up in instruction data
    pub max_top_up: Option<u16>,
    /// Optional fee payer for rent top-ups. If not provided, authority pays.
    pub fee_payer: Option<Pubkey>,
}

/// # Mint to ctoken via CPI:
/// ```rust,no_run
/// # use light_token::instruction::MintToCpi;
/// # use solana_account_info::AccountInfo;
/// # let mint: AccountInfo = todo!();
/// # let destination: AccountInfo = todo!();
/// # let authority: AccountInfo = todo!();
/// # let system_program: AccountInfo = todo!();
/// MintToCpi {
///     mint,
///     destination,
///     amount: 100,
///     authority,
///     system_program,
///     max_top_up: None,
///     fee_payer: None,
/// }
/// .invoke()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct MintToCpi<'info> {
    pub mint: AccountInfo<'info>,
    pub destination: AccountInfo<'info>,
    pub amount: u64,
    pub authority: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    /// Maximum lamports for rent and top-up combined. Transaction fails if exceeded. (0 = no limit)
    pub max_top_up: Option<u16>,
    /// Optional fee payer for rent top-ups. If not provided, authority pays.
    pub fee_payer: Option<AccountInfo<'info>>,
}

impl<'info> MintToCpi<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        MintTo::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = MintTo::from(&self).instruction()?;
        if let Some(fee_payer) = self.fee_payer {
            let account_infos = [
                self.mint,
                self.destination,
                self.authority,
                self.system_program,
                fee_payer,
            ];
            invoke(&instruction, &account_infos)
        } else {
            let account_infos = [
                self.mint,
                self.destination,
                self.authority,
                self.system_program,
            ];
            invoke(&instruction, &account_infos)
        }
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = MintTo::from(&self).instruction()?;
        if let Some(fee_payer) = self.fee_payer {
            let account_infos = [
                self.mint,
                self.destination,
                self.authority,
                self.system_program,
                fee_payer,
            ];
            invoke_signed(&instruction, &account_infos, signer_seeds)
        } else {
            let account_infos = [
                self.mint,
                self.destination,
                self.authority,
                self.system_program,
            ];
            invoke_signed(&instruction, &account_infos, signer_seeds)
        }
    }
}

impl<'info> From<&MintToCpi<'info>> for MintTo {
    fn from(cpi: &MintToCpi<'info>) -> Self {
        Self {
            mint: *cpi.mint.key,
            destination: *cpi.destination.key,
            amount: cpi.amount,
            authority: *cpi.authority.key,
            max_top_up: cpi.max_top_up,
            fee_payer: cpi.fee_payer.as_ref().map(|a| *a.key),
        }
    }
}

impl MintTo {
    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        // Authority is writable only when max_top_up is set AND no fee_payer
        // (authority pays for top-ups only if no separate fee_payer)
        let authority_meta = if self.max_top_up.is_some() && self.fee_payer.is_none() {
            AccountMeta::new(self.authority, true)
        } else {
            AccountMeta::new_readonly(self.authority, true)
        };

        let mut accounts = vec![
            AccountMeta::new(self.mint, false),
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
                let mut data = vec![7u8]; // MintTo discriminator
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
