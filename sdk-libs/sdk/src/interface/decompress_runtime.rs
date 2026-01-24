//! Traits and processor for decompress_accounts_idempotent instruction.
use light_compressed_account::{
    discriminators::INVOKE_CPI_WITH_ACCOUNT_INFO_INSTRUCTION,
    instruction_data::{
        compressed_proof::CompressedProof,
        cpi_context::CompressedCpiContext,
        with_account_info::{
            CompressedAccountInfo, CompressedAccountInfoConfig, InAccountInfoConfig,
            InstructionDataInvokeCpiWithAccountInfo, InstructionDataInvokeCpiWithAccountInfoConfig,
            OutAccountInfoConfig,
        },
    },
};
use light_sdk_types::{
    cpi_accounts::CpiAccountsConfig, cpi_context_write::CpiContextWriteAccounts,
    instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress, CpiSigner,
};
use light_zero_copy::{traits::ZeroCopyAtMut, ZeroCopyNew};
use solana_account_info::AccountInfo;
use solana_msg::msg;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::{cpi::v2::CpiAccounts, AnchorDeserialize, AnchorSerialize, LightDiscriminator};

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

    /// Collect and unpack compressed accounts into PDAs and tokens.
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

/// Configuration for a single account in the decompress CPI.
/// Used to build `CompressedAccountInfoConfig` for zero-copy allocation.
#[derive(Debug, Clone, Copy)]
pub struct DecompressAccountConfig {
    /// Whether the account has an address
    pub has_address: bool,
    /// Whether the account has an input (being nullified)
    pub has_input: bool,
    /// Whether the account has an output (being created)
    pub has_output: bool,
    /// Length of output data (only used if has_output is true)
    pub output_data_len: u32,
}

impl DecompressAccountConfig {
    /// Create config for a decompression account (has both input and output with address)
    pub fn decompress(output_data_len: u32) -> Self {
        Self {
            has_address: true,
            has_input: true,
            has_output: true,
            output_data_len,
        }
    }

    /// Create config for input-only account
    pub fn input_only(has_address: bool) -> Self {
        Self {
            has_address,
            has_input: true,
            has_output: false,
            output_data_len: 0,
        }
    }

    /// Create config for output-only account
    pub fn output_only(has_address: bool, output_data_len: u32) -> Self {
        Self {
            has_address,
            has_input: false,
            has_output: true,
            output_data_len,
        }
    }
}

