use account_compression::utils::constants::NOOP_PUBKEY;
use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, program::invoke},
    Bumps,
};

use crate::{
    errors::CompressedPdaError,
    sdk::{
        accounts::InvokeAccounts,
        event::{PublicTransactionEvent, SizedEvent},
    },
    InstructionDataInvoke,
};
pub fn emit_state_transition_event<'a, 'b, 'c: 'info, 'info, A: InvokeAccounts<'info> + Bumps>(
    inputs: InstructionDataInvoke,
    ctx: &'a Context<'a, 'b, 'c, 'info, A>,
    input_compressed_account_hashes: Vec<[u8; 32]>,
    output_compressed_account_hashes: Vec<[u8; 32]>,
    output_leaf_indices: Vec<u32>,
) -> Result<()> {
    // TODO: add message and compression_lamports
    let event = PublicTransactionEvent {
        input_compressed_account_hashes,
        output_compressed_account_hashes,
        output_compressed_accounts: inputs.output_compressed_accounts,
        output_state_merkle_tree_account_indices: inputs.output_state_merkle_tree_account_indices,
        output_leaf_indices,
        relay_fee: inputs.relay_fee,
        pubkey_array: ctx.remaining_accounts.iter().map(|x| x.key()).collect(),
        compression_lamports: None,
        message: None,
        is_compress: false,
    };

    if ctx.accounts.get_noop_program().key() != Pubkey::new_from_array(NOOP_PUBKEY) {
        return err!(CompressedPdaError::InvalidNoopPubkey);
    }
    let mut data = Vec::with_capacity(event.event_size());
    // TODO: add compression lamports
    event.man_serialize(&mut data)?;
    let instruction = Instruction {
        program_id: ctx.accounts.get_noop_program().key(),
        accounts: vec![],
        data,
    };
    invoke(
        &instruction,
        &[ctx.accounts.get_noop_program().to_account_info()],
    )?;
    Ok(())
}
