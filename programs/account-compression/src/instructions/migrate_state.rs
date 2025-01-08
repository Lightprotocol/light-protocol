use anchor_lang::prelude::*;
use light_batched_merkle_tree::queue::BatchedQueueAccount;
use light_concurrent_merkle_tree::{
    event::{MerkleTreeEvent, NullifierEvent},
    zero_copy::ConcurrentMerkleTreeZeroCopyMut,
};
use light_hasher::{zero_bytes::poseidon::ZERO_BYTES, Poseidon};

use super::from_vec;
use crate::{
    emit_indexer_event,
    errors::AccountCompressionErrorCode,
    state::StateMerkleTreeAccount,
    state_merkle_tree_from_bytes_zero_copy_mut,
    utils::check_signer_is_registered_or_authority::{
        check_signer_is_registered_or_authority, GroupAccounts,
    },
    RegisteredProgram,
};

#[derive(Accounts)]
pub struct MigrateState<'info> {
    /// CHECK: should only be accessed by a registered program or owner.
    pub authority: Signer<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
    /// CHECK: when emitting event.
    pub log_wrapper: UncheckedAccount<'info>,
    #[account(mut)]
    pub merkle_tree: AccountLoader<'info, StateMerkleTreeAccount>,
    /// CHECK: with from_account_info.
    #[account(mut)]
    pub output_queue: AccountInfo<'info>,
}

impl<'info> GroupAccounts<'info> for MigrateState<'info> {
    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Debug, Clone)]
pub struct MigrateLeafParams {
    pub change_log_index: u64,
    pub leaf: [u8; 32],
    pub leaf_index: u64,
    pub proof: [[u8; 32]; 16], // height 26 - canopy 10
}

/// 1. Nullifies a leaf in the state merkle tree.
/// 2. Emits a nullifier event.
/// 3. Inserts the leaf in the output queue.
pub fn process_migrate_state<'a, 'b, 'c: 'info, 'info>(
    ctx: &'a Context<'a, 'b, 'c, 'info, MigrateState<'info>>,
    migrate_leaf_params: MigrateLeafParams,
) -> Result<()> {
    if ctx.accounts.registered_program_pda.is_none() {
        msg!("Registered program PDA not set");
        return err!(AccountCompressionErrorCode::RegistryProgramIsNone);
    }
    msg!("migrate_leaf_params: {:?}", migrate_leaf_params);
    {
        let merkle_tree = ctx.accounts.merkle_tree.load()?;
        check_signer_is_registered_or_authority::<MigrateState, StateMerkleTreeAccount>(
            ctx,
            &merkle_tree,
        )?;
    }
    let merkle_tree: AccountInfo<'_> = ctx.accounts.merkle_tree.to_account_info();
    let mut merkle_tree_data = merkle_tree.try_borrow_mut_data()?;
    let mut zero_copy_merkle_tree =
        state_merkle_tree_from_bytes_zero_copy_mut(&mut merkle_tree_data)?;
    let output_queue =
        &mut BatchedQueueAccount::output_queue_from_account_info_mut(&ctx.accounts.output_queue)
            .map_err(ProgramError::from)?;
    let nullify_event = migrate_state(
        migrate_leaf_params,
        &mut zero_copy_merkle_tree,
        &merkle_tree.key(),
        output_queue,
    )?;
    emit_indexer_event(nullify_event.try_to_vec()?, &ctx.accounts.log_wrapper)?;

    Ok(())
}

fn migrate_state(
    migrate_leaf_params: MigrateLeafParams,
    merkle_tree: &mut ConcurrentMerkleTreeZeroCopyMut<'_, Poseidon, 26>,
    merkle_tree_pubkey: &Pubkey,
    output_queue: &mut BatchedQueueAccount,
) -> Result<MerkleTreeEvent> {
    if migrate_leaf_params.leaf == [0u8; 32] {
        return Err(AccountCompressionErrorCode::EmptyLeaf.into());
    }

    let mut proof = from_vec(migrate_leaf_params.proof.as_slice(), merkle_tree.height)
        .map_err(ProgramError::from)?;
    merkle_tree
        .update(
            migrate_leaf_params.change_log_index as usize,
            &migrate_leaf_params.leaf,
            &ZERO_BYTES[0],
            migrate_leaf_params.leaf_index as usize,
            &mut proof,
        )
        .map_err(ProgramError::from)?;

    let nullify_event = NullifierEvent {
        id: merkle_tree_pubkey.to_bytes(),
        nullified_leaves_indices: vec![migrate_leaf_params.leaf_index],
        seq: merkle_tree.sequence_number() as u64,
    };

    output_queue
        .insert_into_current_batch(&migrate_leaf_params.leaf)
        .map_err(ProgramError::from)?;

    Ok(MerkleTreeEvent::V2(nullify_event))
}

