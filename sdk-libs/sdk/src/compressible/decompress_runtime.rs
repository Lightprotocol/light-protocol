//! Traits and processor for decompress_accounts_idempotent instruction.
use light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo;
use light_sdk_types::cpi_context_write::CpiContextWriteAccounts;
use light_sdk_types::{
    cpi_accounts::CpiAccountsConfig,
    instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress, CpiSigner,
};
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::{
    cpi::{
        v2::{CpiAccounts, LightSystemProgramCpi},
        InvokeLightSystemProgram, LightCpiInstruction,
    },
    AnchorDeserialize, AnchorSerialize, LightDiscriminator,
};

/// Trait for account variants that can be checked for token, mint, or PDA type.
pub trait HasTokenVariant {
    /// Returns true if this variant represents a token account (PackedCTokenData).
    fn is_packed_ctoken(&self) -> bool;
    /// Returns true if this variant represents a compressed mint.
    fn is_compressed_mint(&self) -> bool {
        false // default impl for backwards compatibility
    }
}

/// Trait for CToken seed providers.
///
/// Also defined in compressed-token-sdk for token-specific runtime helpers.
pub trait CTokenSeedProvider: Copy {
    /// Type of accounts struct needed for seed derivation.
    type Accounts<'info>;

    /// Get seeds for the token account PDA (used for decompression).
    fn get_seeds<'a, 'info>(
        &self,
        accounts: &'a Self::Accounts<'info>,
        remaining_accounts: &'a [AccountInfo<'info>],
    ) -> Result<(Vec<Vec<u8>>, Pubkey), ProgramError>;

    /// Get authority seeds for signing during compression.
    fn get_authority_seeds<'a, 'info>(
        &self,
        accounts: &'a Self::Accounts<'info>,
        remaining_accounts: &'a [AccountInfo<'info>],
    ) -> Result<(Vec<Vec<u8>>, Pubkey), ProgramError>;

    /// Returns true if this variant represents a user-owned ATA.
    /// ATAs use standard ctoken derivation (owner + ctoken_program_id + mint)
    /// and the wallet owner signs instead of the program.
    fn is_ata(&self) -> bool {
        false // Default: not an ATA
    }
}

