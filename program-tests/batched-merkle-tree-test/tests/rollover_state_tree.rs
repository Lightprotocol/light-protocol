use light_batched_merkle_tree::{
    errors::BatchedMerkleTreeError,
    initialize_state_tree::{
        init_batched_state_merkle_tree_accounts,
        test_utils::{
            assert_state_mt_zero_copy_initialized, create_output_queue_account,
            CreateOutputQueueParams,
        },
        InitStateTreeAccountsInstructionData,
    },
    merkle_tree::{
        get_merkle_tree_account_size, test_utils::get_merkle_tree_account_size_default,
        BatchedMerkleTreeAccount,
    },
    merkle_tree_metadata::{BatchedMerkleTreeMetadata, CreateTreeParams},
    queue::{
        get_output_queue_account_size,
        test_utils::{assert_queue_zero_copy_inited, get_output_queue_account_size_default},
        BatchedQueueAccount,
    },
    rollover_state_tree::{
        rollover_batched_state_tree,
        test_utils::{assert_state_mt_roll_over, StateMtRollOverAssertParams},
        RolloverBatchStateTreeParams,
    },
};
use light_compressed_account::pubkey::Pubkey;
use light_merkle_tree_metadata::errors::MerkleTreeMetadataError;
use light_zero_copy::errors::ZeroCopyError;
use rand::{
    rngs::{StdRng, ThreadRng},
    Rng,
};

