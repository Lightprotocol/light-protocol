use account_compression::IndexedArrayAccount;
use anchor_lang::prelude::*;

use crate::{
    instructions::{InstructionDataTransfer, TransferInstruction},
    nullify_state::insert_nullifiers_cpi,
};

pub fn insert_addresses_into_address_merkle_tree_queue<'a, 'b, 'c: 'info, 'info>(
    inputs: &'a InstructionDataTransfer,
    ctx: &'a Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    addresses: &'a [[u8; 32]],
) -> anchor_lang::Result<()> {
    let address_merkle_tree_pubkeys = inputs
        .address_merkle_tree_account_indices
        .iter()
        .map(|index| ctx.remaining_accounts[*index as usize].key())
        .collect::<Vec<Pubkey>>();
    let mut indexed_array_account_infos = Vec::<AccountInfo>::new();
    for index in inputs.address_queue_account_indices.iter() {
        indexed_array_account_infos.push(ctx.remaining_accounts[*index as usize].clone());
        let unpacked_queue_account = AccountLoader::<IndexedArrayAccount>::try_from(
            &ctx.remaining_accounts[*index as usize],
        )
        .unwrap();
        let array_account = unpacked_queue_account.load()?;
        let account_is_associated_with_address_merkle_tree = address_merkle_tree_pubkeys
            .iter()
            .any(|x| *x == array_account.associated_merkle_tree);
        if !account_is_associated_with_address_merkle_tree {
            msg!(
                "Address queue account {:?} is not associated with any address Merkle tree. Provided address Merkle trees {:?}",
                ctx.remaining_accounts[*index as usize].key(), address_merkle_tree_pubkeys);
            return Err(crate::ErrorCode::InvalidAddressQueue.into());
        }
    }
    insert_nullifiers_cpi(
        ctx.program_id,
        &ctx.accounts.account_compression_program,
        &ctx.accounts.psp_account_compression_authority,
        &ctx.accounts.registered_program_pda.to_account_info(),
        indexed_array_account_infos,
        addresses.to_vec(),
    )
}