/// Context trait for decompression.
pub trait DecompressContext<'info> {
    /// The compressed account data type (wraps program's variant enum)
    type CompressedData: HasTokenVariant;

    /// Packed token data type
    type PackedTokenData;

    /// Compressed mint data type for mint decompression
    type CompressedMintData: Clone;

    /// Compressed account metadata type (standardized)
    type CompressedMeta: Clone;

    /// Seed parameters type containing data.* field values from instruction data
    type SeedParams;

    // Account accessors
    fn fee_payer(&self) -> &AccountInfo<'info>;
    fn config(&self) -> &AccountInfo<'info>;
    fn rent_sponsor(&self) -> &AccountInfo<'info>;
    fn ctoken_rent_sponsor(&self) -> Option<&AccountInfo<'info>>;
    fn ctoken_program(&self) -> Option<&AccountInfo<'info>>;
    fn ctoken_cpi_authority(&self) -> Option<&AccountInfo<'info>>;
    fn ctoken_config(&self) -> Option<&AccountInfo<'info>>;

    /// Collect and unpack compressed accounts into PDAs and tokens.
    ///
    /// Caller program-specific: handles variant matching and PDA seed derivation.
    #[allow(clippy::type_complexity)]
    #[allow(clippy::too_many_arguments)]
    fn collect_pda_and_token<'b>(
        &self,
        cpi_accounts: &CpiAccounts<'b, 'info>,
        address_space: Pubkey,
        compressed_accounts: Vec<Self::CompressedData>,
        solana_accounts: &[AccountInfo<'info>],
        seed_params: Option<&Self::SeedParams>,
    ) -> Result<(
        Vec<light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo>,
        Vec<(Self::PackedTokenData, Self::CompressedMeta)>
    ), ProgramError>;

    /// Process token decompression.
    ///
    /// Caller program-specific: handles token account creation and seed derivation.
    /// `has_prior_context`: true if PDAs or Mints already wrote to CPI context
    #[allow(clippy::too_many_arguments)]
    fn process_tokens<'b>(
        &self,
        remaining_accounts: &[AccountInfo<'info>],
        fee_payer: &AccountInfo<'info>,
        ctoken_program: &AccountInfo<'info>,
        ctoken_rent_sponsor: &AccountInfo<'info>,
        ctoken_cpi_authority: &AccountInfo<'info>,
        ctoken_config: &AccountInfo<'info>,
        config: &AccountInfo<'info>,
        ctoken_accounts: Vec<(Self::PackedTokenData, Self::CompressedMeta)>,
        proof: crate::instruction::ValidityProof,
        cpi_accounts: &CpiAccounts<'b, 'info>,
        post_system_accounts: &[AccountInfo<'info>],
        has_prior_context: bool,
    ) -> Result<(), ProgramError>;

    /// Process mint decompression.
    ///
    /// Caller program-specific: handles mint account decompression via CPI to ctoken program.
    /// Default implementation returns Ok(()) for programs that don't handle mints.
    #[allow(clippy::too_many_arguments)]
    fn process_mints<'b>(
        &self,
        _cpi_accounts: &CpiAccounts<'b, 'info>,
        _cmint_accounts: Vec<(Self::CompressedMintData, Self::CompressedMeta)>,
        _proof: crate::instruction::ValidityProof,
        _has_prior_context: bool,
        _has_tokens: bool,
    ) -> Result<(), ProgramError> {
        // Default: no mint handling
        Ok(())
    }

    /// Collect and categorize compressed accounts into PDAs, tokens, and mints.
    ///
    /// Returns (pda_infos, token_accounts, mint_accounts).
    /// Default implementation delegates to collect_pda_and_token and returns empty mints.
    #[allow(clippy::type_complexity)]
    #[allow(clippy::too_many_arguments)]
    fn collect_all_accounts<'b>(
        &self,
        cpi_accounts: &CpiAccounts<'b, 'info>,
        address_space: Pubkey,
        compressed_accounts: Vec<Self::CompressedData>,
        solana_accounts: &[AccountInfo<'info>],
        seed_params: Option<&Self::SeedParams>,
    ) -> Result<
        (
            Vec<CompressedAccountInfo>,
            Vec<(Self::PackedTokenData, Self::CompressedMeta)>,
            Vec<(Self::CompressedMintData, Self::CompressedMeta)>,
        ),
        ProgramError,
    > {
        let (pdas, tokens) = self.collect_pda_and_token(
            cpi_accounts,
            address_space,
            compressed_accounts,
            solana_accounts,
            seed_params,
        )?;
        Ok((pdas, tokens, Vec::new()))
    }
}

/// Trait for PDA types that can derive seeds with full account context access.
///
/// - A: The accounts struct type (typically DecompressAccountsIdempotent<'info>)
/// - S: The SeedParams struct containing data.* field values from instruction data
///
/// This allows PDA seeds to reference:
/// - `data.*` fields from instruction parameters (seed_params.field)
/// - `ctx.*` accounts from the instruction context (accounts.field)
///
/// For off-chain PDA derivation, use the generated client helper functions (get_*_seeds).
pub trait PdaSeedDerivation<A, S> {
    fn derive_pda_seeds_with_accounts(
        &self,
        program_id: &Pubkey,
        accounts: &A,
        seed_params: &S,
    ) -> Result<(Vec<Vec<u8>>, Pubkey), ProgramError>;
}

