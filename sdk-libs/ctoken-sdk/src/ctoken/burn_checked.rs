use light_sdk_types::C_TOKEN_PROGRAM_ID;
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

/// # Burn tokens from a ctoken account with decimals validation:
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_ctoken_sdk::ctoken::BurnCTokenChecked;
/// # let source = Pubkey::new_unique();
/// # let cmint = Pubkey::new_unique();
/// # let authority = Pubkey::new_unique();
/// let instruction = BurnCTokenChecked {
///     source,
///     cmint,
///     amount: 100,
///     decimals: 8,
///     authority,
///     max_top_up: None,
/// }.instruction()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct BurnCTokenChecked {
    /// CToken account to burn from
    pub source: Pubkey,
    /// CMint account (supply tracking)
    pub cmint: Pubkey,
    /// Amount of tokens to burn
    pub amount: u64,
    /// Expected token decimals
    pub decimals: u8,
    /// Owner of the CToken account
    pub authority: Pubkey,
    /// Maximum lamports for rent and top-up combined. Transaction fails if exceeded. (0 = no limit)
    /// When set to a non-zero value, includes max_top_up in instruction data
    pub max_top_up: Option<u16>,
}

/// # Burn ctoken via CPI with decimals validation:
/// ```rust,no_run
/// # use light_ctoken_sdk::ctoken::BurnCTokenCheckedCpi;
/// # use solana_account_info::AccountInfo;
/// # let source: AccountInfo = todo!();
/// # let cmint: AccountInfo = todo!();
/// # let authority: AccountInfo = todo!();
/// BurnCTokenCheckedCpi {
///     source,
///     cmint,
///     amount: 100,
///     decimals: 8,
///     authority,
///     max_top_up: None,
/// }
/// .invoke()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct BurnCTokenCheckedCpi<'info> {
    pub source: AccountInfo<'info>,
    pub cmint: AccountInfo<'info>,
    pub amount: u64,
    pub decimals: u8,
    pub authority: AccountInfo<'info>,
    /// Maximum lamports for rent and top-up combined. Transaction fails if exceeded. (0 = no limit)
    pub max_top_up: Option<u16>,
}

impl<'info> BurnCTokenCheckedCpi<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        BurnCTokenChecked::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = BurnCTokenChecked::from(&self).instruction()?;
        let account_infos = [self.source, self.cmint, self.authority];
        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = BurnCTokenChecked::from(&self).instruction()?;
        let account_infos = [self.source, self.cmint, self.authority];
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> From<&BurnCTokenCheckedCpi<'info>> for BurnCTokenChecked {
    fn from(cpi: &BurnCTokenCheckedCpi<'info>) -> Self {
        Self {
            source: *cpi.source.key,
            cmint: *cpi.cmint.key,
            amount: cpi.amount,
            decimals: cpi.decimals,
            authority: *cpi.authority.key,
            max_top_up: cpi.max_top_up,
        }
    }
}

impl BurnCTokenChecked {
    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        Ok(Instruction {
            program_id: Pubkey::from(C_TOKEN_PROGRAM_ID),
            accounts: vec![
                AccountMeta::new(self.source, false),
                AccountMeta::new(self.cmint, false),
                AccountMeta::new_readonly(self.authority, true),
            ],
            data: {
                let mut data = vec![15u8]; // CTokenBurnChecked discriminator
                data.extend_from_slice(&self.amount.to_le_bytes());
                data.push(self.decimals);
                // Include max_top_up if set (11-byte format)
                if let Some(max_top_up) = self.max_top_up {
                    data.extend_from_slice(&max_top_up.to_le_bytes());
                }
                data
            },
        })
    }
}
