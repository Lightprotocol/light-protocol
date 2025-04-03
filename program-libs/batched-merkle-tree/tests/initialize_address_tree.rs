#![cfg(feature = "test-only")]
use light_batched_merkle_tree::{
    constants::NUM_BATCHES,
    initialize_address_tree::{
        init_batched_address_merkle_tree_account, InitAddressTreeAccountsInstructionData,
    },
    initialize_state_tree::test_utils::assert_address_mt_zero_copy_initialized,
    merkle_tree::{get_merkle_tree_account_size, test_utils::get_merkle_tree_account_size_default},
    merkle_tree_metadata::{BatchedMerkleTreeMetadata, CreateTreeParams},
};
use light_compressed_account::pubkey::Pubkey;
use light_zero_copy::{cyclic_vec::ZeroCopyCyclicVecU64, vec::ZeroCopyVecU64};
use rand::{rngs::StdRng, Rng};

#[test]
fn test_account_init() {
    let owner = Pubkey::new_unique();
    let tree_pubkey = Pubkey::new_unique();

    let mt_account_size = get_merkle_tree_account_size_default();
    let mut mt_account_data = vec![0; mt_account_size];
    let merkle_tree_rent = 1_000_000_000;

    let params = InitAddressTreeAccountsInstructionData::test_default();
    let mt_params = CreateTreeParams::from_address_ix_params(params, owner, tree_pubkey);
    let ref_mt_account = BatchedMerkleTreeMetadata::new_address_tree(mt_params, merkle_tree_rent);
    init_batched_address_merkle_tree_account(
        owner,
        params,
        &mut mt_account_data,
        merkle_tree_rent,
        tree_pubkey,
    )
    .unwrap();

    assert_address_mt_zero_copy_initialized(&mut mt_account_data, ref_mt_account, &tree_pubkey);
}

#[test]
fn test_rnd_account_init() {
    use rand::SeedableRng;
    let mut rng = StdRng::seed_from_u64(0);
    for _ in 0..10000 {
        println!("next iter ------------------------------------");
        let owner = Pubkey::new_unique();
        let tree_pubkey = Pubkey::new_unique();

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
            height: 40,
        };

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
            let bloom_filter_size = ((params.bloom_filter_capacity / 8) * 2u64) as usize;
            let hash_chain_store_size =
                ZeroCopyVecU64::<[u8; 32]>::required_size_for_capacity(num_zkp_batches)
                    * num_batches;
            let root_history_size = ZeroCopyCyclicVecU64::<[u8; 32]>::required_size_for_capacity(
                params.root_history_capacity as u64,
            );
            // Output queue
            let ref_account_size = BatchedMerkleTreeMetadata::LEN
                    + root_history_size
                    + bloom_filter_size
                    // 2 hash chain stores
                    + hash_chain_store_size;
            assert_eq!(mt_account_size, ref_account_size);
        }
        let mut mt_account_data = vec![0; mt_account_size];

        let merkle_tree_rent = rng.gen_range(1..10000000);

        init_batched_address_merkle_tree_account(
            owner,
            params,
            &mut mt_account_data,
            merkle_tree_rent,
            tree_pubkey,
        )
        .unwrap();
        let mt_params = CreateTreeParams::from_address_ix_params(params, owner, tree_pubkey);
        let ref_mt_account =
            BatchedMerkleTreeMetadata::new_address_tree(mt_params, merkle_tree_rent);
        assert_address_mt_zero_copy_initialized(&mut mt_account_data, ref_mt_account, &tree_pubkey);
    }
}
