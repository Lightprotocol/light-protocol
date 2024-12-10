use anchor_lang::prelude::*;
use light_utils::fee::compute_rollover_fee;

use crate::{
    batched_merkle_tree::{
        get_merkle_tree_account_size, BatchedMerkleTreeAccount, TreeType,
        ZeroCopyBatchedMerkleTreeAccount,
    },
    initialize_address_queue::check_rollover_fee_sufficient,
    match_circuit_size,
    utils::{
        check_account::check_account_balance_is_rent_exempt,
        check_signer_is_registered_or_authority::{
            check_signer_is_registered_or_authority, GroupAccounts,
        },
        constants::{
            DEFAULT_BATCH_SIZE, DEFAULT_ZKP_BATCH_SIZE, TEST_DEFAULT_BATCH_SIZE,
            TEST_DEFAULT_ZKP_BATCH_SIZE,
        },
    },
    MerkleTreeMetadata, RegisteredProgram,
};

#[derive(Accounts)]
pub struct InitializeBatchAddressMerkleTree<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(zero)]
    pub merkle_tree: AccountLoader<'info, BatchedMerkleTreeAccount>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
}

impl<'info> GroupAccounts<'info> for InitializeBatchAddressMerkleTree<'info> {
    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

#[derive(Debug, PartialEq, Clone, Copy, AnchorDeserialize, AnchorSerialize)]
pub struct InitAddressTreeAccountsInstructionData {
    pub index: u64,
    pub program_owner: Option<Pubkey>,
    pub forester: Option<Pubkey>,
    pub input_queue_batch_size: u64,
    pub input_queue_zkp_batch_size: u64,
    pub bloom_filter_num_iters: u64,
    pub bloom_filter_capacity: u64,
    pub root_history_capacity: u32,
    pub network_fee: Option<u64>,
    pub rollover_threshold: Option<u64>,
    pub close_threshold: Option<u64>,
    pub input_queue_num_batches: u64,
    pub height: u32,
}

impl InitAddressTreeAccountsInstructionData {
    pub fn test_default() -> Self {
        Self {
            index: 0,
            program_owner: None,
            forester: None,
            bloom_filter_num_iters: 3,
            input_queue_batch_size: TEST_DEFAULT_BATCH_SIZE,
            input_queue_zkp_batch_size: TEST_DEFAULT_ZKP_BATCH_SIZE,
            input_queue_num_batches: 2,
            height: 26,
            root_history_capacity: 20,
            bloom_filter_capacity: 20_000 * 8,
            network_fee: Some(5000),
            rollover_threshold: Some(95),
            close_threshold: None,
        }
    }

    pub fn e2e_test_default() -> Self {
        Self {
            index: 0,
            program_owner: None,
            forester: None,
            bloom_filter_num_iters: 3,
            input_queue_batch_size: 500,
            input_queue_zkp_batch_size: TEST_DEFAULT_ZKP_BATCH_SIZE,
            input_queue_num_batches: 2,
            height: 26,
            root_history_capacity: 20,
            bloom_filter_capacity: 20_000 * 8,
            network_fee: Some(5000),
            rollover_threshold: Some(95),
            close_threshold: None,
        }
    }
}

impl Default for InitAddressTreeAccountsInstructionData {
    fn default() -> Self {
        Self {
            index: 0,
            program_owner: None,
            forester: None,
            bloom_filter_num_iters: 3,
            input_queue_batch_size: DEFAULT_BATCH_SIZE,
            input_queue_zkp_batch_size: DEFAULT_ZKP_BATCH_SIZE,
            input_queue_num_batches: 2,
            height: 26,
            root_history_capacity: (DEFAULT_BATCH_SIZE / DEFAULT_ZKP_BATCH_SIZE * 2) as u32,
            bloom_filter_capacity: (DEFAULT_BATCH_SIZE + 1) * 8,
            network_fee: Some(5000),
            rollover_threshold: Some(95),
            close_threshold: None,
        }
    }
}

pub fn process_initialize_batched_address_merkle_tree<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializeBatchAddressMerkleTree<'info>>,
    params: InitAddressTreeAccountsInstructionData,
) -> Result<()> {
    #[cfg(feature = "test")]
    validate_batched_address_tree_params(params);
    #[cfg(not(feature = "test"))]
    {
        if params != InitAddressTreeAccountsInstructionData::default() {
            return err!(AccountCompressionErrorCode::UnsupportedParameters);
        }
    }

    let owner = match ctx.accounts.registered_program_pda.as_ref() {
        Some(registered_program_pda) => {
            check_signer_is_registered_or_authority::<
                InitializeBatchAddressMerkleTree,
                RegisteredProgram,
            >(&ctx, registered_program_pda)?;
            registered_program_pda.group_authority_pda
        }
        None => ctx.accounts.authority.key(),
    };
    let mt_account_size = get_merkle_tree_account_size(
        params.input_queue_batch_size,
        params.bloom_filter_capacity,
        params.input_queue_zkp_batch_size,
        params.root_history_capacity,
        params.height,
        params.input_queue_num_batches,
    );

    let merkle_tree_rent = check_account_balance_is_rent_exempt(
        &ctx.accounts.merkle_tree.to_account_info(),
        mt_account_size,
    )?;

    let mt_account_info = ctx.accounts.merkle_tree.to_account_info();
    let mt_data = &mut mt_account_info.try_borrow_mut_data()?;

    init_batched_address_merkle_tree_account(owner, params, mt_data, merkle_tree_rent)?;

    Ok(())
}

