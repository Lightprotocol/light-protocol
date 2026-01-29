//! Traits and processor for decompress_accounts_idempotent instruction.
//!
//! This module implements a zero-allocation two-pass approach for PDA decompression:
//! - Pass 1: Count PDAs, collect pda_indices, collect tokens
//! - Pass 2: Allocate CPI buffer, create PDAs, write directly to zero-copy buffer
//!
//! Stack usage is minimal (~128 bytes for pda_indices array).
use light_compressed_account::{
    discriminators::INVOKE_CPI_WITH_ACCOUNT_INFO_INSTRUCTION,
    instruction_data::{
        compressed_proof::CompressedProof,
        cpi_context::CompressedCpiContext,
        with_account_info::{
            CompressedAccountInfoConfig, InAccountInfoConfig,
            InstructionDataInvokeCpiWithAccountInfo, InstructionDataInvokeCpiWithAccountInfoConfig,
            OutAccountInfoConfig, ZCompressedAccountInfoMut,
        },
    },
};
use light_sdk_types::{
    cpi_accounts::CpiAccountsConfig, cpi_context_write::CpiContextWriteAccounts, CpiSigner,
};
use light_zero_copy::{traits::ZeroCopyAtMut, ZeroCopyNew};
use solana_account_info::AccountInfo;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::cpi::v2::CpiAccounts;

/// Maximum number of accounts that can be decompressed in a single instruction.
/// Stack usage: pda_indices[usize; 16] = 128 bytes
pub const MAX_DECOMPRESS_ACCOUNTS: usize = 16;

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

