//! Traits and processor for decompress_accounts_idempotent instruction.
//!
//! This module provides:
//! - `DecompressCtx` - A context struct holding all data needed for decompression
//! - `DecompressibleAccount` - A trait for account variants that can be decompressed
//! - `process_decompress_accounts_idempotent` - The main processor function

use light_compressed_account::instruction_data::{
    cpi_context::CompressedCpiContext,
    with_account_info::{CompressedAccountInfo, InstructionDataInvokeCpiWithAccountInfo},
};
use light_sdk_types::{
    cpi_accounts::CpiAccountsConfig, cpi_context_write::CpiContextWriteAccounts,
    instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress, CpiSigner,
};
use solana_account_info::AccountInfo;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::cpi::{v2::CpiAccounts, InvokeLightSystemProgram};

// =============================================================================
// NEW SIMPLIFIED ARCHITECTURE
// =============================================================================

/// Context struct for decompression operations.
///
/// This replaces the complex `DecompressContext` trait with a simple struct
/// containing all the data needed for decompression.
pub struct DecompressCtx<'a, 'info> {
    /// The program ID for PDA derivation
    pub program_id: &'a Pubkey,
    /// The address space for compressed account derivation
    pub address_space: Pubkey,
    /// CPI accounts for invoking the Light system program
    pub cpi_accounts: &'a CpiAccounts<'a, 'info>,
    /// Remaining accounts for resolving packed indices
    pub remaining_accounts: &'a [AccountInfo<'info>],
    /// Account to sponsor rent for decompressed accounts
    pub rent_sponsor: &'a AccountInfo<'info>,
    /// Rent sysvar for calculating minimum balance
    pub rent: &'a solana_sysvar::rent::Rent,
    /// Current slot for compression info
    pub current_slot: u64,
}

/// Trait for account variants that can be decompressed.
///
/// Each packed account variant implements this trait to handle its own
/// decompression logic, eliminating complex match statements in the processor.
pub trait DecompressibleAccount {
    /// Returns true if this is a token account variant.
    fn is_token(&self) -> bool;

    /// Prepare this account for decompression.
    ///
    /// This method:
    /// 1. Resolves any packed indices to actual Pubkeys
    /// 2. Unpacks the data
    /// 3. Derives and verifies the PDA
    /// 4. Creates the Solana account and writes data
    ///
    /// Returns `Some(CompressedAccountInfo)` if decompression was performed,
    /// or `None` if the account was already decompressed (idempotent).
    fn prepare<'a, 'info>(
        self,
        ctx: &DecompressCtx<'a, 'info>,
        solana_account: &AccountInfo<'info>,
        meta: &CompressedAccountMetaNoLamportsNoAddress,
        index: usize,
    ) -> Result<Option<CompressedAccountInfo>, ProgramError>;
}

// =============================================================================
// LEGACY TRAITS (kept for backward compatibility during transition)
// =============================================================================

/// Trait for account variants that can be checked for token or PDA type.
pub trait HasTokenVariant {
    /// Returns true if this variant represents a token account (PackedTokenData).
    fn is_packed_token(&self) -> bool;
}

/// Trait for token seed providers.
///
/// After Phase 8 refactor: The variant itself contains resolved seed pubkeys,
/// so no accounts struct is needed for seed derivation.
pub trait TokenSeedProvider: Copy {
    /// Get seeds for the token account PDA (used for decompression).
    fn get_seeds(&self, program_id: &Pubkey) -> Result<(Vec<Vec<u8>>, Pubkey), ProgramError>;

    /// Get authority seeds for signing during compression.
    fn get_authority_seeds(
        &self,
        program_id: &Pubkey,
    ) -> Result<(Vec<Vec<u8>>, Pubkey), ProgramError>;
}