/// Build the CPI config for decompression.
///
/// # Arguments
/// * `account_configs` - Configuration for each account (address, input, output)
/// * `has_proof` - Whether a validity proof is included
///
/// # Returns
/// `InstructionDataInvokeCpiWithAccountInfoConfig` ready for `byte_len()` and `new_zero_copy()`
#[inline(never)]
pub fn build_decompress_cpi_config(
    account_configs: &[DecompressAccountConfig],
    has_proof: bool,
) -> InstructionDataInvokeCpiWithAccountInfoConfig {
    let account_infos = account_configs
        .iter()
        .map(|cfg| CompressedAccountInfoConfig {
            address: (cfg.has_address, ()),
            input: (cfg.has_input, InAccountInfoConfig { merkle_context: () }),
            output: (
                cfg.has_output,
                OutAccountInfoConfig {
                    data: cfg.output_data_len,
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

/// Build config from collected CompressedAccountInfo list.
///
/// This is a convenience function that extracts config metadata from
/// an existing list of CompressedAccountInfo.
#[inline(never)]
pub fn build_config_from_account_infos(
    account_infos: &[CompressedAccountInfo],
    has_proof: bool,
) -> InstructionDataInvokeCpiWithAccountInfoConfig {
    let configs: Vec<DecompressAccountConfig> = account_infos
        .iter()
        .map(|info| DecompressAccountConfig {
            has_address: info.address.is_some(),
            has_input: info.input.is_some(),
            has_output: info.output.is_some(),
            output_data_len: info
                .output
                .as_ref()
                .map(|o| o.data.len() as u32)
                .unwrap_or(0),
        })
        .collect();

    build_decompress_cpi_config(&configs, has_proof)
}

/// Populate a zero-copy mutable struct from collected CompressedAccountInfo.
///
/// This copies data from the collected `CompressedAccountInfo` list into
/// the zero-copy mutable struct fields.
#[inline(never)]
pub fn populate_zero_copy_cpi<'a>(
    cpi_struct: &mut <InstructionDataInvokeCpiWithAccountInfo as ZeroCopyAtMut<'a>>::ZeroCopyAtMut,
    bump: u8,
    invoking_program_id: &Pubkey,
    proof: Option<&CompressedProof>,
    cpi_context: &CompressedCpiContext,
    with_cpi_context: bool,
    account_infos: &[CompressedAccountInfo],
) -> Result<(), ProgramError> {
    // Set meta fields via DerefMut
    cpi_struct.mode = 1; // V2 mode
    cpi_struct.bump = bump;
    cpi_struct.invoking_program_id = (*invoking_program_id).into();
    cpi_struct.compress_or_decompress_lamports = 0u64.into();
    cpi_struct.is_compress = 0; // false
    cpi_struct.with_cpi_context = with_cpi_context as u8;
    cpi_struct.with_transaction_hash = 0; // false

    // Set CPI context
    cpi_struct.cpi_context.cpi_context_account_index = cpi_context.cpi_context_account_index;
    cpi_struct.cpi_context.first_set_context = cpi_context.first_set_context as u8;
    cpi_struct.cpi_context.set_context = cpi_context.set_context as u8;

    // Set proof if present
    if let Some(input_proof) = proof {
        if let Some(ref mut proof_ref) = cpi_struct.proof {
            proof_ref.a = input_proof.a;
            proof_ref.b = input_proof.b;
            proof_ref.c = input_proof.c;
        }
    }

    // Populate account_infos
    let zc_account_infos = cpi_struct.account_infos.as_mut_slice();
    for (i, info) in account_infos.iter().enumerate() {
        let zc_info = &mut zc_account_infos[i];

        // Set address if present
        if let (Some(addr), Some(ref mut zc_addr)) = (&info.address, &mut zc_info.address) {
            zc_addr.copy_from_slice(addr);
        }

        // Set input if present
        if let (Some(input), Some(ref mut zc_input)) = (&info.input, &mut zc_info.input) {
            zc_input.discriminator = input.discriminator;
            zc_input.data_hash = input.data_hash;
            // Set merkle_context fields
            zc_input.merkle_context.merkle_tree_pubkey_index =
                input.merkle_context.merkle_tree_pubkey_index;
            zc_input.merkle_context.queue_pubkey_index = input.merkle_context.queue_pubkey_index;
            zc_input
                .merkle_context
                .leaf_index
                .set(input.merkle_context.leaf_index);
            zc_input.merkle_context.prove_by_index = input.merkle_context.prove_by_index as u8;
            zc_input.root_index.set(input.root_index);
            zc_input.lamports.set(input.lamports);
        }

        // Set output if present
        if let (Some(output), Some(ref mut zc_output)) = (&info.output, &mut zc_info.output) {
            zc_output.discriminator = output.discriminator;
            zc_output.data_hash = output.data_hash;
            zc_output.output_merkle_tree_index = output.output_merkle_tree_index;
            zc_output.lamports.set(output.lamports);
            zc_output.data.copy_from_slice(&output.data);
        }
    }

    Ok(())
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

/// Execute CPI to light-system-program with pre-populated instruction bytes.
///
/// This is the SDK version of the zero-copy CPI pattern. It takes pre-allocated
/// and populated CPI bytes and invokes the Light system program.
///
/// # Arguments
/// * `cpi_accounts` - The CPI accounts struct
/// * `cpi_bytes` - Pre-allocated and populated instruction bytes (with discriminator)
/// * `bump` - The CPI authority bump seed
///
/// # Returns
/// `Result<(), ProgramError>` - Success or error from the CPI call
#[inline(never)]
pub fn execute_cpi_invoke_sdk<'info>(
    cpi_accounts: &CpiAccounts<'_, 'info>,
    cpi_bytes: Vec<u8>,
    bump: u8,
) -> Result<(), ProgramError> {
    use light_sdk_types::constants::{CPI_AUTHORITY_PDA_SEED, LIGHT_SYSTEM_PROGRAM_ID};

    // Verify bump is set (basic sanity check)
    if cpi_bytes.len() < 10 || cpi_bytes[9] == 0 {
        msg!("Bump not set in cpi struct.");
        return Err(ProgramError::InvalidInstructionData);
    }

    // Get account metas and infos from CpiAccounts
    let account_metas = crate::cpi::v2::lowlevel::to_account_metas(cpi_accounts)?;
    let account_infos = cpi_accounts.to_account_infos();

    // Build instruction with raw bytes
    let instruction = solana_instruction::Instruction {
        program_id: LIGHT_SYSTEM_PROGRAM_ID.into(),
        accounts: account_metas,
        data: cpi_bytes,
    };

    // Invoke with PDA signer
    let signer_seeds = [CPI_AUTHORITY_PDA_SEED, &[bump]];
    solana_cpi::invoke_signed(&instruction, &account_infos, &[signer_seeds.as_slice()])
}

/// Execute CPI to write to CPI context.
///
/// This is used when PDAs and tokens both need to be processed - PDAs write
/// to CPI context first, then tokens execute and consume the context.
#[inline(never)]
pub fn execute_cpi_write_to_context<'info>(
    accounts: &CpiContextWriteAccounts<'_, AccountInfo<'info>>,
    cpi_bytes: Vec<u8>,
    bump: u8,
) -> Result<(), ProgramError> {
    use light_sdk_types::constants::{CPI_AUTHORITY_PDA_SEED, LIGHT_SYSTEM_PROGRAM_ID};

    // Verify bump is set
    if cpi_bytes.len() < 10 || cpi_bytes[9] == 0 {
        msg!("Bump not set in cpi struct.");
        return Err(ProgramError::InvalidInstructionData);
    }

    // Build minimal account metas for CPI context write
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

    let instruction = solana_instruction::Instruction {
        program_id: LIGHT_SYSTEM_PROGRAM_ID.into(),
        accounts: account_metas,
        data: cpi_bytes,
    };

    let signer_seeds = [CPI_AUTHORITY_PDA_SEED, &[bump]];
    solana_cpi::invoke_signed(&instruction, &account_infos, &[signer_seeds.as_slice()])
}

/// Execute CPI using zero-copy pattern for PDA decompression.
///
/// This function builds the config, allocates bytes, populates the zero-copy
/// struct, and executes the CPI in one call.
#[inline(never)]
pub fn invoke_zero_copy_cpi<'info>(
    cpi_accounts: &CpiAccounts<'_, 'info>,
    bump: u8,
    invoking_program_id: &Pubkey,
    proof: Option<&CompressedProof>,
    cpi_context: &CompressedCpiContext,
    with_cpi_context: bool,
    account_infos: &[CompressedAccountInfo],
) -> Result<(), ProgramError> {
    // Build config from collected account infos
    let config = build_config_from_account_infos(account_infos, proof.is_some());

    // Allocate CPI bytes
    let mut cpi_bytes = allocate_decompress_cpi_bytes(&config)?;

    // Get zero-copy mutable struct
    let (mut cpi_struct, _remaining) =
        InstructionDataInvokeCpiWithAccountInfo::new_zero_copy(&mut cpi_bytes[8..], config)
            .map_err(|_| ProgramError::InvalidAccountData)?;

    // Populate the struct
    populate_zero_copy_cpi(
        &mut cpi_struct,
        bump,
        invoking_program_id,
        proof,
        cpi_context,
        with_cpi_context,
        account_infos,
    )?;

    // Execute CPI
    execute_cpi_invoke_sdk(cpi_accounts, cpi_bytes, bump)
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
        + crate::interface::HasCompressionInfo
        + 'info,
    P: crate::interface::Unpack<Unpacked = T>,
    S: Default,
{
    let data: T = P::unpack(packed, post_system_accounts)?;

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

    let compressed_infos = {
        // Use fixed-size array to avoid heap allocation (MAX_SEEDS = 16)
        const MAX_SEEDS: usize = 16;
        let mut seed_refs: [&[u8]; MAX_SEEDS] = [&[]; MAX_SEEDS];
        let len = seeds_vec.len().min(MAX_SEEDS);
        for i in 0..len {
            seed_refs[i] = seeds_vec[i].as_slice();
        }
        crate::interface::decompress_idempotent::prepare_account_for_decompression_idempotent::<T>(
            program_id,
            data,
            crate::interface::decompress_idempotent::into_compressed_meta_with_address(
                meta,
                solana_account,
                address_space,
                program_id,
            ),
            solana_account,
            accounts_rent_sponsor,
            cpi_accounts,
            &seed_refs[..len],
        )?
    };
    compressed_pda_infos.extend(compressed_infos);
    Ok(())
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
        seed_params,
    )?;

    let has_pdas = !compressed_pda_infos.is_empty();
    let has_tokens = !compressed_token_accounts.is_empty();

    if !has_pdas && !has_tokens {
        return Ok(());
    }

    let fee_payer = ctx.fee_payer();

    // Process PDAs (if any) using zero-copy pattern
    if has_pdas {
        let cpi_signer_config = cpi_accounts.config().cpi_signer;

        if !has_tokens {
            // PDAs only - execute directly using zero-copy
            invoke_zero_copy_cpi(
                &cpi_accounts,
                cpi_signer_config.bump,
                &cpi_signer_config.program_id.into(),
                proof.0.as_ref(),
                &CompressedCpiContext::default(),
                false, // with_cpi_context
                &compressed_pda_infos,
            )?;
        } else {
            // PDAs + tokens - write to CPI context first, tokens will execute
            // For CPI context write, we need a minimal accounts set
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

            // Build zero-copy CPI for writing to context
            let config = build_config_from_account_infos(&compressed_pda_infos, proof.0.is_some());
            let mut cpi_bytes = allocate_decompress_cpi_bytes(&config)?;

            let (mut cpi_struct, _remaining) =
                InstructionDataInvokeCpiWithAccountInfo::new_zero_copy(&mut cpi_bytes[8..], config)
                    .map_err(|_| ProgramError::InvalidAccountData)?;

            populate_zero_copy_cpi(
                &mut cpi_struct,
                cpi_signer.bump,
                &cpi_signer.program_id.into(),
                proof.0.as_ref(),
                &CompressedCpiContext::first(),
                true, // with_cpi_context
                &compressed_pda_infos,
            )?;

            // Execute CPI to write to context
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
