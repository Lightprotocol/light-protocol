use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

/// # Revoke delegation for a Light Token account:
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_token::instruction::Revoke;
/// # let token_account = Pubkey::new_unique();
/// # let owner = Pubkey::new_unique();
/// let instruction = Revoke {
///     token_account,
///     owner,
///     max_top_up: None,
/// }.instruction()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct Revoke {
    /// Light Token account to revoke delegation for
    pub token_account: Pubkey,
    /// Owner of the Light Token account (signer, payer for top-up)
    pub owner: Pubkey,
    /// Maximum lamports for compressible top-up. Transaction fails if exceeded.
    /// When set, includes max_top_up in instruction data and marks owner writable.
    pub max_top_up: Option<u16>,
}

/// # Revoke Light Token via CPI:
/// ```rust,no_run
/// # use light_token::instruction::RevokeCpi;
/// # use solana_account_info::AccountInfo;
/// # let token_account: AccountInfo = todo!();
/// # let owner: AccountInfo = todo!();
/// # let system_program: AccountInfo = todo!();
/// RevokeCpi {
///     token_account,
///     owner,
///     system_program,
///     max_top_up: None,
/// }
/// .invoke()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct RevokeCpi<'info> {
    pub token_account: AccountInfo<'info>,
    pub owner: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub max_top_up: Option<u16>,
}

impl<'info> RevokeCpi<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        Revoke::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = Revoke::from(&self).instruction()?;
        let account_infos = [self.token_account, self.owner, self.system_program];
        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = Revoke::from(&self).instruction()?;
        let account_infos = [self.token_account, self.owner, self.system_program];
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> From<&RevokeCpi<'info>> for Revoke {
    fn from(cpi: &RevokeCpi<'info>) -> Self {
        Self {
            token_account: *cpi.token_account.key,
            owner: *cpi.owner.key,
            max_top_up: cpi.max_top_up,
        }
    }
}

impl Revoke {
    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        let mut data = vec![5u8]; // CTokenRevoke discriminator
        if let Some(max_top_up) = self.max_top_up {
            data.extend_from_slice(&max_top_up.to_le_bytes());
        }

        let owner_meta = if self.max_top_up.is_some() {
            AccountMeta::new(self.owner, true)
        } else {
            AccountMeta::new_readonly(self.owner, true)
        };

        Ok(Instruction {
            program_id: Pubkey::from(LIGHT_TOKEN_PROGRAM_ID),
            accounts: vec![
                AccountMeta::new(self.token_account, false),
                owner_meta,
                AccountMeta::new_readonly(Pubkey::default(), false),
            ],
            data,
        })
    }
}
