//! SDK generic decompression functions.
//!
//! These functions are generic over account types and can be reused by the macro.
//! The decompress flow creates PDAs from compressed state (needs validity proof, packed data, seeds).

use anchor_lang::prelude::*;
use light_compressed_account::{
    compressed_account::PackedMerkleContext,
    instruction_data::with_account_info::{CompressedAccountInfo, InAccountInfo, OutAccountInfo},
};
use light_hasher::{Hasher, Sha256};
use light_sdk::{
    cpi::{
        v2::{CpiAccounts, LightSystemProgramCpi},
        InvokeLightSystemProgram, LightCpiInstruction,
    },
    interface::{create_pda_account, LightConfig},
    light_account_checks::{account_iterator::AccountIterator, checks::check_data_is_zeroed},
    proof::borsh_compat::ValidityProof,
    LightDiscriminator,
};
use light_sdk_types::instruction::account_meta::CompressedAccountMeta;
use light_sdk_types::CpiSigner;
use solana_program::clock::Clock;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program_error::ProgramError;

use crate::traits::LightAccount;

/// Per-account data for decompression.
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct DecompressAccountData {
    /// Account discriminator to determine type
    pub discriminator: [u8; 8],
    /// Packed variant data (seeds + data, Pubkeys converted to indices)
    /// Includes bump in the packed seeds
    pub packed_variant: Vec<u8>,
    /// Compressed account metadata (tree info, address, output tree index)
    pub meta: CompressedAccountMeta,
}

/// Parameters for decompress_idempotent instruction.
/// Fully generic - discriminator + raw packed bytes. No program-specific params needed.
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct DecompressIdempotentParams {
    /// Accounts to decompress
    pub accounts: Vec<DecompressAccountData>,
    /// Validity proof for compressed account verification
    pub proof: ValidityProof,
    /// Offset into remaining_accounts where Light system accounts begin
    pub system_accounts_offset: u8,
}

/// Context struct holding all data needed for decompression.
/// Contains internal vec for collecting CompressedAccountInfo results.
pub struct DecompressCtx<'a, 'info> {
    pub program_id: &'a Pubkey,
    pub cpi_accounts: &'a CpiAccounts<'a, 'info>,
    pub remaining_accounts: &'a [AccountInfo<'info>],
    pub rent_sponsor: &'a AccountInfo<'info>,
    pub light_config: &'a LightConfig,
    pub rent: &'a Rent,
    pub current_slot: u64,
    /// Internal vec - dispatch functions push results here
    pub compressed_account_infos: Vec<CompressedAccountInfo>,
}

/// Callback type for discriminator-based dispatch.
/// MACRO-GENERATED: Just a match statement routing to prepare_account_for_decompression.
/// Takes &mut DecompressCtx and pushes CompressedAccountInfo into ctx.compressed_account_infos.
///
/// The dispatch function is responsible for:
/// 1. Deserializing the packed variant (type-specific)
/// 2. Extracting signer seeds via variant.seed_refs_with_bump()
/// 3. Calling prepare_account_for_decompression with the seeds
pub type DecompressDispatchFn<'info> = fn(
    discriminator: [u8; 8],
    packed_variant: &[u8],
    pda_account: &AccountInfo<'info>,
    meta: &CompressedAccountMeta,
    ctx: &mut DecompressCtx<'_, 'info>,
) -> std::result::Result<(), ProgramError>;

/// Remaining accounts layout:
/// [0]: fee_payer (Signer, mut)
/// [1]: config (LightConfig PDA)
/// [2]: rent_sponsor (mut)
/// [system_accounts_offset..]: Light system accounts for CPI
/// [remaining_accounts.len() - num_pda_accounts..]: PDA accounts to decompress

