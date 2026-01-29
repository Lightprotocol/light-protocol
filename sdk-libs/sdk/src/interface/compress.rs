//! SDK generic compression functions.
//!
//! These functions are generic over account types and can be reused by the macro.
//! The compress flow uses a dispatch callback pattern (same as decompress).

use anchor_lang::{
    prelude::*,
    solana_program::{clock::Clock, rent::Rent, sysvar::Sysvar},
};
use light_compressed_account::{
    address::derive_address,
    compressed_account::PackedMerkleContext,
    instruction_data::with_account_info::{CompressedAccountInfo, InAccountInfo, OutAccountInfo},
};
use light_compressible::{rent::AccountRentState, DECOMPRESSED_PDA_DISCRIMINATOR};
use light_hasher::{sha256::Sha256BE, Hasher, Sha256};
use light_sdk_types::{
    instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress, CpiSigner,
};
use solana_program_error::ProgramError;

use super::traits::LightAccount;
use crate::{
    cpi::{
        v2::{CpiAccounts, LightSystemProgramCpi},
        InvokeLightSystemProgram, LightCpiInstruction,
    },
    instruction::{
        account_meta::{CompressedAccountMeta, CompressedAccountMetaTrait},
        ValidityProof,
    },
    interface::LightConfig,
    LightDiscriminator,
};

/// Parameters for compress_and_close instruction.
/// Matches SDK's SaveAccountsData field order for compatibility.
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CompressAndCloseParams {
    /// Validity proof for compressed account verification
    pub proof: ValidityProof,
    /// Accounts to compress (meta only - data read from PDA)
    pub compressed_accounts: Vec<CompressedAccountMetaNoLamportsNoAddress>,
    /// Offset into remaining_accounts where Light system accounts begin
    pub system_accounts_offset: u8,
}

/// Context struct holding all data needed for compression.
/// Contains internal vec for collecting CompressedAccountInfo results.
pub struct CompressCtx<'a, 'info> {
    pub program_id: &'a Pubkey,
    pub cpi_accounts: &'a CpiAccounts<'a, 'info>,
    pub remaining_accounts: &'a [AccountInfo<'info>],
    pub rent_sponsor: &'a AccountInfo<'info>,
    pub light_config: &'a LightConfig,
    /// Internal vec - dispatch functions push results here
    pub compressed_account_infos: Vec<CompressedAccountInfo>,
    /// Track which PDA indices to close
    pub pda_indices_to_close: Vec<usize>,
}

/// Callback type for discriminator-based dispatch.
/// MACRO-GENERATED: Just a match statement routing to prepare_account_for_compression.
/// Takes &mut CompressCtx and pushes CompressedAccountInfo into ctx.compressed_account_infos.
///
/// The dispatch function is responsible for:
/// 1. Reading the discriminator from the account data
/// 2. Deserializing the account based on discriminator
/// 3. Calling prepare_account_for_compression with the deserialized data
pub type CompressDispatchFn<'info> = fn(
    account_info: &AccountInfo<'info>,
    compressed_account_meta: &CompressedAccountMetaNoLamportsNoAddress,
    index: usize,
    ctx: &mut CompressCtx<'_, 'info>,
) -> std::result::Result<(), ProgramError>;

