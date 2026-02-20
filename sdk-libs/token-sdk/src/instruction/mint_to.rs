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
/// # let fee_payer = Pubkey::new_unique();
/// let instruction = MintTo {
///     mint,
///     destination,
///     amount: 100,
///     authority,
///     fee_payer,
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
    /// Fee payer for rent top-ups (writable signer). Authority stays readonly.
    pub fee_payer: Pubkey,
}

/// # Mint to ctoken via CPI:
/// ```rust,no_run
/// # use light_token::instruction::MintToCpi;
/// # use solana_account_info::AccountInfo;
/// # let mint: AccountInfo = todo!();
/// # let destination: AccountInfo = todo!();
/// # let authority: AccountInfo = todo!();
/// # let system_program: AccountInfo = todo!();
/// # let fee_payer: AccountInfo = todo!();
/// MintToCpi {
///     mint,
///     destination,
///     amount: 100,
///     authority,
///     system_program,
///     fee_payer,
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
    /// Fee payer for rent top-ups (writable signer). Authority stays readonly.
    pub fee_payer: AccountInfo<'info>,
}

impl<'info> MintToCpi<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        MintTo::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = MintTo::from(&self).instruction()?;
        let account_infos = [
            self.mint,
            self.destination,
            self.authority,
            self.system_program,
            self.fee_payer,
        ];
        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = MintTo::from(&self).instruction()?;
        let account_infos = [
            self.mint,
            self.destination,
            self.authority,
            self.system_program,
            self.fee_payer,
        ];
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> From<&MintToCpi<'info>> for MintTo {
    fn from(cpi: &MintToCpi<'info>) -> Self {
        Self {
            mint: *cpi.mint.key,
            destination: *cpi.destination.key,
            amount: cpi.amount,
            authority: *cpi.authority.key,
            fee_payer: *cpi.fee_payer.key,
        }
    }
}

impl MintTo {
    pub fn with_max_top_up(self, max_top_up: u16) -> MintToWithTopUp {
        MintToWithTopUp {
            inner: self,
            max_top_up,
        }
    }

    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        self.build_instruction(None)
    }

    fn build_instruction(self, max_top_up: Option<u16>) -> Result<Instruction, ProgramError> {
        let accounts = vec![
            AccountMeta::new(self.mint, false),
            AccountMeta::new(self.destination, false),
            AccountMeta::new_readonly(self.authority, true),
            AccountMeta::new_readonly(Pubkey::default(), false),
            AccountMeta::new(self.fee_payer, true),
        ];

        let mut data = vec![7u8];
        data.extend_from_slice(&self.amount.to_le_bytes());
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

pub struct MintToWithTopUp {
    inner: MintTo,
    max_top_up: u16,
}

impl MintToWithTopUp {
    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        self.inner.build_instruction(Some(self.max_top_up))
    }
}
