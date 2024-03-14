use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use light_bounded_vec::BoundedVec;
use light_hasher::zero_bytes::poseidon::ZERO_BYTES;

use crate::{
    emit_indexer_event, errors::AccountCompressionErrorCode,
    indexed_array_from_bytes_zero_copy_mut, state::StateMerkleTreeAccount, ChangelogEvent,
    ChangelogEventV1, Changelogs, IndexedArrayAccount, RegisteredProgram,
};

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

    let indexed_array = unsafe {
        indexed_array_from_bytes_zero_copy_mut(
            ctx.accounts
                .indexed_array
                .to_account_info()
                .try_borrow_mut_data()?,
        )
        .unwrap()
    };
    for element in indexed_array.iter() {
        msg!("ELEMENT: {:?}", element);
    }
    let leaf_cell = indexed_array
        .by_value_index(leaves_queue_indices[0] as usize)
        .ok_or(AccountCompressionErrorCode::LeafNotFound)?;
    let leaf = leaf_cell.value_bytes();
    drop(indexed_array);

    if change_log_indices.len() != 1 {
        return Err(AccountCompressionErrorCode::NumberOfChangeLogIndicesMismatch.into());
    }
    if leaves_queue_indices.len() != change_log_indices.len() {
        return Err(AccountCompressionErrorCode::NumberOfLeavesMismatch.into());
    }
    if indices.len() != change_log_indices.len() {
        return Err(AccountCompressionErrorCode::NumberOfIndicesMismatch.into());
    }
    if proofs.len() != change_log_indices.len() {
        return Err(AccountCompressionErrorCode::NumberOfProofsMismatch.into());
    }
    let changelog_event = insert_nullifier(
        proofs,
        change_log_indices,
        leaf,
        indices,
        ctx,
        leaves_queue_indices,
    )?;
    emit_indexer_event(
        changelog_event.try_to_vec()?,
        &ctx.accounts.log_wrapper,
        &ctx.accounts.authority,
    )
}

#[inline(never)]
fn insert_nullifier(
    proofs: &[Vec<[u8; 32]>],
    change_log_indices: &[u64],
    leaf: [u8; 32],
    indices: &[u64],
    ctx: &Context<'_, '_, '_, '_, NullifyLeaves<'_>>,
    leaves_queue_indices: &[u16],
) -> Result<Changelogs> {
    let mut merkle_tree_account = ctx.accounts.merkle_tree.load_mut()?;

    if merkle_tree_account.associated_queue != ctx.accounts.indexed_array.key() {
        msg!(
            "Merkle tree and nullifier queue are not associated. Merkle tree associated nullifier queue {} != nullifier queue {}",
            merkle_tree_account.associated_queue,
            ctx.accounts.indexed_array.key()
        );
        return Err(AccountCompressionErrorCode::InvalidIndexedArray.into());
    }
    let loaded_merkle_tree = merkle_tree_account.load_merkle_tree_mut()?;
    let allowed_proof_size = loaded_merkle_tree.height - loaded_merkle_tree.canopy_depth;
    if proofs[0].len() != allowed_proof_size {
        msg!(
            "Invalid Proof Length {} allowed height {} - canopy {} {}",
            proofs[0].len(),
            loaded_merkle_tree.height,
            loaded_merkle_tree.canopy_depth,
            allowed_proof_size,
        );
        return Err(AccountCompressionErrorCode::InvalidMerkleProof.into());
    }

    let mut bounded_vec = from_vec(proofs[0].as_slice())?;

    let indexed_array = unsafe {
        indexed_array_from_bytes_zero_copy_mut(
            ctx.accounts
                .indexed_array
                .to_account_info()
                .try_borrow_mut_data()?,
        )
        .unwrap()
    };
    let leaf_cell = indexed_array
        .by_value_index(leaves_queue_indices[0] as usize)
        .ok_or(AccountCompressionErrorCode::LeafNotFound)?;

    let changelog_entries = loaded_merkle_tree
        .update(
            change_log_indices[0] as usize,
            &leaf,
            &ZERO_BYTES[0],
            indices[0] as usize,
            &mut bounded_vec,
        )
        .map_err(ProgramError::from)?;
    let sequence_number = u64::try_from(loaded_merkle_tree.sequence_number)
        .map_err(|_| AccountCompressionErrorCode::IntegerOverflow)?;
    indexed_array
        .mark_with_sequence_number(&leaf_cell.value_biguint(), sequence_number as usize)
        .map_err(ProgramError::from)?;
    Ok(Changelogs {
        changelogs: vec![ChangelogEvent::V1(ChangelogEventV1::new(
            ctx.accounts.merkle_tree.key(),
            &[changelog_entries],
            sequence_number,
        )?)],
    })
}

#[inline(never)]
pub fn from_vec(vec: &[[u8; 32]]) -> Result<BoundedVec<[u8; 32]>> {
    let proof: [[u8; 32]; 16] = (vec).try_into().unwrap();
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