/// Context trait for decompression.
pub trait DecompressContext<'info> {
    /// The compressed account data type (wraps program's variant enum)
    type CompressedData: HasTokenVariant;

    /// Packed token data type
    type PackedTokenData;

    /// Compressed account metadata type (standardized)
    type CompressedMeta: Clone;

    // Account accessors
    fn fee_payer(&self) -> &AccountInfo<'info>;
    fn config(&self) -> &AccountInfo<'info>;
    fn rent_sponsor(&self) -> &AccountInfo<'info>;
    fn token_rent_sponsor(&self) -> Option<&AccountInfo<'info>>;
    fn token_program(&self) -> Option<&AccountInfo<'info>>;
    fn token_cpi_authority(&self) -> Option<&AccountInfo<'info>>;
    fn token_config(&self) -> Option<&AccountInfo<'info>>;

    /// Collect and unpack compressed accounts into PDAs and tokens.
    #[allow(clippy::type_complexity)]
    #[allow(clippy::too_many_arguments)]
    fn collect_pda_and_token<'b>(
        &self,
        cpi_accounts: &CpiAccounts<'b, 'info>,
        address_space: Pubkey,
        compressed_accounts: Vec<Self::CompressedData>,
        solana_accounts: &[AccountInfo<'info>],
        rent: &solana_sysvar::rent::Rent,
        current_slot: u64,
    ) -> Result<(
        Vec<light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo>,
        Vec<(Self::PackedTokenData, Self::CompressedMeta)>
    ), ProgramError>;

    /// Process token decompression.
    #[allow(clippy::too_many_arguments)]
    fn process_tokens<'b>(
        &self,
        remaining_accounts: &[AccountInfo<'info>],
        fee_payer: &AccountInfo<'info>,
        token_program: &AccountInfo<'info>,
        token_rent_sponsor: &AccountInfo<'info>,
        token_cpi_authority: &AccountInfo<'info>,
        token_config: &AccountInfo<'info>,
        config: &AccountInfo<'info>,
        token_accounts: Vec<(Self::PackedTokenData, Self::CompressedMeta)>,
        proof: crate::instruction::ValidityProof,
        cpi_accounts: &CpiAccounts<'b, 'info>,
        post_system_accounts: &[AccountInfo<'info>],
        has_prior_context: bool,
    ) -> Result<(), ProgramError>;
}

/// Trait for PDA types that can derive seeds with full account context access.
pub trait PdaSeedDerivation<A, S> {
    fn derive_pda_seeds_with_accounts(
        &self,
        program_id: &Pubkey,
        accounts: &A,
        seed_params: &S,
    ) -> Result<(Vec<Vec<u8>>, Pubkey), ProgramError>;
}

/// Check what types of accounts are in the batch.
/// Returns (has_tokens, has_pdas).
#[inline(never)]
pub fn check_account_types<T: HasTokenVariant>(compressed_accounts: &[T]) -> (bool, bool) {
    let (mut has_tokens, mut has_pdas) = (false, false);
    for account in compressed_accounts {
        if account.is_packed_token() {
            has_tokens = true;
        } else {
            has_pdas = true;
        }
        if has_tokens && has_pdas {
            break;
        }
    }
    (has_tokens, has_pdas)
}

