use account_compression::emit_indexer_event;
use anchor_lang::{prelude::*, Bumps};

use crate::{
    errors::SystemProgramError,
    sdk::{
        accounts::InvokeAccounts,
        event::{MerkleTreeSequenceNumber, PublicTransactionEvent},
    },
    InstructionDataInvoke,
};

pub fn emit_state_transition_event<'a, 'b, 'c: 'info, 'info, A: InvokeAccounts<'info> + Bumps>(
    inputs: InstructionDataInvoke,
    ctx: &'a Context<'a, 'b, 'c, 'info, A>,
    mut input_compressed_account_hashes: Vec<[u8; 32]>,
    output_compressed_account_hashes: Vec<[u8; 32]>,
    output_leaf_indices: Vec<u32>,
    sequence_numbers: Vec<MerkleTreeSequenceNumber>,
) -> Result<()> {
    msg!(
        "input_compressed_account_hashes: {:?}",
        input_compressed_account_hashes
    );
    let mut num_removed_values = 0;
    // Do not include read-only accounts in the event.
    for (i, account) in inputs
        .input_compressed_accounts_with_merkle_context
        .iter()
        .enumerate()
    {
        if account.read_only {
            input_compressed_account_hashes.remove(i - num_removed_values);
            num_removed_values += 1;
        }
    }
    // Note: message is unimplemented
    // (if we compute the tx hash in indexer we don't need to modify the event.)
    let event = PublicTransactionEvent {
        input_compressed_account_hashes,
        output_compressed_account_hashes,
        output_compressed_accounts: inputs.output_compressed_accounts,
        output_leaf_indices,
        sequence_numbers,
        relay_fee: inputs.relay_fee,
        pubkey_array: ctx.remaining_accounts.iter().map(|x| x.key()).collect(),
        compress_or_decompress_lamports: inputs.compress_or_decompress_lamports,
        message: None,
        is_compress: inputs.is_compress,
    };

    // 10240 = 10 * 1024 the max instruction data of a cpi.
    let data_capacity = 10240;
    let mut data = Vec::with_capacity(data_capacity);
    event.man_serialize(&mut data)?;

    if data_capacity != data.capacity() {
        msg!(
            "Event serialization exceeded capacity. Used {}, allocated {}.",
            data.capacity(),
            data_capacity
        );
        return err!(SystemProgramError::InvalidCapacity);
    }

    emit_indexer_event(data, ctx.accounts.get_noop_program())
}
