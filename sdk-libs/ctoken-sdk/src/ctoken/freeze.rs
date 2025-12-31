use light_sdk_types::C_TOKEN_PROGRAM_ID;
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

/// # Freeze a CToken account:
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_ctoken_sdk::ctoken::FreezeCToken;
/// # let token_account = Pubkey::new_unique();
/// # let mint = Pubkey::new_unique();
/// # let freeze_authority = Pubkey::new_unique();
/// let instruction = FreezeCToken {
///     token_account,
///     mint,
///     freeze_authority,
/// }.instruction()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct FreezeCToken {
    /// CToken account to freeze
    pub token_account: Pubkey,
    /// Mint of the token account
    pub mint: Pubkey,
    /// Freeze authority (signer)
    pub freeze_authority: Pubkey,
}

/// # Freeze CToken via CPI:
/// ```rust,no_run
/// # use light_ctoken_sdk::ctoken::FreezeCTokenCpi;
/// # use solana_account_info::AccountInfo;
/// # let token_account: AccountInfo = todo!();
/// # let mint: AccountInfo = todo!();
/// # let freeze_authority: AccountInfo = todo!();
/// FreezeCTokenCpi {
///     token_account,
///     mint,
///     freeze_authority,
/// }
/// .invoke()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct FreezeCTokenCpi<'info> {
    pub token_account: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub freeze_authority: AccountInfo<'info>,
}

impl<'info> FreezeCTokenCpi<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        FreezeCToken::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = FreezeCToken::from(&self).instruction()?;
        let account_infos = [self.token_account, self.mint, self.freeze_authority];
        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = FreezeCToken::from(&self).instruction()?;
        let account_infos = [self.token_account, self.mint, self.freeze_authority];
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> From<&FreezeCTokenCpi<'info>> for FreezeCToken {
    fn from(cpi: &FreezeCTokenCpi<'info>) -> Self {
        Self {
            token_account: *cpi.token_account.key,
            mint: *cpi.mint.key,
            freeze_authority: *cpi.freeze_authority.key,
        }
    }
}

impl FreezeCToken {
    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        Ok(Instruction {
            program_id: Pubkey::from(C_TOKEN_PROGRAM_ID),
            accounts: vec![
                AccountMeta::new(self.token_account, false),
                AccountMeta::new_readonly(self.mint, false),
                AccountMeta::new_readonly(self.freeze_authority, true),
            ],
            data: vec![10u8], // CTokenFreezeAccount discriminator
        })
    }
}
