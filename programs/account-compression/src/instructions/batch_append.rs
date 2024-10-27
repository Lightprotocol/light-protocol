use crate::{
    batched_merkle_tree::{
        InstructionDataBatchAppendProofInputs, ZeroCopyBatchedMerkleTreeAccount,
    },
    utils::check_signer_is_registered_or_authority::{
        check_signer_is_registered_or_authority, GroupAccounts,
    },
    RegisteredProgram,
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct BatchAppend<'info> {
    /// CHECK: should only be accessed by a registered program or owner.
    pub authority: Signer<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
    /// CHECK: when emitting event.
    pub log_wrapper: UncheckedAccount<'info>,
    /// CHECK: in from_bytes_mut.
    #[account(mut)]
    pub merkle_tree: AccountInfo<'info>,
    /// CHECK: in from_bytes_mut.
    #[account(mut)]
    pub output_queue: AccountInfo<'info>,
}

impl<'info> GroupAccounts<'info> for BatchAppend<'info> {
    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

pub fn process_batch_append_leaves<'a, 'b, 'c: 'info, 'info>(
    ctx: &'a Context<'a, 'b, 'c, 'info, BatchAppend<'info>>,
    instruction_data: InstructionDataBatchAppendProofInputs,
) -> Result<()> {
    let account_data = &mut ctx.accounts.merkle_tree.try_borrow_mut_data()?;
    let merkle_tree = &mut ZeroCopyBatchedMerkleTreeAccount::from_bytes_mut(account_data)?;
    check_signer_is_registered_or_authority::<BatchAppend, ZeroCopyBatchedMerkleTreeAccount>(
        ctx,
        &merkle_tree,
    )?;
    let output_queue_data = &mut ctx.accounts.output_queue.try_borrow_mut_data()?;
    merkle_tree.update_output_queue(output_queue_data, instruction_data)?;

    // TODO: create a new event, difficulty is how do I tie the update to a batch
    // I should number the batches, is the sequence number enough?
    // let nullify_event = NullifierEvent {
    //     id: ctx.accounts.merkle_tree.key().to_bytes(),
    //     nullified_leaves_indices: leaf_indices.to_vec(),
    //     seq,
    // };
    // let nullify_event = MerkleTreeEvent::V2(nullify_event);
    // emit_indexer_event(nullify_event.try_to_vec()?, &ctx.accounts.log_wrapper)?;
    Ok(())
}

// #[cfg(not(target_os = "solana"))]
// pub mod sdk_nullify {
//     use anchor_lang::{InstructionData, ToAccountMetas};
//     use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

//     use crate::utils::constants::NOOP_PUBKEY;

//     pub fn create_nullify_instruction(
//         change_log_indices: &[u64],
//         leaves_queue_indices: &[u16],
//         leaf_indices: &[u64],
//         proofs: &[Vec<[u8; 32]>],
//         payer: &Pubkey,
//         merkle_tree_pubkey: &Pubkey,
//         nullifier_queue_pubkey: &Pubkey,
//     ) -> Instruction {
//         let instruction_data = crate::instruction::NullifyLeaves {
//             leaves_queue_indices: leaves_queue_indices.to_vec(),
//             leaf_indices: leaf_indices.to_vec(),
//             change_log_indices: change_log_indices.to_vec(),
//             proofs: proofs.to_vec(),
//         };

//         let accounts = crate::accounts::NullifyLeaves {
//             authority: *payer,
//             registered_program_pda: None,
//             log_wrapper: Pubkey::new_from_array(NOOP_PUBKEY),
//             merkle_tree: *merkle_tree_pubkey,
//             nullifier_queue: *nullifier_queue_pubkey,
//         };

//         Instruction {
//             program_id: crate::ID,
//             accounts: accounts.to_account_metas(Some(true)),
//             data: instruction_data.data(),
//         }
//     }
// }