/// Test rollover of state tree
/// 1. failing: not ready for rollover
/// 2. failing: already rolled over
/// 3. failing: mt size mismatch
/// 4. failing: queue size mismatch
/// 5. failing: invalid network fee
/// 5. functional: rollover address tree
/// 6. failing: rollover threshold not set
/// 7. failing: invalid network fee
/// 8. functional: rollover address tree with network fee 0 additional bytes 0
#[test]
fn test_rollover() {
    let owner = Pubkey::new_unique();

    {
        let mt_account_size = get_merkle_tree_account_size_default();
        let mut mt_account_data = vec![0; mt_account_size];
        let mt_pubkey = Pubkey::new_unique();

        let queue_account_size = get_output_queue_account_size_default();
        let mut queue_account_data = vec![0; queue_account_size];
        let queue_pubkey = Pubkey::new_unique();
        let params = InitStateTreeAccountsInstructionData::test_default();
        let merkle_tree_rent = 1_000_000_000;
        let queue_rent = 1_000_000_001;
        let additional_bytes_rent = 1_000_000_002;
        let additional_bytes = 1_000;
        // create first merkle tree

        init_batched_state_merkle_tree_accounts(
            owner,
            params,
            &mut queue_account_data,
            queue_pubkey,
            queue_rent,
            &mut mt_account_data,
            mt_pubkey,
            merkle_tree_rent,
            additional_bytes_rent,
        )
        .unwrap();

        let create_tree_params = CreateTreeParams::from_state_ix_params(params, owner, mt_pubkey);
        let ref_mt_account =
            BatchedMerkleTreeMetadata::new_state_tree(create_tree_params, queue_pubkey);
        assert_state_mt_zero_copy_initialized(&mut mt_account_data, ref_mt_account, &mt_pubkey);
        let total_rent = merkle_tree_rent + additional_bytes_rent + queue_rent;
        let output_queue_params =
            CreateOutputQueueParams::from(params, owner, total_rent, mt_pubkey, queue_pubkey);
        let ref_output_queue_account = create_output_queue_account(output_queue_params);
        assert_queue_zero_copy_inited(queue_account_data.as_mut_slice(), ref_output_queue_account);
        let mut new_mt_account_data = vec![0; mt_account_size];
        let new_mt_pubkey = Pubkey::new_unique();

        let mut new_queue_account_data = vec![0; queue_account_size];
        let new_output_queue_pubkey = Pubkey::new_unique();
        println!("pre 1");
        // 1. Failing: not ready for rollover
        {
            let mut mt_account_data = mt_account_data.clone();
            let mut queue_account_data = queue_account_data.clone();
            let params = RolloverBatchStateTreeParams {
                old_merkle_tree: &mut BatchedMerkleTreeAccount::state_from_bytes(
                    &mut mt_account_data,
                    &mt_pubkey,
                )
                .unwrap(),
                old_mt_pubkey: mt_pubkey,
                new_mt_data: &mut new_mt_account_data,
                new_mt_rent: merkle_tree_rent,
                new_mt_pubkey,
                old_output_queue: &mut BatchedQueueAccount::output_from_bytes(
                    &mut queue_account_data,
                )
                .unwrap(),
                old_queue_pubkey: queue_pubkey,
                new_output_queue_data: &mut new_queue_account_data,
                new_output_queue_rent: queue_rent,
                new_output_queue_pubkey,
                additional_bytes_rent,
                additional_bytes,
                network_fee: params.network_fee,
            };
            let result = rollover_batched_state_tree(params);

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
                &mut BatchedMerkleTreeAccount::state_from_bytes(&mut mt_account_data, &mt_pubkey)
                    .unwrap();
            merkle_tree
                .get_metadata_mut()
                .metadata
                .rollover_metadata
                .rollover_threshold = u64::MAX;
            let mut queue_account_data = queue_account_data.clone();
            let params = RolloverBatchStateTreeParams {
                old_merkle_tree: &mut BatchedMerkleTreeAccount::state_from_bytes(
                    &mut mt_account_data,
                    &mt_pubkey,
                )
                .unwrap(),
                old_mt_pubkey: mt_pubkey,
                new_mt_data: &mut new_mt_account_data,
                new_mt_rent: merkle_tree_rent,
                new_mt_pubkey,
                old_output_queue: &mut BatchedQueueAccount::output_from_bytes(
                    &mut queue_account_data,
                )
                .unwrap(),
                old_queue_pubkey: queue_pubkey,
                new_output_queue_data: &mut new_queue_account_data,
                new_output_queue_rent: queue_rent,
                new_output_queue_pubkey,
                additional_bytes_rent,
                additional_bytes,
                network_fee: params.network_fee,
            };
            let result = rollover_batched_state_tree(params);
            assert_eq!(
                result,
                Err(MerkleTreeMetadataError::RolloverNotConfigured.into())
            );
        }
        println!("pre 3");
        // 3. Failing: invalid mt size
        {
            let mut mt_account_data = mt_account_data.clone();
            let mut queue_account_data = queue_account_data.clone();
            let merkle_tree =
                &mut BatchedMerkleTreeAccount::state_from_bytes(&mut mt_account_data, &mt_pubkey)
                    .unwrap();
            merkle_tree.get_metadata_mut().next_index = 1 << merkle_tree.get_metadata().height;
            let mut new_mt_account_data = vec![0; mt_account_size - 1];
            let mut new_queue_account_data = vec![0; queue_account_size];

            let params = RolloverBatchStateTreeParams {
                old_merkle_tree: merkle_tree,
                old_mt_pubkey: mt_pubkey,
                new_mt_data: &mut new_mt_account_data,
                new_mt_rent: merkle_tree_rent,
                new_mt_pubkey,
                old_output_queue: &mut BatchedQueueAccount::output_from_bytes(
                    &mut queue_account_data,
                )
                .unwrap(),
                old_queue_pubkey: queue_pubkey,
                new_output_queue_data: &mut new_queue_account_data,
                new_output_queue_rent: queue_rent,
                new_output_queue_pubkey,
                additional_bytes_rent,
                additional_bytes,
                network_fee: params.network_fee,
            };
            let result = rollover_batched_state_tree(params);
            assert_eq!(result, Err(ZeroCopyError::Size.into()));
        }
        println!("pre 4");
        // 4. Failing: invalid queue size
        {
            let mut mt_account_data = mt_account_data.clone();
            let mut queue_account_data = queue_account_data.clone();
            let merkle_tree =
                &mut BatchedMerkleTreeAccount::state_from_bytes(&mut mt_account_data, &mt_pubkey)
                    .unwrap();
            merkle_tree.get_metadata_mut().next_index = 1 << merkle_tree.get_metadata().height;
            let mut new_mt_account_data = vec![0; mt_account_size];
            let mut new_queue_account_data = vec![0; queue_account_size - 1];

            let params = RolloverBatchStateTreeParams {
                old_merkle_tree: merkle_tree,
                old_mt_pubkey: mt_pubkey,
                new_mt_data: &mut new_mt_account_data,
                new_mt_rent: merkle_tree_rent,
                new_mt_pubkey,
                old_output_queue: &mut BatchedQueueAccount::output_from_bytes(
                    &mut queue_account_data,
                )
                .unwrap(),
                old_queue_pubkey: queue_pubkey,
                new_output_queue_data: &mut new_queue_account_data,
                new_output_queue_rent: queue_rent,
                new_output_queue_pubkey,
                additional_bytes_rent,
                additional_bytes,
                network_fee: params.network_fee,
            };
            let result = rollover_batched_state_tree(params);
            assert_eq!(result, Err(ZeroCopyError::Size.into()));
        }
        println!("pre 5");
        // 5. Functional: rollover address tree
        {
            let merkle_tree =
                &mut BatchedMerkleTreeAccount::state_from_bytes(&mut mt_account_data, &mt_pubkey)
                    .unwrap();
            let height = merkle_tree.get_metadata().height;
            merkle_tree.get_metadata_mut().next_index = 1 << height;
            println!("new_mt_pubkey {:?}", new_mt_pubkey);
            println!("new_output_queue_pubkey {:?}", new_output_queue_pubkey);
            let rollover_batch_state_tree_params = RolloverBatchStateTreeParams {
                old_merkle_tree: merkle_tree,
                old_mt_pubkey: mt_pubkey,
                new_mt_data: &mut new_mt_account_data,
                new_mt_rent: merkle_tree_rent,
                new_mt_pubkey,
                old_output_queue: &mut BatchedQueueAccount::output_from_bytes(
                    &mut queue_account_data,
                )
                .unwrap(),
                old_queue_pubkey: queue_pubkey,
                new_output_queue_data: &mut new_queue_account_data,
                new_output_queue_rent: queue_rent,
                new_output_queue_pubkey,
                additional_bytes_rent,
                additional_bytes,
                network_fee: params.network_fee,
            };
            rollover_batched_state_tree(rollover_batch_state_tree_params).unwrap();

            let mut ref_rolledover_mt = ref_mt_account;
            ref_rolledover_mt.next_index = 1 << height;

            let output_queue_params = CreateOutputQueueParams::from(
                params,
                owner,
                total_rent,
                new_mt_pubkey,
                new_output_queue_pubkey,
            );
            let mut new_ref_output_queue_account = create_output_queue_account(output_queue_params);
            new_ref_output_queue_account
                .metadata
                .rollover_metadata
                .additional_bytes = additional_bytes;
            let create_tree_params =
                CreateTreeParams::from_state_ix_params(params, owner, new_mt_pubkey);
            let new_ref_merkle_tree_account = BatchedMerkleTreeMetadata::new_state_tree(
                create_tree_params,
                new_output_queue_pubkey,
            );
            let assert_state_mt_roll_over_params = StateMtRollOverAssertParams {
                mt_account_data: mt_account_data.to_vec(),
                ref_mt_account: new_ref_merkle_tree_account,
                new_mt_account_data: new_mt_account_data.to_vec(),
                old_mt_pubkey: mt_pubkey,
                new_mt_pubkey,
                ref_rolledover_mt,
                queue_account_data: queue_account_data.to_vec(),
                ref_queue_account: new_ref_output_queue_account,
                new_queue_account_data: new_queue_account_data.to_vec(),
                new_queue_pubkey: new_output_queue_pubkey,
                ref_rolledover_queue: ref_output_queue_account,
                old_queue_pubkey: queue_pubkey,
                slot: 1,
            };

            assert_state_mt_roll_over(assert_state_mt_roll_over_params);
        }
        println!("pre 6");
        // 6. Failing: already rolled over
        {
            let mut mt_account_data = mt_account_data.clone();
            let mut queue_account_data = queue_account_data.clone();

            let mut new_mt_account_data = vec![0; mt_account_size];
            let mut new_queue_account_data = vec![0; queue_account_size];

            let params = RolloverBatchStateTreeParams {
                old_merkle_tree: &mut BatchedMerkleTreeAccount::state_from_bytes(
                    &mut mt_account_data,
                    &mt_pubkey,
                )
                .unwrap(),
                old_mt_pubkey: mt_pubkey,
                new_mt_data: &mut new_mt_account_data,
                new_mt_rent: merkle_tree_rent,
                new_mt_pubkey,
                old_output_queue: &mut BatchedQueueAccount::output_from_bytes(
                    &mut queue_account_data,
                )
                .unwrap(),
                old_queue_pubkey: queue_pubkey,
                new_output_queue_data: &mut new_queue_account_data,
                new_output_queue_rent: queue_rent,
                new_output_queue_pubkey,
                additional_bytes_rent,
                additional_bytes,
                network_fee: params.network_fee,
            };
            let result = rollover_batched_state_tree(params);
            assert_eq!(
                result,
                Err(MerkleTreeMetadataError::MerkleTreeAlreadyRolledOver.into())
            );
        }
    }
    println!("pre 7");
    {
        let mt_account_size = get_merkle_tree_account_size_default();
        let mut mt_account_data = vec![0; mt_account_size];
        let mt_pubkey = Pubkey::new_unique();

        let queue_account_size = get_output_queue_account_size_default();
        let mut queue_account_data = vec![0; queue_account_size];
        let queue_pubkey = Pubkey::new_unique();
        let forester = Pubkey::new_unique();
        let mut params = InitStateTreeAccountsInstructionData::test_default();
        params.network_fee = None;
        params.forester = Some(forester);
        let merkle_tree_rent = 1_000_000_000;
        let queue_rent = 1_000_000_001;
        let additional_bytes_rent = 0;
        let additional_bytes = 0;
        // create first merkle tree

        init_batched_state_merkle_tree_accounts(
            owner,
            params,
            &mut queue_account_data,
            queue_pubkey,
            queue_rent,
            &mut mt_account_data,
            mt_pubkey,
            merkle_tree_rent,
            additional_bytes_rent,
        )
        .unwrap();
        let new_mt_pubkey = Pubkey::new_unique();
        let new_output_queue_pubkey = Pubkey::new_unique();
        println!("pre 7.1");
        // 7. failing Invalid network fee
        {
            let mut mt_account_data = mt_account_data.clone();
            let mut queue_account_data = queue_account_data.clone();
            let merkle_tree =
                &mut BatchedMerkleTreeAccount::state_from_bytes(&mut mt_account_data, &mt_pubkey)
                    .unwrap();
            merkle_tree.get_metadata_mut().next_index = 1 << merkle_tree.get_metadata().height;
            let mut new_mt_account_data = vec![0; mt_account_size];
            let mut new_queue_account_data = vec![0; queue_account_size];

            let params = RolloverBatchStateTreeParams {
                old_merkle_tree: merkle_tree,
                old_mt_pubkey: mt_pubkey,
                new_mt_data: &mut new_mt_account_data,
                new_mt_rent: merkle_tree_rent,
                new_mt_pubkey,
                old_output_queue: &mut BatchedQueueAccount::output_from_bytes(
                    &mut queue_account_data,
                )
                .unwrap(),
                old_queue_pubkey: queue_pubkey,
                new_output_queue_data: &mut new_queue_account_data,
                new_output_queue_rent: queue_rent,
                new_output_queue_pubkey,
                additional_bytes_rent,
                additional_bytes,
                network_fee: Some(1),
            };
            let result = rollover_batched_state_tree(params);
            assert_eq!(result, Err(BatchedMerkleTreeError::InvalidNetworkFee));
        }
        let mut new_mt_account_data = vec![0; mt_account_size];
        let mut new_queue_account_data = vec![0; queue_account_size];
        let create_tree_params = CreateTreeParams::from_state_ix_params(params, owner, mt_pubkey);

        let mut ref_mt_account =
            BatchedMerkleTreeMetadata::new_state_tree(create_tree_params, queue_pubkey);
        ref_mt_account.metadata.access_metadata.forester = forester;
        let total_rent = merkle_tree_rent + additional_bytes_rent + queue_rent;
        let output_queue_params =
            CreateOutputQueueParams::from(params, owner, total_rent, mt_pubkey, queue_pubkey);
        let mut ref_output_queue_account = create_output_queue_account(output_queue_params);
        ref_output_queue_account
            .metadata
            .rollover_metadata
            .network_fee = 0;
        ref_output_queue_account.metadata.access_metadata.forester = forester;
        assert_queue_zero_copy_inited(queue_account_data.as_mut_slice(), ref_output_queue_account);
        println!("pre 8");
        // 8. Functional: rollover address tree with network fee 0 additional bytes 0
        {
            let pre_mt_data = mt_account_data.clone();
            let merkle_tree =
                &mut BatchedMerkleTreeAccount::state_from_bytes(&mut mt_account_data, &mt_pubkey)
                    .unwrap();
            let height = merkle_tree.get_metadata().height;
            merkle_tree.get_metadata_mut().next_index = 1 << height;
            println!("new_mt_pubkey {:?}", new_mt_pubkey);
            println!("new_output_queue_pubkey {:?}", new_output_queue_pubkey);
            let rollover_batch_state_tree_params = RolloverBatchStateTreeParams {
                old_merkle_tree: merkle_tree,
                old_mt_pubkey: mt_pubkey,
                new_mt_data: &mut new_mt_account_data,
                new_mt_rent: merkle_tree_rent,
                new_mt_pubkey,
                old_output_queue: &mut BatchedQueueAccount::output_from_bytes(
                    &mut queue_account_data,
                )
                .unwrap(),
                old_queue_pubkey: queue_pubkey,
                new_output_queue_data: &mut new_queue_account_data,
                new_output_queue_rent: queue_rent,
                new_output_queue_pubkey,
                additional_bytes_rent,
                additional_bytes,
                network_fee: params.network_fee,
            };
            rollover_batched_state_tree(rollover_batch_state_tree_params).unwrap();

            let mut ref_rolledover_mt = ref_mt_account;
            ref_rolledover_mt.next_index = 1 << height;

            let output_queue_params = CreateOutputQueueParams::from(
                params,
                owner,
                total_rent,
                new_mt_pubkey,
                new_output_queue_pubkey,
            );
            let mut new_ref_output_queue_account = create_output_queue_account(output_queue_params);
            new_ref_output_queue_account
                .metadata
                .rollover_metadata
                .additional_bytes = additional_bytes;
            let create_tree_params =
                CreateTreeParams::from_state_ix_params(params, owner, new_mt_pubkey);
            let new_ref_merkle_tree_account = BatchedMerkleTreeMetadata::new_state_tree(
                create_tree_params,
                new_output_queue_pubkey,
            );
            let assert_state_mt_roll_over_params = StateMtRollOverAssertParams {
                mt_account_data: mt_account_data.to_vec(),
                ref_mt_account: new_ref_merkle_tree_account,
                new_mt_account_data: new_mt_account_data.to_vec(),
                old_mt_pubkey: mt_pubkey,
                new_mt_pubkey,
                ref_rolledover_mt,
                queue_account_data: queue_account_data.to_vec(),
                ref_queue_account: new_ref_output_queue_account,
                new_queue_account_data: new_queue_account_data.to_vec(),
                new_queue_pubkey: new_output_queue_pubkey,
                ref_rolledover_queue: ref_output_queue_account,
                old_queue_pubkey: queue_pubkey,
                slot: 1,
            };
            assert_eq!(
                pre_mt_data[size_of::<BatchedMerkleTreeMetadata>()..],
                mt_account_data[size_of::<BatchedMerkleTreeMetadata>()..],
                "remainder of old_mt_account_data is not changed"
            );
            assert_state_mt_roll_over(assert_state_mt_roll_over_params);
        }
    }
}

