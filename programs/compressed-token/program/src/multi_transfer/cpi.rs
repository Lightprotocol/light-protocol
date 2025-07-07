use arrayvec::ArrayVec;
use light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnlyConfig;
use pinocchio::pubkey::Pubkey;

use crate::{
    multi_transfer::{
        accounts::MultiTransferPackedAccounts,
        instruction_data::ZCompressedTokenInstructionDataMultiTransfer,
    },
    shared::cpi_bytes_size::{
        allocate_invoke_with_read_only_cpi_bytes, cpi_bytes_config, CpiConfigInput,
    },
};

/// Build CPI configuration from instruction data
pub fn allocate_cpi_bytes(
    inputs: &ZCompressedTokenInstructionDataMultiTransfer,
) -> (Vec<u8>, InstructionDataInvokeCpiWithReadOnlyConfig) {
    // Build CPI configuration based on delegate flags
    let mut input_delegate_flags = ArrayVec::new();
    for input_data in inputs.in_token_data.iter() {
        input_delegate_flags.push(input_data.with_delegate != 0);
    }

    let mut output_delegate_flags = ArrayVec::new();
    for output_data in inputs.out_token_data.iter() {
        // Check if output has delegate (delegate index != 0 means delegate is present)
        output_delegate_flags.push(output_data.delegate != 0);
    }

    // Add extra output account for change account if needed (no delegate, no token data)
    if inputs.with_lamports_change_account_merkle_tree_index != 0 {
        output_delegate_flags.push(false);
    }

    let config_input = CpiConfigInput {
        input_accounts: input_delegate_flags,
        output_accounts: output_delegate_flags,
        has_proof: inputs.proof.is_some(),
        compressed_mint: false,
        compressed_mint_with_freeze_authority: false,
    };
    let config = cpi_bytes_config(config_input);
    (allocate_invoke_with_read_only_cpi_bytes(&config), config)
}

/// Extract tree accounts from merkle contexts for CPI call
pub fn get_packed_cpi_accounts<'a>(
    inputs: &ZCompressedTokenInstructionDataMultiTransfer<'a>,
    packed_accounts: &MultiTransferPackedAccounts<'a>,
) -> Vec<&'a Pubkey> {
    //  don't pass any tree accounts if we write into the cpi context
    if inputs.cpi_context.is_some()
        && (inputs.cpi_context.unwrap().first_set_context
            || inputs.cpi_context.unwrap().set_context)
    {
        return vec![];
    }
    let mut tree_accounts = Vec::new();

    // Add input merkle trees and queues (skip non-tree accounts)
    for input_data in inputs.in_token_data.iter() {
        let merkle_tree_index = input_data.merkle_context.merkle_tree_pubkey_index;
        let queue_index = input_data.merkle_context.queue_pubkey_index;

        // Only add accounts that are actually trees/queues (typically higher indices)
        if let Some(merkle_tree_account) = packed_accounts.accounts.get(merkle_tree_index as usize)
        {
            tree_accounts.push(merkle_tree_account.key());
        }
        if let Some(queue_account) = packed_accounts.accounts.get(queue_index as usize) {
            tree_accounts.push(queue_account.key());
        }
    }

    // Add output merkle trees (skip non-tree accounts)
    for output_data in inputs.out_token_data.iter() {
        if let Some(tree_account) = packed_accounts
            .accounts
            .get(output_data.merkle_tree as usize)
        {
            tree_accounts.push(tree_account.key());
        }
    }

    tree_accounts
}