/// Remaining accounts layout:
/// [0]: fee_payer (Signer, mut)
/// [1]: config (LightConfig PDA)
/// [2]: rent_sponsor (mut)
/// [3]: compression_authority (Signer)
/// [system_accounts_offset..]: Light system accounts for CPI
/// [remaining_accounts.len() - num_pda_accounts..]: PDA accounts to compress
///
/// Runtime processor - handles all the plumbing, delegates dispatch to callback.
///
/// **Takes raw instruction data** and deserializes internally - minimizes macro code.
/// **Uses only remaining_accounts** - no Context struct needed.
pub fn process_compress_pda_accounts_idempotent<'info>(
    remaining_accounts: &[AccountInfo<'info>],
    instruction_data: &[u8],
    dispatch_fn: CompressDispatchFn<'info>,
    cpi_signer: CpiSigner,
    program_id: &Pubkey,
) -> std::result::Result<(), ProgramError> {
    // Deserialize params internally
    let params = CompressAndCloseParams::try_from_slice(instruction_data).map_err(|e| {
        solana_msg::msg!("compress: params deser failed: {:?}", e);
        ProgramError::InvalidInstructionData
    })?;

    // Extract and validate accounts using shared validation
    let validated_ctx =
        crate::interface::validation::validate_compress_accounts(remaining_accounts, program_id)?;
    let fee_payer = &validated_ctx.fee_payer;
    let rent_sponsor = &validated_ctx.rent_sponsor;
    let light_config = validated_ctx.light_config;

    let (_, system_accounts) = crate::interface::validation::split_at_system_accounts_offset(
        remaining_accounts,
        params.system_accounts_offset,
    )?;

    let cpi_accounts = CpiAccounts::new(fee_payer, system_accounts, cpi_signer);

    // Build context struct with all needed data (includes internal vec)
    let mut compress_ctx = CompressCtx {
        program_id,
        cpi_accounts: &cpi_accounts,
        remaining_accounts,
        rent_sponsor,
        light_config: &light_config,
        compressed_account_infos: Vec::with_capacity(params.compressed_accounts.len()),
        pda_indices_to_close: Vec::with_capacity(params.compressed_accounts.len()),
    };

    // PDA accounts at end of remaining_accounts
    let pda_accounts = crate::interface::validation::extract_tail_accounts(
        remaining_accounts,
        params.compressed_accounts.len(),
    )?;

    for (i, account_data) in params.compressed_accounts.iter().enumerate() {
        let pda_account = &pda_accounts[i];

        // Skip empty accounts or accounts not owned by this program
        if crate::interface::validation::should_skip_compression(pda_account, program_id) {
            continue;
        }

        // Delegate to dispatch callback (macro-generated match)
        dispatch_fn(pda_account, account_data, i, &mut compress_ctx)?;
    }

    // CPI to Light System Program
    if !compress_ctx.compressed_account_infos.is_empty() {
        LightSystemProgramCpi::new_cpi(cpi_signer, params.proof)
            .with_account_infos(&compress_ctx.compressed_account_infos)
            .invoke(cpi_accounts.clone())
            .map_err(|e| {
                solana_msg::msg!("compress: CPI failed: {:?}", e);
                ProgramError::Custom(200)
            })?;

        // Close the PDA accounts
        for idx in compress_ctx.pda_indices_to_close {
            let mut info = pda_accounts[idx].clone();
            crate::interface::close::close(&mut info, rent_sponsor).map_err(ProgramError::from)?;
        }
    }

    Ok(())
}

