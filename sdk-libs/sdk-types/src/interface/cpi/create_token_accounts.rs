//! Generic CPI builders for creating CToken accounts and ATAs.
//!
//! Provides `CreateTokenAccountCpi` and `CreateTokenAtaCpi`, both generic over
//! `AccountInfoTrait` so they work with both `solana_account_info::AccountInfo`
//! and `pinocchio::account_info::AccountInfo`.

use alloc::{vec, vec::Vec};

use borsh::BorshSerialize;
use light_account_checks::{AccountInfoTrait, CpiMeta};
use light_token_interface::{
    instructions::{
        create_associated_token_account::CreateAssociatedTokenAccountInstructionData,
        create_token_account::CreateTokenAccountInstructionData,
        extensions::{CompressToPubkey, CompressibleExtensionInstructionData},
    },
    LIGHT_TOKEN_PROGRAM_ID,
};

use crate::error::LightSdkTypesError;

/// Discriminator for `InitializeAccount3` (create token account).
const CREATE_TOKEN_ACCOUNT_DISCRIMINATOR: u8 = 18;
/// Discriminator for `CreateAssociatedTokenAccount`.
const CREATE_ATA_DISCRIMINATOR: u8 = 100;
/// Discriminator for `CreateAssociatedTokenAccountIdempotent`.
const CREATE_ATA_IDEMPOTENT_DISCRIMINATOR: u8 = 102;

/// Default rent payment epochs (~24 hours).
const DEFAULT_PRE_PAY_NUM_EPOCHS: u8 = 16;
/// Default lamports for write operations (~3 hours per write).
const DEFAULT_LAMPORTS_PER_WRITE: u32 = 766;
/// Default token account version (ShaFlat = 3).
const DEFAULT_TOKEN_ACCOUNT_VERSION: u8 = 3;

// ============================================================================
// derive_associated_token_account
// ============================================================================

/// Derive the associated token account address for a given owner and mint.
///
/// Returns `([u8; 32], u8)` -- the ATA address and bump seed.
pub fn derive_associated_token_account<AI: AccountInfoTrait>(
    owner: &[u8; 32],
    mint: &[u8; 32],
) -> ([u8; 32], u8) {
    AI::find_program_address(
        &[
            owner.as_ref(),
            LIGHT_TOKEN_PROGRAM_ID.as_ref(),
            mint.as_ref(),
        ],
        &LIGHT_TOKEN_PROGRAM_ID,
    )
}

// ============================================================================
// CreateTokenAccountCpi
// ============================================================================

/// CPI builder for creating CToken accounts (vaults).
///
/// Generic over `AccountInfoTrait` for framework independence.
///
/// # Example
/// ```rust,ignore
/// CreateTokenAccountCpi {
///     payer: &ctx.accounts.payer,
///     account: &ctx.accounts.vault,
///     mint: &ctx.accounts.mint,
///     owner: ctx.accounts.vault_authority.key(),
/// }
/// .rent_free(
///     &ctx.accounts.ctoken_config,
///     &ctx.accounts.rent_sponsor,
///     &ctx.accounts.system_program,
///     &crate::ID.to_bytes(),
/// )
/// .invoke_signed(vault_seeds)?;
/// ```
pub struct CreateTokenAccountCpi<'a, AI: AccountInfoTrait + Clone> {
    pub payer: &'a AI,
    pub account: &'a AI,
    pub mint: &'a AI,
    pub owner: [u8; 32],
}

impl<'a, AI: AccountInfoTrait + Clone> CreateTokenAccountCpi<'a, AI> {
    /// Enable rent-free mode with compressible config.
    ///
    /// Returns a builder that can call `.invoke()` or `.invoke_signed(seeds)`.
    /// When using `invoke_signed`, the seeds are used for both PDA signing
    /// and deriving the compress_to address.
    pub fn rent_free(
        self,
        config: &'a AI,
        sponsor: &'a AI,
        system_program: &'a AI,
        program_id: &[u8; 32],
    ) -> CreateTokenAccountRentFreeCpi<'a, AI> {
        CreateTokenAccountRentFreeCpi {
            base: self,
            config,
            sponsor,
            system_program,
            program_id: *program_id,
        }
    }
}

/// Rent-free enabled CToken account creation CPI.
pub struct CreateTokenAccountRentFreeCpi<'a, AI: AccountInfoTrait + Clone> {
    base: CreateTokenAccountCpi<'a, AI>,
    config: &'a AI,
    sponsor: &'a AI,
    system_program: &'a AI,
    program_id: [u8; 32],
}

