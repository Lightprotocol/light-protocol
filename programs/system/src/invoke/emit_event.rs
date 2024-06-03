use crate::{
    errors::SystemProgramError,
    sdk::{
        accounts::InvokeAccounts,
        event::{MerkleTreeSequenceNumber, PublicTransactionEvent, SizedEvent},
    },
    InstructionDataInvoke,
};
use account_compression::utils::constants::NOOP_PUBKEY;
use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, program::invoke},
    Bumps,
};

pub fn emit_state_transition_event<'a, 'b, 'c: 'info, 'info, A: InvokeAccounts<'info> + Bumps>(
    inputs: InstructionDataInvoke,
    ctx: &'a Context<'a, 'b, 'c, 'info, A>,
    input_compressed_account_hashes: Vec<[u8; 32]>,
    output_compressed_account_hashes: Vec<[u8; 32]>,
    output_leaf_indices: Vec<u32>,
    sequence_numbers: Vec<MerkleTreeSequenceNumber>,
) -> Result<()> {
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

    if ctx.accounts.get_noop_program().key() != Pubkey::new_from_array(NOOP_PUBKEY)
        && !ctx.accounts.get_noop_program().executable
    {
        return err!(SystemProgramError::InvalidNoopPubkey);
    }
    let mut data = Vec::with_capacity(event.event_size());
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
