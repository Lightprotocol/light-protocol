use light_batched_merkle_tree::{
    batch::Batch,
    initialize_state_tree::{
        assert_state_mt_zero_copy_inited, create_output_queue_account,
        init_batched_state_merkle_tree_accounts, CreateOutputQueueParams,
        InitStateTreeAccountsInstructionData,
    },
    merkle_tree::{
        get_merkle_tree_account_size, get_merkle_tree_account_size_default,
        BatchedMerkleTreeMetadata, CreateTreeParams,
    },
    queue::{
        assert_queue_zero_copy_inited, get_output_queue_account_size,
        get_output_queue_account_size_default, BatchedQueueMetadata,
    },
};
use light_utils::pubkey::Pubkey;
use light_zero_copy::{
    cyclic_vec::ZeroCopyCyclicVecU64, slice_mut::ZeroCopySliceMutU64, vec::ZeroCopyVecU64,
};
use rand::{rngs::StdRng, Rng};
#[test]
fn test_different_parameters() {
    let params = InitStateTreeAccountsInstructionData::test_default();
    let e2e_test_params = InitStateTreeAccountsInstructionData::e2e_test_default();
    let default_params = InitStateTreeAccountsInstructionData::default();
    for params in vec![params, e2e_test_params, default_params] {
        println!("params: {:?}", params);
        let owner = Pubkey::new_unique();
        let queue_account_size = get_output_queue_account_size(
            params.output_queue_batch_size,
            params.output_queue_zkp_batch_size,
            params.output_queue_num_batches,
        );

        let mut output_queue_account_data = vec![0; queue_account_size];
        let output_queue_pubkey = Pubkey::new_unique();

        let mt_account_size = get_merkle_tree_account_size(
            params.input_queue_batch_size,
            params.bloom_filter_capacity,
            params.input_queue_zkp_batch_size,
            params.root_history_capacity,
            params.height,
            params.input_queue_num_batches,
        );
        let mut mt_account_data = vec![0; mt_account_size];
        let mt_pubkey = Pubkey::new_unique();

        let merkle_tree_rent = 1_000_000_000;
        let queue_rent = 1_000_000_000;
        let additional_bytes_rent = 1000;
        init_batched_state_merkle_tree_accounts(
            owner,
            params.clone(),
            &mut output_queue_account_data,
            output_queue_pubkey,
            queue_rent,
            &mut mt_account_data,
            mt_pubkey,
            merkle_tree_rent,
            additional_bytes_rent,
        )
        .unwrap();
        let queue_account_params = CreateOutputQueueParams::from(
            params,
            owner,
            merkle_tree_rent + queue_rent + additional_bytes_rent,
            mt_pubkey,
        );
        let ref_output_queue_account = create_output_queue_account(queue_account_params);
        assert_queue_zero_copy_inited(
            output_queue_account_data.as_mut_slice(),
            ref_output_queue_account,
            0,
        );
        let mt_params = CreateTreeParams::from_state_ix_params(params, owner);
        let ref_mt_account =
            BatchedMerkleTreeMetadata::new_state_tree(mt_params, output_queue_pubkey);
        assert_state_mt_zero_copy_inited(
            &mut mt_account_data,
            ref_mt_account,
            params.bloom_filter_num_iters,
        );
    }
}

#[test]
fn test_account_init() {
    let owner = Pubkey::new_unique();

    let queue_account_size = get_output_queue_account_size_default();

    let mut output_queue_account_data = vec![0; queue_account_size];
    let output_queue_pubkey = Pubkey::new_unique();

    let mt_account_size = get_merkle_tree_account_size_default();
    let mut mt_account_data = vec![0; mt_account_size];
    let mt_pubkey = Pubkey::new_unique();

    let params = InitStateTreeAccountsInstructionData::test_default();

    let merkle_tree_rent = 1_000_000_000;
    let queue_rent = 1_000_000_000;
    let additional_bytes_rent = 1000;
    init_batched_state_merkle_tree_accounts(
        owner,
        params.clone(),
        &mut output_queue_account_data,
        output_queue_pubkey,
        queue_rent,
        &mut mt_account_data,
        mt_pubkey,
        merkle_tree_rent,
        additional_bytes_rent,
    )
    .unwrap();
    let queue_account_params = CreateOutputQueueParams::from(
        params,
        owner,
        merkle_tree_rent + queue_rent + additional_bytes_rent,
        mt_pubkey,
    );
    let ref_output_queue_account = create_output_queue_account(queue_account_params);
    assert_queue_zero_copy_inited(
        output_queue_account_data.as_mut_slice(),
        ref_output_queue_account,
        0,
    );
    let mt_params = CreateTreeParams::from_state_ix_params(params, owner);
    let ref_mt_account = BatchedMerkleTreeMetadata::new_state_tree(mt_params, output_queue_pubkey);
    assert_state_mt_zero_copy_inited(
        &mut mt_account_data,
        ref_mt_account,
        params.bloom_filter_num_iters,
    );
}