/// Processor for decompress_accounts_idempotent.
///
/// CPI context batching rules:
/// - Can use inputs from N trees
/// - All inputs must use the FIRST CPI context account of the FIRST input
#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub fn process_decompress_accounts_idempotent<'info, Ctx>(
    ctx: &Ctx,
    remaining_accounts: &[AccountInfo<'info>],
    compressed_accounts: Vec<Ctx::CompressedData>,
    proof: crate::instruction::ValidityProof,
    system_accounts_offset: u8,
    cpi_signer: CpiSigner,
    program_id: &Pubkey,
    rent: &solana_sysvar::rent::Rent,
    current_slot: u64,
) -> Result<(), ProgramError>
where
    Ctx: DecompressContext<'info>,
{
    let compression_config = crate::interface::LightConfig::load_checked(ctx.config(), program_id)?;
    let address_space = compression_config.address_space[0];

    let (has_tokens, has_pdas) = check_account_types(&compressed_accounts);

    if !has_tokens && !has_pdas {
        return Ok(());
    }

    let system_accounts_offset_usize = system_accounts_offset as usize;
    if system_accounts_offset_usize > remaining_accounts.len() {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Use CPI context batching when we have both PDAs and tokens
    // CPI context can handle inputs from N trees - all use FIRST cpi context of FIRST input
    let needs_cpi_context = has_tokens && has_pdas;
    let cpi_accounts = if needs_cpi_context {
        CpiAccounts::new_with_config(
            ctx.fee_payer(),
            &remaining_accounts[system_accounts_offset_usize..],
            CpiAccountsConfig::new_with_cpi_context(cpi_signer),
        )
    } else {
        CpiAccounts::new(
            ctx.fee_payer(),
            &remaining_accounts[system_accounts_offset_usize..],
            cpi_signer,
        )
    };

    let pda_accounts_start = remaining_accounts
        .len()
        .checked_sub(compressed_accounts.len())
        .ok_or_else(|| ProgramError::from(crate::error::LightSdkError::ConstraintViolation))?;
    let solana_accounts = remaining_accounts
        .get(pda_accounts_start..)
        .ok_or_else(|| ProgramError::from(crate::error::LightSdkError::ConstraintViolation))?;

    let (compressed_pda_infos, compressed_token_accounts) = ctx.collect_pda_and_token(
        &cpi_accounts,
        address_space,
        compressed_accounts,
        solana_accounts,
        rent,
        current_slot,
    )?;

    let has_pdas = !compressed_pda_infos.is_empty();
    let has_tokens = !compressed_token_accounts.is_empty();

    if !has_pdas && !has_tokens {
        return Ok(());
    }

    let fee_payer = ctx.fee_payer();

    // Process PDAs (if any)
    if has_pdas {
        if !has_tokens {
            // PDAs only - execute directly (manual construction to avoid extra allocations)
            let cpi_signer_config = cpi_accounts.config().cpi_signer;
            let instruction_data = InstructionDataInvokeCpiWithAccountInfo {
                mode: 1,
                bump: cpi_signer_config.bump,
                invoking_program_id: cpi_signer_config.program_id.into(),
                compress_or_decompress_lamports: 0,
                is_compress: false,
                with_cpi_context: false,
                with_transaction_hash: false,
                cpi_context: CompressedCpiContext::default(),
                proof: proof.0,
                new_address_params: Vec::new(),
                account_infos: compressed_pda_infos,
                read_only_addresses: Vec::new(),
                read_only_accounts: Vec::new(),
            };
            instruction_data.invoke(cpi_accounts.clone())?;
        } else {
            // PDAs + tokens - write to CPI context first, tokens will execute
            let authority = cpi_accounts
                .authority()
                .map_err(|_| ProgramError::MissingRequiredSignature)?;
            let cpi_context_account = cpi_accounts
                .cpi_context()
                .map_err(|_| ProgramError::MissingRequiredSignature)?;
            let system_cpi_accounts = CpiContextWriteAccounts {
                fee_payer,
                authority,
                cpi_context: cpi_context_account,
                cpi_signer,
            };

            // Manual construction to avoid extra allocations
            let instruction_data = InstructionDataInvokeCpiWithAccountInfo {
                mode: 1,
                bump: cpi_signer.bump,
                invoking_program_id: cpi_signer.program_id.into(),
                compress_or_decompress_lamports: 0,
                is_compress: false,
                with_cpi_context: true,
                with_transaction_hash: false,
                cpi_context: CompressedCpiContext::first(),
                proof: proof.0,
                new_address_params: Vec::new(),
                account_infos: compressed_pda_infos,
                read_only_addresses: Vec::new(),
                read_only_accounts: Vec::new(),
            };
            instruction_data.invoke_write_to_cpi_context_first(system_cpi_accounts)?;
        }
    }

    // Process tokens (if any) - executes and consumes CPI context if PDAs wrote to it
    if has_tokens {
        let post_system_offset = cpi_accounts.system_accounts_end_offset();
        let all_infos = cpi_accounts.account_infos();
        let post_system_accounts = all_infos
            .get(post_system_offset..)
            .ok_or_else(|| ProgramError::from(crate::error::LightSdkError::ConstraintViolation))?;

        let light_token_program = ctx
            .token_program()
            .ok_or(ProgramError::NotEnoughAccountKeys)?;
        let token_rent_sponsor = ctx
            .token_rent_sponsor()
            .ok_or(ProgramError::NotEnoughAccountKeys)?;
        let token_cpi_authority = ctx
            .token_cpi_authority()
            .ok_or(ProgramError::NotEnoughAccountKeys)?;
        let token_config = ctx
            .token_config()
            .ok_or(ProgramError::NotEnoughAccountKeys)?;

        ctx.process_tokens(
            remaining_accounts,
            fee_payer,
            light_token_program,
            token_rent_sponsor,
            token_cpi_authority,
            token_config,
            ctx.config(),
            compressed_token_accounts,
            proof,
            &cpi_accounts,
            post_system_accounts,
            has_pdas, // has_prior_context: PDAs wrote to CPI context
        )?;
    }

    Ok(())
}