/// Context trait for decompression with zero-allocation two-pass approach.
///
/// The trait provides two key methods for the two-pass approach:
/// - `collect_layout_and_tokens`: Pass 1 - count PDAs, collect output_data_lens, collect tokens
/// - `create_and_write_pda`: Pass 2 - create PDA on Solana, write to zero-copy buffer
pub trait DecompressContext<'info> {
    /// The compressed account data type (wraps program's variant enum)
    type CompressedData: HasTokenVariant;

    /// Packed token data type
    type PackedTokenData;

    /// Compressed account metadata type (standardized)
    type CompressedMeta: Clone;

    /// Seed parameters type containing data.* field values from instruction data
    type SeedParams;

    // Account accessors
    fn fee_payer(&self) -> &AccountInfo<'info>;
    fn config(&self) -> &AccountInfo<'info>;
    fn rent_sponsor(&self) -> &AccountInfo<'info>;
    fn token_rent_sponsor(&self) -> Option<&AccountInfo<'info>>;
    fn token_program(&self) -> Option<&AccountInfo<'info>>;
    fn token_cpi_authority(&self) -> Option<&AccountInfo<'info>>;
    fn token_config(&self) -> Option<&AccountInfo<'info>>;

    /// Pass 1: Count non-initialized PDAs and collect tokens.
    ///
    /// Iterates through compressed accounts and:
    /// - For each PDA: checks if already initialized, if not: stores index in pda_indices
    /// - For each token: collects into the returned Vec
    ///
    /// # Arguments
    /// * `compressed_accounts` - The compressed accounts to process
    /// * `solana_accounts` - The corresponding Solana accounts (to check if already initialized)
    /// * `pda_indices` - Array to fill with indices of PDAs that need decompression
    ///
    /// # Returns
    /// * `pda_count` - Number of PDAs that need decompression (not already initialized)
    /// * `tokens` - Vec of (PackedTokenData, CompressedMeta) for token accounts
    #[allow(clippy::type_complexity)]
    fn collect_layout_and_tokens(
        &self,
        compressed_accounts: &[Self::CompressedData],
        solana_accounts: &[AccountInfo<'info>],
        pda_indices: &mut [usize; MAX_DECOMPRESS_ACCOUNTS],
    ) -> Result<(usize, Vec<(Self::PackedTokenData, Self::CompressedMeta)>), ProgramError>;

    /// Pass 2: Create single PDA on Solana and write directly to zero-copy buffer.
    ///
    /// This method:
    /// 1. Unpacks the compressed data to get the account state
    /// 2. Derives PDA seeds and verifies against solana_account
    /// 3. Creates the PDA account on Solana (via create_account CPI)
    /// 4. Writes address, input, and output directly to the zero-copy buffer
    ///
    /// # Arguments
    /// * `cpi_accounts` - CPI accounts for system program calls
    /// * `address_space` - The address space for deriving compressed addresses
    /// * `compressed_data` - The compressed account data to decompress (includes meta)
    /// * `solana_account` - The target Solana PDA account
    /// * `seed_params` - Optional seed parameters for PDA derivation
    /// * `zc_info` - Mutable reference to zero-copy buffer to write directly into
    ///
    /// # Returns
    /// * `true` if PDA was created and written
    /// * `false` if account was already initialized (idempotent skip)
    #[allow(clippy::too_many_arguments)]
    fn create_and_write_pda<'b, 'c>(
        &self,
        cpi_accounts: &CpiAccounts<'b, 'info>,
        address_space: &Pubkey,
        compressed_data: &Self::CompressedData,
        solana_account: &AccountInfo<'info>,
        seed_params: Option<&Self::SeedParams>,
        zc_info: &mut ZCompressedAccountInfoMut<'c>,
    ) -> Result<bool, ProgramError>;

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

/// Output data length for decompressed PDAs (always 32 bytes = PDA pubkey).
pub const PDA_OUTPUT_DATA_LEN: u32 = 32;

/// Build CPI config for PDA decompression.
///
/// For PDA decompression, all accounts have identical config structure:
/// - has_address: true
/// - has_input: true
/// - has_output: true
/// - output_data_len: 32 (PDA pubkey)
///
/// # Arguments
/// * `pda_count` - Number of PDAs to decompress
/// * `has_proof` - Whether a validity proof is included
///
/// # Returns
/// `InstructionDataInvokeCpiWithAccountInfoConfig` ready for `byte_len()` and `new_zero_copy()`
#[inline(never)]
pub fn build_decompression_cpi_config(
    pda_count: usize,
    has_proof: bool,
) -> InstructionDataInvokeCpiWithAccountInfoConfig {
    let account_infos = (0..pda_count)
        .map(|_| CompressedAccountInfoConfig {
            address: (true, ()),
            input: (true, InAccountInfoConfig { merkle_context: () }),
            output: (
                true,
                OutAccountInfoConfig {
                    data: PDA_OUTPUT_DATA_LEN,
                },
            ),
        })
        .collect();

    InstructionDataInvokeCpiWithAccountInfoConfig {
        cpi_context: (),
        proof: (has_proof, ()),
        new_address_params: vec![],
        account_infos,
        read_only_addresses: vec![],
        read_only_accounts: vec![],
    }
}

/// Allocate CPI instruction bytes with discriminator.
///
/// # Arguments
/// * `config` - The CPI config describing byte layout
///
/// # Returns
/// A zeroed Vec with space for discriminator + instruction data
#[inline(never)]
pub fn allocate_decompress_cpi_bytes(
    config: &InstructionDataInvokeCpiWithAccountInfoConfig,
) -> Result<Vec<u8>, ProgramError> {
    let data_len = InstructionDataInvokeCpiWithAccountInfo::byte_len(config)
        .map_err(|_| ProgramError::InvalidAccountData)?;
    let mut cpi_bytes = vec![0u8; data_len + 8];
    cpi_bytes[0..8].copy_from_slice(&INVOKE_CPI_WITH_ACCOUNT_INFO_INSTRUCTION);
    Ok(cpi_bytes)
}

/// Core CPI invocation to light-system-program.
/// Shared by both direct execution and CPI context write paths.
#[inline(never)]
fn invoke_light_system_cpi(
    account_metas: Vec<crate::AccountMeta>,
    account_infos: &[AccountInfo<'_>],
    cpi_bytes: Vec<u8>,
    bump: u8,
) -> Result<(), ProgramError> {
    use light_sdk_types::constants::{CPI_AUTHORITY_PDA_SEED, LIGHT_SYSTEM_PROGRAM_ID};

    let instruction = solana_instruction::Instruction {
        program_id: LIGHT_SYSTEM_PROGRAM_ID.into(),
        accounts: account_metas,
        data: cpi_bytes,
    };

    let signer_seeds = [CPI_AUTHORITY_PDA_SEED, &[bump]];
    solana_cpi::invoke_signed(&instruction, account_infos, &[signer_seeds.as_slice()])
}

/// Execute CPI to light-system-program with full account set.
#[inline(never)]
pub fn execute_cpi_invoke_sdk<'info>(
    cpi_accounts: &CpiAccounts<'_, 'info>,
    cpi_bytes: Vec<u8>,
    bump: u8,
) -> Result<(), ProgramError> {
    let account_metas = crate::cpi::v2::lowlevel::to_account_metas(cpi_accounts)?;
    let account_infos = cpi_accounts.to_account_infos();
    invoke_light_system_cpi(account_metas, &account_infos, cpi_bytes, bump)
}

/// Execute CPI to write to CPI context (minimal accounts).
#[inline(never)]
pub fn execute_cpi_write_to_context<'info>(
    accounts: &CpiContextWriteAccounts<'_, AccountInfo<'info>>,
    cpi_bytes: Vec<u8>,
    bump: u8,
) -> Result<(), ProgramError> {
    let account_metas = vec![
        crate::AccountMeta {
            pubkey: *accounts.fee_payer.key,
            is_writable: true,
            is_signer: true,
        },
        crate::AccountMeta {
            pubkey: *accounts.authority.key,
            is_writable: false,
            is_signer: true,
        },
        crate::AccountMeta {
            pubkey: *accounts.cpi_context.key,
            is_writable: true,
            is_signer: false,
        },
    ];
    let account_infos = [
        accounts.fee_payer.clone(),
        accounts.authority.clone(),
        accounts.cpi_context.clone(),
    ];
    invoke_light_system_cpi(account_metas, &account_infos, cpi_bytes, bump)
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

/// Populate CPI struct metadata (mode, bump, program_id, proof, cpi_context).
#[inline(never)]
fn populate_cpi_metadata<'a>(
    cpi_struct: &mut <InstructionDataInvokeCpiWithAccountInfo as ZeroCopyAtMut<'a>>::ZeroCopyAtMut,
    bump: u8,
    invoking_program_id: &Pubkey,
    proof: Option<&CompressedProof>,
    cpi_context: &CompressedCpiContext,
    with_cpi_context: bool,
) {
    cpi_struct.mode = 1;
    cpi_struct.bump = bump;
    cpi_struct.invoking_program_id = invoking_program_id.to_bytes().into();
    cpi_struct.compress_or_decompress_lamports = 0u64.into();
    cpi_struct.is_compress = 0;
    cpi_struct.with_cpi_context = with_cpi_context as u8;
    cpi_struct.with_transaction_hash = 0;
    cpi_struct.cpi_context.cpi_context_account_index = cpi_context.cpi_context_account_index;
    cpi_struct.cpi_context.first_set_context = cpi_context.first_set_context as u8;
    cpi_struct.cpi_context.set_context = cpi_context.set_context as u8;

    if let Some(input_proof) = proof {
        if let Some(ref mut proof_ref) = cpi_struct.proof {
            proof_ref.a = input_proof.a;
            proof_ref.b = input_proof.b;
            proof_ref.c = input_proof.c;
        }
    }
}