#[test]
fn test_rnd_account_init() {
    use rand::SeedableRng;
    let mut rng = StdRng::seed_from_u64(0);
    for _ in 0..10000 {
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
            network_fee: Some(rng.gen_range(0..1000)),
            rollover_threshold: Some(rng.gen_range(0..100)),
            close_threshold: None,
            root_history_capacity: rng.gen_range(1..1000),
            input_queue_num_batches: rng.gen_range(1..4),
            output_queue_num_batches: rng.gen_range(1..4),
            height: rng.gen_range(1..32),
        };
        let queue_account_size = get_output_queue_account_size(
            params.output_queue_batch_size,
            params.output_queue_zkp_batch_size,
            params.output_queue_num_batches,
        );

        {
            let num_batches = params.output_queue_num_batches as usize;
            let num_zkp_batches =
                params.output_queue_batch_size / params.output_queue_zkp_batch_size;
            let batch_size = ZeroCopySliceMutU64::<Batch>::required_size_for_capacity(
                params.output_queue_num_batches,
            );
            let value_vec_size = ZeroCopyVecU64::<[u8; 32]>::required_size_for_capacity(
                params.output_queue_batch_size,
            ) * num_batches;
            let hash_chain_store_size =
                ZeroCopyVecU64::<[u8; 32]>::required_size_for_capacity(num_zkp_batches)
                    * num_batches;
            // Output queue
            let ref_queue_account_size =
                    // metadata
                    BatchedQueueMetadata::LEN
                    + batch_size
                    // 2 value vecs
                    + value_vec_size
                    // 2 hash chain stores
                    + hash_chain_store_size;

            assert_eq!(queue_account_size, ref_queue_account_size);
        }

        let mut output_queue_account_data = vec![0; queue_account_size];
        let output_queue_pubkey = Pubkey::new_unique();

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
            let num_batches = params.input_queue_num_batches;
            let batch_size = ZeroCopySliceMutU64::<Batch>::required_size_for_capacity(num_batches);
            let bloom_filter_size = ZeroCopySliceMutU64::<u8>::required_size_for_capacity(
                params.bloom_filter_capacity / 8,
            ) * num_batches as usize;
            let hash_chain_store_size =
                ZeroCopyVecU64::<[u8; 32]>::required_size_for_capacity(num_zkp_batches)
                    * num_batches as usize;
            let root_history_size = ZeroCopyCyclicVecU64::<[u8; 32]>::required_size_for_capacity(
                params.root_history_capacity as u64,
            );
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
        let mt_pubkey = Pubkey::new_unique();

        let merkle_tree_rent = rng.gen_range(0..10000000);
        let queue_rent = rng.gen_range(0..10000000);
        let additional_bytes_rent = rng.gen_range(0..10000000);
        init_batched_state_merkle_tree_accounts(
            owner,
            params.clone(),
            &mut output_queue_account_data,
            output_queue_pubkey,
            queue_rent,
            &mut mt_account_data,
            mt_pubkey,
            merkle_tree_rent,
            additional_bytes_rent,
        )
        .unwrap();
        let queue_account_params = CreateOutputQueueParams::from(
            params,
            owner,
            merkle_tree_rent + queue_rent + additional_bytes_rent,
            mt_pubkey,
        );
        let ref_output_queue_account = create_output_queue_account(queue_account_params);
        assert_queue_zero_copy_inited(
            output_queue_account_data.as_mut_slice(),
            ref_output_queue_account,
            0,
        );
        let mt_params = CreateTreeParams::from_state_ix_params(params, owner);

        let ref_mt_account =
            BatchedMerkleTreeMetadata::new_state_tree(mt_params, output_queue_pubkey);
        assert_state_mt_zero_copy_inited(
            &mut mt_account_data,
            ref_mt_account,
            params.bloom_filter_num_iters,
        );
    }
}
