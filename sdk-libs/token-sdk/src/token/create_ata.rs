use borsh::BorshSerialize;
use light_token_interface::instructions::{
    create_associated_token_account::CreateAssociatedTokenAccountInstructionData,
    extensions::CompressibleExtensionInstructionData,
};
use solana_account_info::AccountInfo;
use solana_cpi::{invoke, invoke_signed};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::token::{compressible::CompressibleParamsCpi, CompressibleParams};

const CREATE_ATA_DISCRIMINATOR: u8 = 100;
const CREATE_ATA_IDEMPOTENT_DISCRIMINATOR: u8 = 102;

pub fn derive_token_ata(owner: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            owner.as_ref(),
            light_token_interface::LIGHT_TOKEN_PROGRAM_ID.as_ref(),
            mint.as_ref(),
        ],
        &Pubkey::from(light_token_interface::LIGHT_TOKEN_PROGRAM_ID),
    )
}

/// # Create an associated ctoken account instruction:
/// ```rust
/// # use solana_pubkey::Pubkey;
/// # use light_token_sdk::token::CreateAssociatedTokenAccount;
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
    pub compressible: CompressibleParams,
    pub idempotent: bool,
}

impl CreateAssociatedTokenAccount {
    pub fn new(payer: Pubkey, owner: Pubkey, mint: Pubkey) -> Self {
        let (ata, bump) = derive_token_ata(&owner, &mint);
        Self {
            payer,
            owner,
            mint,
            associated_token_account: ata,
            bump,
            compressible: CompressibleParams::default_ata(),
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
            compressible: CompressibleParams::default_ata(),
            idempotent: false,
        }
    }

    pub fn with_compressible(mut self, compressible_params: CompressibleParams) -> Self {
        self.compressible = compressible_params;
        self
    }

    pub fn idempotent(mut self) -> Self {
        self.idempotent = true;
        self
    }

    pub fn instruction(self) -> Result<Instruction, ProgramError> {
        let instruction_data = CreateAssociatedTokenAccountInstructionData {
            bump: self.bump,
            compressible_config: Some(CompressibleExtensionInstructionData {
                token_account_version: self.compressible.token_account_version as u8,
                rent_payment: self.compressible.pre_pay_num_epochs,
                compression_only: self.compressible.compression_only as u8,
                write_top_up: self.compressible.lamports_per_write.unwrap_or(0),
                compress_to_account_pubkey: self.compressible.compress_to_account_pubkey.clone(),
            }),
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

        let accounts = vec![
            AccountMeta::new_readonly(self.owner, false),
            AccountMeta::new_readonly(self.mint, false),
            AccountMeta::new(self.payer, true),
            AccountMeta::new(self.associated_token_account, false),
            AccountMeta::new_readonly(Pubkey::new_from_array([0; 32]), false), // system_program
            AccountMeta::new_readonly(self.compressible.compressible_config, false),
            AccountMeta::new(self.compressible.rent_sponsor, false),
        ];

        Ok(Instruction {
            program_id: Pubkey::from(light_token_interface::LIGHT_TOKEN_PROGRAM_ID),
            accounts,
            data,
        })
    }
}

/// CPI builder for creating CToken ATAs.
///
/// # Example - Rent-free ATA (idempotent)
/// ```rust,ignore
/// CreateTokenAtaCpi {
///     payer: ctx.accounts.payer.to_account_info(),
///     owner: ctx.accounts.owner.to_account_info(),
///     mint: ctx.accounts.mint.to_account_info(),
///     ata: ctx.accounts.user_ata.to_account_info(),
///     bump: params.user_ata_bump,
/// }
/// .idempotent()
/// .rent_free(
///     ctx.accounts.ctoken_config.to_account_info(),
///     ctx.accounts.rent_sponsor.to_account_info(),
///     ctx.accounts.system_program.to_account_info(),
/// )
/// .invoke()?;
/// ```
pub struct CreateTokenAtaCpi<'info> {
    pub payer: AccountInfo<'info>,
    pub owner: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
    pub ata: AccountInfo<'info>,
    pub bump: u8,
}

impl<'info> CreateTokenAtaCpi<'info> {
    /// Make this an idempotent create (won't fail if ATA already exists).
    pub fn idempotent(self) -> CreateTokenAtaCpiIdempotent<'info> {
        CreateTokenAtaCpiIdempotent { base: self }
    }

    /// Enable rent-free mode with compressible config.
    pub fn rent_free(
        self,
        config: AccountInfo<'info>,
        sponsor: AccountInfo<'info>,
        system_program: AccountInfo<'info>,
    ) -> CreateTokenAtaRentFreeCpi<'info> {
        CreateTokenAtaRentFreeCpi {
            payer: self.payer,
            owner: self.owner,
            mint: self.mint,
            ata: self.ata,
            bump: self.bump,
            idempotent: false,
            config,
            sponsor,
            system_program,
        }
    }

    /// Invoke without rent-free (requires manually constructed compressible params).
    pub fn invoke_with(
        self,
        compressible: CompressibleParamsCpi<'info>,
        system_program: AccountInfo<'info>,
    ) -> Result<(), ProgramError> {
        InternalCreateAtaCpi {
            owner: self.owner,
            mint: self.mint,
            payer: self.payer,
            associated_token_account: self.ata,
            system_program,
            bump: self.bump,
            compressible,
            idempotent: false,
        }
        .invoke()
    }
}