pub fn init_batched_address_merkle_tree_account(
    owner: Pubkey,
    params: InitAddressTreeAccountsInstructionData,
    mt_account_data: &mut [u8],
    merkle_tree_rent: u64,
) -> Result<()> {
    let num_batches_input_queue = params.input_queue_num_batches;
    let height = params.height;

    let rollover_fee = match params.rollover_threshold {
        Some(rollover_threshold) => {
            let rent = merkle_tree_rent;
            let rollover_fee = compute_rollover_fee(rollover_threshold, height, rent)
                .map_err(ProgramError::from)?;
            check_rollover_fee_sufficient(rollover_fee, 0, rent, rollover_threshold, height)?;
            rollover_fee
        }
        None => 0,
    };

    let metadata = MerkleTreeMetadata {
        next_merkle_tree: Pubkey::default(),
        access_metadata: crate::AccessMetadata::new(owner, params.program_owner, params.forester),
        rollover_metadata: crate::RolloverMetadata::new(
            params.index,
            rollover_fee,
            params.rollover_threshold,
            params.network_fee.unwrap_or_default(),
            params.close_threshold,
            None,
        ),
        associated_queue: Pubkey::default(),
    };
    ZeroCopyBatchedMerkleTreeAccount::init(
        metadata,
        params.root_history_capacity,
        num_batches_input_queue,
        params.input_queue_batch_size,
        params.input_queue_zkp_batch_size,
        height,
        mt_account_data,
        params.bloom_filter_num_iters,
        params.bloom_filter_capacity,
        TreeType::BatchedAddress,
    )?;
    Ok(())
}

pub fn validate_batched_address_tree_params(params: InitAddressTreeAccountsInstructionData) {
    assert!(params.input_queue_batch_size > 0);
    assert_eq!(
        params.input_queue_batch_size % params.input_queue_zkp_batch_size,
        0,
        "Input queue batch size must divisible by input_queue_zkp_batch_size."
    );
    assert!(
        match_circuit_size(params.input_queue_zkp_batch_size),
        "Zkp batch size not supported. Supported 1, 10, 100, 500, 1000"
    );

    assert!(params.bloom_filter_num_iters > 0);
    assert!(params.bloom_filter_capacity > params.input_queue_batch_size * 8);
    assert_eq!(
        params.bloom_filter_capacity % 8,
        0,
        "Bloom filter capacity must be divisible by 8."
    );
    assert!(params.bloom_filter_capacity > 0);
    assert!(params.root_history_capacity > 0);
    assert!(params.input_queue_batch_size > 0);
    assert_eq!(params.input_queue_num_batches, 2);
    assert_eq!(params.close_threshold, None);
    assert_eq!(params.height, 26);
}

#[cfg(test)]
pub mod address_tree_tests {

    use light_bounded_vec::{BoundedVecMetadata, CyclicBoundedVecMetadata};
    use rand::{rngs::StdRng, Rng};

    use crate::{
        assert_address_mt_zero_copy_inited,
        batch::Batch,
        batched_merkle_tree::{get_merkle_tree_account_size, get_merkle_tree_account_size_default},
    };

    use super::*;

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

        let ref_mt_account = BatchedMerkleTreeAccount::get_address_tree_default(
            owner,
            None,
            None,
            params.rollover_threshold,
            0,
            params.network_fee.unwrap_or_default(),
            params.input_queue_batch_size,
            params.input_queue_zkp_batch_size,
            params.bloom_filter_capacity,
            params.root_history_capacity,
            params.height,
            params.input_queue_num_batches,
            merkle_tree_rent,
        );
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
                let num_zkp_batches =
                    params.input_queue_batch_size / params.input_queue_zkp_batch_size;
                let num_batches = params.input_queue_num_batches as usize;
                let batch_size = size_of::<Batch>() * num_batches + size_of::<BoundedVecMetadata>();
                let bloom_filter_size = (params.bloom_filter_capacity as usize / 8
                    + size_of::<BoundedVecMetadata>())
                    * num_batches;
                let hash_chain_store_size =
                    (num_zkp_batches as usize * 32 + size_of::<BoundedVecMetadata>()) * num_batches;
                let root_history_size = params.root_history_capacity as usize * 32
                    + size_of::<CyclicBoundedVecMetadata>();
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
            let ref_mt_account = BatchedMerkleTreeAccount::get_address_tree_default(
                owner,
                program_owner,
                forester,
                params.rollover_threshold,
                params.index,
                params.network_fee.unwrap_or_default(),
                params.input_queue_batch_size,
                params.input_queue_zkp_batch_size,
                params.bloom_filter_capacity,
                params.root_history_capacity,
                params.height,
                params.input_queue_num_batches,
                merkle_tree_rent,
            );
            assert_address_mt_zero_copy_inited(
                &mut mt_account_data,
                ref_mt_account,
                params.bloom_filter_num_iters,
            );
        }
    }
}
