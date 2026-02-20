use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

/// # Approve a delegate for a Light Token account:
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_token::instruction::Approve;
/// # let token_account = Pubkey::new_unique();
/// # let delegate = Pubkey::new_unique();
/// # let owner = Pubkey::new_unique();
/// # let fee_payer = Pubkey::new_unique();
/// let instruction = Approve {
///     token_account,
///     delegate,
///     owner,
///     amount: 100,
///     fee_payer,
/// }.instruction()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct Approve {
    /// Light Token account to approve delegation for
    pub token_account: Pubkey,
    /// Delegate to approve
    pub delegate: Pubkey,
    /// Owner of the Light Token account (signer, writable — on-chain requires owner to pay)
    pub owner: Pubkey,
    /// Amount of tokens to delegate
    pub amount: u64,
    // TODO: fee_payer will be sent as a separate account when on-chain supports it.
    // Currently owner pays for top-ups directly.
    /// Fee payer for rent top-ups. Not yet sent to on-chain (owner pays instead).
    pub fee_payer: Pubkey,
}

/// # Approve Light Token via CPI:
/// ```rust,no_run
/// # use light_token::instruction::ApproveCpi;
/// # use solana_account_info::AccountInfo;
/// # let token_account: AccountInfo = todo!();
/// # let delegate: AccountInfo = todo!();
/// # let owner: AccountInfo = todo!();
/// # let system_program: AccountInfo = todo!();
/// # let fee_payer: AccountInfo = todo!();
/// ApproveCpi {
///     token_account,
///     delegate,
///     owner,
///     system_program,
///     amount: 100,
///     fee_payer,
/// }
/// .invoke()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct ApproveCpi<'info> {
    pub token_account: AccountInfo<'info>,
    pub delegate: AccountInfo<'info>,
    pub owner: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub amount: u64,
    // TODO: fee_payer will be sent as a separate account when on-chain supports it.
    /// Fee payer for rent top-ups. Not yet sent to on-chain (owner pays instead).
    pub fee_payer: AccountInfo<'info>,
}

impl<'info> ApproveCpi<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        Approve::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = Approve::from(&self).instruction()?;
        let account_infos = [
            self.token_account,
            self.delegate,
            self.owner,
            self.system_program,
        ];
        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = Approve::from(&self).instruction()?;
        let account_infos = [
            self.token_account,
            self.delegate,
            self.owner,
            self.system_program,
        ];
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> From<&ApproveCpi<'info>> for Approve {
    fn from(cpi: &ApproveCpi<'info>) -> Self {
        Self {
            token_account: *cpi.token_account.key,
            delegate: *cpi.delegate.key,
            owner: *cpi.owner.key,
            amount: cpi.amount,
            fee_payer: *cpi.fee_payer.key,
        }
    }
}

impl Approve {
    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        let mut data = vec![4u8]; // CTokenApprove discriminator
        data.extend_from_slice(&self.amount.to_le_bytes());

        Ok(Instruction {
            program_id: Pubkey::from(LIGHT_TOKEN_PROGRAM_ID),
            // Owner is writable (on-chain requires it — no fee_payer support yet)
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
