use light_sdk_types::C_TOKEN_PROGRAM_ID;
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

/// # Revoke delegation for a CToken account:
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_ctoken_sdk::ctoken::RevokeCToken;
/// # let token_account = Pubkey::new_unique();
/// # let owner = Pubkey::new_unique();
/// let instruction = RevokeCToken {
///     token_account,
///     owner,
/// }.instruction()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct RevokeCToken {
    /// CToken account to revoke delegation for
    pub token_account: Pubkey,
    /// Owner of the CToken account (signer, payer for top-up)
    pub owner: Pubkey,
}

/// # Revoke CToken via CPI:
/// ```rust,no_run
/// # use light_ctoken_sdk::ctoken::RevokeCTokenCpi;
/// # use solana_account_info::AccountInfo;
/// # let token_account: AccountInfo = todo!();
/// # let owner: AccountInfo = todo!();
/// # let system_program: AccountInfo = todo!();
/// RevokeCTokenCpi {
///     token_account,
///     owner,
///     system_program,
/// }
/// .invoke()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct RevokeCTokenCpi<'info> {
    pub token_account: AccountInfo<'info>,
    pub owner: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
}

impl<'info> RevokeCTokenCpi<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        RevokeCToken::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = RevokeCToken::from(&self).instruction()?;
        let account_infos = [self.token_account, self.owner, self.system_program];
        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = RevokeCToken::from(&self).instruction()?;
        let account_infos = [self.token_account, self.owner, self.system_program];
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> From<&RevokeCTokenCpi<'info>> for RevokeCToken {
    fn from(cpi: &RevokeCTokenCpi<'info>) -> Self {
        Self {
            token_account: *cpi.token_account.key,
            owner: *cpi.owner.key,
        }
    }
}

impl RevokeCToken {
    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        Ok(Instruction {
            program_id: Pubkey::from(C_TOKEN_PROGRAM_ID),
            accounts: vec![
                AccountMeta::new(self.token_account, false),
                AccountMeta::new(self.owner, true),
                AccountMeta::new_readonly(Pubkey::default(), false),
            ],
            data: vec![5u8], // CTokenRevoke discriminator
        })
    }
}