/// Generic prepare_account_for_compression.
///
/// Called by the dispatch function after it has:
/// 1. Read the discriminator from the account
/// 2. Deserialized the account data
///
/// Pushes CompressedAccountInfo into ctx.compressed_account_infos.
/// Pushes pda_index into ctx.pda_indices_to_close.
///
/// # Arguments
/// * `account_info` - The PDA account to compress
/// * `account_data` - Deserialized account data (will be modified to mark as compressed)
/// * `compressed_account_meta` - Compressed account metadata
/// * `pda_index` - Index of the PDA in the accounts array (for tracking closes)
/// * `ctx` - Mutable context ref - pushes results here
pub fn prepare_account_for_compression<'info, A>(
    account_info: &AccountInfo<'info>,
    account_data: &mut A,
    compressed_account_meta: &CompressedAccountMetaNoLamportsNoAddress,
    pda_index: usize,
    ctx: &mut CompressCtx<'_, 'info>,
) -> std::result::Result<(), ProgramError>
where
    A: LightAccount + LightDiscriminator + Clone + AnchorSerialize,
{
    // v2 address derive using PDA as seed
    let derived_c_pda = derive_address(
        &account_info.key.to_bytes(),
        &ctx.light_config.address_space[0].to_bytes(),
        &ctx.program_id.to_bytes(),
    );

    let meta_with_address = CompressedAccountMeta {
        tree_info: compressed_account_meta.tree_info,
        address: derived_c_pda,
        output_state_tree_index: compressed_account_meta.output_state_tree_index,
    };

    let current_slot = Clock::get()?.slot;
    let bytes = account_info.data_len() as u64;
    let current_lamports = account_info.lamports();
    let rent_exemption_lamports = Rent::get()
        .map_err(|_| ProgramError::Custom(0))?
        .minimum_balance(bytes as usize);

    let ci = account_data.compression_info();
    let last_claimed_slot = ci.last_claimed_slot();
    let rent_cfg = ci.rent_config;

    let state = AccountRentState {
        num_bytes: bytes,
        current_slot,
        current_lamports,
        last_claimed_slot,
    };

    // Check if account is compressible by rent function
    if state
        .is_compressible(&rent_cfg, rent_exemption_lamports)
        .is_none()
    {
        return Err(ProgramError::Custom(1)); // Not compressible
    }

    // Mark as compressed using LightAccount trait
    account_data.compression_info_mut().set_compressed();

    // Serialize updated account data back (includes 8-byte discriminator)
    {
        let mut data = account_info
            .try_borrow_mut_data()
            .map_err(|_| ProgramError::Custom(2))?;
        // Write discriminator first
        data[..8].copy_from_slice(&A::LIGHT_DISCRIMINATOR);
        // Write serialized account data after discriminator
        let writer = &mut &mut data[8..];
        account_data
            .serialize(writer)
            .map_err(|_| ProgramError::Custom(3))?;
    }

    // Create compressed account with canonical compressed CompressionInfo for hashing
    let mut compressed_data = account_data.clone();
    *compressed_data.compression_info_mut() = crate::compressible::CompressionInfo::compressed();

    // Hash the data (discriminator NOT included per protocol convention)
    let data_bytes = compressed_data
        .try_to_vec()
        .map_err(|_| ProgramError::Custom(4))?;
    let mut output_data_hash = Sha256::hash(&data_bytes).map_err(|_| ProgramError::Custom(5))?;
    output_data_hash[0] = 0; // Zero first byte per protocol convention

    // Build input account info (placeholder compressed account from init)
    // The init created a placeholder with DECOMPRESSED_PDA_DISCRIMINATOR and PDA pubkey as data
    let tree_info = compressed_account_meta.tree_info;
    let input_data_hash =
        Sha256BE::hash(&account_info.key.to_bytes()).map_err(|_| ProgramError::Custom(6))?;
    let input_account_info = InAccountInfo {
        data_hash: input_data_hash,
        lamports: 0,
        merkle_context: PackedMerkleContext {
            merkle_tree_pubkey_index: tree_info.merkle_tree_pubkey_index,
            queue_pubkey_index: tree_info.queue_pubkey_index,
            leaf_index: tree_info.leaf_index,
            prove_by_index: tree_info.prove_by_index,
        },
        root_index: compressed_account_meta.get_root_index().unwrap_or_default(),
        discriminator: DECOMPRESSED_PDA_DISCRIMINATOR,
    };

    // Build output account info
    let output_account_info = OutAccountInfo {
        lamports: 0,
        output_merkle_tree_index: meta_with_address.output_state_tree_index,
        discriminator: A::LIGHT_DISCRIMINATOR,
        data: data_bytes,
        data_hash: output_data_hash,
    };

    // Push to ctx's internal vecs
    ctx.compressed_account_infos.push(CompressedAccountInfo {
        address: Some(meta_with_address.address),
        input: Some(input_account_info),
        output: Some(output_account_info),
    });
    ctx.pda_indices_to_close.push(pda_index);

    Ok(())
}