#[cfg(test)]
mod migrate_state_test {
    use light_batched_merkle_tree::{
        batch_metadata::BatchMetadata,
        queue::{queue_account_size, BatchedQueueAccount, BatchedQueueMetadata},
    };
    use light_concurrent_merkle_tree::ConcurrentMerkleTree;
    use light_hasher::Poseidon;
    use light_merkle_tree_metadata::{
        access::AccessMetadata,
        queue::{QueueMetadata, QueueType},
        rollover::RolloverMetadata,
    };
    use rand::Rng;
    use solana_sdk::pubkey::Pubkey;
    const HEIGHT: usize = 26;
    const CHANGELOG: usize = 100;
    const ROOTS: usize = 100;
    use super::*;

    pub struct MockQueueAccount<'a> {
        pub account_data: Vec<u8>,
        pub account: Option<BatchedQueueAccount<'a>>,
    }

    fn get_output_queue<'a>() -> MockQueueAccount<'a> {
        let metadata = QueueMetadata {
            next_queue: Pubkey::new_unique(),
            access_metadata: AccessMetadata::default(),
            rollover_metadata: RolloverMetadata::default(),
            queue_type: QueueType::Output as u64,
            associated_merkle_tree: Pubkey::new_unique(),
        };

        let account = BatchedQueueMetadata {
            metadata: metadata.clone(),
            next_index: 0,
            batch_metadata: BatchMetadata {
                batch_size: 1000,
                num_batches: 2,
                currently_processing_batch_index: 0,
                next_full_batch_index: 0,
                bloom_filter_capacity: 0,
                zkp_batch_size: 10,
            },
        };
        let account_data: Vec<u8> =
            vec![
                0;
                queue_account_size(&account.batch_metadata, account.metadata.queue_type).unwrap()
            ];
        let mut mock_account = MockQueueAccount {
            account_data,
            account: None,
        };
        let output_queue = BatchedQueueAccount::init(
            &mut mock_account.account_data,
            metadata,
            account.batch_metadata.num_batches,
            account.batch_metadata.batch_size,
            account.batch_metadata.zkp_batch_size,
            3,
            account.batch_metadata.bloom_filter_capacity,
        )
        .unwrap();
        mock_account.account = Some(output_queue);
        mock_account
    }

    fn get_concurrent_merkle_tree(
        bytes: &mut [u8],
    ) -> ConcurrentMerkleTreeZeroCopyMut<Poseidon, HEIGHT> {
        let mut mt =
            ConcurrentMerkleTreeZeroCopyMut::<Poseidon, HEIGHT>::from_bytes_zero_copy_init(
                bytes, HEIGHT, 10, CHANGELOG, ROOTS,
            )
            .unwrap();
        mt.init().unwrap();
        mt
    }

    // TODO: randomized test
    /// Tests:
    /// 1. functional - migrate 1 leaf
    /// 2. functional -migrate 2nd leaf
    /// 3. failing - invalid proof
    /// 4. failing - invalid leaf index
    /// 5. failing - invalid change log index
    /// 6. failing - invalid leaf
    /// 7. functional - migrate 3rd leaf
    #[test]
    fn test_migrate_state() {
        let merkle_tree_pubkey = Pubkey::new_unique();
        let mut mt_bytes = vec![
            0u8;
            ConcurrentMerkleTree::<Poseidon, HEIGHT>::size_in_account(
                HEIGHT, CHANGELOG, ROOTS, 10,
            )
        ];
        let mut concurrent_mt_with_canopy = get_concurrent_merkle_tree(&mut mt_bytes);
        let mut ref_merkle_tree =
            light_merkle_tree_reference::MerkleTree::<Poseidon>::new(HEIGHT, 10);
        let mut queue_account = get_output_queue();
        let output_queue = &mut queue_account.account.as_mut().unwrap();

        // insert two test leaves into the merkle tree
        let mut leaves = vec![];
        for i in 1..5 {
            let mut leaf = [0u8; 32];
            leaf[31] = i as u8;
            leaves.push(leaf);
            ref_merkle_tree.append(&leaf).unwrap();
            concurrent_mt_with_canopy.append_batch(&[&leaf]).unwrap();
        }

        assert_eq!(ref_merkle_tree.root(), concurrent_mt_with_canopy.root());

        // Functional 1 migrate 1 leaf
        {
            let input = MigrateLeafParams {
                change_log_index: concurrent_mt_with_canopy.changelog_index() as u64,
                leaf: leaves[0].clone(),
                leaf_index: 0,
                proof: ref_merkle_tree
                    .get_proof_of_leaf(0, false)
                    .unwrap()
                    .to_array()
                    .unwrap(),
            };
            let event = migrate_state(
                input,
                &mut concurrent_mt_with_canopy,
                &merkle_tree_pubkey,
                output_queue,
            )
            .unwrap();
            ref_merkle_tree.update(&[0u8; 32], 0).unwrap();
            let expected_event = NullifierEvent {
                id: merkle_tree_pubkey.to_bytes(),
                nullified_leaves_indices: vec![0],
                seq: concurrent_mt_with_canopy.sequence_number() as u64,
            };
            assert_eq!(MerkleTreeEvent::V2(expected_event), event);
            assert_eq!(output_queue.value_vecs[0][0], leaves[0]);
            assert_eq!(ref_merkle_tree.root(), concurrent_mt_with_canopy.root());
        }

        // Functional 2 migrate 2nd leaf
        {
            let input = MigrateLeafParams {
                change_log_index: concurrent_mt_with_canopy.changelog_index() as u64,
                leaf: leaves[1].clone(),
                leaf_index: 1,
                proof: ref_merkle_tree
                    .get_proof_of_leaf(1, false)
                    .unwrap()
                    .to_array()
                    .unwrap(),
            };
            let event = migrate_state(
                input,
                &mut concurrent_mt_with_canopy,
                &merkle_tree_pubkey,
                output_queue,
            )
            .unwrap();
            ref_merkle_tree.update(&[0u8; 32], 1).unwrap();
            let expected_event = NullifierEvent {
                id: merkle_tree_pubkey.to_bytes(),
                nullified_leaves_indices: vec![1],
                seq: concurrent_mt_with_canopy.sequence_number() as u64,
            };
            assert_eq!(MerkleTreeEvent::V2(expected_event), event);
            assert_eq!(output_queue.value_vecs[0][1], leaves[1]);
            assert_eq!(ref_merkle_tree.root(), concurrent_mt_with_canopy.root());
        }
        let input = MigrateLeafParams {
            change_log_index: concurrent_mt_with_canopy.changelog_index() as u64,
            leaf: leaves[2].clone(),
            leaf_index: 2,
            proof: ref_merkle_tree
                .get_proof_of_leaf(2, false)
                .unwrap()
                .to_array()
                .unwrap(),
        };
        // Failing 3 Invalid Proof
        {
            let mut input = input.clone();
            input.proof[0][0] = 1;
            let result = migrate_state(
                input,
                &mut concurrent_mt_with_canopy,
                &merkle_tree_pubkey,
                output_queue,
            );
            result.unwrap_err();
        }
        // Failing 4 Invalid Leaf Index
        {
            let mut input = input.clone();
            input.leaf_index = 100;
            let result = migrate_state(
                input,
                &mut concurrent_mt_with_canopy,
                &merkle_tree_pubkey,
                output_queue,
            );
            result.unwrap_err();
        }
        // Failing 5 Invalid Change Log Index
        {
            let mut input = input.clone();
            input.change_log_index = 100;
            let result = migrate_state(
                input,
                &mut concurrent_mt_with_canopy,
                &merkle_tree_pubkey,
                output_queue,
            );
            result.unwrap_err();
        }
        // Failing 6 Invalid Leaf
        {
            let mut input = input.clone();
            input.leaf[0] = 1;
            let result = migrate_state(
                input,
                &mut concurrent_mt_with_canopy,
                &merkle_tree_pubkey,
                output_queue,
            );
            result.unwrap_err();
        }
        // Failing 6 Empty leaf
        {
            let mut input = input.clone();
            input.leaf = [0u8; 32];
            let result = migrate_state(
                input,
                &mut concurrent_mt_with_canopy,
                &merkle_tree_pubkey,
                output_queue,
            );
            result.unwrap_err();
        }
        // Functional 7 3rd leaf
        {
            let event = migrate_state(
                input,
                &mut concurrent_mt_with_canopy,
                &merkle_tree_pubkey,
                output_queue,
            )
            .unwrap();
            ref_merkle_tree.update(&[0u8; 32], 2).unwrap();
            let expected_event = NullifierEvent {
                id: merkle_tree_pubkey.to_bytes(),
                nullified_leaves_indices: vec![2],
                seq: concurrent_mt_with_canopy.sequence_number() as u64,
            };
            assert_eq!(MerkleTreeEvent::V2(expected_event), event);
            assert_eq!(output_queue.value_vecs[0][2], leaves[2]);
            assert_eq!(ref_merkle_tree.root(), concurrent_mt_with_canopy.root());
        }
    }

    #[test]
    fn test_rnd_migrate_state() {
        let rng = &mut rand::thread_rng();
        let merkle_tree_pubkey = Pubkey::new_unique();
        let mut mt_bytes = vec![
            0u8;
            ConcurrentMerkleTree::<Poseidon, HEIGHT>::size_in_account(
                HEIGHT, CHANGELOG, ROOTS, 10,
            )
        ];
        let mut concurrent_mt_with_canopy = get_concurrent_merkle_tree(&mut mt_bytes);
        let mut ref_merkle_tree =
            light_merkle_tree_reference::MerkleTree::<Poseidon>::new(HEIGHT, 10);
        let mut queue_account = get_output_queue();
        let output_queue = &mut queue_account.account.as_mut().unwrap();
        let batch_size = output_queue.get_metadata().batch_metadata.batch_size as usize;
        // insert two test leaves into the merkle tree

        let num_leaves = 2000;
        let mut leaves = vec![];
        for _ in 0..num_leaves {
            let mut leaf = rng.gen::<[u8; 32]>();
            leaf[0] = 0;
            leaves.push(leaf);
            ref_merkle_tree.append(&leaf).unwrap();
            concurrent_mt_with_canopy.append_batch(&[&leaf]).unwrap();
        }

        assert_eq!(ref_merkle_tree.root(), concurrent_mt_with_canopy.root());

        fn get_rnd_leaf(leaves: &mut Vec<[u8; 32]>, rng: &mut rand::rngs::ThreadRng) -> [u8; 32] {
            let index = rng.gen_range(0..leaves.len());
            leaves.remove(index)
        }

        // Functional 1 migrate 1 leaf
        for i in 0..num_leaves {
            let leaf = get_rnd_leaf(&mut leaves, rng);
            let leaf_index = ref_merkle_tree.get_leaf_index(&leaf).unwrap();
            let input = MigrateLeafParams {
                change_log_index: concurrent_mt_with_canopy.changelog_index() as u64,
                leaf: leaf.clone(),
                leaf_index: leaf_index as u64,
                proof: ref_merkle_tree
                    .get_proof_of_leaf(leaf_index, false)
                    .unwrap()
                    .to_array()
                    .unwrap(),
            };
            let current_batch = output_queue
                .get_metadata()
                .batch_metadata
                .currently_processing_batch_index;

            let event = migrate_state(
                input,
                &mut concurrent_mt_with_canopy,
                &merkle_tree_pubkey,
                output_queue,
            )
            .unwrap();
            ref_merkle_tree.update(&[0u8; 32], leaf_index).unwrap();
            let expected_event = NullifierEvent {
                id: merkle_tree_pubkey.to_bytes(),
                nullified_leaves_indices: vec![leaf_index as u64],
                seq: concurrent_mt_with_canopy.sequence_number() as u64,
            };
            assert_eq!(MerkleTreeEvent::V2(expected_event), event);
            assert_eq!(
                output_queue.value_vecs[current_batch as usize][i % batch_size],
                leaf
            );
            assert_eq!(ref_merkle_tree.root(), concurrent_mt_with_canopy.root());
        }
    }
}