#[test]
fn test_rnd_rollover() {
    use rand::SeedableRng;
    let seed: u64 = ThreadRng::default().gen();
    println!("seed {}", seed);
    let mut rng = StdRng::seed_from_u64(seed);
    for _ in 0..1000 {
        println!("next iter ------------------------------------");
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
        let output_queue_zkp_batch_size = rng.gen_range(1..1000);
        let network_fee = if rng.gen_bool(0.5) && forester.is_some() {
            None
        } else {
            Some(rng.gen_range(1..1000))
        };

        let params = InitStateTreeAccountsInstructionData {
            index: rng.gen_range(0..1000),
            program_owner,
            forester,
            additional_bytes: rng.gen_range(0..1000),
            bloom_filter_num_iters: rng.gen_range(0..4),
            input_queue_batch_size: rng.gen_range(1..1000) * input_queue_zkp_batch_size,
            output_queue_batch_size: rng.gen_range(1..1000) * output_queue_zkp_batch_size,
            input_queue_zkp_batch_size,
            output_queue_zkp_batch_size,
            // 8 bits per byte, divisible by 8 for aligned memory
            bloom_filter_capacity: rng.gen_range(0..100) * 8 * 8,
            network_fee,
            rollover_threshold: Some(rng.gen_range(0..100)),
            close_threshold: None,
            root_history_capacity: rng.gen_range(1..1000),
            height: rng.gen_range(1..32),
        };

        let queue_account_size = get_output_queue_account_size(
            params.output_queue_batch_size,
            params.output_queue_zkp_batch_size,
        );

        let mut output_queue_account_data = vec![0; queue_account_size];
        let output_queue_pubkey = Pubkey::new_unique();

        let mt_account_size = get_merkle_tree_account_size(
            params.input_queue_batch_size,
            params.bloom_filter_capacity,
            params.input_queue_zkp_batch_size,
            params.root_history_capacity,
            params.height,
        );

        let mut mt_account_data = vec![0; mt_account_size];
        let mt_pubkey = Pubkey::new_unique();
        println!("mt_pubkey {:?}", mt_pubkey);
        println!("queue_pubkey {:?}", output_queue_pubkey);

        let merkle_tree_rent = rng.gen_range(0..10000000);
        let queue_rent = rng.gen_range(0..10000000);
        let additional_bytes_rent = rng.gen_range(0..10000000);
        let additional_bytes = rng.gen_range(0..1000);
        init_batched_state_merkle_tree_accounts(
            owner,
            params,
            &mut output_queue_account_data,
            output_queue_pubkey,
            queue_rent,
            &mut mt_account_data,
            mt_pubkey,
            merkle_tree_rent,
            additional_bytes_rent,
        )
        .unwrap();
        let total_rent = merkle_tree_rent + queue_rent + additional_bytes_rent;
        let queue_account_params = CreateOutputQueueParams::from(
            params,
            owner,
            total_rent,
            mt_pubkey,
            output_queue_pubkey,
        );
        let ref_output_queue_account = create_output_queue_account(queue_account_params);
        assert_queue_zero_copy_inited(
            output_queue_account_data.as_mut_slice(),
            ref_output_queue_account,
        );
        let create_tree_params = CreateTreeParams::from_state_ix_params(params, owner, mt_pubkey);

        let ref_mt_account =
            BatchedMerkleTreeMetadata::new_state_tree(create_tree_params, output_queue_pubkey);
        assert_state_mt_zero_copy_initialized(&mut mt_account_data, ref_mt_account, &mt_pubkey);

        let mut new_mt_account_data = vec![0; mt_account_size];
        let new_mt_pubkey = Pubkey::new_unique();

        let mut new_queue_account_data = vec![0; queue_account_size];
        let new_output_queue_pubkey = Pubkey::new_unique();

        let merkle_tree =
            &mut BatchedMerkleTreeAccount::state_from_bytes(&mut mt_account_data, &mt_pubkey)
                .unwrap();
        let height = merkle_tree.get_metadata().height;
        merkle_tree.get_metadata_mut().next_index = 1 << height;
        let rollover_batch_state_tree_params = RolloverBatchStateTreeParams {
            old_merkle_tree: merkle_tree,
            old_mt_pubkey: mt_pubkey,
            new_mt_data: &mut new_mt_account_data,
            new_mt_rent: merkle_tree_rent,
            new_mt_pubkey,
            old_output_queue: &mut BatchedQueueAccount::output_from_bytes(
                &mut output_queue_account_data,
            )
            .unwrap(),
            old_queue_pubkey: output_queue_pubkey,
            new_output_queue_data: &mut new_queue_account_data,
            new_output_queue_rent: queue_rent,
            new_output_queue_pubkey,
            additional_bytes_rent,
            additional_bytes,
            network_fee: params.network_fee,
        };

        rollover_batched_state_tree(rollover_batch_state_tree_params).unwrap();

        let mut ref_rolledover_mt = ref_mt_account;
        ref_rolledover_mt.next_index = 1 << height;

        let output_queue_params = CreateOutputQueueParams::from(
            params,
            owner,
            total_rent,
            new_mt_pubkey,
            new_output_queue_pubkey,
        );
        let mut new_ref_output_queue_account = create_output_queue_account(output_queue_params);
        new_ref_output_queue_account
            .metadata
            .rollover_metadata
            .additional_bytes = additional_bytes;
        let create_tree_params =
            CreateTreeParams::from_state_ix_params(params, owner, new_mt_pubkey);
        let new_ref_merkle_tree_account =
            BatchedMerkleTreeMetadata::new_state_tree(create_tree_params, new_output_queue_pubkey);

        let assert_state_mt_roll_over_params = StateMtRollOverAssertParams {
            mt_account_data: mt_account_data.to_vec(),
            ref_mt_account: new_ref_merkle_tree_account,
            new_mt_account_data: new_mt_account_data.to_vec(),
            old_mt_pubkey: mt_pubkey,
            new_mt_pubkey,
            ref_rolledover_mt,
            queue_account_data: output_queue_account_data.to_vec(),
            ref_queue_account: new_ref_output_queue_account,
            new_queue_account_data: new_queue_account_data.to_vec(),
            new_queue_pubkey: new_output_queue_pubkey,
            ref_rolledover_queue: ref_output_queue_account,
            old_queue_pubkey: output_queue_pubkey,
            slot: 1,
        };

        assert_state_mt_roll_over(assert_state_mt_roll_over_params);
    }
}
