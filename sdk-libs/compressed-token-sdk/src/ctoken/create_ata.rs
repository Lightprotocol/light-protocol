use borsh::BorshSerialize;
use light_ctoken_types::instructions::{
    create_associated_token_account::CreateAssociatedTokenAccountInstructionData,
    create_associated_token_account2::CreateAssociatedTokenAccount2InstructionData,
    extensions::compressible::CompressibleExtensionInstructionData,
};
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::ctoken::{compressible::CompressibleParamsInfos, CompressibleParams};

const CREATE_ATA_DISCRIMINATOR: u8 = 100;
const CREATE_ATA_IDEMPOTENT_DISCRIMINATOR: u8 = 102;
const CREATE_ATA2_DISCRIMINATOR: u8 = 106;
const CREATE_ATA2_IDEMPOTENT_DISCRIMINATOR: u8 = 107;

pub fn derive_ctoken_ata(owner: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            owner.as_ref(),
            light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID.as_ref(),
            mint.as_ref(),
        ],
        &Pubkey::from(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
    )
}

// ============================================================================
// V1: Associated Token Account (owner/mint in instruction data)
// ============================================================================

#[derive(Debug, Clone)]
pub struct CreateAssociatedTokenAccount {
    pub idempotent: bool,
    pub bump: u8,
    pub payer: Pubkey,
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub associated_token_account: Pubkey,
    pub compressible: Option<CompressibleParams>,
}

impl CreateAssociatedTokenAccount {
    pub fn new(
        payer: Pubkey,
        owner: Pubkey,
        mint: Pubkey,
        compressible_params: CompressibleParams,
    ) -> Self {
        let (ata, bump) = derive_ctoken_ata(&owner, &mint);
        Self {
            payer,
            owner,
            mint,
            associated_token_account: ata,
            bump,
            compressible: Some(compressible_params),
            idempotent: false,
        }
    }

    pub fn new_with_bump(
        payer: Pubkey,
        owner: Pubkey,
        mint: Pubkey,
        compressible_params: CompressibleParams,
        associated_token_account: Pubkey,
        bump: u8,
    ) -> Self {
        Self {
            payer,
            owner,
            mint,
            associated_token_account,
            bump,
            compressible: Some(compressible_params),
            idempotent: false,
        }
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
                    write_top_up: config.lamports_per_write.unwrap_or(0),
                    compress_to_account_pubkey: None,
                });

        let instruction_data = CreateAssociatedTokenAccountInstructionData {
            owner: light_compressed_account::Pubkey::from(self.owner.to_bytes()),
            mint: light_compressed_account::Pubkey::from(self.mint.to_bytes()),
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
            AccountMeta::new(self.payer, true),
            AccountMeta::new(self.associated_token_account, false),
            AccountMeta::new_readonly(Pubkey::default(), false), // system_program
        ];

        if let Some(config) = &self.compressible {
            accounts.push(AccountMeta::new_readonly(config.compressible_config, false));
            accounts.push(AccountMeta::new(config.rent_sponsor, false));
        }

        Ok(Instruction {
            program_id: Pubkey::from(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
            accounts,
            data,
        })
    }
}

pub struct CreateAssociatedTokenAccountInfos<'info> {
    pub bump: u8,
    pub idempotent: bool,
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub payer: AccountInfo<'info>,
    pub associated_token_account: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub compressible: Option<CompressibleParamsInfos<'info>>,
}

impl<'info> CreateAssociatedTokenAccountInfos<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        CreateAssociatedTokenAccount::from(self).instruction()
    }

    pub fn invoke(self) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;
        if let Some(compressible) = self.compressible {
            let account_infos = [
                self.payer,
                self.associated_token_account,
                self.system_program,
                compressible.compressible_config,
                compressible.rent_sponsor,
            ];
            invoke(&instruction, &account_infos)
        } else {
            let account_infos = [
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
                self.payer,
                self.associated_token_account,
                self.system_program,
                compressible.compressible_config,
                compressible.rent_sponsor,
            ];
            invoke_signed(&instruction, &account_infos, signer_seeds)
        } else {
            let account_infos = [
                self.payer,
                self.associated_token_account,
                self.system_program,
            ];
            invoke_signed(&instruction, &account_infos, signer_seeds)
        }
    }
}

impl<'info> From<&CreateAssociatedTokenAccountInfos<'info>> for CreateAssociatedTokenAccount {
    fn from(account_infos: &CreateAssociatedTokenAccountInfos<'info>) -> Self {
        Self {
            payer: *account_infos.payer.key,
            owner: account_infos.owner,
            mint: account_infos.mint,
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

// ============================================================================
// V2: Associated Token Account (owner/mint as accounts)
// ============================================================================

#[derive(Debug, Clone)]
pub struct CreateAssociatedTokenAccount2 {
    pub payer: Pubkey,
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub associated_token_account: Pubkey,
    pub bump: u8,
    pub compressible: Option<CompressibleParams>,
    pub idempotent: bool,
}

impl CreateAssociatedTokenAccount2 {
    pub fn new(
        payer: Pubkey,
        owner: Pubkey,
        mint: Pubkey,
        compressible_params: CompressibleParams,
    ) -> Self {
        let (ata, bump) = derive_ctoken_ata(&owner, &mint);
        Self {
            payer,
            owner,
            mint,
            associated_token_account: ata,
            bump,
            compressible: Some(compressible_params),
            idempotent: false,
        }
    }

    pub fn new_with_bump(
        payer: Pubkey,
        owner: Pubkey,
        mint: Pubkey,
        compressible_params: CompressibleParams,
        associated_token_account: Pubkey,
        bump: u8,
    ) -> Self {
        Self {
            payer,
            owner,
            mint,
            associated_token_account,
            bump,
            compressible: Some(compressible_params),
            idempotent: false,
        }
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
                    write_top_up: config.lamports_per_write.unwrap_or(0),
                    compress_to_account_pubkey: None,
                });

        let instruction_data = CreateAssociatedTokenAccount2InstructionData {
            bump: self.bump,
            compressible_config: compressible_extension,
        };

        let discriminator = if self.idempotent {
            CREATE_ATA2_IDEMPOTENT_DISCRIMINATOR
        } else {
            CREATE_ATA2_DISCRIMINATOR
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
            program_id: Pubkey::from(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
            accounts,
            data,
        })
    }
}

pub struct CreateAssociatedTokenAccount2Infos<'info> {
    pub owner: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub payer: AccountInfo<'info>,
    pub associated_token_account: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
    pub bump: u8,
    pub compressible: Option<CompressibleParamsInfos<'info>>,
    pub idempotent: bool,
}

impl<'info> CreateAssociatedTokenAccount2Infos<'info> {
    pub fn instruction(&self) -> Result<Instruction, ProgramError> {
        CreateAssociatedTokenAccount2::from(self).instruction()
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

impl<'info> From<&CreateAssociatedTokenAccount2Infos<'info>> for CreateAssociatedTokenAccount2 {
    fn from(account_infos: &CreateAssociatedTokenAccount2Infos<'info>) -> Self {
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
