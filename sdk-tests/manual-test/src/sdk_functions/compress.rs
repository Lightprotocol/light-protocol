//! SDK generic compression functions.
//!
//! These functions are generic over account types and can be reused by the macro.

use anchor_lang::prelude::*;
use light_compressed_account::{
    address::derive_address,
    compressed_account::PackedMerkleContext,
    instruction_data::with_account_info::{CompressedAccountInfo, InAccountInfo, OutAccountInfo},
};
use light_compressible::rent::AccountRentState;
use light_hasher::{Hasher, Sha256};
use light_sdk::{
    cpi::v2::CpiAccounts,
    instruction::account_meta::{CompressedAccountMeta, CompressedAccountMetaTrait},
    LightDiscriminator,
};
use light_sdk_types::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress;
use solana_program::clock::Clock;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program_error::ProgramError;

use crate::traits::LightAccount;

const DEFAULT_DATA_HASH: [u8; 32] = [0u8; 32];

/// Prepare account for compression using local LightAccount trait.
///
/// This function is generic over any type that implements `LightAccount`.
/// It handles:
/// - Address derivation (v2 style using PDA as seed)
/// - Rent function validation
/// - Marking account as compressed
/// - Serializing updated state back to account
/// - Building CompressedAccountInfo for CPI
///
/// The macro-generated code calls this function for each account type.
pub fn prepare_account_for_compression<'info, A>(
    program_id: &Pubkey,
    account_info: &AccountInfo<'info>,
    account_data: &mut A,
    compressed_account_meta: &CompressedAccountMetaNoLamportsNoAddress,
    _cpi_accounts: &CpiAccounts<'_, 'info>,
    address_space: &[Pubkey],
) -> std::result::Result<CompressedAccountInfo, ProgramError>
where
    A: LightAccount + LightDiscriminator + Clone + AnchorSerialize,
{
    // v2 address derive using PDA as seed
    let derived_c_pda = derive_address(
        &account_info.key.to_bytes(),
        &address_space[0].to_bytes(),
        &program_id.to_bytes(),
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
    *compressed_data.compression_info_mut() =
        light_sdk::compressible::CompressionInfo::compressed();

    // Hash the data (discriminator NOT included per protocol convention)
    let data_bytes = compressed_data
        .try_to_vec()
        .map_err(|_| ProgramError::Custom(4))?;
    let mut output_data_hash = Sha256::hash(&data_bytes).map_err(|_| ProgramError::Custom(5))?;
    output_data_hash[0] = 0; // Zero first byte per protocol convention

    // Build input account info (empty compressed account from init)
    let tree_info = compressed_account_meta.tree_info;
    let input_account_info = InAccountInfo {
        data_hash: DEFAULT_DATA_HASH,
        lamports: 0,
        merkle_context: PackedMerkleContext {
            merkle_tree_pubkey_index: tree_info.merkle_tree_pubkey_index,
            queue_pubkey_index: tree_info.queue_pubkey_index,
            leaf_index: tree_info.leaf_index,
            prove_by_index: tree_info.prove_by_index,
        },
        root_index: compressed_account_meta.get_root_index().unwrap_or_default(),
        discriminator: [0u8; 8],
    };

    // Build output account info
    let output_account_info = OutAccountInfo {
        lamports: 0,
        output_merkle_tree_index: meta_with_address.output_state_tree_index,
        discriminator: A::LIGHT_DISCRIMINATOR,
        data: data_bytes,
        data_hash: output_data_hash,
    };

    Ok(CompressedAccountInfo {
        address: Some(meta_with_address.address),
        input: Some(input_account_info),
        output: Some(output_account_info),
    })
}

/// Parameters for compress_and_close instruction.
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CompressAndCloseParams {
    pub compressed_accounts: Vec<CompressedAccountMetaNoLamportsNoAddress>,
    pub system_accounts_offset: u8,
}
