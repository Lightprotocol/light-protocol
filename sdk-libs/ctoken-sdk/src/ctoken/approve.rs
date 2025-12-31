use light_sdk_types::C_TOKEN_PROGRAM_ID;
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

/// # Approve a delegate for a CToken account:
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_ctoken_sdk::ctoken::ApproveCToken;
/// # let token_account = Pubkey::new_unique();
/// # let delegate = Pubkey::new_unique();
/// # let owner = Pubkey::new_unique();
/// let instruction = ApproveCToken {
///     token_account,
///     delegate,
///     owner,
///     amount: 100,
/// }.instruction()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct ApproveCToken {
    /// CToken account to approve delegation for
    pub token_account: Pubkey,
    /// Delegate to approve
    pub delegate: Pubkey,
    /// Owner of the CToken account (signer, payer for top-up)
    pub owner: Pubkey,
    /// Amount of tokens to delegate
    pub amount: u64,
}

/// # Approve CToken via CPI:
/// ```rust,no_run
/// # use light_ctoken_sdk::ctoken::ApproveCTokenCpi;
/// # use solana_account_info::AccountInfo;
/// # let token_account: AccountInfo = todo!();
/// # let delegate: AccountInfo = todo!();
/// # let owner: AccountInfo = todo!();
/// # let system_program: AccountInfo = todo!();
/// ApproveCTokenCpi {
///     token_account,
///     delegate,
///     owner,
///     system_program,
///     amount: 100,
/// }
/// .invoke()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct ApproveCTokenCpi<'info> {
    pub token_account: AccountInfo<'info>,
    pub delegate: AccountInfo<'info>,
    pub owner: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub amount: u64,
}

impl<'info> ApproveCTokenCpi<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        ApproveCToken::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = ApproveCToken::from(&self).instruction()?;
        let account_infos = [
            self.token_account,
            self.delegate,
            self.owner,
            self.system_program,
        ];
        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = ApproveCToken::from(&self).instruction()?;
        let account_infos = [
            self.token_account,
            self.delegate,
            self.owner,
            self.system_program,
        ];
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> From<&ApproveCTokenCpi<'info>> for ApproveCToken {
    fn from(cpi: &ApproveCTokenCpi<'info>) -> Self {
        Self {
            token_account: *cpi.token_account.key,
            delegate: *cpi.delegate.key,
            owner: *cpi.owner.key,
            amount: cpi.amount,
        }
    }
}

impl ApproveCToken {
    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        let mut data = vec![4u8]; // CTokenApprove discriminator
        data.extend_from_slice(&self.amount.to_le_bytes());

        Ok(Instruction {
            program_id: Pubkey::from(C_TOKEN_PROGRAM_ID),
            accounts: vec![
                AccountMeta::new(self.token_account, false),
                AccountMeta::new_readonly(self.delegate, false),
                AccountMeta::new(self.owner, true),
                AccountMeta::new_readonly(Pubkey::default(), false),
            ],
            data,
        })
    }
}
