use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use ark_ff::BigInteger;
use light_bounded_vec::BoundedVec;
use light_hasher::zero_bytes::poseidon::ZERO_BYTES;

use crate::{
    emit_indexer_event, errors::AccountCompressionErrorCode, indexed_array_from_bytes_mut,
    state::StateMerkleTreeAccount, ChangelogEvent, ChangelogEventV1, Changelogs,
    IndexedArrayAccount, RegisteredProgram,
};
// TODO: implement group access control
#[derive(Accounts)]
pub struct NullifyLeaves<'info> {
    /// CHECK: should only be accessed by a registered program/owner/delegate.
    #[account(mut)]
    pub authority: Signer<'info>,
    // TODO: Add fee payer.
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
    /// CHECK: in event emitting
    pub log_wrapper: UncheckedAccount<'info>,
    /// CHECK: when parsing account
    #[account(mut)]
    pub merkle_tree: AccountLoader<'info, StateMerkleTreeAccount>,
    #[account(mut)]
    pub indexed_array: AccountLoader<'info, IndexedArrayAccount>,
}

// TODO: implement for multiple nullifiers got a stack frame error with a loop
#[inline(never)]
pub fn process_nullify_leaves<'a, 'b, 'c: 'info, 'info>(
    ctx: &'a Context<'a, 'b, 'c, 'info, NullifyLeaves<'info>>,
    change_log_indices: &'a [u64],
    leaves_indices: &'a [u16],
    indices: &'a [u64],
    proofs: &'a [Vec<[u8; 32]>], // TODO: make height independent
) -> Result<()> {
    // TODO: activate when group access control is implemented
    // check_registered_or_signer::<AppendLeaves, StateMerkleTreeAccount>(
    //     &ctx,
    //     &merkle_tree_account,
    // )?;
    let mut array_account = ctx.accounts.indexed_array.load_mut()?;
    let array = indexed_array_from_bytes_mut(&mut array_account.indexed_array);

    let leaf: [u8; 32] = array
        .get(leaves_indices[0] as usize)
        .ok_or(AccountCompressionErrorCode::IndexOutOfBounds)?
        .value
        .to_bytes_be()
        .try_into()
        .unwrap();
    msg!("leaf {:?}", leaf);
    if change_log_indices.len() != 1 {
        msg!("only implemented for 1 nullifier update");
        return Err(AccountCompressionErrorCode::NumberOfChangeLogIndicesMismatch.into());
    }
    if leaves_indices.len() != change_log_indices.len() {
        msg!("only implemented for 1 nullifier update");
        return Err(AccountCompressionErrorCode::NumberOfLeavesMismatch.into());
    }
    if indices.len() != change_log_indices.len() {
        msg!("only implemented for 1 nullifier update");
        return Err(AccountCompressionErrorCode::NumberOfIndicesMismatch.into());
    }
    let mut changelog_events: Vec<ChangelogEvent> = Vec::new();
    let mut merkle_tree = ctx.accounts.merkle_tree.load_mut()?;
    let loaded_merkle_tree = merkle_tree.load_merkle_tree_mut()?;
    let mut bounded_vec = from_vec(proofs[0].as_slice())?;
    let changelog_entries = loaded_merkle_tree
        .update(
            change_log_indices[0] as usize,
            &leaf,
            &ZERO_BYTES[0],
            indices[0] as usize,
            &mut bounded_vec,
        )
        .map_err(ProgramError::from)?;
    let sequence_number = u64::try_from(merkle_tree.load_merkle_tree()?.sequence_number)
        .map_err(|_| AccountCompressionErrorCode::IntegerOverflow)?;
    changelog_events.push(ChangelogEvent::V1(ChangelogEventV1::new(
        ctx.accounts.merkle_tree.key(),
        &[changelog_entries],
        sequence_number,
    )?));
    let changelog_event = Changelogs {
        changelogs: changelog_events,
    };
    emit_indexer_event(
        changelog_event.try_to_vec()?,
        &ctx.accounts.log_wrapper,
        &ctx.accounts.authority,
    )?;

    Ok(())
}
#[inline(never)]
pub fn from_vec(vec: &[[u8; 32]]) -> Result<BoundedVec<[u8; 32]>> {
    let proof: [[u8; 32]; 26] = (vec).try_into().unwrap();
    let bounded_vec = BoundedVec::from_array(&proof);
    Ok(bounded_vec)
}
#[cfg(not(target_os = "solana"))]
pub mod sdk {
    use anchor_lang::{InstructionData, ToAccountMetas};
    use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

    pub fn create_nullify_instruction(
        change_log_indices: &[u64],
        leaves_indices: &[u16],
        indices: &[u64],
        proofs: &[Vec<[u8; 32]>],
        payer: &Pubkey,
        merkle_tree_pubkey: &Pubkey,
        indexed_array_pubkey: &Pubkey,
    ) -> Instruction {
        let instruction_data = crate::instruction::NullifyLeaves {
            leaves_indices: leaves_indices.to_vec(),
            indices: indices.to_vec(),
            change_log_indices: change_log_indices.to_vec(),
            proofs: proofs.to_vec(),
        };

        let accounts = crate::accounts::NullifyLeaves {
            authority: *payer,
            registered_program_pda: None,
            log_wrapper: crate::state::change_log_event::NOOP_PROGRAM_ID,
            merkle_tree: *merkle_tree_pubkey,
            indexed_array: *indexed_array_pubkey,
        };

        Instruction {
            program_id: crate::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        }
    }
}
