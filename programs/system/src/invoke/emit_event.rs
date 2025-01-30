use account_compression::emit_indexer_event;
use anchor_lang::{prelude::*, Bumps};

use crate::{
    errors::SystemProgramError,
    instruction_data::ZInstructionDataInvoke,
    sdk::{
        accounts::InvokeAccounts,
        compressed_account::{CompressedAccount, CompressedAccountData},
        event::{MerkleTreeSequenceNumber, PublicTransactionEvent},
    },
};

pub fn emit_state_transition_event<'a, 'b, 'c: 'info, 'info, A: InvokeAccounts<'info> + Bumps>(
    inputs: ZInstructionDataInvoke<'a>,
    ctx: &'a Context<'a, 'b, 'c, 'info, A>,
    input_compressed_account_hashes: Vec<[u8; 32]>,
    output_compressed_account_hashes: Vec<[u8; 32]>,
    output_leaf_indices: Vec<u32>,
    sequence_numbers: Vec<MerkleTreeSequenceNumber>,
) -> Result<()> {
    // Note: message is unimplemented
    // (if we compute the tx hash in indexer we don't need to modify the event.)
    let event = PublicTransactionEvent {
        input_compressed_account_hashes,
        output_compressed_account_hashes,
        output_compressed_accounts: inputs
            .output_compressed_accounts
            .iter()
            .map(|x| {
                let data = if let Some(data) = x.compressed_account.data.as_ref() {
                    Some(CompressedAccountData {
                        discriminator: *data.discriminator,
                        data: data.data.to_vec(),
                        data_hash: *data.data_hash,
                    })
                } else {
                    None
                };
                super::OutputCompressedAccountWithPackedContext {
                    compressed_account: CompressedAccount {
                        owner: x.compressed_account.owner.into(),
                        lamports: u64::from(x.compressed_account.lamports),
                        address: x.compressed_account.address.map(|x| *x),
                        data,
                    },
                    merkle_tree_index: x.merkle_tree_index,
                }
            })
            .collect(),
        output_leaf_indices,
        sequence_numbers,
        relay_fee: inputs.relay_fee.map(|x| (*x).into()),
        pubkey_array: ctx.remaining_accounts.iter().map(|x| x.key()).collect(),
        compress_or_decompress_lamports: inputs
            .compress_or_decompress_lamports
            .map(|x| (*x).into()),
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