/// Check compressed accounts to determine if we have tokens, mints, PDAs, and/or ATAs.
/// Returns (has_tokens, has_pdas, has_mints, has_atas)
#[inline(never)]
/// Check what types of accounts are in the batch.
/// Returns (has_tokens, has_pdas, has_mints).
/// Note: ATAs are handled within the token processing path via `is_ata()` on the variant.
pub fn check_account_types<T: HasTokenVariant>(compressed_accounts: &[T]) -> (bool, bool, bool) {
    let (mut has_tokens, mut has_pdas, mut has_mints) = (false, false, false);
    for account in compressed_accounts {
        if account.is_packed_ctoken() {
            has_tokens = true;
        } else if account.is_compressed_mint() {
            has_mints = true;
        } else {
            has_pdas = true;
        }
        if has_tokens && has_pdas && has_mints {
            break;
        }
    }
    (has_tokens, has_pdas, has_mints)
}

/// Handler for unpacking and preparing a single PDA variant for decompression.
#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub fn handle_packed_pda_variant<'a, 'b, 'info, T, P, A, S>(
    accounts_rent_sponsor: &AccountInfo<'info>,
    cpi_accounts: &CpiAccounts<'b, 'info>,
    address_space: Pubkey,
    solana_account: &AccountInfo<'info>,
    index: usize,
    packed: &P,
    meta: &CompressedAccountMetaNoLamportsNoAddress,
    post_system_accounts: &[AccountInfo<'info>],
    compressed_pda_infos: &mut Vec<CompressedAccountInfo>,
    program_id: &Pubkey,
    seed_accounts: &A,
    seed_params: Option<&S>,
) -> Result<(), ProgramError>
where
    T: PdaSeedDerivation<A, S>
        + Clone
        + crate::account::Size
        + LightDiscriminator
        + Default
        + AnchorSerialize
        + AnchorDeserialize
        + crate::compressible::HasCompressionInfo
        + 'info,
    P: crate::compressible::Unpack<Unpacked = T>,
    S: Default,
{
    let data: T = P::unpack(packed, post_system_accounts)?;

    // CHECK: pda match
    // Call the method with account context and seed params
    // Note: Some implementations may use S::default() when seed_params is None for static seeds
    let (seeds_vec, derived_pda) = if let Some(params) = seed_params {
        data.derive_pda_seeds_with_accounts(program_id, seed_accounts, params)?
    } else {
        let default_params = S::default();
        data.derive_pda_seeds_with_accounts(program_id, seed_accounts, &default_params)?
    };
    if derived_pda != *solana_account.key {
        msg!(
            "Derived PDA does not match account at index {}: expected {:?}, got {:?}, seeds: {:?}",
            index,
            solana_account.key,
            derived_pda,
            seeds_vec
        );
        return Err(ProgramError::from(
            crate::error::LightSdkError::ConstraintViolation,
        ));
    }

    // prepare decompression
    let compressed_infos = {
        let seed_refs: Vec<&[u8]> = seeds_vec.iter().map(|v| v.as_slice()).collect();
        crate::compressible::decompress_idempotent::prepare_account_for_decompression_idempotent::<T>(
            program_id,
            data,
            crate::compressible::decompress_idempotent::into_compressed_meta_with_address(
                meta,
                solana_account,
                address_space,
                program_id,
            ),
            solana_account,
            accounts_rent_sponsor,
            cpi_accounts,
            seed_refs.as_slice(),
        )?
    };
    compressed_pda_infos.extend(compressed_infos);
    Ok(())
}

