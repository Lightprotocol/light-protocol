use anchor_lang::prelude::*;
use light_bounded_vec::BoundedVec;
use light_concurrent_merkle_tree::event::{ChangelogEvent, Changelogs};
use light_hasher::zero_bytes::poseidon::ZERO_BYTES;
use light_macros::heap_neutral;

use crate::{
    emit_indexer_event, errors::AccountCompressionErrorCode,
    indexed_array_from_bytes_zero_copy_mut, state::StateMerkleTreeAccount, IndexedArrayAccount,
    RegisteredProgram,
};

#[derive(Accounts)]
pub struct NullifyLeaves<'info> {
    /// CHECK: should only be accessed by a registered program/owner/delegate.
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
#[heap_neutral]
pub fn process_nullify_leaves<'a, 'b, 'c: 'info, 'info>(
    ctx: &'a Context<'a, 'b, 'c, 'info, NullifyLeaves<'info>>,
    change_log_indices: &'a [u64],
    leaves_queue_indices: &'a [u16],
    indices: &'a [u64],
    proofs: &'a [Vec<[u8; 32]>],
) -> Result<()> {
    {
        let array_account = ctx.accounts.indexed_array.load()?;
        if array_account.associated_merkle_tree != ctx.accounts.merkle_tree.key() {
            msg!(
            "Nullifier queue and Merkle tree are not associated. Associated mt of nullifier queue {} != merkle tree {}",
            array_account.associated_merkle_tree,
            ctx.accounts.merkle_tree.key(),
        );
            return Err(AccountCompressionErrorCode::InvalidMerkleTree.into());
        }
        drop(array_account);
    }

    if change_log_indices.len() != 1 {
        msg!("only implemented for 1 nullifier update");
        return Err(AccountCompressionErrorCode::NumberOfChangeLogIndicesMismatch.into());
    }
    if leaves_queue_indices.len() != change_log_indices.len() {
        msg!("only implemented for 1 nullifier update");
        return Err(AccountCompressionErrorCode::NumberOfLeavesMismatch.into());
    }
    if indices.len() != change_log_indices.len() {
        msg!("only implemented for 1 nullifier update");
        return Err(AccountCompressionErrorCode::NumberOfIndicesMismatch.into());
    }
    if proofs.len() != change_log_indices.len() {
        msg!("only implemented for 1 nullifier update");
        return Err(AccountCompressionErrorCode::NumberOfProofsMismatch.into());
    }
    insert_nullifier(
        proofs,
        change_log_indices,
        leaves_queue_indices,
        indices,
        ctx,
    )?;

    Ok(())
}

#[inline(never)]
fn insert_nullifier(
    proofs: &[Vec<[u8; 32]>],
    change_log_indices: &[u64],
    leaves_queue_indices: &[u16],
    indices: &[u64],
    ctx: &Context<'_, '_, '_, '_, NullifyLeaves<'_>>,
) -> Result<()> {
    let mut merkle_tree = ctx.accounts.merkle_tree.load_mut()?;
    if merkle_tree.associated_queue != ctx.accounts.indexed_array.key() {
        msg!(
            "Merkle tree and nullifier queue are not associated. Merkle tree associated nullifier queue {} != nullifier queue {}",
            merkle_tree.associated_queue,
            ctx.accounts.indexed_array.key()
        );
        return Err(AccountCompressionErrorCode::InvalidIndexedArray.into());
    }
    let merkle_tree = merkle_tree.load_merkle_tree_mut()?;

    let indexed_array = ctx.accounts.indexed_array.to_account_info();
    let mut indexed_array = indexed_array.try_borrow_mut_data()?;
    let mut indexed_array = unsafe { indexed_array_from_bytes_zero_copy_mut(&mut indexed_array)? };

    let allowed_proof_size = merkle_tree.height - merkle_tree.canopy_depth;
    if proofs[0].len() != allowed_proof_size {
        msg!(
            "Invalid Proof Length {} allowed height {} - canopy {} {}",
            proofs[0].len(),
            merkle_tree.height,
            merkle_tree.canopy_depth,
            allowed_proof_size,
        );
        return Err(AccountCompressionErrorCode::InvalidMerkleProof.into());
    }

    let mut changelogs: Vec<ChangelogEvent> = Vec::with_capacity(leaves_queue_indices.len());
    for (i, leaf_queue_index) in leaves_queue_indices.iter().enumerate() {
        let leaf_cell = indexed_array
            .by_value_index(*leaf_queue_index as usize, None)
            .cloned()
            .ok_or(AccountCompressionErrorCode::LeafNotFound)?;

        let mut proof = from_vec(proofs[i].as_slice()).map_err(ProgramError::from)?;
        let (changelog_index, sequence_number) = merkle_tree
            .update(
                change_log_indices[i] as usize,
                &leaf_cell.value_bytes(),
                &ZERO_BYTES[i],
                indices[i] as usize,
                &mut proof,
            )
            .map_err(ProgramError::from)?;
        let changelog_event = merkle_tree
            .get_changelog_event(
                ctx.accounts.merkle_tree.key().to_bytes(),
                changelog_index,
                sequence_number,
                1,
            )
            .map_err(ProgramError::from)?;
        changelogs.push(changelog_event);

        // TODO: replace with root history sequence number
        indexed_array
            .mark_with_sequence_number(&leaf_cell.value_biguint(), merkle_tree.sequence_number)
            .map_err(ProgramError::from)?;
    }

    let changelog_event = Changelogs { changelogs };

    emit_indexer_event(
        changelog_event.try_to_vec()?,
        &ctx.accounts.log_wrapper,
        &ctx.accounts.authority,
    )?;

    Ok(())
}

#[inline(never)]
pub fn from_vec(vec: &[[u8; 32]]) -> Result<BoundedVec<[u8; 32]>> {
    let proof: [[u8; 32]; 16] = vec.try_into().unwrap();
    let mut bounded_vec = BoundedVec::with_capacity(26);
    bounded_vec.extend(proof).map_err(ProgramError::from)?;
    Ok(bounded_vec)
}

#[cfg(not(target_os = "solana"))]
pub mod sdk_nullify {
    use anchor_lang::{InstructionData, ToAccountMetas};
    use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

    pub fn create_nullify_instruction(
        change_log_indices: &[u64],
        leaves_queue_indices: &[u16],
        indices: &[u64],
        proofs: &[Vec<[u8; 32]>],
        payer: &Pubkey,
        merkle_tree_pubkey: &Pubkey,
        indexed_array_pubkey: &Pubkey,
    ) -> Instruction {
        let instruction_data = crate::instruction::NullifyLeaves {
            leaves_queue_indices: leaves_queue_indices.to_vec(),
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
