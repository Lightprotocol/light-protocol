use anchor_lang::prelude::*;
use light_bounded_vec::BoundedVec;
use light_concurrent_merkle_tree::event::{MerkleTreeEvent, NullifierEvent};
use light_hasher::zero_bytes::poseidon::ZERO_BYTES;

use crate::{
    emit_indexer_event,
    errors::AccountCompressionErrorCode,
    state::{
        queue::{queue_from_bytes_zero_copy_mut, QueueAccount},
        StateMerkleTreeAccount,
    },
    state_merkle_tree_from_bytes_zero_copy_mut,
    utils::check_signer_is_registered_or_authority::{
        check_signer_is_registered_or_authority, GroupAccess, GroupAccounts,
    },
    RegisteredProgram,
};

#[derive(Accounts)]
pub struct NullifyLeaves<'info> {
    /// CHECK: should only be accessed by a registered program or owner.
    pub authority: Signer<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
    /// CHECK: when emitting event.
    pub log_wrapper: UncheckedAccount<'info>,
    #[account(mut)]
    pub merkle_tree: AccountLoader<'info, StateMerkleTreeAccount>,
    #[account(mut)]
    pub nullifier_queue: AccountLoader<'info, QueueAccount>,
}

impl GroupAccess for StateMerkleTreeAccount {
    fn get_owner(&self) -> Pubkey {
        self.metadata.access_metadata.owner.into()
    }

    fn get_program_owner(&self) -> Pubkey {
        self.metadata
            .access_metadata
            .program_owner
            .to_bytes()
            .into()
    }
}

impl<'info> GroupAccounts<'info> for NullifyLeaves<'info> {
    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

pub fn process_nullify_leaves<'a, 'b, 'c: 'info, 'info>(
    ctx: &'a Context<'a, 'b, 'c, 'info, NullifyLeaves<'info>>,
    change_log_indices: &'a [u64],
    leaves_queue_indices: &'a [u16],
    leaf_indices: &'a [u64],
    proofs: &'a [Vec<[u8; 32]>],
) -> Result<()> {
    if change_log_indices.len() != 1 {
        msg!("only implemented for 1 nullifier update");
        return Err(AccountCompressionErrorCode::NumberOfChangeLogIndicesMismatch.into());
    }
    if leaves_queue_indices.len() != change_log_indices.len() {
        msg!("only implemented for 1 nullifier update");
        return Err(AccountCompressionErrorCode::NumberOfLeavesMismatch.into());
    }
    if leaf_indices.len() != change_log_indices.len() {
        msg!("only implemented for 1 nullifier update");
        return Err(AccountCompressionErrorCode::NumberOfIndicesMismatch.into());
    }
    if proofs.len() != change_log_indices.len() {
        msg!("only implemented for 1 nullifier update");
        return Err(AccountCompressionErrorCode::NumberOfProofsMismatch.into());
    }
    if proofs.len() > 1 && proofs[0].len() != proofs[1].len() {
        msg!(
            "Proofs length mismatch {} {}",
            proofs[0].len(),
            proofs[1].len()
        );
        return Err(AccountCompressionErrorCode::ProofLengthMismatch.into());
    }
    insert_nullifier(
        proofs,
        change_log_indices,
        leaves_queue_indices,
        leaf_indices,
        ctx,
    )?;

    Ok(())
}

