use std::str::FromStr;

use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, program::invoke},
};

use crate::{
    compressed_account::{CompressedAccount, CompressedAccountWithMerkleContext},
    InstructionDataTransfer, TransferInstruction,
};

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct PublicTransactionEvent {
    pub input_compressed_account_hashes: Vec<[u8; 32]>,
    pub output_account_hashes: Vec<[u8; 32]>,
    pub input_compressed_accounts: Vec<CompressedAccountWithMerkleContext>,
    pub output_compressed_accounts: Vec<CompressedAccount>,
    // index of Merkle tree account in remaining accounts
    pub output_state_merkle_tree_account_indices: Vec<u8>,
    pub output_leaf_indices: Vec<u32>,
    pub relay_fee: Option<u64>,
    pub de_compress_amount: Option<u64>,
    pub pubkey_array: Vec<Pubkey>,
    pub message: Option<Vec<u8>>,
}

#[inline(never)]
pub fn invoke_indexer_transaction_event<T>(event: &T, noop_program: &AccountInfo) -> Result<()>
where
    T: AnchorSerialize,
{
    if noop_program.key()
        != Pubkey::from_str("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV").unwrap()
    {
        return err!(crate::ErrorCode::InvalidNoopPubkey);
    }
    let instruction = Instruction {
        program_id: noop_program.key(),
        accounts: vec![],
        data: event.try_to_vec()?,
    };
    invoke(&instruction, &[noop_program.to_account_info()])?;
    Ok(())
}

pub fn emit_state_transition_event<'a, 'b, 'c: 'info, 'info>(
    inputs: &'a InstructionDataTransfer,
    ctx: &'a Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    input_compressed_account_hashes: &[[u8; 32]],
    output_compressed_account_hashes: &[[u8; 32]],
    output_leaf_indices: &[u32],
) -> anchor_lang::Result<PublicTransactionEvent> {
    // TODO: add message and de_compress_amount
    let event = PublicTransactionEvent {
        input_compressed_account_hashes: input_compressed_account_hashes.to_vec(),
        output_account_hashes: output_compressed_account_hashes.to_vec(),
        input_compressed_accounts: inputs.input_compressed_accounts_with_merkle_context.clone(),
        output_compressed_accounts: inputs.output_compressed_accounts.to_vec(),
        output_state_merkle_tree_account_indices: inputs
            .output_state_merkle_tree_account_indices
            .to_vec(),
        output_leaf_indices: output_leaf_indices.to_vec(),
        relay_fee: inputs.relay_fee,
        pubkey_array: ctx.remaining_accounts.iter().map(|x| x.key()).collect(),
        de_compress_amount: None,
        message: None,
    };
    invoke_indexer_transaction_event(&event, &ctx.accounts.noop_program)?;
    Ok(event)
}
