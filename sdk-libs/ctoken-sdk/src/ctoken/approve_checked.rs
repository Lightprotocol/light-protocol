use light_sdk_types::C_TOKEN_PROGRAM_ID;
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

/// # Approve a delegate for a CToken account with decimals validation:
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_ctoken_sdk::ctoken::ApproveCTokenChecked;
/// # let token_account = Pubkey::new_unique();
/// # let mint = Pubkey::new_unique();
/// # let delegate = Pubkey::new_unique();
/// # let owner = Pubkey::new_unique();
/// let instruction = ApproveCTokenChecked {
///     token_account,
///     mint,
///     delegate,
///     owner,
///     amount: 100,
///     decimals: 8,
///     max_top_up: None,
/// }.instruction()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct ApproveCTokenChecked {
    /// CToken account to approve delegation for
    pub token_account: Pubkey,
    /// Mint account (for decimals validation - may be skipped if CToken has cached decimals)
    pub mint: Pubkey,
    /// Delegate to approve
    pub delegate: Pubkey,
    /// Owner of the CToken account (signer, payer for top-up)
    pub owner: Pubkey,
    /// Amount of tokens to delegate
    pub amount: u64,
    /// Expected token decimals
    pub decimals: u8,
    /// Maximum lamports for rent top-up. Transaction fails if exceeded. (0 = no limit)
    pub max_top_up: Option<u16>,
}

/// # Approve CToken via CPI with decimals validation:
/// ```rust,no_run
/// # use light_ctoken_sdk::ctoken::ApproveCTokenCheckedCpi;
/// # use solana_account_info::AccountInfo;
/// # let token_account: AccountInfo = todo!();
/// # let mint: AccountInfo = todo!();
/// # let delegate: AccountInfo = todo!();
/// # let owner: AccountInfo = todo!();
/// # let system_program: AccountInfo = todo!();
/// ApproveCTokenCheckedCpi {
///     token_account,
///     mint,
///     delegate,
///     owner,
///     system_program,
///     amount: 100,
///     decimals: 8,
///     max_top_up: None,
/// }
/// .invoke()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct ApproveCTokenCheckedCpi<'info> {
    pub token_account: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub delegate: AccountInfo<'info>,
    pub owner: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub amount: u64,
    pub decimals: u8,
    /// Maximum lamports for rent top-up. Transaction fails if exceeded. (0 = no limit)
    pub max_top_up: Option<u16>,
}

impl<'info> ApproveCTokenCheckedCpi<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        ApproveCTokenChecked::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = ApproveCTokenChecked::from(&self).instruction()?;
        let account_infos = [
            self.token_account,
            self.mint,
            self.delegate,
            self.owner,
            self.system_program,
        ];
        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = ApproveCTokenChecked::from(&self).instruction()?;
        let account_infos = [
            self.token_account,
            self.mint,
            self.delegate,
            self.owner,
            self.system_program,
        ];
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> From<&ApproveCTokenCheckedCpi<'info>> for ApproveCTokenChecked {
    fn from(cpi: &ApproveCTokenCheckedCpi<'info>) -> Self {
        Self {
            token_account: *cpi.token_account.key,
            mint: *cpi.mint.key,
            delegate: *cpi.delegate.key,
            owner: *cpi.owner.key,
            amount: cpi.amount,
            decimals: cpi.decimals,
            max_top_up: cpi.max_top_up,
        }
    }
}

impl ApproveCTokenChecked {
    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        let mut data = vec![13u8]; // CTokenApproveChecked discriminator (SPL compatible)
        data.extend_from_slice(&self.amount.to_le_bytes());
        data.push(self.decimals);
        // Include max_top_up if set (11-byte format)
        if let Some(max_top_up) = self.max_top_up {
            data.extend_from_slice(&max_top_up.to_le_bytes());
        }

        Ok(Instruction {
            program_id: Pubkey::from(C_TOKEN_PROGRAM_ID),
            accounts: vec![
                AccountMeta::new(self.token_account, false),
                AccountMeta::new_readonly(self.mint, false),
                AccountMeta::new_readonly(self.delegate, false),
                AccountMeta::new(self.owner, true),
                AccountMeta::new_readonly(Pubkey::default(), false),
            ],
            data,
        })
    }
}