impl<'a, AI: AccountInfoTrait + Clone> CreateTokenAccountRentFreeCpi<'a, AI> {
    /// Invoke CPI for non-program-owned accounts.
    pub fn invoke(self) -> Result<(), LightSdkTypesError> {
        let (data, metas, account_infos) = self.build_instruction_inner(None)?;
        AI::invoke_cpi(&LIGHT_TOKEN_PROGRAM_ID, &data, &metas, &account_infos, &[])
            .map_err(|_| LightSdkTypesError::CpiFailed)
    }

    /// Invoke CPI with PDA signing for program-owned accounts.
    ///
    /// Seeds are used for both signing AND deriving the compress_to address.
    pub fn invoke_signed(self, seeds: &[&[u8]]) -> Result<(), LightSdkTypesError> {
        // Build CompressToPubkey from signer seeds
        let bump = seeds.last().and_then(|s| s.first()).copied().unwrap_or(0);

        let seed_vecs: Vec<Vec<u8>> = seeds
            .iter()
            .take(seeds.len().saturating_sub(1))
            .map(|s| s.to_vec())
            .collect();

        let compress_to = CompressToPubkey {
            bump,
            program_id: self.program_id,
            seeds: seed_vecs,
        };

        let (data, metas, account_infos) = self.build_instruction_inner(Some(compress_to))?;
        AI::invoke_cpi(
            &LIGHT_TOKEN_PROGRAM_ID,
            &data,
            &metas,
            &account_infos,
            &[seeds],
        )
        .map_err(|_| LightSdkTypesError::CpiFailed)
    }

    /// Build instruction data, account metas, and account infos.
    #[allow(clippy::type_complexity)]
    fn build_instruction_inner(
        &self,
        compress_to: Option<CompressToPubkey>,
    ) -> Result<(Vec<u8>, Vec<CpiMeta>, Vec<AI>), LightSdkTypesError> {
        let instruction_data = CreateTokenAccountInstructionData {
            owner: self.base.owner.into(),
            compressible_config: Some(CompressibleExtensionInstructionData {
                token_account_version: DEFAULT_TOKEN_ACCOUNT_VERSION,
                rent_payment: DEFAULT_PRE_PAY_NUM_EPOCHS,
                compression_only: 0, // false
                write_top_up: DEFAULT_LAMPORTS_PER_WRITE,
                compress_to_account_pubkey: compress_to,
            }),
        };

        let mut data = Vec::new();
        data.push(CREATE_TOKEN_ACCOUNT_DISCRIMINATOR);
        instruction_data
            .serialize(&mut data)
            .map_err(|_| LightSdkTypesError::Borsh)?;

        // Account order matches the cToken program:
        // [0] account (signer, writable)
        // [1] mint (readonly)
        // [2] payer (signer, writable)
        // [3] compressible_config (readonly)
        // [4] system_program (readonly)
        // [5] rent_sponsor (writable)
        let metas = vec![
            CpiMeta {
                pubkey: self.base.account.key(),
                is_signer: true,
                is_writable: true,
            },
            CpiMeta {
                pubkey: self.base.mint.key(),
                is_signer: false,
                is_writable: false,
            },
            CpiMeta {
                pubkey: self.base.payer.key(),
                is_signer: true,
                is_writable: true,
            },
            CpiMeta {
                pubkey: self.config.key(),
                is_signer: false,
                is_writable: false,
            },
            CpiMeta {
                pubkey: self.system_program.key(),
                is_signer: false,
                is_writable: false,
            },
            CpiMeta {
                pubkey: self.sponsor.key(),
                is_signer: false,
                is_writable: true,
            },
        ];

        let account_infos = vec![
            self.base.account.clone(),
            self.base.mint.clone(),
            self.base.payer.clone(),
            self.config.clone(),
            self.system_program.clone(),
            self.sponsor.clone(),
        ];

        Ok((data, metas, account_infos))
    }
}

// ============================================================================
// CreateTokenAtaCpi
// ============================================================================

/// CPI builder for creating CToken ATAs.
///
/// Generic over `AccountInfoTrait` for framework independence.
///
/// # Example - Rent-free ATA (idempotent)
/// ```rust,ignore
/// CreateTokenAtaCpi {
///     payer: &ctx.accounts.payer,
///     owner: &ctx.accounts.owner,
///     mint: &ctx.accounts.mint,
///     ata: &ctx.accounts.user_ata,
///     bump: params.user_ata_bump,
/// }
/// .idempotent()
/// .rent_free(
///     &ctx.accounts.ctoken_config,
///     &ctx.accounts.rent_sponsor,
///     &ctx.accounts.system_program,
/// )
/// .invoke()?;
/// ```
pub struct CreateTokenAtaCpi<'a, AI: AccountInfoTrait + Clone> {
    pub payer: &'a AI,
    pub owner: &'a AI,
    pub mint: &'a AI,
    pub ata: &'a AI,
    pub bump: u8,
}