/// Processor for decompress_accounts_idempotent using zero-allocation two-pass approach.
///
/// This function implements the two-pass approach for minimal stack usage:
/// - Pass 1: Count PDAs, collect pda_indices, collect tokens
/// - Pass 2: Allocate CPI buffer, create PDAs, write directly to zero-copy buffer
///
/// CPI context batching rules:
/// - Can use inputs from N trees
/// - All inputs must use the FIRST CPI context account of the FIRST input
///
/// # Stack Usage
/// - pda_indices: [usize; 16] = 128 bytes
/// - Counters = ~16 bytes
/// - Total: ~144 bytes (acceptable)
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

    // Stack array for tracking PDA indices
    let mut pda_indices: [usize; MAX_DECOMPRESS_ACCOUNTS] = [0; MAX_DECOMPRESS_ACCOUNTS];

    // Pass 1: Collect layout and tokens (checks which PDAs need decompression)
    let (pda_count, compressed_token_accounts) =
        ctx.collect_layout_and_tokens(&compressed_accounts, solana_accounts, &mut pda_indices)?;

    let has_pdas = pda_count > 0;
    let has_tokens = !compressed_token_accounts.is_empty();

    if !has_pdas && !has_tokens {
        return Ok(());
    }

    let fee_payer = ctx.fee_payer();

    // Process PDAs (if any) using zero-copy pattern with two-pass approach
    if has_pdas {
        let cpi_signer_config = cpi_accounts.config().cpi_signer;

        // Build CPI config from pda_count
        let cpi_config = build_decompression_cpi_config(pda_count, proof.0.is_some());

        // Allocate CPI bytes (single acceptable allocation)
        let mut cpi_bytes = allocate_decompress_cpi_bytes(&cpi_config)?;

        // Get zero-copy mutable struct
        let (mut cpi_struct, _remaining) =
            InstructionDataInvokeCpiWithAccountInfo::new_zero_copy(&mut cpi_bytes[8..], cpi_config)
                .map_err(|_| ProgramError::InvalidAccountData)?;

        // Populate CPI metadata (mode, bump, program_id, proof, cpi_context)
        let cpi_context_for_pdas = if has_tokens {
            CompressedCpiContext::first()
        } else {
            CompressedCpiContext::default()
        };
        populate_cpi_metadata(
            &mut cpi_struct,
            cpi_signer_config.bump,
            &cpi_signer_config.program_id.into(),
            proof.0.as_ref(),
            &cpi_context_for_pdas,
            has_tokens, // with_cpi_context
        );

        // Pass 2: Create PDAs and write directly to zero-copy buffer
        let zc_account_infos = cpi_struct.account_infos.as_mut_slice();
        for (zc_idx, &account_idx) in pda_indices[..pda_count].iter().enumerate() {
            let zc_info = &mut zc_account_infos[zc_idx];

            // Create PDA and write directly to zero-copy buffer
            ctx.create_and_write_pda(
                &cpi_accounts,
                &address_space,
                &compressed_accounts[account_idx],
                &solana_accounts[account_idx],
                seed_params,
                zc_info,
            )?;
        }

        // Execute CPI
        if !has_tokens {
            execute_cpi_invoke_sdk(&cpi_accounts, cpi_bytes, cpi_signer_config.bump)?;
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
            execute_cpi_write_to_context(&system_cpi_accounts, cpi_bytes, cpi_signer.bump)?;
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
