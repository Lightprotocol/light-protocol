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
    input_compressed_account_hashes: Vec<[u8; 32]>,
    output_compressed_account_hashes: Vec<[u8; 32]>,
    output_leaf_indices: Vec<u32>,
    sequence_numbers: Vec<MerkleTreeSequenceNumber>,
) -> Result<()> {
    // TODO: add tx hashchain of inputs, outputs, message, compress and decompress
    //       consider whether it should only be created if inputs exist.
    // TODO: extend event by the batch inputs and outputs are inserted in, None means v0 insert.
    // Note: message is unimplemented
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