/// Runtime processor - handles all the plumbing, delegates dispatch to callback.
///
/// **Takes raw instruction data** and deserializes internally - minimizes macro code.
/// **Uses only remaining_accounts** - no Context struct needed.
pub fn process_decompress_pda_accounts_idempotent<'info>(
    remaining_accounts: &[AccountInfo<'info>],
    instruction_data: &[u8],
    dispatch_fn: DecompressDispatchFn<'info>,
    cpi_signer: CpiSigner,
    program_id: &Pubkey,
) -> std::result::Result<(), ProgramError> {
    // Deserialize params internally
    let params = DecompressIdempotentParams::try_from_slice(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    // Extract and validate accounts using AccountIterator
    let mut account_iter = AccountIterator::new(remaining_accounts);
    let fee_payer = account_iter
        .next_signer_mut("fee_payer")
        .map_err(ProgramError::from)?;
    let config = account_iter
        .next_non_mut("config")
        .map_err(ProgramError::from)?;
    let rent_sponsor = account_iter
        .next_mut("rent_sponsor")
        .map_err(ProgramError::from)?;

    // Load and validate config
    let light_config = LightConfig::load_checked(config, program_id)
        .map_err(|_| ProgramError::InvalidAccountData)?;
    let rent = Rent::get()?;
    let current_slot = Clock::get()?.slot;

    let system_accounts_offset_usize = params.system_accounts_offset as usize;
    if system_accounts_offset_usize > remaining_accounts.len() {
        return Err(ProgramError::InvalidInstructionData);
    }

    let cpi_accounts = CpiAccounts::new(
        fee_payer,
        &remaining_accounts[system_accounts_offset_usize..],
        cpi_signer,
    );

    // Build context struct with all needed data (includes internal vec)
    let mut decompress_ctx = DecompressCtx {
        program_id,
        cpi_accounts: &cpi_accounts,
        remaining_accounts,
        rent_sponsor,
        light_config: &light_config,
        rent: &rent,
        current_slot,
        compressed_account_infos: Vec::new(),
    };

    // PDA accounts at end of remaining_accounts
    let pda_accounts_start = remaining_accounts
        .len()
        .checked_sub(params.accounts.len())
        .ok_or(ProgramError::InvalidInstructionData)?;
    let pda_accounts = &remaining_accounts[pda_accounts_start..];

    for (i, account_data) in params.accounts.iter().enumerate() {
        let pda_account = &pda_accounts[i];

        // Delegate to dispatch callback (macro-generated match)
        // The dispatch function:
        // 1. Deserializes the packed variant (type-specific)
        // 2. Extracts signer seeds via variant.seed_refs_with_bump()
        // 3. Calls prepare_account_for_decompression with the seeds
        dispatch_fn(
            account_data.discriminator,
            &account_data.packed_variant,
            pda_account,
            &account_data.meta,
            &mut decompress_ctx,
        )?;
    }

    // CPI to Light System Program with proof
    if !decompress_ctx.compressed_account_infos.is_empty() {
        LightSystemProgramCpi::new_cpi(cpi_signer, params.proof.into())
            .with_account_infos(&decompress_ctx.compressed_account_infos)
            .invoke(cpi_accounts.clone())
            .map_err(|_| ProgramError::Custom(200))?;
    }

    Ok(())
}
/// Generic prepare_account_for_decompression.
///
/// Called by the dispatch function after it has:
/// 1. Deserialized the packed variant
/// 2. Extracted signer seeds via variant.seed_refs_with_bump()
/// 3. Unpacked the data
///
/// Pushes CompressedAccountInfo into ctx.compressed_account_infos.
///
/// # Arguments
/// * `account_data` - Unpacked account data (already has CompressionInfo::compressed())
/// * `pda_account` - The PDA account to create/initialize
/// * `compressed_meta` - Compressed account metadata
/// * `signer_seeds` - Seeds for PDA signing (from variant.seed_refs_with_bump)
/// * `ctx` - Mutable context ref - pushes result here
pub fn prepare_account_for_decompression<'info, A>(
    account_data: A,
    pda_account: &AccountInfo<'info>,
    compressed_meta: &CompressedAccountMeta,
    signer_seeds: &[&[u8]],
    ctx: &mut DecompressCtx<'_, 'info>,
) -> std::result::Result<(), ProgramError>
where
    A: LightAccount + LightDiscriminator + Clone + AnchorSerialize + AnchorDeserialize,
{
    // 1. Idempotency check - if PDA already has data (non-zero discriminator), skip
    if !pda_account.data_is_empty() {
        let data = pda_account.try_borrow_data()?;
        if check_data_is_zeroed::<8>(&data).is_err() {
            // Already initialized - skip
            return Ok(());
        }
    }

    // 2. Hash with canonical CompressionInfo::compressed() for input verification
    // The unpacked data already has compression_info set to compressed()
    let data_bytes = account_data
        .try_to_vec()
        .map_err(|_| ProgramError::InvalidAccountData)?;
    let data_len = data_bytes.len();
    let mut input_data_hash = Sha256::hash(&data_bytes).map_err(|_| ProgramError::Custom(100))?;
    input_data_hash[0] = 0; // Zero first byte per protocol convention

    // 3. Calculate space and create PDA
    let discriminator_len = 8;
    let space = discriminator_len + data_len.max(A::INIT_SPACE);
    let rent_minimum = ctx.rent.minimum_balance(space);

    let system_program = ctx
        .cpi_accounts
        .system_program()
        .map_err(|_| ProgramError::InvalidAccountData)?;

    create_pda_account(
        ctx.rent_sponsor,
        pda_account,
        rent_minimum,
        space as u64,
        ctx.program_id,
        signer_seeds,
        system_program,
    )?;

    // 4. Write discriminator + data to PDA
    let mut pda_data = pda_account.try_borrow_mut_data()?;
    pda_data[..8].copy_from_slice(&A::LIGHT_DISCRIMINATOR);

    // 5. Set decompressed state and serialize
    let mut decompressed = account_data;
    decompressed.set_decompressed(ctx.light_config, ctx.current_slot);
    let writer = &mut &mut pda_data[8..];
    decompressed
        .serialize(writer)
        .map_err(|_| ProgramError::InvalidAccountData)?;

    // 6. Build CompressedAccountInfo for CPI
    let tree_info = compressed_meta.tree_info;
    let input = InAccountInfo {
        data_hash: input_data_hash,
        lamports: 0,
        merkle_context: PackedMerkleContext {
            merkle_tree_pubkey_index: tree_info.merkle_tree_pubkey_index,
            queue_pubkey_index: tree_info.queue_pubkey_index,
            leaf_index: tree_info.leaf_index,
            prove_by_index: tree_info.prove_by_index,
        },
        root_index: tree_info.root_index,
        discriminator: A::LIGHT_DISCRIMINATOR,
    };

    // Output is empty (nullifying the compressed account)
    let output = OutAccountInfo {
        lamports: 0,
        output_merkle_tree_index: compressed_meta.output_state_tree_index,
        discriminator: [0u8; 8],
        data: Vec::new(),
        data_hash: [0u8; 32],
    };

    // 7. Push to ctx's internal vec
    ctx.compressed_account_infos.push(CompressedAccountInfo {
        address: Some(compressed_meta.address),
        input: Some(input),
        output: Some(output),
    });

    Ok(())
}
