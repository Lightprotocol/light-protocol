use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

/// # Burn tokens from a ctoken account:
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_token::instruction::Burn;
/// # let source = Pubkey::new_unique();
/// # let mint = Pubkey::new_unique();
/// # let authority = Pubkey::new_unique();
/// let instruction = Burn {
///     source,
///     mint,
///     amount: 100,
///     authority,
///     max_top_up: None,
///     fee_payer: None,
/// }.instruction()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct Burn {
    /// Light Token account to burn from
    pub source: Pubkey,
    /// Mint account (supply tracking)
    pub mint: Pubkey,
    /// Amount of tokens to burn
    pub amount: u64,
    /// Owner of the Light Token account
    pub authority: Pubkey,
    /// Maximum lamports for rent and top-up combined. Transaction fails if exceeded. (0 = no limit)
    /// When set to a non-zero value, includes max_top_up in instruction data
    pub max_top_up: Option<u16>,
    /// Optional fee payer for rent top-ups. If not provided, authority pays.
    pub fee_payer: Option<Pubkey>,
}

/// # Burn ctoken via CPI:
/// ```rust,no_run
/// # use light_token::instruction::BurnCpi;
/// # use solana_account_info::AccountInfo;
/// # let source: AccountInfo = todo!();
/// # let mint: AccountInfo = todo!();
/// # let authority: AccountInfo = todo!();
/// BurnCpi {
///     source,
///     mint,
///     amount: 100,
///     authority,
///     max_top_up: None,
///     fee_payer: None,
/// }
/// .invoke()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct BurnCpi<'info> {
    pub source: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub amount: u64,
    pub authority: AccountInfo<'info>,
    /// Maximum lamports for rent and top-up combined. Transaction fails if exceeded. (0 = no limit)
    pub max_top_up: Option<u16>,
    /// Optional fee payer for rent top-ups. If not provided, authority pays.
    pub fee_payer: Option<AccountInfo<'info>>,
}

impl<'info> BurnCpi<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        Burn::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = Burn::from(&self).instruction()?;
        if let Some(fee_payer) = self.fee_payer {
            let account_infos = [self.source, self.mint, self.authority, fee_payer];
            invoke(&instruction, &account_infos)
        } else {
            let account_infos = [self.source, self.mint, self.authority];
            invoke(&instruction, &account_infos)
        }
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = Burn::from(&self).instruction()?;
        if let Some(fee_payer) = self.fee_payer {
            let account_infos = [self.source, self.mint, self.authority, fee_payer];
            invoke_signed(&instruction, &account_infos, signer_seeds)
        } else {
            let account_infos = [self.source, self.mint, self.authority];
            invoke_signed(&instruction, &account_infos, signer_seeds)
        }
    }
}

impl<'info> From<&BurnCpi<'info>> for Burn {
    fn from(cpi: &BurnCpi<'info>) -> Self {
        Self {
            source: *cpi.source.key,
            mint: *cpi.mint.key,
            amount: cpi.amount,
            authority: *cpi.authority.key,
            max_top_up: cpi.max_top_up,
            fee_payer: cpi.fee_payer.as_ref().map(|a| *a.key),
        }
    }
}

impl Burn {
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
            AccountMeta::new(self.mint, false),
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
                let mut data = vec![8u8]; // CTokenBurn discriminator
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
