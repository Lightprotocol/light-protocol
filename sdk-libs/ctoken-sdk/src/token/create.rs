use borsh::BorshSerialize;
use light_token_interface::instructions::{
    create_token_account::CreateTokenAccountInstructionData,
    extensions::compressible::CompressibleExtensionInstructionData,
};
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::token::{compressible::CompressibleParamsCpi, CompressibleParams};

/// # Create a token account instruction:
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_token_sdk::token::CreateTokenAccount;
/// # let payer = Pubkey::new_unique();
/// # let account = Pubkey::new_unique();
/// # let mint = Pubkey::new_unique();
/// # let owner = Pubkey::new_unique();
/// let instruction =
///     CreateTokenAccount::new(payer, account, mint, owner)
///     .instruction()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
#[derive(Debug, Clone)]
pub struct CreateTokenAccount {
    pub payer: Pubkey,
    pub account: Pubkey,
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub compressible: Option<CompressibleParams>,
}

impl CreateTokenAccount {
    pub fn new(payer: Pubkey, account: Pubkey, mint: Pubkey, owner: Pubkey) -> Self {
        Self {
            payer,
            account,
            mint,
            owner,
            compressible: Some(CompressibleParams::default()),
        }
    }

    pub fn with_compressible(mut self, compressible: CompressibleParams) -> Self {
        self.compressible = Some(compressible);
        self
    }

    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        let compressible_extension =
            self.compressible
                .as_ref()
                .map(|config| CompressibleExtensionInstructionData {
                    token_account_version: config.token_account_version as u8,
                    rent_payment: config.pre_pay_num_epochs,
                    compression_only: 0,
                    write_top_up: config.lamports_per_write.unwrap_or(0),
                    compress_to_account_pubkey: config.compress_to_account_pubkey.clone(),
                });

        let instruction_data = CreateTokenAccountInstructionData {
            owner: light_compressed_account::Pubkey::from(self.owner.to_bytes()),
            compressible_config: compressible_extension,
        };

        let mut data = Vec::new();
        data.push(18u8); // InitializeAccount3 opcode
        instruction_data
            .serialize(&mut data)
            .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

        let accounts = if let Some(config) = &self.compressible {
            // Compressible account: requires payer, system program, config, and rent sponsor
            vec![
                AccountMeta::new(self.account, true),
                AccountMeta::new_readonly(self.mint, false),
                AccountMeta::new(self.payer, true),
                AccountMeta::new_readonly(config.compressible_config, false),
                AccountMeta::new_readonly(Pubkey::default(), false), // system_program
                AccountMeta::new(config.rent_sponsor, false),
            ]
        } else {
            // Non-compressible account: only account and mint
            vec![
                AccountMeta::new(self.account, false),
                AccountMeta::new_readonly(self.mint, false),
            ]
        };

        Ok(Instruction {
            program_id: Pubkey::from(light_token_interface::LIGHT_TOKEN_PROGRAM_ID),
            accounts,
            data,
        })
    }
}

/// # Create a token account via CPI:
/// ```rust,no_run
/// # use light_token_sdk::token::{CreateTokenAccountCpi, CompressibleParamsCpi};
/// # use solana_account_info::AccountInfo;
/// # use solana_pubkey::Pubkey;
/// # let payer: AccountInfo = todo!();
/// # let account: AccountInfo = todo!();
/// # let mint: AccountInfo = todo!();
/// # let owner: Pubkey = todo!();
/// # let compressible: CompressibleParamsCpi = todo!();
/// CreateTokenAccountCpi {
///     payer,
///     account,
///     mint,
///     owner,
///     compressible: Some(compressible),
/// }
/// .invoke()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct CreateTokenAccountCpi<'info> {
    pub payer: AccountInfo<'info>,
    pub account: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub owner: Pubkey,
    pub compressible: Option<CompressibleParamsCpi<'info>>,
}

impl<'info> CreateTokenAccountCpi<'info> {
    pub fn new(
        payer: AccountInfo<'info>,
        account: AccountInfo<'info>,
        mint: AccountInfo<'info>,
        owner: Pubkey,
        compressible: CompressibleParamsCpi<'info>,
    ) -> Self {
        Self {
            payer,
            account,
            mint,
            owner,
            compressible: Some(compressible),
        }
    }

    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        CreateTokenAccount::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;
        if let Some(compressible) = self.compressible {
            let account_infos = [
                self.account,
                self.mint,
                self.payer,
                compressible.compressible_config,
                compressible.system_program,
                compressible.rent_sponsor,
            ];
            invoke(&instruction, &account_infos)
        } else {
            let account_infos = [self.account, self.mint];
            invoke(&instruction, &account_infos)
        }
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;
        if let Some(compressible) = self.compressible {
            let account_infos = [
                self.account,
                self.mint,
                self.payer,
                compressible.compressible_config,
                compressible.system_program,
                compressible.rent_sponsor,
            ];
            invoke_signed(&instruction, &account_infos, signer_seeds)
        } else {
            let account_infos = [self.account, self.mint];
            invoke_signed(&instruction, &account_infos, signer_seeds)
        }
    }
}

impl<'info> From<&CreateTokenAccountCpi<'info>> for CreateTokenAccount {
    fn from(account_infos: &CreateTokenAccountCpi<'info>) -> Self {
        Self {
            payer: *account_infos.payer.key,
            account: *account_infos.account.key,
            mint: *account_infos.mint.key,
            owner: account_infos.owner,
            compressible: account_infos
                .compressible
                .as_ref()
                .map(|config| CompressibleParams {
                    compressible_config: *config.compressible_config.key,
                    rent_sponsor: *config.rent_sponsor.key,
                    pre_pay_num_epochs: config.pre_pay_num_epochs,
                    lamports_per_write: config.lamports_per_write,
                    compress_to_account_pubkey: config.compress_to_account_pubkey.clone(),
                    token_account_version: config.token_account_version,
                }),
        }
    }
}