/// Processor for decompress_accounts_idempotent.
///
/// CPI context batching rules:
/// - Single type (only PDAs, only mints, or only tokens): execute directly, no CPI context
/// - 2+ different types: use CPI context batching
///   - First type writes (first_set_context)
///   - Middle types write (set_context)
///   - Last type executes (consumes context)
///
/// Order: PDAs -> Mints -> Tokens (tokens always last if present)
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
    seed_params: Option<&Ctx::SeedParams>,
) -> Result<(), ProgramError>
where
    Ctx: DecompressContext<'info>,
{
    let compression_config =
        crate::compressible::CompressibleConfig::load_checked(ctx.config(), program_id)?;
    let address_space = compression_config.address_space[0];

    let (has_tokens, has_pdas, has_mints) = check_account_types(&compressed_accounts);
    if !has_tokens && !has_pdas && !has_mints {
        return Ok(());
    }

    let system_accounts_offset_usize = system_accounts_offset as usize;
    if system_accounts_offset_usize > remaining_accounts.len() {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Count how many different types we have
    let type_count = has_tokens as u8 + has_pdas as u8 + has_mints as u8;

    // Use CPI context only if we have 2+ different types (need batching)
    let needs_cpi_context = type_count >= 2;
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

    // Call trait method for program-specific collection (handles PDAs, tokens, mints)
    let (compressed_pda_infos, compressed_token_accounts, compressed_mint_accounts) = ctx
        .collect_all_accounts(
            &cpi_accounts,
            address_space,
            compressed_accounts,
            solana_accounts,
            seed_params,
        )?;

    let has_pdas = !compressed_pda_infos.is_empty();
    let has_tokens = !compressed_token_accounts.is_empty();
    let has_mints = !compressed_mint_accounts.is_empty();

    if !has_pdas && !has_tokens && !has_mints {
        return Ok(());
    }

    // Recalculate type count after collection (may differ from initial check)
    let type_count = has_tokens as u8 + has_pdas as u8 + has_mints as u8;

    let fee_payer = ctx.fee_payer();

    // Process PDAs (if any)
    if has_pdas {
        if type_count == 1 {
            // PDAs only - execute directly (no CPI context)
            LightSystemProgramCpi::new_cpi(cpi_accounts.config().cpi_signer, proof)
                .with_account_infos(&compressed_pda_infos)
                .invoke(cpi_accounts.clone())?;
        } else {
            // PDAs + other types - write to CPI context first
            let authority = cpi_accounts
                .authority()
                .map_err(|_| ProgramError::MissingRequiredSignature)?;
            let cpi_context = cpi_accounts
                .cpi_context()
                .map_err(|_| ProgramError::MissingRequiredSignature)?;
            let system_cpi_accounts = CpiContextWriteAccounts {
                fee_payer,
                authority,
                cpi_context,
                cpi_signer,
            };

            LightSystemProgramCpi::new_cpi(cpi_signer, proof)
                .with_account_infos(&compressed_pda_infos)
                .write_to_cpi_context_first()
                .invoke_write_to_cpi_context_first(system_cpi_accounts)?;
        }
    }

    // Process mints (if any)
    if has_mints {
        // has_prior_context: PDAs already wrote to context
        // has_subsequent: tokens will execute after
        ctx.process_mints(
            &cpi_accounts,
            compressed_mint_accounts,
            proof,
            has_pdas,   // has_prior_context
            has_tokens, // has_subsequent (tokens will execute after)
        )?;
    }

    // Process tokens (if any) - always last in the chain
    if has_tokens {
        let post_system_offset = cpi_accounts.system_accounts_end_offset();
        let all_infos = cpi_accounts.account_infos();
        let post_system_accounts = all_infos
            .get(post_system_offset..)
            .ok_or_else(|| ProgramError::from(crate::error::LightSdkError::ConstraintViolation))?;

        let ctoken_program = ctx
            .ctoken_program()
            .ok_or(ProgramError::NotEnoughAccountKeys)?;
        let ctoken_rent_sponsor = ctx
            .ctoken_rent_sponsor()
            .ok_or(ProgramError::NotEnoughAccountKeys)?;
        let ctoken_cpi_authority = ctx
            .ctoken_cpi_authority()
            .ok_or(ProgramError::NotEnoughAccountKeys)?;
        let ctoken_config = ctx
            .ctoken_config()
            .ok_or(ProgramError::NotEnoughAccountKeys)?;

        ctx.process_tokens(
            remaining_accounts,
            fee_payer,
            ctoken_program,
            ctoken_rent_sponsor,
            ctoken_cpi_authority,
            ctoken_config,
            ctx.config(),
            compressed_token_accounts,
            proof,
            &cpi_accounts,
            post_system_accounts,
            has_pdas || has_mints, // has_prior_context: something wrote to CPI context before
        )?;
    }

    Ok(())
}
