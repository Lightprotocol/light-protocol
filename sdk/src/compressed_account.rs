use anchor_lang::prelude::{AccountInfo, Key, ProgramError, Pubkey, Result};
use borsh::BorshSerialize;
use light_hasher::{DataHasher, Discriminator, Poseidon};
use light_system_program::{
    sdk::{
        address::derive_address,
        compressed_account::{
            CompressedAccount, CompressedAccountData, CompressedAccountWithMerkleContext,
            PackedCompressedAccountWithMerkleContext, PackedMerkleContext,
        },
    },
    NewAddressParamsPacked, OutputCompressedAccountWithPackedContext,
};

use crate::merkle_context::{
    pack_merkle_context, PackedAddressMerkleContext, PackedMerkleOutputContext, RemainingAccounts,
};

pub fn serialize_and_hash_account<T>(
    account: &T,
    address_seed: &[u8; 32],
    program_id: &Pubkey,
    address_merkle_context: &PackedAddressMerkleContext,
    remaining_accounts: &[AccountInfo],
) -> Result<CompressedAccount>
where
    T: BorshSerialize + DataHasher + Discriminator,
{
    let data = account.try_to_vec()?;
    let data_hash = account.hash::<Poseidon>().map_err(ProgramError::from)?;
    let compressed_account_data = CompressedAccountData {
        discriminator: T::discriminator(),
        data,
        data_hash,
    };

    let address = derive_address(
        &remaining_accounts[address_merkle_context.address_merkle_tree_pubkey_index as usize].key(),
        address_seed,
    )
    .map_err(|_| ProgramError::InvalidArgument)?;

    let compressed_account = CompressedAccount {
        owner: *program_id,
        lamports: 0,
        address: Some(address),
        data: Some(compressed_account_data),
    };

    Ok(compressed_account)
}

pub fn new_compressed_account<T>(
    account: &T,
    address_seed: &[u8; 32],
    program_id: &Pubkey,
    merkle_output_context: &PackedMerkleOutputContext,
    address_merkle_context: &PackedAddressMerkleContext,
    address_merkle_tree_root_index: u16,
    remaining_accounts: &[AccountInfo],
) -> Result<(
    OutputCompressedAccountWithPackedContext,
    NewAddressParamsPacked,
)>
where
    T: BorshSerialize + DataHasher + Discriminator,
{
    let compressed_account = serialize_and_hash_account(
        account,
        address_seed,
        program_id,
        address_merkle_context,
        remaining_accounts,
    )?;

    let compressed_account = OutputCompressedAccountWithPackedContext {
        compressed_account,
        merkle_tree_index: merkle_output_context.merkle_tree_pubkey_index,
    };

    let new_address_params = NewAddressParamsPacked {
        seed: *address_seed,
        address_merkle_tree_account_index: address_merkle_context.address_merkle_tree_pubkey_index,
        address_queue_account_index: address_merkle_context.address_queue_pubkey_index,
        address_merkle_tree_root_index,
    };

    Ok((compressed_account, new_address_params))
}

pub fn input_compressed_account<T>(
    account: &T,
    address_seed: &[u8; 32],
    program_id: &Pubkey,
    merkle_context: &PackedMerkleContext,
    merkle_tree_root_index: u16,
    address_merkle_context: &PackedAddressMerkleContext,
    remaining_accounts: &[AccountInfo],
) -> Result<PackedCompressedAccountWithMerkleContext>
where
    T: BorshSerialize + DataHasher + Discriminator,
{
    let compressed_account = serialize_and_hash_account(
        account,
        address_seed,
        program_id,
        address_merkle_context,
        remaining_accounts,
    )?;

    Ok(PackedCompressedAccountWithMerkleContext {
        compressed_account,
        merkle_context: *merkle_context,
        root_index: merkle_tree_root_index,
        read_only: false,
    })
}

pub fn output_compressed_account<T>(
    account: &T,
    address_seed: &[u8; 32],
    program_id: &Pubkey,
    merkle_context: &PackedMerkleContext,
    address_merkle_context: &PackedAddressMerkleContext,
    remaining_accounts: &[AccountInfo],
) -> Result<OutputCompressedAccountWithPackedContext>
where
    T: BorshSerialize + DataHasher + Discriminator,
{
    let compressed_account = serialize_and_hash_account(
        account,
        address_seed,
        program_id,
        address_merkle_context,
        remaining_accounts,
    )?;

    Ok(OutputCompressedAccountWithPackedContext {
        compressed_account,
        merkle_tree_index: merkle_context.merkle_tree_pubkey_index,
    })
}

pub fn pack_compressed_accounts(
    compressed_accounts: &[CompressedAccountWithMerkleContext],
    root_indices: &[u16],
    remaining_accounts: &mut RemainingAccounts,
) -> Vec<PackedCompressedAccountWithMerkleContext> {
    compressed_accounts
        .iter()
        .zip(root_indices.iter())
        .map(|(x, root_index)| PackedCompressedAccountWithMerkleContext {
            compressed_account: x.compressed_account.clone(),
            merkle_context: pack_merkle_context(x.merkle_context, remaining_accounts),
            root_index: *root_index,
            read_only: false,
        })
        .collect::<Vec<_>>()
}

pub fn pack_compressed_account(
    compressed_account: CompressedAccountWithMerkleContext,
    root_index: u16,
    remaining_accounts: &mut RemainingAccounts,
) -> PackedCompressedAccountWithMerkleContext {
    pack_compressed_accounts(&[compressed_account], &[root_index], remaining_accounts)[0].clone()
}
