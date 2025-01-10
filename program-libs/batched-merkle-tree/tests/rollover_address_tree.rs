use light_batched_merkle_tree::{
    batch::Batch,
    initialize_address_tree::{
        init_batched_address_merkle_tree_account, InitAddressTreeAccountsInstructionData,
    },
    initialize_state_tree::assert_address_mt_zero_copy_inited,
    merkle_tree::{
        get_merkle_tree_account_size, get_merkle_tree_account_size_default,
        BatchedMerkleTreeAccount, BatchedMerkleTreeMetadata, CreateTreeParams,
    },
    rollover_address_tree::{assert_address_mt_roll_over, rollover_batched_address_tree},
};
use light_merkle_tree_metadata::errors::MerkleTreeMetadataError;
use light_utils::pubkey::Pubkey;
use light_zero_copy::{
    cyclic_vec::ZeroCopyCyclicVecU64, slice_mut::ZeroCopySliceMutU64, vec::ZeroCopyVecU64,
};
use rand::thread_rng;

/// Test rollover of address tree
/// 1. failing: not ready for rollover
/// 2. failing: already rolled over
/// 3. functional: rollover address tree
/// 4. failing: rollover threshold not set
#[test]
fn test_rollover() {
    let owner = Pubkey::new_unique();

    let mt_account_size = get_merkle_tree_account_size_default();
    let mut mt_account_data = vec![0; mt_account_size];

    let params = InitAddressTreeAccountsInstructionData::test_default();
    let merkle_tree_rent = 1_000_000_000;
    // create first merkle tree

    init_batched_address_merkle_tree_account(
        owner,
        params.clone(),
        &mut mt_account_data,
        merkle_tree_rent,
    )
    .unwrap();

    let create_tree_params = CreateTreeParams::from_address_ix_params(params, owner);

    let ref_mt_account =
        BatchedMerkleTreeMetadata::new_address_tree(create_tree_params, merkle_tree_rent);
    assert_address_mt_zero_copy_inited(
        &mut mt_account_data,
        ref_mt_account,
        params.bloom_filter_num_iters,
    );

    let mut new_mt_account_data = vec![0; mt_account_size];
    let new_mt_pubkey = Pubkey::new_unique();

    // 1. Failing: not ready for rollover
    {
        let mut mt_account_data = mt_account_data.clone();
        let result = rollover_batched_address_tree(
            &mut BatchedMerkleTreeAccount::address_tree_from_bytes_mut(&mut mt_account_data)
                .unwrap(),
            &mut new_mt_account_data,
            merkle_tree_rent,
            new_mt_pubkey,
            params.network_fee,
        );
        assert_eq!(
            result,
            Err(MerkleTreeMetadataError::NotReadyForRollover.into())
        );
    }
    // 2. Failing rollover threshold not set
    {
        let mut mt_account_data = mt_account_data.clone();
        let merkle_tree =
            &mut BatchedMerkleTreeAccount::address_tree_from_bytes_mut(&mut mt_account_data)
                .unwrap();
        merkle_tree
            .get_metadata_mut()
            .metadata
            .rollover_metadata
            .rollover_threshold = u64::MAX;
        let result = rollover_batched_address_tree(
            merkle_tree,
            &mut new_mt_account_data,
            merkle_tree_rent,
            new_mt_pubkey,
            params.network_fee,
        );
        assert_eq!(
            result,
            Err(MerkleTreeMetadataError::RolloverNotConfigured.into())
        );
    }
    // 3. Functional: rollover address tree
    {
        let merkle_tree =
            &mut BatchedMerkleTreeAccount::address_tree_from_bytes_mut(&mut mt_account_data)
                .unwrap();
        merkle_tree.get_metadata_mut().next_index = 1 << merkle_tree.get_metadata().height;

        rollover_batched_address_tree(
            merkle_tree,
            &mut new_mt_account_data,
            merkle_tree_rent,
            new_mt_pubkey,
            params.network_fee,
        )
        .unwrap();
        let new_ref_mt_account = ref_mt_account.clone();

        let mut ref_rolledover_mt = ref_mt_account.clone();
        ref_rolledover_mt.next_index = 1 << ref_rolledover_mt.height;
        assert_address_mt_roll_over(
            mt_account_data.to_vec(),
            ref_rolledover_mt,
            new_mt_account_data.to_vec(),
            new_ref_mt_account,
            new_mt_pubkey,
            params.bloom_filter_num_iters,
        );
    }
    // 4. Failing: already rolled over
    {
        let mut mt_account_data = mt_account_data.clone();
        let mut new_mt_account_data = vec![0; mt_account_size];

        let result = rollover_batched_address_tree(
            &mut BatchedMerkleTreeAccount::address_tree_from_bytes_mut(&mut mt_account_data)
                .unwrap(),
            &mut new_mt_account_data,
            merkle_tree_rent,
            new_mt_pubkey,
            params.network_fee,
        );
        assert_eq!(
            result,
            Err(MerkleTreeMetadataError::MerkleTreeAlreadyRolledOver.into())
        );
    }
}

