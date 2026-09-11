//! Create light-token associated token account CPI builder for pinocchio.

use alloc::vec::Vec;

use borsh::BorshSerialize;
use light_account_checks::AccountInfoTrait;
use light_token_interface::{
    instructions::{
        create_associated_token_account::CreateAssociatedTokenAccountInstructionData,
        extensions::CompressibleExtensionInstructionData,
    },
    LIGHT_TOKEN_PROGRAM_ID,
};
use pinocchio::{
    address::Address,
    cpi::{invoke_signed_with_slice, Signer},
    error::ProgramError,
    instruction::{InstructionAccount, InstructionView},
    AccountView as AccountInfo,
};

use super::compressible::CompressibleParamsCpi;

/// Discriminator for `CreateAssociatedTokenAccount`.
const CREATE_ATA_DISCRIMINATOR: u8 = 100;
/// Discriminator for `CreateAssociatedTokenAccountIdempotent`.
const CREATE_ATA_IDEMPOTENT_DISCRIMINATOR: u8 = 102;

/// Derive the associated token account address for a given owner and mint.
///
/// Returns `[u8; 32]` -- the ATA address.
pub fn derive_associated_token_account(owner: &[u8; 32], mint: &[u8; 32]) -> [u8; 32] {
    AccountInfo::find_program_address(
        &[
            owner.as_ref(),
            LIGHT_TOKEN_PROGRAM_ID.as_ref(),
            mint.as_ref(),
        ],
        &LIGHT_TOKEN_PROGRAM_ID,
    )
    .0
}

/// CPI builder for creating CToken ATAs.
///
/// # Example - Rent-free ATA (idempotent)
/// ```rust,ignore
/// CreateTokenAtaCpi {
///     payer: &ctx.accounts.payer,
///     owner: &ctx.accounts.owner,
///     mint: &ctx.accounts.mint,
///     ata: &ctx.accounts.user_ata,
/// }
/// .idempotent()
/// .rent_free(
///     &ctx.accounts.ctoken_config,
///     &ctx.accounts.rent_sponsor,
///     &ctx.accounts.system_program,
/// )
/// .invoke()?;
/// ```
pub struct CreateTokenAtaCpi<'info> {
    pub payer: &'info AccountInfo,
    pub owner: &'info AccountInfo,
    pub mint: &'info AccountInfo,
    pub ata: &'info AccountInfo,
}

impl<'info> CreateTokenAtaCpi<'info> {
    /// Make this an idempotent create (won't fail if ATA already exists).
    pub fn idempotent(self) -> CreateTokenAtaCpiIdempotent<'info> {
        CreateTokenAtaCpiIdempotent { base: self }
    }

    /// Enable rent-free mode with compressible config.
    pub fn rent_free(
        self,
        config: &'info AccountInfo,
        sponsor: &'info AccountInfo,
        system_program: &'info AccountInfo,
    ) -> CreateTokenAtaRentFreeCpi<'info> {
        CreateTokenAtaRentFreeCpi {
            payer: self.payer,
            owner: self.owner,
            mint: self.mint,
            ata: self.ata,
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
    ) -> Result<(), ProgramError> {
        let (data, metas, account_infos) = build_create_ata_instruction_inner(
            self.owner,
            self.mint,
            self.payer,
            self.ata,
            &compressible,
            false,
        )?;
        invoke_cpi(&data, &metas, &account_infos, &[])
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
        config: &'info AccountInfo,
        sponsor: &'info AccountInfo,
        system_program: &'info AccountInfo,
    ) -> CreateTokenAtaRentFreeCpi<'info> {
        CreateTokenAtaRentFreeCpi {
            payer: self.base.payer,
            owner: self.base.owner,
            mint: self.base.mint,
            ata: self.base.ata,
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
    ) -> Result<(), ProgramError> {
        let (data, metas, account_infos) = build_create_ata_instruction_inner(
            self.base.owner,
            self.base.mint,
            self.base.payer,
            self.base.ata,
            &compressible,
            true,
        )?;
        invoke_cpi(&data, &metas, &account_infos, &[])
    }
}

