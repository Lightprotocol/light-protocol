use light_batched_merkle_tree::{
    batch::Batch,
    initialize_address_tree::{
        init_batched_address_merkle_tree_account, InitAddressTreeAccountsInstructionData,
    },
    initialize_state_tree::assert_address_mt_zero_copy_inited,
    merkle_tree::{
        get_merkle_tree_account_size, get_merkle_tree_account_size_default,
        BatchedMerkleTreeAccount, CreateTreeParams,
    },
};
use light_bounded_vec::{BoundedVecMetadata, CyclicBoundedVecMetadata};
use rand::{rngs::StdRng, Rng};
use solana_program::pubkey::Pubkey;

#[test]
fn test_account_init() {
    let owner = Pubkey::new_unique();

    let mt_account_size = get_merkle_tree_account_size_default();
    let mut mt_account_data = vec![0; mt_account_size];

    let params = InitAddressTreeAccountsInstructionData::test_default();

    let merkle_tree_rent = 1_000_000_000;
    init_batched_address_merkle_tree_account(
        owner,
        params.clone(),
        &mut mt_account_data,
        merkle_tree_rent,
    )
    .unwrap();
    let mt_params = CreateTreeParams::from_address_ix_params(params, owner);
    let ref_mt_account =
        BatchedMerkleTreeAccount::get_address_tree_default(mt_params, merkle_tree_rent);
    assert_address_mt_zero_copy_inited(
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

        let params = InitAddressTreeAccountsInstructionData {
            index: rng.gen_range(0..1000),
            program_owner,
            forester,
            bloom_filter_num_iters: rng.gen_range(0..4),
            input_queue_batch_size: rng.gen_range(1..1000) * input_queue_zkp_batch_size,
            input_queue_zkp_batch_size,
            // 8 bits per byte, divisible by 8 for aligned memory
            bloom_filter_capacity: rng.gen_range(0..100) * 8 * 8,
            network_fee: Some(rng.gen_range(0..1000)),
            rollover_threshold: Some(rng.gen_range(0..100)),
            close_threshold: None,
            root_history_capacity: rng.gen_range(1..1000),
            input_queue_num_batches: rng.gen_range(1..4),
            height: rng.gen_range(1..32),
        };

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
            let batch_size = size_of::<Batch>() * num_batches + size_of::<BoundedVecMetadata>();
            let bloom_filter_size = (params.bloom_filter_capacity as usize / 8
                + size_of::<BoundedVecMetadata>())
                * num_batches;
            let hash_chain_store_size =
                (num_zkp_batches as usize * 32 + size_of::<BoundedVecMetadata>()) * num_batches;
            let root_history_size =
                params.root_history_capacity as usize * 32 + size_of::<CyclicBoundedVecMetadata>();
            // Output queue
            let ref_account_size =
                    // metadata
                    BatchedMerkleTreeAccount::LEN
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
        let mt_params = CreateTreeParams::from_address_ix_params(params, owner);
        let ref_mt_account =
            BatchedMerkleTreeAccount::get_address_tree_default(mt_params, merkle_tree_rent);
        assert_address_mt_zero_copy_inited(
            &mut mt_account_data,
            ref_mt_account,
            params.bloom_filter_num_iters,
        );
    }
}
