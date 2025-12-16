use light_sdk_types::C_TOKEN_PROGRAM_ID;
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

/// # Mint tokens to a ctoken account (simple 3-account instruction):
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_ctoken_sdk::ctoken::CTokenMintTo;
/// # let cmint = Pubkey::new_unique();
/// # let destination = Pubkey::new_unique();
/// # let authority = Pubkey::new_unique();
/// let instruction = CTokenMintTo {
///     cmint,
///     destination,
///     amount: 100,
///     authority,
///     max_top_up: None,
/// }.instruction()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct CTokenMintTo {
    /// CMint account (supply tracking)
    pub cmint: Pubkey,
    /// Destination CToken account to mint to
    pub destination: Pubkey,
    /// Amount of tokens to mint
    pub amount: u64,
    /// Mint authority
    pub authority: Pubkey,
    /// Maximum lamports for rent and top-up combined. Transaction fails if exceeded. (0 = no limit)
    /// When set to a non-zero value, includes max_top_up in instruction data
    pub max_top_up: Option<u16>,
}

/// # Mint to ctoken via CPI:
/// ```rust,no_run
/// # use light_ctoken_sdk::ctoken::CTokenMintToCpi;
/// # use solana_account_info::AccountInfo;
/// # let cmint: AccountInfo = todo!();
/// # let destination: AccountInfo = todo!();
/// # let authority: AccountInfo = todo!();
/// CTokenMintToCpi {
///     cmint,
///     destination,
///     amount: 100,
///     authority,
///     max_top_up: None,
/// }
/// .invoke()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct CTokenMintToCpi<'info> {
    pub cmint: AccountInfo<'info>,
    pub destination: AccountInfo<'info>,
    pub amount: u64,
    pub authority: AccountInfo<'info>,
    /// Maximum lamports for rent and top-up combined. Transaction fails if exceeded. (0 = no limit)
    pub max_top_up: Option<u16>,
}

impl<'info> CTokenMintToCpi<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        CTokenMintTo::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = CTokenMintTo::from(&self).instruction()?;
        let account_infos = [self.cmint, self.destination, self.authority];
        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = CTokenMintTo::from(&self).instruction()?;
        let account_infos = [self.cmint, self.destination, self.authority];
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> From<&CTokenMintToCpi<'info>> for CTokenMintTo {
    fn from(cpi: &CTokenMintToCpi<'info>) -> Self {
        Self {
            cmint: *cpi.cmint.key,
            destination: *cpi.destination.key,
            amount: cpi.amount,
            authority: *cpi.authority.key,
            max_top_up: cpi.max_top_up,
        }
    }
}

impl CTokenMintTo {
    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        Ok(Instruction {
            program_id: Pubkey::from(C_TOKEN_PROGRAM_ID),
            accounts: vec![
                AccountMeta::new(self.cmint, false),
                AccountMeta::new(self.destination, false),
                AccountMeta::new_readonly(self.authority, true),
            ],
            data: {
                let mut data = vec![7u8]; // CTokenMintTo discriminator
                data.extend_from_slice(&self.amount.to_le_bytes());
                // Include max_top_up if set (10-byte format)
                if let Some(max_top_up) = self.max_top_up {
                    data.extend_from_slice(&max_top_up.to_le_bytes());
                }
                data
            },
        })
    }
}