#[test]
fn test_rnd_rollover() {
    use rand::Rng;
    let mut rng = thread_rng();
    for _ in 0..10000 {
        let owner = Pubkey::new_unique();

        let program_owner = if rng.gen_bool(0.5) {
            Some(Pubkey::new_unique())
        } else {
            None
        };
        let forester = if rng.gen_bool(0.5) {
            Some(Pubkey::new_unique())
        } else {
            None
        };
        let input_queue_zkp_batch_size = rng.gen_range(1..1000);

        let mut params = InitAddressTreeAccountsInstructionData {
            index: rng.gen_range(0..1000),
            program_owner,
            forester,
            bloom_filter_num_iters: rng.gen_range(0..4),
            input_queue_batch_size: rng.gen_range(1..1000) * input_queue_zkp_batch_size,
            input_queue_zkp_batch_size,
            // 8 bits per byte, divisible by 8 for aligned memory
            bloom_filter_capacity: rng.gen_range(0..100) * 8 * 8,
            network_fee: Some(rng.gen_range(1..1000)),
            rollover_threshold: Some(rng.gen_range(0..100)),
            close_threshold: None,
            root_history_capacity: rng.gen_range(1..1000),
            input_queue_num_batches: rng.gen_range(1..4),
            height: rng.gen_range(1..32),
        };
        if forester.is_some() {
            params.network_fee = None;
        }

        use std::mem::size_of;

        let mt_account_size = get_merkle_tree_account_size(
            params.input_queue_batch_size,
            params.bloom_filter_capacity,
            params.input_queue_zkp_batch_size,
            params.root_history_capacity,
            params.height,
            params.input_queue_num_batches,
        );
        {
            let num_zkp_batches = params.input_queue_batch_size / params.input_queue_zkp_batch_size;
            let num_batches = params.input_queue_num_batches as usize;
            let batch_size =
                size_of::<Batch>() * num_batches + ZeroCopySliceMutU64::<Batch>::metadata_size();
            let bloom_filter_size = (params.bloom_filter_capacity as usize / 8
                + ZeroCopySliceMutU64::<u8>::metadata_size())
                * num_batches;
            let hash_chain_store_size = (num_zkp_batches as usize * 32
                + ZeroCopyVecU64::<[u8; 32]>::metadata_size())
                * num_batches;
            let root_history_size = params.root_history_capacity as usize * 32
                + ZeroCopyCyclicVecU64::<[u8; 32]>::metadata_size();
            // Output queue
            let ref_account_size =
                // metadata
                BatchedMerkleTreeMetadata::LEN
                + root_history_size
                + batch_size
                + bloom_filter_size
                // 2 hash chain stores
                + hash_chain_store_size;
            assert_eq!(mt_account_size, ref_account_size);
        }
        let mut mt_account_data = vec![0; mt_account_size];

        let merkle_tree_rent = rng.gen_range(0..10000000);

        init_batched_address_merkle_tree_account(
            owner,
            params.clone(),
            &mut mt_account_data,
            merkle_tree_rent,
        )
        .unwrap();
        let create_tree_params = CreateTreeParams::from_address_ix_params(params, owner);

        let ref_mt_account =
            BatchedMerkleTreeMetadata::new_address_tree(create_tree_params, merkle_tree_rent);
        assert_address_mt_zero_copy_inited(
            &mut mt_account_data,
            ref_mt_account,
            params.bloom_filter_num_iters,
        );
        let mut new_mt_data = vec![0; mt_account_size];
        let new_mt_rent = merkle_tree_rent;
        let network_fee = params.network_fee;
        let new_mt_pubkey = Pubkey::new_unique();
        let mut zero_copy_old_mt =
            BatchedMerkleTreeAccount::address_tree_from_bytes_mut(&mut mt_account_data).unwrap();
        zero_copy_old_mt.get_metadata_mut().next_index = 1 << params.height;
        rollover_batched_address_tree(
            &mut zero_copy_old_mt,
            &mut new_mt_data,
            new_mt_rent,
            new_mt_pubkey,
            network_fee,
        )
        .unwrap();
        let new_ref_mt_account = ref_mt_account.clone();
        let mut ref_rolled_over_account = ref_mt_account.clone();
        ref_rolled_over_account.next_index = 1 << params.height;

        assert_address_mt_roll_over(
            mt_account_data,
            ref_rolled_over_account,
            new_mt_data,
            new_ref_mt_account,
            new_mt_pubkey,
            params.bloom_filter_num_iters,
        );
    }
}
