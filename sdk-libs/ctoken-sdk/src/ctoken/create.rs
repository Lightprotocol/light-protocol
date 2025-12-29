use borsh::BorshSerialize;
use light_ctoken_interface::instructions::create_ctoken_account::CreateTokenAccountInstructionData;
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::ctoken::{compressible::CompressibleParamsCpi, CompressibleParams};

/// # Create a create ctoken account instruction:
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_ctoken_sdk::ctoken::CreateCTokenAccount;
/// # let payer = Pubkey::new_unique();
/// # let account = Pubkey::new_unique();
/// # let mint = Pubkey::new_unique();
/// # let owner = Pubkey::new_unique();
/// let instruction =
///     CreateCTokenAccount::new(payer, account, mint, owner)
///     .instruction()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
#[derive(Debug, Clone)]
pub struct CreateCTokenAccount {
    pub payer: Pubkey,
    pub account: Pubkey,
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub compressible: CompressibleParams,
}

impl CreateCTokenAccount {
    pub fn new(payer: Pubkey, account: Pubkey, mint: Pubkey, owner: Pubkey) -> Self {
        Self {
            payer,
            account,
            mint,
            owner,
            compressible: CompressibleParams::default(),
        }
    }

    pub fn with_compressible(mut self, compressible: CompressibleParams) -> Self {
        self.compressible = compressible;
        self
    }

    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        let instruction_data = CreateTokenAccountInstructionData {
            owner: light_compressed_account::Pubkey::from(self.owner.to_bytes()),
            token_account_version: self.compressible.token_account_version as u8,
            rent_payment: self.compressible.pre_pay_num_epochs,
            compression_only: self.compressible.compression_only as u8,
            write_top_up: self.compressible.lamports_per_write.unwrap_or(0),
            compressible_config: self.compressible.compress_to_account_pubkey.clone(),
        };

        let mut data = Vec::new();
        data.push(18u8); // InitializeAccount3 opcode
        instruction_data
            .serialize(&mut data)
            .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

        let accounts = vec![
            AccountMeta::new(self.account, true),
            AccountMeta::new_readonly(self.mint, false),
            AccountMeta::new(self.payer, true),
            AccountMeta::new_readonly(self.compressible.compressible_config, false),
            AccountMeta::new_readonly(Pubkey::default(), false), // system_program
            AccountMeta::new(self.compressible.rent_sponsor, false),
        ];

        Ok(Instruction {
            program_id: Pubkey::from(light_ctoken_interface::CTOKEN_PROGRAM_ID),
            accounts,
            data,
        })
    }
}

/// # Create a ctoken account via CPI:
/// ```rust,no_run
/// # use light_ctoken_sdk::ctoken::{CreateCTokenAccountCpi, CompressibleParamsCpi};
/// # use solana_account_info::AccountInfo;
/// # use solana_pubkey::Pubkey;
/// # let payer: AccountInfo = todo!();
/// # let account: AccountInfo = todo!();
/// # let mint: AccountInfo = todo!();
/// # let owner: Pubkey = todo!();
/// # let compressible: CompressibleParamsCpi = todo!();
/// CreateCTokenAccountCpi {
///     payer,
///     account,
///     mint,
///     owner,
///     compressible,
/// }
/// .invoke()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct CreateCTokenAccountCpi<'info> {
    pub payer: AccountInfo<'info>,
    pub account: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub owner: Pubkey,
    pub compressible: CompressibleParamsCpi<'info>,
}

impl<'info> CreateCTokenAccountCpi<'info> {
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
            compressible,
        }
    }

    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        CreateCTokenAccount::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;
        let account_infos = [
            self.account,
            self.mint,
            self.payer,
            self.compressible.compressible_config,
            self.compressible.system_program,
            self.compressible.rent_sponsor,
        ];
        invoke(&instruction, &account_infos)
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;
        let account_infos = [
            self.account,
            self.mint,
            self.payer,
            self.compressible.compressible_config,
            self.compressible.system_program,
            self.compressible.rent_sponsor,
        ];
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}

impl<'info> From<&CreateCTokenAccountCpi<'info>> for CreateCTokenAccount {
    fn from(account_infos: &CreateCTokenAccountCpi<'info>) -> Self {
        Self {
            payer: *account_infos.payer.key,
            account: *account_infos.account.key,
            mint: *account_infos.mint.key,
            owner: account_infos.owner,
            compressible: CompressibleParams {
                compressible_config: *account_infos.compressible.compressible_config.key,
                rent_sponsor: *account_infos.compressible.rent_sponsor.key,
                pre_pay_num_epochs: account_infos.compressible.pre_pay_num_epochs,
                lamports_per_write: account_infos.compressible.lamports_per_write,
                compress_to_account_pubkey: account_infos
                    .compressible
                    .compress_to_account_pubkey
                    .clone(),
                token_account_version: account_infos.compressible.token_account_version,
                compression_only: account_infos.compressible.compression_only,
            },
        }
    }
}