/// Idempotent ATA creation (intermediate type).
pub struct CreateTokenAtaCpiIdempotent<'info> {
    base: CreateTokenAtaCpi<'info>,
}

impl<'info> CreateTokenAtaCpiIdempotent<'info> {
    /// Enable rent-free mode with compressible config.
    pub fn rent_free(
        self,
        config: AccountInfo<'info>,
        sponsor: AccountInfo<'info>,
        system_program: AccountInfo<'info>,
    ) -> CreateTokenAtaRentFreeCpi<'info> {
        CreateTokenAtaRentFreeCpi {
            payer: self.base.payer,
            owner: self.base.owner,
            mint: self.base.mint,
            ata: self.base.ata,
            bump: self.base.bump,
            idempotent: true,
            config,
            sponsor,
            system_program,
        }
    }

    /// Invoke without rent-free (requires manually constructed compressible params).
    pub fn invoke_with(
        self,
        compressible: CompressibleParamsCpi<'info>,
        system_program: AccountInfo<'info>,
    ) -> Result<(), ProgramError> {
        InternalCreateAtaCpi {
            owner: self.base.owner,
            mint: self.base.mint,
            payer: self.base.payer,
            associated_token_account: self.base.ata,
            system_program,
            bump: self.base.bump,
            compressible,
            idempotent: true,
        }
        .invoke()
    }
}

/// Rent-free enabled CToken ATA creation CPI.
pub struct CreateTokenAtaRentFreeCpi<'info> {
    payer: AccountInfo<'info>,
    owner: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    ata: AccountInfo<'info>,
    bump: u8,
    idempotent: bool,
    config: AccountInfo<'info>,
    sponsor: AccountInfo<'info>,
    system_program: AccountInfo<'info>,
}

impl<'info> CreateTokenAtaRentFreeCpi<'info> {
    /// Invoke CPI.
    pub fn invoke(self) -> Result<(), ProgramError> {
        InternalCreateAtaCpi {
            owner: self.owner,
            mint: self.mint,
            payer: self.payer,
            associated_token_account: self.ata,
            system_program: self.system_program.clone(),
            bump: self.bump,
            compressible: CompressibleParamsCpi::new_ata(
                self.config,
                self.sponsor,
                self.system_program,
            ),
            idempotent: self.idempotent,
        }
        .invoke()
    }

    /// Invoke CPI with signer seeds (when caller needs to sign for another account).
    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        InternalCreateAtaCpi {
            owner: self.owner,
            mint: self.mint,
            payer: self.payer,
            associated_token_account: self.ata,
            system_program: self.system_program.clone(),
            bump: self.bump,
            compressible: CompressibleParamsCpi::new_ata(
                self.config,
                self.sponsor,
                self.system_program,
            ),
            idempotent: self.idempotent,
        }
        .invoke_signed(signer_seeds)
    }
}

/// Internal CPI struct for ATAs with full params.
struct InternalCreateAtaCpi<'info> {
    owner: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    payer: AccountInfo<'info>,
    associated_token_account: AccountInfo<'info>,
    system_program: AccountInfo<'info>,
    bump: u8,
    compressible: CompressibleParamsCpi<'info>,
    idempotent: bool,
}

impl<'info> InternalCreateAtaCpi<'info> {
    fn instruction(&self) -> Result<Instruction, ProgramError> {
        CreateAssociatedTokenAccount {
            payer: *self.payer.key,
            owner: *self.owner.key,
            mint: *self.mint.key,
            associated_token_account: *self.associated_token_account.key,
            bump: self.bump,
            compressible: CompressibleParams {
                compressible_config: *self.compressible.compressible_config.key,
                rent_sponsor: *self.compressible.rent_sponsor.key,
                pre_pay_num_epochs: self.compressible.pre_pay_num_epochs,
                lamports_per_write: self.compressible.lamports_per_write,
                compress_to_account_pubkey: self.compressible.compress_to_account_pubkey.clone(),
                token_account_version: self.compressible.token_account_version,
                compression_only: self.compressible.compression_only,
            },
            idempotent: self.idempotent,
        }
        .instruction()
    }

    fn invoke(self) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;
        let account_infos = [
            self.owner,
            self.mint,
            self.payer,
            self.associated_token_account,
            self.system_program,
            self.compressible.compressible_config,
            self.compressible.rent_sponsor,
        ];
        invoke(&instruction, &account_infos)
    }

    fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), ProgramError> {
        let instruction = self.instruction()?;
        let account_infos = [
            self.owner,
            self.mint,
            self.payer,
            self.associated_token_account,
            self.system_program,
            self.compressible.compressible_config,
            self.compressible.rent_sponsor,
        ];
        invoke_signed(&instruction, &account_infos, signer_seeds)
    }
}
