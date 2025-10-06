use light_batched_merkle_tree::{
    constants::NUM_BATCHES,
    errors::BatchedMerkleTreeError,
    initialize_address_tree::{
        init_batched_address_merkle_tree_account,
        test_utils::InitAddressTreeAccountsInstructionData,
    },
    initialize_state_tree::test_utils::assert_address_mt_zero_copy_initialized,
    merkle_tree::{
        get_merkle_tree_account_size, test_utils::get_merkle_tree_account_size_default,
        BatchedMerkleTreeAccount,
    },
    merkle_tree_metadata::{BatchedMerkleTreeMetadata, CreateTreeParams},
    rollover_address_tree::{
        rollover_batched_address_tree, test_utils::assert_address_mt_roll_over,
    },
    rollover_state_tree::batched_tree_is_ready_for_rollover,
};
use light_compressed_account::{pubkey::Pubkey, TreeType};
use light_merkle_tree_metadata::{
    errors::MerkleTreeMetadataError, merkle_tree::MerkleTreeMetadata, rollover::RolloverMetadata,
};
use light_zero_copy::{cyclic_vec::ZeroCopyCyclicVecU64, vec::ZeroCopyVecU64};
use rand::thread_rng;

/// Test rollover of address tree
/// 1. failing: not ready for rollover
/// 2. failing: already rolled over
/// 3. functional: rollover address tree
/// 4. failing: rollover threshold not set
#[test]
fn test_rollover() {
    let owner = Pubkey::new_unique();
    let mt_pubkey = Pubkey::new_unique();

    let mt_account_size = get_merkle_tree_account_size_default();
    let mut mt_account_data = vec![0; mt_account_size];

    let params = InitAddressTreeAccountsInstructionData::test_default();
    let merkle_tree_rent = 1_000_000_000;
    // create first merkle tree

    init_batched_address_merkle_tree_account(
        owner,
        params,
        &mut mt_account_data,
        merkle_tree_rent,
        mt_pubkey,
    )
    .unwrap();

    let create_tree_params = CreateTreeParams::from_address_ix_params(params, owner, mt_pubkey);

    let ref_mt_account =
        BatchedMerkleTreeMetadata::new_address_tree(create_tree_params, merkle_tree_rent);
    assert_address_mt_zero_copy_initialized(&mut mt_account_data, ref_mt_account, &mt_pubkey);

    let mut new_mt_account_data = vec![0; mt_account_size];
    let new_mt_pubkey = Pubkey::new_unique();
    println!("pre 1");
    // 1. Failing: not ready for rollover
    {
        let mut mt_account_data = mt_account_data.clone();
        let result = rollover_batched_address_tree(
            &mut BatchedMerkleTreeAccount::address_from_bytes(&mut mt_account_data, &mt_pubkey)
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
    println!("pre 2");
    // 2. Failing rollover threshold not set
    {
        let mut mt_account_data = mt_account_data.clone();
        let merkle_tree =
            &mut BatchedMerkleTreeAccount::address_from_bytes(&mut mt_account_data, &mt_pubkey)
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
    println!("pre 3");
    // 3. Functional: rollover address tree
    {
        let pre_mt_data = mt_account_data.clone();
        let merkle_tree =
            &mut BatchedMerkleTreeAccount::address_from_bytes(&mut mt_account_data, &mt_pubkey)
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
        let create_tree_params =
            CreateTreeParams::from_address_ix_params(params, owner, new_mt_pubkey);

        let new_ref_mt_account =
            BatchedMerkleTreeMetadata::new_address_tree(create_tree_params, merkle_tree_rent);
        let mut ref_rolledover_mt = ref_mt_account;
        ref_rolledover_mt.next_index = 1 << ref_rolledover_mt.height;
        assert_eq!(
            pre_mt_data[size_of::<BatchedMerkleTreeMetadata>()..],
            mt_account_data[size_of::<BatchedMerkleTreeMetadata>()..],
            "remainder of old_mt_account_data is not changed"
        );
        assert_address_mt_roll_over(
            mt_account_data.to_vec(),
            ref_rolledover_mt,
            mt_pubkey,
            new_mt_account_data.to_vec(),
            new_ref_mt_account,
            new_mt_pubkey,
        );
    }
    // 4. Failing: already rolled over
    {
        let mut mt_account_data = mt_account_data.clone();
        let mut new_mt_account_data = vec![0; mt_account_size];

        let result = rollover_batched_address_tree(
            &mut BatchedMerkleTreeAccount::address_from_bytes(&mut mt_account_data, &mt_pubkey)
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
    for _ in 0..1000 {
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
            height: 40,
        };
        if forester.is_some() {
            params.network_fee = None;
        }

        let mt_account_size = get_merkle_tree_account_size(
            params.input_queue_batch_size,
            params.bloom_filter_capacity,
            params.input_queue_zkp_batch_size,
            params.root_history_capacity,
            params.height,
        );
        {
            let num_zkp_batches = params.input_queue_batch_size / params.input_queue_zkp_batch_size;
            let num_batches = NUM_BATCHES;

            let bloom_filter_size = (params.bloom_filter_capacity / 8) as usize * num_batches;
            let hash_chain_store_size =
                ZeroCopyVecU64::<[u8; 32]>::required_size_for_capacity(num_zkp_batches)
                    * num_batches;
            let root_history_size = ZeroCopyCyclicVecU64::<[u8; 32]>::required_size_for_capacity(
                params.root_history_capacity as u64,
            );
            // Output queue
            let ref_account_size =
                // metadata
                BatchedMerkleTreeMetadata::LEN
                + root_history_size
                + bloom_filter_size
                // 2 hash chain stores
                + hash_chain_store_size;
            assert_eq!(mt_account_size, ref_account_size);
        }
        let mut mt_account_data = vec![0; mt_account_size];
        let mt_pubkey = Pubkey::new_unique();
        let merkle_tree_rent = rng.gen_range(0..10000000);

        init_batched_address_merkle_tree_account(
            owner,
            params,
            &mut mt_account_data,
            merkle_tree_rent,
            mt_pubkey,
        )
        .unwrap();
        let create_tree_params = CreateTreeParams::from_address_ix_params(params, owner, mt_pubkey);

        let ref_mt_account =
            BatchedMerkleTreeMetadata::new_address_tree(create_tree_params, merkle_tree_rent);
        assert_address_mt_zero_copy_initialized(&mut mt_account_data, ref_mt_account, &mt_pubkey);
        let mut new_mt_data = vec![0; mt_account_size];
        let new_mt_rent = merkle_tree_rent;
        let network_fee = params.network_fee;
        let new_mt_pubkey = Pubkey::new_unique();
        let mut zero_copy_old_mt =
            BatchedMerkleTreeAccount::address_from_bytes(&mut mt_account_data, &mt_pubkey).unwrap();
        zero_copy_old_mt.get_metadata_mut().next_index = 1 << params.height;
        rollover_batched_address_tree(
            &mut zero_copy_old_mt,
            &mut new_mt_data,
            new_mt_rent,
            new_mt_pubkey,
            network_fee,
        )
        .unwrap();
        let create_tree_params =
            CreateTreeParams::from_address_ix_params(params, owner, new_mt_pubkey);

        let new_ref_mt_account =
            BatchedMerkleTreeMetadata::new_address_tree(create_tree_params, merkle_tree_rent);
        let mut ref_rolled_over_account = ref_mt_account;
        ref_rolled_over_account.next_index = 1 << params.height;

        assert_address_mt_roll_over(
            mt_account_data,
            ref_rolled_over_account,
            mt_pubkey,
            new_mt_data,
            new_ref_mt_account,
            new_mt_pubkey,
        );
    }
}

/// Test if the tree is ready for rollover
/// 1. failing: empty tree is not ready for rollover
/// 2. failing: not ready for rollover next_index == rollover_threshold - 1
/// 3. functional: ready for rollover next_index == rollover_threshold
/// 4. functional: ready for rollover next_index >= rollover_threshold
/// 5. failing: network fee must not be set if it was 0 before
/// 6. failing: rollower threshold not set
#[test]
fn test_batched_tree_is_ready_for_rollover() {
    let mut account_data = vec![0u8; 6256];
    let batch_size = 50;
    let zkp_batch_size = 1;
    let root_history_len = 10;
    let num_iter = 1;
    let bloom_filter_capacity = 8000;
    let height = 4;
    let metadata = MerkleTreeMetadata {
        rollover_metadata: RolloverMetadata {
            rollover_threshold: 75,
            ..Default::default()
        },
        ..Default::default()
    };
    let mt_pubkey = Pubkey::new_unique();

    let mut account = BatchedMerkleTreeAccount::init(
        &mut account_data,
        &mt_pubkey,
        metadata,
        root_history_len,
        batch_size,
        zkp_batch_size,
        height,
        num_iter,
        bloom_filter_capacity,
        TreeType::StateV2,
    )
    .unwrap();

    // 1. Failing: empty tree is not ready for rollover
    assert_eq!(
        batched_tree_is_ready_for_rollover(&account, &None),
        Err(MerkleTreeMetadataError::NotReadyForRollover.into())
    );

    let tree_capacity = 2u64.pow(height);
    let start_index = 0;
    let rollover_threshold =
        tree_capacity * metadata.rollover_metadata.rollover_threshold / 100 - start_index;
    // fill tree almost to the rollover threshold
    for _ in 0..rollover_threshold - 1 {
        account.next_index += 1;
    }

    // 2. Failing: not ready for rollover next_index == rollover_threshold - 1
    assert_eq!(
        batched_tree_is_ready_for_rollover(&account, &None),
        Err(MerkleTreeMetadataError::NotReadyForRollover.into())
    );

    // 3. Functional: ready for rollover next_index == rollover_threshold
    {
        account.next_index += 1;
        assert!(batched_tree_is_ready_for_rollover(&account, &None).is_ok());
    }

    // 4. Functional: ready for rollover next_index >= rollover_threshold
    for _ in 0..tree_capacity - rollover_threshold - 1 {
        account.next_index += 1;
        assert!(batched_tree_is_ready_for_rollover(&account, &None).is_ok());
    }

    // 5. Failing: network fee must not be set if it was 0 before
    assert_eq!(
        batched_tree_is_ready_for_rollover(&account, &Some(1)),
        Err(BatchedMerkleTreeError::InvalidNetworkFee)
    );

    // 6. Failing: rollower threshold not set
    account
        .get_metadata_mut()
        .metadata
        .rollover_metadata
        .rollover_threshold = u64::MAX;
    assert_eq!(
        batched_tree_is_ready_for_rollover(&account, &None),
        Err(MerkleTreeMetadataError::RolloverNotConfigured.into())
    );
}
