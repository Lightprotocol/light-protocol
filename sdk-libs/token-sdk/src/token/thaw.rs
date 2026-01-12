use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

/// # Thaw a frozen CToken account:
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_token_sdk::token::ThawToken;
/// # let token_account = Pubkey::new_unique();
/// # let mint = Pubkey::new_unique();
/// # let freeze_authority = Pubkey::new_unique();
/// let instruction = ThawToken {
///     token_account,
///     mint,
///     freeze_authority,
/// }.instruction()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct ThawToken {
    /// CToken account to thaw
    pub token_account: Pubkey,
    /// Mint of the token account
    pub mint: Pubkey,
    /// Freeze authority (signer)
    pub freeze_authority: Pubkey,
}

/// # Thaw CToken via CPI:
/// ```rust,no_run
/// # use light_token_sdk::token::ThawTokenCpi;
/// # use solana_account_info::AccountInfo;
/// # let token_account: AccountInfo = todo!();
/// # let mint: AccountInfo = todo!();
/// # let freeze_authority: AccountInfo = todo!();
/// ThawTokenCpi {
///     token_account,
///     mint,
///     freeze_authority,
/// }
/// .invoke()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct ThawTokenCpi<'info> {
    pub token_account: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub freeze_authority: AccountInfo<'info>,
}

impl<'info> ThawTokenCpi<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        ThawToken::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = ThawToken::from(&self).instruction()?;
        let account_infos = [self.token_account, self.mint, self.freeze_authority];
        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = ThawToken::from(&self).instruction()?;
        let account_infos = [self.token_account, self.mint, self.freeze_authority];
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> From<&ThawTokenCpi<'info>> for ThawToken {
    fn from(cpi: &ThawTokenCpi<'info>) -> Self {
        Self {
            token_account: *cpi.token_account.key,
            mint: *cpi.mint.key,
            freeze_authority: *cpi.freeze_authority.key,
        }
    }
}

impl ThawToken {
    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        Ok(Instruction {
            program_id: Pubkey::from(LIGHT_TOKEN_PROGRAM_ID),
            accounts: vec![
                AccountMeta::new(self.token_account, false),
                AccountMeta::new_readonly(self.mint, false),
                AccountMeta::new_readonly(self.freeze_authority, true),
            ],
            data: vec![11u8], // CTokenThawAccount discriminator
        })
    }
}
