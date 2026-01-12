use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

/// # Mint tokens to a ctoken account (simple 3-account instruction):
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_token_sdk::token::TokenMintTo;
/// # let cmint = Pubkey::new_unique();
/// # let destination = Pubkey::new_unique();
/// # let authority = Pubkey::new_unique();
/// let instruction = TokenMintTo {
///     cmint,
///     destination,
///     amount: 100,
///     authority,
///     max_top_up: None,
/// }.instruction()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct TokenMintTo {
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
/// # use light_token_sdk::token::TokenMintToCpi;
/// # use solana_account_info::AccountInfo;
/// # let cmint: AccountInfo = todo!();
/// # let destination: AccountInfo = todo!();
/// # let authority: AccountInfo = todo!();
/// # let system_program: AccountInfo = todo!();
/// TokenMintToCpi {
///     cmint,
///     destination,
///     amount: 100,
///     authority,
///     system_program,
///     max_top_up: None,
/// }
/// .invoke()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct TokenMintToCpi<'info> {
    pub cmint: AccountInfo<'info>,
    pub destination: AccountInfo<'info>,
    pub amount: u64,
    pub authority: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    /// Maximum lamports for rent and top-up combined. Transaction fails if exceeded. (0 = no limit)
    pub max_top_up: Option<u16>,
}

impl<'info> TokenMintToCpi<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        TokenMintTo::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = TokenMintTo::from(&self).instruction()?;
        let account_infos = [
            self.cmint,
            self.destination,
            self.authority,
            self.system_program,
        ];
        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = TokenMintTo::from(&self).instruction()?;
        let account_infos = [
            self.cmint,
            self.destination,
            self.authority,
            self.system_program,
        ];
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> From<&TokenMintToCpi<'info>> for TokenMintTo {
    fn from(cpi: &TokenMintToCpi<'info>) -> Self {
        Self {
            cmint: *cpi.cmint.key,
            destination: *cpi.destination.key,
            amount: cpi.amount,
            authority: *cpi.authority.key,
            max_top_up: cpi.max_top_up,
        }
    }
}

impl TokenMintTo {
    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        Ok(Instruction {
            program_id: Pubkey::from(LIGHT_TOKEN_PROGRAM_ID),
            accounts: vec![
                AccountMeta::new(self.cmint, false),
                AccountMeta::new(self.destination, false),
                AccountMeta::new(self.authority, true),
                AccountMeta::new_readonly(Pubkey::default(), false), // System program for lamport transfers
            ],
            data: {
                let mut data = vec![7u8]; // TokenMintTo discriminator
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
