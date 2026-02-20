use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

/// # Burn tokens from a ctoken account with decimals validation:
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_token::instruction::BurnChecked;
/// # let source = Pubkey::new_unique();
/// # let mint = Pubkey::new_unique();
/// # let authority = Pubkey::new_unique();
/// # let fee_payer = Pubkey::new_unique();
/// let instruction = BurnChecked {
///     source,
///     mint,
///     amount: 100,
///     decimals: 8,
///     authority,
///     fee_payer,
/// }.instruction()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct BurnChecked {
    /// Light Token account to burn from
    pub source: Pubkey,
    /// Mint account (supply tracking)
    pub mint: Pubkey,
    /// Amount of tokens to burn
    pub amount: u64,
    /// Expected token decimals
    pub decimals: u8,
    /// Owner of the Light Token account
    pub authority: Pubkey,
    /// Fee payer for rent top-ups (writable signer). Authority stays readonly.
    pub fee_payer: Pubkey,
}

/// # Burn ctoken via CPI with decimals validation:
/// ```rust,no_run
/// # use light_token::instruction::BurnCheckedCpi;
/// # use solana_account_info::AccountInfo;
/// # let source: AccountInfo = todo!();
/// # let mint: AccountInfo = todo!();
/// # let authority: AccountInfo = todo!();
/// # let system_program: AccountInfo = todo!();
/// # let fee_payer: AccountInfo = todo!();
/// BurnCheckedCpi {
///     source,
///     mint,
///     amount: 100,
///     decimals: 8,
///     authority,
///     system_program,
///     fee_payer,
/// }
/// .invoke()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct BurnCheckedCpi<'info> {
    pub source: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub amount: u64,
    pub decimals: u8,
    pub authority: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    /// Fee payer for rent top-ups (writable signer). Authority stays readonly.
    pub fee_payer: AccountInfo<'info>,
}

impl<'info> BurnCheckedCpi<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        BurnChecked::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = BurnChecked::from(&self).instruction()?;
        let account_infos = [
            self.source,
            self.mint,
            self.authority,
            self.system_program,
            self.fee_payer,
        ];
        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = BurnChecked::from(&self).instruction()?;
        let account_infos = [
            self.source,
            self.mint,
            self.authority,
            self.system_program,
            self.fee_payer,
        ];
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> From<&BurnCheckedCpi<'info>> for BurnChecked {
    fn from(cpi: &BurnCheckedCpi<'info>) -> Self {
        Self {
            source: *cpi.source.key,
            mint: *cpi.mint.key,
            amount: cpi.amount,
            decimals: cpi.decimals,
            authority: *cpi.authority.key,
            fee_payer: *cpi.fee_payer.key,
        }
    }
}

impl BurnChecked {
    pub fn with_max_top_up(self, max_top_up: u16) -> BurnCheckedWithTopUp {
        BurnCheckedWithTopUp {
            inner: self,
            max_top_up,
        }
    }

    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        self.build_instruction(None)
    }

    fn build_instruction(self, max_top_up: Option<u16>) -> Result<Instruction, ProgramError> {
        let accounts = vec![
            AccountMeta::new(self.source, false),
            AccountMeta::new(self.mint, false),
            AccountMeta::new_readonly(self.authority, true),
            AccountMeta::new_readonly(Pubkey::default(), false),
            AccountMeta::new(self.fee_payer, true),
        ];

        let mut data = vec![15u8];
        data.extend_from_slice(&self.amount.to_le_bytes());
        data.push(self.decimals);
        if let Some(max_top_up) = max_top_up {
            data.extend_from_slice(&max_top_up.to_le_bytes());
        }

        Ok(Instruction {
            program_id: Pubkey::from(LIGHT_TOKEN_PROGRAM_ID),
            accounts,
            data,
        })
    }
}

pub struct BurnCheckedWithTopUp {
    inner: BurnChecked,
    max_top_up: u16,
}

impl BurnCheckedWithTopUp {
    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        self.inner.build_instruction(Some(self.max_top_up))
    }
}