#[inline(never)]
fn insert_nullifier<'a, 'c: 'info, 'info>(
    proofs: &[Vec<[u8; 32]>],
    change_log_indices: &[u64],
    leaves_queue_indices: &[u16],
    leaf_indices: &[u64],
    ctx: &Context<'a, '_, 'c, 'info, NullifyLeaves<'info>>,
) -> Result<()> {
    {
        let merkle_tree = ctx.accounts.merkle_tree.load()?;

        if merkle_tree.metadata.associated_queue != ctx.accounts.nullifier_queue.key() {
            msg!(
            "Merkle tree and nullifier queue are not associated. Merkle tree associated nullifier queue {:?} != nullifier queue {}",
            merkle_tree.metadata.associated_queue,
            ctx.accounts.nullifier_queue.key()
        );
            return err!(AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated);
        }
        check_signer_is_registered_or_authority::<NullifyLeaves, StateMerkleTreeAccount>(
            ctx,
            &merkle_tree,
        )?;
    }

    let merkle_tree = ctx.accounts.merkle_tree.to_account_info();
    let mut merkle_tree = merkle_tree.try_borrow_mut_data()?;
    let mut merkle_tree = state_merkle_tree_from_bytes_zero_copy_mut(&mut merkle_tree)?;

    let nullifier_queue = ctx.accounts.nullifier_queue.to_account_info();
    let mut nullifier_queue = nullifier_queue.try_borrow_mut_data()?;
    let mut nullifier_queue = unsafe { queue_from_bytes_zero_copy_mut(&mut nullifier_queue)? };

    let allowed_proof_size = merkle_tree.height - merkle_tree.canopy_depth;
    if proofs[0].len() != allowed_proof_size {
        msg!(
            "Invalid Proof Length {} allowed height {} - canopy {} {}",
            proofs[0].len(),
            merkle_tree.height,
            merkle_tree.canopy_depth,
            allowed_proof_size,
        );
        return err!(AccountCompressionErrorCode::InvalidMerkleProof);
    }
    let seq = (merkle_tree.sequence_number() + 1) as u64;
    for (i, leaf_queue_index) in leaves_queue_indices.iter().enumerate() {
        let leaf_cell = nullifier_queue
            .get_unmarked_bucket(*leaf_queue_index as usize)
            .ok_or(AccountCompressionErrorCode::LeafNotFound)?
            .ok_or(AccountCompressionErrorCode::LeafNotFound)?;

        let mut proof =
            from_vec(proofs[i].as_slice(), merkle_tree.height).map_err(ProgramError::from)?;
        merkle_tree
            .update(
                change_log_indices[i] as usize,
                &leaf_cell.value_bytes(),
                &ZERO_BYTES[0],
                leaf_indices[i] as usize,
                &mut proof,
            )
            .map_err(ProgramError::from)?;

        nullifier_queue
            .mark_with_sequence_number(*leaf_queue_index as usize, merkle_tree.sequence_number())
            .map_err(ProgramError::from)?;
    }
    let nullify_event = NullifierEvent {
        id: ctx.accounts.merkle_tree.key().to_bytes(),
        nullified_leaves_indices: leaf_indices.to_vec(),
        seq,
    };
    let nullify_event = MerkleTreeEvent::V2(nullify_event);
    emit_indexer_event(nullify_event.try_to_vec()?, &ctx.accounts.log_wrapper)?;
    Ok(())
}

#[inline(never)]
pub fn from_vec(vec: &[[u8; 32]], height: usize) -> Result<BoundedVec<[u8; 32]>> {
    let proof: [[u8; 32]; 16] = vec.try_into().unwrap();
    let mut bounded_vec = BoundedVec::with_capacity(height);
    bounded_vec.extend(proof).map_err(ProgramError::from)?;
    Ok(bounded_vec)
}

#[cfg(not(target_os = "solana"))]
pub mod sdk_nullify {
    use anchor_lang::{InstructionData, ToAccountMetas};
    use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

    use crate::utils::constants::NOOP_PUBKEY;

    pub fn create_nullify_instruction(
        change_log_indices: &[u64],
        leaves_queue_indices: &[u16],
        leaf_indices: &[u64],
        proofs: &[Vec<[u8; 32]>],
        payer: &Pubkey,
        merkle_tree_pubkey: &Pubkey,
        nullifier_queue_pubkey: &Pubkey,
    ) -> Instruction {
        let instruction_data = crate::instruction::NullifyLeaves {
            leaves_queue_indices: leaves_queue_indices.to_vec(),
            leaf_indices: leaf_indices.to_vec(),
            change_log_indices: change_log_indices.to_vec(),
            proofs: proofs.to_vec(),
        };

        let accounts = crate::accounts::NullifyLeaves {
            authority: *payer,
            registered_program_pda: None,
            log_wrapper: Pubkey::new_from_array(NOOP_PUBKEY),
            merkle_tree: *merkle_tree_pubkey,
            nullifier_queue: *nullifier_queue_pubkey,
        };

        Instruction {
            program_id: crate::ID,
            accounts: accounts.to_account_metas(Some(true)),
            data: instruction_data.data(),
        }
    }
}
