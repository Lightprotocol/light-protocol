//! Create CToken account CPI builder for pinocchio.

use alloc::vec::Vec;

use borsh::BorshSerialize;
use light_token_interface::{
    instructions::{
        create_token_account::CreateTokenAccountInstructionData,
        extensions::{CompressToPubkey, CompressibleExtensionInstructionData},
    },
    LIGHT_TOKEN_PROGRAM_ID,
};
use pinocchio::{
    account_info::AccountInfo,
    cpi::slice_invoke_signed,
    instruction::{AccountMeta, Instruction, Signer},
    program_error::ProgramError,
};

use super::compressible::CompressibleParamsCpi;

/// Discriminator for `InitializeAccount3` (create token account).
const CREATE_TOKEN_ACCOUNT_DISCRIMINATOR: u8 = 18;

/// CPI builder for creating CToken accounts (vaults).
///
/// # Example - Rent-free vault with PDA signing
/// ```rust,ignore
/// CreateTokenAccountCpi {
///     payer: &ctx.accounts.payer,
///     account: &ctx.accounts.vault,
///     mint: &ctx.accounts.mint,
///     owner: ctx.accounts.vault_authority.key().clone(),
/// }
/// .rent_free(
///     &ctx.accounts.ctoken_config,
///     &ctx.accounts.rent_sponsor,
///     &ctx.accounts.system_program,
/// )
/// .invoke_signed(&[Signer::from(&[b"vault", mint_key.as_ref(), &[bump]])])?;
/// ```
pub struct CreateTokenAccountCpi<'info> {
    pub payer: &'info AccountInfo,
    pub account: &'info AccountInfo,
    pub mint: &'info AccountInfo,
    pub owner: [u8; 32],
}

impl<'info> CreateTokenAccountCpi<'info> {
    /// Enable rent-free mode with compressible config.
    ///
    /// Returns a builder that can call `.invoke()` or `.invoke_signed(signers)`.
    /// When using `invoke_signed`, the seeds are used for both PDA signing
    /// and deriving the compress_to address.
    pub fn rent_free(
        self,
        config: &'info AccountInfo,
        sponsor: &'info AccountInfo,
        system_program: &'info AccountInfo,
    ) -> CreateTokenAccountRentFreeCpi<'info> {
        CreateTokenAccountRentFreeCpi {
            base: self,
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
        let (data, metas, account_infos) = build_instruction_inner(&self, &compressible, None)?;
        invoke_cpi(&data, &metas, &account_infos, &[])
    }

    /// Invoke with signing, without rent-free (requires manually constructed compressible params).
    pub fn invoke_signed_with(
        self,
        compressible: CompressibleParamsCpi<'info>,
        signers: &[Signer],
    ) -> Result<(), ProgramError> {
        let (data, metas, account_infos) = build_instruction_inner(&self, &compressible, None)?;
        invoke_cpi(&data, &metas, &account_infos, signers)
    }
}

/// Rent-free enabled CToken account creation CPI.
pub struct CreateTokenAccountRentFreeCpi<'info> {
    base: CreateTokenAccountCpi<'info>,
    config: &'info AccountInfo,
    sponsor: &'info AccountInfo,
    system_program: &'info AccountInfo,
}

impl<'info> CreateTokenAccountRentFreeCpi<'info> {
    /// Invoke CPI for non-program-owned accounts.
    pub fn invoke(self) -> Result<(), ProgramError> {
        let compressible =
            CompressibleParamsCpi::new(self.config, self.sponsor, self.system_program);
        let (data, metas, account_infos) =
            build_instruction_inner(&self.base, &compressible, None)?;
        invoke_cpi(&data, &metas, &account_infos, &[])
    }

    /// Invoke CPI with PDA signing for program-owned accounts.
    ///
    /// For compress_to derivation, use `CreateTokenAccountCpi::invoke_signed_with()`
    /// with a `CompressibleParamsCpi` that has `compress_to_account_pubkey` set.
    pub fn invoke_signed(self, signers: &[Signer]) -> Result<(), ProgramError> {
        let compressible =
            CompressibleParamsCpi::new(self.config, self.sponsor, self.system_program);
        let (data, metas, account_infos) =
            build_instruction_inner(&self.base, &compressible, None)?;
        invoke_cpi(&data, &metas, &account_infos, signers)
    }
}

/// Build instruction data, account metas, and account infos for CreateTokenAccount.
#[allow(clippy::type_complexity)]
fn build_instruction_inner<'a>(
    base: &CreateTokenAccountCpi<'a>,
    compressible: &CompressibleParamsCpi<'a>,
    compress_to: Option<CompressToPubkey>,
) -> Result<(Vec<u8>, [AccountMeta<'a>; 6], [&'a AccountInfo; 6]), ProgramError> {
    let instruction_data = CreateTokenAccountInstructionData {
        owner: base.owner.into(),
        compressible_config: Some(CompressibleExtensionInstructionData {
            token_account_version: compressible.token_account_version as u8,
            rent_payment: compressible.pre_pay_num_epochs,
            compression_only: compressible.compression_only as u8,
            write_top_up: compressible.lamports_per_write.unwrap_or(0),
            compress_to_account_pubkey: compress_to
                .or_else(|| compressible.compress_to_account_pubkey.clone()),
        }),
    };

    let mut data = Vec::new();
    data.push(CREATE_TOKEN_ACCOUNT_DISCRIMINATOR);
    instruction_data
        .serialize(&mut data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    // Account order matches the cToken program:
    // [0] account (signer, writable)
    // [1] mint (readonly)
    // [2] payer (signer, writable)
    // [3] compressible_config (readonly)
    // [4] system_program (readonly)
    // [5] rent_sponsor (writable)
    let metas = [
        AccountMeta::writable_signer(base.account.key()),
        AccountMeta::readonly(base.mint.key()),
        AccountMeta::writable_signer(base.payer.key()),
        AccountMeta::readonly(compressible.compressible_config.key()),
        AccountMeta::readonly(compressible.system_program.key()),
        AccountMeta::writable(compressible.rent_sponsor.key()),
    ];

    let account_infos = [
        base.account,
        base.mint,
        base.payer,
        compressible.compressible_config,
        compressible.system_program,
        compressible.rent_sponsor,
    ];

    Ok((data, metas, account_infos))
}

/// Helper to invoke CPI to Light Token program.
fn invoke_cpi(
    data: &[u8],
    metas: &[AccountMeta],
    account_infos: &[&AccountInfo],
    signers: &[Signer],
) -> Result<(), ProgramError> {
    let instruction = Instruction {
        program_id: &LIGHT_TOKEN_PROGRAM_ID,
        accounts: metas,
        data,
    };
    slice_invoke_signed(&instruction, account_infos, signers)
}
