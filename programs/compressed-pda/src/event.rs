use std::{mem, str::FromStr};

use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, program::invoke},
};
use light_macros::heap_neutral;

use crate::{
    compressed_account::{CompressedAccount, CompressedAccountWithMerkleContext},
    InstructionDataTransfer, TransferInstruction,
};

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, Default, PartialEq)]
pub struct PublicTransactionEvent {
    pub input_compressed_account_hashes: Vec<[u8; 32]>,
    pub output_compressed_account_hashes: Vec<[u8; 32]>,
    pub input_compressed_accounts: Vec<CompressedAccountWithMerkleContext>,
    pub output_compressed_accounts: Vec<CompressedAccount>,
    // index of Merkle tree account in remaining accounts
    pub output_state_merkle_tree_account_indices: Vec<u8>,
    pub output_leaf_indices: Vec<u32>,
    pub relay_fee: Option<u64>,
    pub is_compress: bool,
    pub compression_lamports: Option<u64>,
    pub pubkey_array: Vec<Pubkey>,
    pub message: Option<Vec<u8>>,
}

pub trait SizedEvent {
    fn event_size(&self) -> usize;
}

impl SizedEvent for PublicTransactionEvent {
    fn event_size(&self) -> usize {
        mem::size_of::<Self>()
            + self.input_compressed_account_hashes.len() * mem::size_of::<[u8; 32]>()
            + self.output_compressed_account_hashes.len() * mem::size_of::<[u8; 32]>()
            + self.input_compressed_accounts.len()
                * mem::size_of::<CompressedAccountWithMerkleContext>()
            + self.output_compressed_accounts.len() * mem::size_of::<CompressedAccount>()
            + self.output_state_merkle_tree_account_indices.len()
            + self.output_leaf_indices.len() * mem::size_of::<u32>()
            + self.pubkey_array.len() * mem::size_of::<Pubkey>()
            + self
                .message
                .as_ref()
                .map(|message| message.len())
                .unwrap_or(0)
    }
}

#[inline(never)]
pub fn invoke_indexer_transaction_event<T>(event: &T, noop_program: &AccountInfo) -> Result<()>
where
    T: AnchorSerialize + SizedEvent,
{
    if noop_program.key()
        != Pubkey::from_str("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV").unwrap()
    {
        return err!(crate::ErrorCode::InvalidNoopPubkey);
    }
    let mut data = Vec::with_capacity(event.event_size());
    event.serialize(&mut data)?;
    let instruction = Instruction {
        program_id: noop_program.key(),
        accounts: vec![],
        data,
    };
    invoke(&instruction, &[noop_program.to_account_info()])?;
    Ok(())
}

#[heap_neutral]
pub fn emit_state_transition_event<'a, 'b, 'c: 'info, 'info>(
    inputs: InstructionDataTransfer,
    ctx: &'a Context<'a, 'b, 'c, 'info, TransferInstruction<'info>>,
    input_compressed_account_hashes: Vec<[u8; 32]>,
    output_compressed_account_hashes: Vec<[u8; 32]>,
    output_leaf_indices: Vec<u32>,
) -> Result<()> {
    // TODO: add message and compression_lamports
    let event = PublicTransactionEvent {
        input_compressed_account_hashes,
        output_compressed_account_hashes,
        input_compressed_accounts: inputs.input_compressed_accounts_with_merkle_context,
        output_compressed_accounts: inputs.output_compressed_accounts,
        output_state_merkle_tree_account_indices: inputs.output_state_merkle_tree_account_indices,
        output_leaf_indices,
        relay_fee: inputs.relay_fee,
        pubkey_array: ctx.remaining_accounts.iter().map(|x| x.key()).collect(),
        compression_lamports: None,
        message: None,
        is_compress: false,
    };
    invoke_indexer_transaction_event(&event, &ctx.accounts.noop_program)?;
    Ok(())
}
