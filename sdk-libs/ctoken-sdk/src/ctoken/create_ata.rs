use borsh::BorshSerialize;
use light_ctoken_interface::instructions::{
    create_associated_token_account::CreateAssociatedTokenAccountInstructionData,
    extensions::compressible::CompressibleExtensionInstructionData,
};
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::ctoken::{compressible::CompressibleParamsCpi, CompressibleParams};

const CREATE_ATA_DISCRIMINATOR: u8 = 100;
const CREATE_ATA_IDEMPOTENT_DISCRIMINATOR: u8 = 102;

pub fn derive_ctoken_ata(owner: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            owner.as_ref(),
            light_ctoken_interface::COMPRESSED_TOKEN_PROGRAM_ID.as_ref(),
            mint.as_ref(),
        ],
        &Pubkey::from(light_ctoken_interface::COMPRESSED_TOKEN_PROGRAM_ID),
    )
}

/// # Create an associated ctoken account instruction:
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_ctoken_sdk::ctoken::CreateAssociatedTokenAccount;
/// # let payer = Pubkey::new_unique();
/// # let owner = Pubkey::new_unique();
/// # let mint = Pubkey::new_unique();
/// let instruction =
///     CreateAssociatedTokenAccount::new(payer, owner, mint)
///     .instruction()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
#[derive(Debug, Clone)]
pub struct CreateAssociatedTokenAccount {
    pub payer: Pubkey,
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub associated_token_account: Pubkey,
    pub bump: u8,
    pub compressible: Option<CompressibleParams>,
    pub idempotent: bool,
}

impl CreateAssociatedTokenAccount {
    pub fn new(payer: Pubkey, owner: Pubkey, mint: Pubkey) -> Self {
        let (ata, bump) = derive_ctoken_ata(&owner, &mint);
        Self {
            payer,
            owner,
            mint,
            associated_token_account: ata,
            bump,
            compressible: Some(CompressibleParams::default()),
            idempotent: false,
        }
    }

    pub fn new_with_bump(
        payer: Pubkey,
        owner: Pubkey,
        mint: Pubkey,
        associated_token_account: Pubkey,
        bump: u8,
    ) -> Self {
        Self {
            payer,
            owner,
            mint,
            associated_token_account,
            bump,
            compressible: Some(CompressibleParams::default()),
            idempotent: false,
        }
    }

    pub fn with_compressible(mut self, compressible_params: CompressibleParams) -> Self {
        self.compressible = Some(compressible_params);
        self
    }

    pub fn idempotent(mut self) -> Self {
        self.idempotent = true;
        self
    }

    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        let compressible_extension =
            self.compressible
                .as_ref()
                .map(|config| CompressibleExtensionInstructionData {
                    token_account_version: config.token_account_version as u8,
                    rent_payment: config.pre_pay_num_epochs,
                    has_top_up: if config.lamports_per_write.is_some() {
                        1
                    } else {
                        0
                    },
                    compression_only: 0,
                    write_top_up: config.lamports_per_write.unwrap_or(0),
                    compress_to_account_pubkey: None,
                });

        let instruction_data = CreateAssociatedTokenAccountInstructionData {
            bump: self.bump,
            compressible_config: compressible_extension,
        };

        let discriminator = if self.idempotent {
            CREATE_ATA_IDEMPOTENT_DISCRIMINATOR
        } else {
            CREATE_ATA_DISCRIMINATOR
        };

        let mut data = Vec::new();
        data.push(discriminator);
        instruction_data
            .serialize(&mut data)
            .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

        let mut accounts = vec![
            AccountMeta::new_readonly(self.owner, false),
            AccountMeta::new_readonly(self.mint, false),
            AccountMeta::new(self.payer, true),
            AccountMeta::new(self.associated_token_account, false),
            AccountMeta::new_readonly(Pubkey::new_from_array([0; 32]), false), // system_program
        ];

        if let Some(config) = &self.compressible {
            accounts.push(AccountMeta::new_readonly(config.compressible_config, false));
            accounts.push(AccountMeta::new(config.rent_sponsor, false));
        }

        Ok(Instruction {
            program_id: Pubkey::from(light_ctoken_interface::COMPRESSED_TOKEN_PROGRAM_ID),
            accounts,
            data,
        })
    }
}

/// # Create an associated ctoken account via CPI:
/// ```rust,no_run
/// # use light_ctoken_sdk::ctoken::{CreateAssociatedTokenAccountCpi, CompressibleParamsCpi};
/// # use solana_account_info::AccountInfo;
/// # let owner: AccountInfo = todo!();
/// # let mint: AccountInfo = todo!();
/// # let payer: AccountInfo = todo!();
/// # let associated_token_account: AccountInfo = todo!();
/// # let system_program: AccountInfo = todo!();
/// # let bump: u8 = todo!();
/// # let compressible: CompressibleParamsCpi = todo!();
/// CreateAssociatedTokenAccountCpi {
///     owner,
///     mint,
///     payer,
///     associated_token_account,
///     system_program,
///     bump,
///     compressible: Some(compressible),
///     idempotent: true,
/// }
/// .invoke()?;
/// # Ok::<(), solana_program_error::ProgramError>(())
/// ```
pub struct CreateAssociatedTokenAccountCpi<'info> {
    pub owner: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub payer: AccountInfo<'info>,
    pub associated_token_account: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub bump: u8,
    pub compressible: Option<CompressibleParamsCpi<'info>>,
    pub idempotent: bool,
}

impl<'info> CreateAssociatedTokenAccountCpi<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        CreateAssociatedTokenAccount::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;
        if let Some(compressible) = self.compressible {
            let account_infos = [
                self.owner,
                self.mint,
                self.payer,
                self.associated_token_account,
                self.system_program,
                compressible.compressible_config,
                compressible.rent_sponsor,
            ];
            invoke(&instruction, &account_infos)
        } else {
            let account_infos = [
                self.owner,
                self.mint,
                self.payer,
                self.associated_token_account,
                self.system_program,
            ];
            invoke(&instruction, &account_infos)
        }
    }

    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;
        if let Some(compressible) = self.compressible {
            let account_infos = [
                self.owner,
                self.mint,
                self.payer,
                self.associated_token_account,
                self.system_program,
                compressible.compressible_config,
                compressible.rent_sponsor,
            ];
            invoke_signed(&instruction, &account_infos, signer_seeds)
        } else {
            let account_infos = [
                self.owner,
                self.mint,
                self.payer,
                self.associated_token_account,
                self.system_program,
            ];
            invoke_signed(&instruction, &account_infos, signer_seeds)
        }
    }
}

impl<'info> From<&CreateAssociatedTokenAccountCpi<'info>> for CreateAssociatedTokenAccount {
    fn from(account_infos: &CreateAssociatedTokenAccountCpi<'info>) -> Self {
        Self {
            payer: *account_infos.payer.key,
            owner: *account_infos.owner.key,
            mint: *account_infos.mint.key,
            associated_token_account: *account_infos.associated_token_account.key,
            bump: account_infos.bump,
            compressible: account_infos
                .compressible
                .as_ref()
                .map(|config| CompressibleParams {
                    compressible_config: *config.compressible_config.key,
                    rent_sponsor: *config.rent_sponsor.key,
                    pre_pay_num_epochs: config.pre_pay_num_epochs,
                    lamports_per_write: config.lamports_per_write,
                    compress_to_account_pubkey: None,
                    token_account_version: config.token_account_version,
                }),
            idempotent: account_infos.idempotent,
        }
    }
}