/// Rent-free enabled CToken ATA creation CPI.
pub struct CreateTokenAtaRentFreeCpi<'info> {
    payer: &'info AccountInfo,
    owner: &'info AccountInfo,
    mint: &'info AccountInfo,
    ata: &'info AccountInfo,
    idempotent: bool,
    config: &'info AccountInfo,
    sponsor: &'info AccountInfo,
    system_program: &'info AccountInfo,
}

impl<'info> CreateTokenAtaRentFreeCpi<'info> {
    /// Invoke CPI.
    pub fn invoke(self) -> Result<(), ProgramError> {
        let compressible =
            CompressibleParamsCpi::new_ata(self.config, self.sponsor, self.system_program);
        let (data, metas, account_infos) = build_create_ata_instruction_inner(
            self.owner,
            self.mint,
            self.payer,
            self.ata,
            &compressible,
            self.idempotent,
        )?;
        invoke_cpi(&data, &metas, &account_infos, &[])
    }

    /// Invoke CPI with signer seeds.
    pub fn invoke_signed(self, signers: &[Signer]) -> Result<(), ProgramError> {
        let compressible =
            CompressibleParamsCpi::new_ata(self.config, self.sponsor, self.system_program);
        let (data, metas, account_infos) = build_create_ata_instruction_inner(
            self.owner,
            self.mint,
            self.payer,
            self.ata,
            &compressible,
            self.idempotent,
        )?;
        invoke_cpi(&data, &metas, &account_infos, signers)
    }
}

/// Build instruction data, account metas, and account infos for CreateAssociatedTokenAccount.
#[allow(clippy::type_complexity)]
fn build_create_ata_instruction_inner<'a>(
    owner: &'a AccountInfo,
    mint: &'a AccountInfo,
    payer: &'a AccountInfo,
    ata: &'a AccountInfo,
    compressible: &CompressibleParamsCpi<'a>,
    idempotent: bool,
) -> Result<(Vec<u8>, [InstructionAccount<'a>; 7], [&'a AccountInfo; 7]), ProgramError> {
    let instruction_data = CreateAssociatedTokenAccountInstructionData {
        compressible_config: Some(CompressibleExtensionInstructionData {
            token_account_version: compressible.token_account_version as u8,
            rent_payment: compressible.pre_pay_num_epochs,
            compression_only: compressible.compression_only as u8,
            write_top_up: compressible.lamports_per_write.unwrap_or(0),
            compress_to_account_pubkey: compressible.compress_to_account_pubkey.clone(),
        }),
    };

    let discriminator = if idempotent {
        CREATE_ATA_IDEMPOTENT_DISCRIMINATOR
    } else {
        CREATE_ATA_DISCRIMINATOR
    };

    let mut data = Vec::new();
    data.push(discriminator);
    instruction_data
        .serialize(&mut data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    // Account order matches the cToken program:
    // [0] owner (readonly)
    // [1] mint (readonly)
    // [2] payer (signer, writable)
    // [3] associated_token_account (writable)
    // [4] system_program (readonly)
    // [5] compressible_config (readonly)
    // [6] rent_sponsor (writable)
    let metas = [
        InstructionAccount::readonly(owner.address()),
        InstructionAccount::readonly(mint.address()),
        InstructionAccount::writable_signer(payer.address()),
        InstructionAccount::writable(ata.address()),
        InstructionAccount::readonly(compressible.system_program.address()),
        InstructionAccount::readonly(compressible.compressible_config.address()),
        InstructionAccount::writable(compressible.rent_sponsor.address()),
    ];

    let account_infos = [
        owner,
        mint,
        payer,
        ata,
        compressible.system_program,
        compressible.compressible_config,
        compressible.rent_sponsor,
    ];

    Ok((data, metas, account_infos))
}

/// Helper to invoke CPI to Light Token program.
fn invoke_cpi(
    data: &[u8],
    metas: &[InstructionAccount],
    account_infos: &[&AccountInfo],
    signers: &[Signer],
) -> Result<(), ProgramError> {
    let program_id = Address::from(LIGHT_TOKEN_PROGRAM_ID);
    let instruction = InstructionView {
        program_id: &program_id,
        accounts: metas,
        data,
    };
    invoke_signed_with_slice(&instruction, account_infos, signers)
}
