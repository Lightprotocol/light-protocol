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
use light_sdk_types::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;
use solana_program_error::ProgramError;

use crate::{
    instruction::account_meta::{CompressedAccountMeta, CompressedAccountMetaTrait},
    interface::{program::compression::processor::CompressCtx, LightAccount},
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
        solana_msg::msg!("pda not yet compressible, skipping batch");
        ctx.has_non_compressible = true;
        return Ok(());
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