impl<'a, AI: AccountInfoTrait + Clone> CreateTokenAtaCpi<'a, AI> {
    /// Make this an idempotent create (won't fail if ATA already exists).
    pub fn idempotent(self) -> CreateTokenAtaCpiIdempotent<'a, AI> {
        CreateTokenAtaCpiIdempotent { base: self }
    }

    /// Enable rent-free mode with compressible config.
    pub fn rent_free(
        self,
        config: &'a AI,
        sponsor: &'a AI,
        system_program: &'a AI,
    ) -> CreateTokenAtaRentFreeCpi<'a, AI> {
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
}

/// Idempotent ATA creation (intermediate type).
pub struct CreateTokenAtaCpiIdempotent<'a, AI: AccountInfoTrait + Clone> {
    base: CreateTokenAtaCpi<'a, AI>,
}

impl<'a, AI: AccountInfoTrait + Clone> CreateTokenAtaCpiIdempotent<'a, AI> {
    /// Enable rent-free mode with compressible config.
    pub fn rent_free(
        self,
        config: &'a AI,
        sponsor: &'a AI,
        system_program: &'a AI,
    ) -> CreateTokenAtaRentFreeCpi<'a, AI> {
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
}

/// Rent-free enabled CToken ATA creation CPI.
pub struct CreateTokenAtaRentFreeCpi<'a, AI: AccountInfoTrait + Clone> {
    payer: &'a AI,
    owner: &'a AI,
    mint: &'a AI,
    ata: &'a AI,
    bump: u8,
    idempotent: bool,
    config: &'a AI,
    sponsor: &'a AI,
    system_program: &'a AI,
}

impl<'a, AI: AccountInfoTrait + Clone> CreateTokenAtaRentFreeCpi<'a, AI> {
    /// Invoke CPI.
    pub fn invoke(self) -> Result<(), LightSdkTypesError> {
        let (data, metas, account_infos) = self.build_instruction_inner()?;
        AI::invoke_cpi(&LIGHT_TOKEN_PROGRAM_ID, &data, &metas, &account_infos, &[])
            .map_err(|_| LightSdkTypesError::CpiFailed)
    }

    /// Invoke CPI with signer seeds (when caller needs to sign for another account).
    pub fn invoke_signed(self, signer_seeds: &[&[&[u8]]]) -> Result<(), LightSdkTypesError> {
        let (data, metas, account_infos) = self.build_instruction_inner()?;
        AI::invoke_cpi(
            &LIGHT_TOKEN_PROGRAM_ID,
            &data,
            &metas,
            &account_infos,
            signer_seeds,
        )
        .map_err(|_| LightSdkTypesError::CpiFailed)
    }

    /// Build instruction data, account metas, and account infos.
    #[allow(clippy::type_complexity)]
    fn build_instruction_inner(
        &self,
    ) -> Result<(Vec<u8>, Vec<CpiMeta>, Vec<AI>), LightSdkTypesError> {
        let instruction_data = CreateAssociatedTokenAccountInstructionData {
            bump: self.bump,
            compressible_config: Some(CompressibleExtensionInstructionData {
                token_account_version: DEFAULT_TOKEN_ACCOUNT_VERSION,
                rent_payment: DEFAULT_PRE_PAY_NUM_EPOCHS,
                compression_only: 1, // ATAs are always compression_only
                write_top_up: DEFAULT_LAMPORTS_PER_WRITE,
                compress_to_account_pubkey: None,
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
            .map_err(|_| LightSdkTypesError::Borsh)?;

        // Account order matches the cToken program:
        // [0] owner (readonly)
        // [1] mint (readonly)
        // [2] payer (signer, writable)
        // [3] associated_token_account (writable)
        // [4] system_program (readonly)
        // [5] compressible_config (readonly)
        // [6] rent_sponsor (writable)
        let metas = vec![
            CpiMeta {
                pubkey: self.owner.key(),
                is_signer: false,
                is_writable: false,
            },
            CpiMeta {
                pubkey: self.mint.key(),
                is_signer: false,
                is_writable: false,
            },
            CpiMeta {
                pubkey: self.payer.key(),
                is_signer: true,
                is_writable: true,
            },
            CpiMeta {
                pubkey: self.ata.key(),
                is_signer: false,
                is_writable: true,
            },
            CpiMeta {
                pubkey: self.system_program.key(),
                is_signer: false,
                is_writable: false,
            },
            CpiMeta {
                pubkey: self.config.key(),
                is_signer: false,
                is_writable: false,
            },
            CpiMeta {
                pubkey: self.sponsor.key(),
                is_signer: false,
                is_writable: true,
            },
        ];

        let account_infos = vec![
            self.owner.clone(),
            self.mint.clone(),
            self.payer.clone(),
            self.ata.clone(),
            self.system_program.clone(),
            self.config.clone(),
            self.sponsor.clone(),
        ];

        Ok((data, metas, account_infos))
    }
}
