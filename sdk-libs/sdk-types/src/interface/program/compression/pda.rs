//! SDK generic compression functions.
//!
//! These functions are generic over account types and can be reused by the macro.
//! The compress flow uses a dispatch callback pattern (same as decompress).

use light_account_checks::AccountInfoTrait;
use light_compressed_account::{
    address::derive_address,
    compressed_account::PackedMerkleContext,
    instruction_data::with_account_info::{CompressedAccountInfo, InAccountInfo, OutAccountInfo},
};
use light_compressible::{rent::AccountRentState, DECOMPRESSED_PDA_DISCRIMINATOR};
use light_hasher::{sha256::Sha256BE, Hasher, Sha256};

use crate::{
    error::LightSdkTypesError,
    instruction::account_meta::{
        CompressedAccountMeta, CompressedAccountMetaNoLamportsNoAddress, CompressedAccountMetaTrait,
    },
    interface::{
        account::compression_info::HasCompressionInfo, program::compression::processor::CompressCtx,
    },
    LightDiscriminator,
};

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
pub fn prepare_account_for_compression<AI, A>(
    account_info: &AI,
    account_data: &mut A,
    compressed_account_meta: &CompressedAccountMetaNoLamportsNoAddress,
    pda_index: usize,
    ctx: &mut CompressCtx<'_, AI>,
) -> Result<(), LightSdkTypesError>
where
    AI: AccountInfoTrait,
    A: HasCompressionInfo + LightDiscriminator + Clone + borsh::BorshSerialize,
{
    // v2 address derive using PDA as seed
    let account_key = account_info.key();
    let derived_c_pda = derive_address(
        &account_key,
        &ctx.light_config.address_space[0],
        ctx.program_id,
    );

    let meta_with_address = CompressedAccountMeta {
        tree_info: compressed_account_meta.tree_info,
        address: derived_c_pda,
        output_state_tree_index: compressed_account_meta.output_state_tree_index,
    };

    let current_slot = AI::get_current_slot().map_err(LightSdkTypesError::AccountError)?;
    let bytes = account_info.data_len() as u64;
    let current_lamports = account_info.lamports();
    let rent_exemption_lamports =
        AI::get_min_rent_balance(bytes as usize).map_err(LightSdkTypesError::AccountError)?;

    let ci = account_data.compression_info()?;
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
        ctx.has_non_compressible = true;
        return Ok(());
    }

    // Mark as compressed
    account_data.compression_info_mut()?.set_compressed();

    // Serialize updated account data back (includes discriminator prefix)
    {
        let mut data = account_info
            .try_borrow_mut_data()
            .map_err(LightSdkTypesError::AccountError)?;
        // Write discriminator first (variable length: LIGHT_DISCRIMINATOR_SLICE may be < 8 bytes)
        let disc_slice = A::LIGHT_DISCRIMINATOR_SLICE;
        let disc_len = disc_slice.len();
        data[..disc_len].copy_from_slice(disc_slice);
        // Write serialized account data after discriminator
        let writer = &mut &mut data[disc_len..];
        account_data
            .serialize(writer)
            .map_err(|_| LightSdkTypesError::Borsh)?;
    }

    // Create compressed account with canonical compressed CompressionInfo for hashing
    let mut compressed_data = account_data.clone();
    *compressed_data.compression_info_mut()? =
        crate::interface::account::compression_info::CompressionInfo::compressed();

    // Hash the data (discriminator NOT included per protocol convention)
    let data_bytes = borsh::to_vec(&compressed_data).map_err(|_| LightSdkTypesError::Borsh)?;
    let mut output_data_hash = Sha256::hash(&data_bytes).map_err(LightSdkTypesError::Hasher)?;
    output_data_hash[0] = 0; // Zero first byte per protocol convention

    // Build input account info (placeholder compressed account from init)
    // The init created a placeholder with DECOMPRESSED_PDA_DISCRIMINATOR and PDA pubkey as data
    let tree_info = compressed_account_meta.tree_info;
    let input_data_hash = Sha256BE::hash(&account_key).map_err(LightSdkTypesError::Hasher)?;
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
