use std::{
    borrow::{Borrow, BorrowMut},
    cell::RefMut,
    default,
    io::Read,
    mem::ManuallyDrop,
};

use anchor_lang::{prelude::*, Discriminator};
use light_utils::fee::compute_rollover_fee;

use crate::{
    batched_merkle_tree::{
        get_merkle_tree_account_size, BatchedMerkleTreeAccount, BatchedTreeType,
        ZeroCopyBatchedMerkleTreeAccount,
    },
    batched_queue::{
        assert_queue_inited, get_output_queue_account_size, BatchedQueueAccount,
        ZeroCopyBatchedQueueAccount,
    },
    errors::AccountCompressionErrorCode,
    initialize_address_queue::check_rollover_fee_sufficient,
    utils::{
        check_account::check_account_balance_is_rent_exempt,
        check_signer_is_registered_or_authority::{
            check_signer_is_registered_or_authority, GroupAccounts,
        },
        constants::{DEFAULT_BATCH_SIZE, HEIGHT_26_SUBTREE_ZERO_HASH},
    },
    AccessMetadata, MerkleTreeMetadata, QueueMetadata, QueueType, RegisteredProgram,
    RolloverMetadata,
};

#[derive(Accounts)]
pub struct InitializeBatchedStateMerkleTreeAndQueue<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(zero)]
    pub merkle_tree: AccountLoader<'info, BatchedMerkleTreeAccount>,
    #[account(zero)]
    pub queue: AccountLoader<'info, BatchedQueueAccount>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
}

impl<'info> GroupAccounts<'info> for InitializeBatchedStateMerkleTreeAndQueue<'info> {
    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

#[derive(Debug, PartialEq, Clone, Copy, AnchorDeserialize, AnchorSerialize)]
pub struct InitStateTreeAccountsInstructionData {
    pub index: u64,
    pub program_owner: Option<Pubkey>,
    pub forester: Option<Pubkey>,
    pub additional_bytes: u64,
    pub bloomfilter_num_iters: u64,
    pub input_queue_batch_size: u64,
    pub output_queue_batch_size: u64,
    pub input_queue_zkp_batch_size: u64,
    pub output_queue_zkp_batch_size: u64,
    pub root_history_capacity: u32,
    pub bloom_filter_capacity: u64,
    pub network_fee: Option<u64>,
    pub rollover_threshold: Option<u64>,
    pub close_threshold: Option<u64>,
}

impl default::Default for InitStateTreeAccountsInstructionData {
    fn default() -> Self {
        Self {
            index: 0,
            program_owner: None,
            forester: None,
            additional_bytes: 1,
            bloomfilter_num_iters: 3,
            input_queue_batch_size: DEFAULT_BATCH_SIZE,
            output_queue_batch_size: DEFAULT_BATCH_SIZE,
            input_queue_zkp_batch_size: 10,
            output_queue_zkp_batch_size: 10,
            root_history_capacity: 20,
            bloom_filter_capacity: 200_000 * 8,
            network_fee: Some(5000),
            rollover_threshold: Some(95),
            close_threshold: None,
        }
    }
}

pub fn process_initialize_batched_state_merkle_tree<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializeBatchedStateMerkleTreeAndQueue<'info>>,
    params: InitStateTreeAccountsInstructionData,
) -> Result<()> {
    let owner = match ctx.accounts.registered_program_pda.as_ref() {
        Some(registered_program_pda) => {
            check_signer_is_registered_or_authority::<
                InitializeBatchedStateMerkleTreeAndQueue,
                RegisteredProgram,
            >(&ctx, registered_program_pda)?;
            registered_program_pda.group_authority_pda
        }
        None => ctx.accounts.authority.key(),
    };
    {
        ctx.accounts.queue.load_init()?;
        ctx.accounts.merkle_tree.load_init()?;
    }
    msg!("output_queue_account: ");
    let output_queue_pubkey = ctx.accounts.queue.key();
    let queue_account_size = get_output_queue_account_size(
        params.output_queue_batch_size,
        params.output_queue_zkp_batch_size,
    );
    let mt_account_size = get_merkle_tree_account_size(
        params.input_queue_batch_size,
        params.bloom_filter_capacity,
        params.input_queue_zkp_batch_size,
        params.root_history_capacity,
    );
    // TODO: use actual size
    let queue_rent = check_account_balance_is_rent_exempt(
        &ctx.accounts.queue.to_account_info(),
        queue_account_size,
    )?;

    msg!("mt_account: ");

    let mt_pubkey = ctx.accounts.merkle_tree.key();
    let merkle_tree_rent = check_account_balance_is_rent_exempt(
        &ctx.accounts.merkle_tree.to_account_info(),
        mt_account_size,
    )?;
    msg!("queue_rent: {}", queue_rent);
    let additional_bytes_rent = (Rent::get()?).minimum_balance(params.additional_bytes as usize);

    let output_queue_account_data: AccountInfo<'info> = ctx.accounts.queue.to_account_info();
    let queue_data = &mut output_queue_account_data.try_borrow_mut_data()?;
    let output_queue_account = &mut bytes_to_struct::<BatchedQueueAccount, true>(queue_data);

    let mt_account_info = ctx.accounts.merkle_tree.to_account_info();
    let mt_data = &mut mt_account_info.try_borrow_mut_data()?;
    let mt_account = &mut bytes_to_struct::<BatchedMerkleTreeAccount, true>(mt_data);

    init_batched_state_merkle_tree_accounts(
        owner,
        params,
        output_queue_account,
        queue_data,
        output_queue_pubkey,
        queue_rent,
        mt_account,
        mt_data,
        mt_pubkey,
        merkle_tree_rent,
        additional_bytes_rent,
    )?;
    // if params.bloom_filter_capacity % 8 != 0 {
    //     println!(
    //         "params.bloom_filter_capacity: {}",
    //         params.bloom_filter_capacity
    //     );
    //     println!("Blooms must be divisible by 8 or it will create unaligned memory.");
    //     return err!(AccountCompressionErrorCode::InvalidBloomFilterCapacity);
    // }

    // let num_batches_input_queue = 4;
    // let num_batches_output_queue = 2;
    // let height = 26;

    // // Output queue
    // {
    //     let rollover_fee = match params.rollover_threshold {
    //         Some(rollover_threshold) => {
    //             let rent = merkle_tree_rent + additional_bytes_rent + queue_rent;
    //             let rollover_fee = compute_rollover_fee(rollover_threshold, height, rent)
    //                 .map_err(ProgramError::from)?;
    //             check_rollover_fee_sufficient(rollover_fee, 0, rent, rollover_threshold, height)?;
    //             rollover_fee
    //         }
    //         None => 0,
    //     };
    //     let metadata = QueueMetadata {
    //         next_queue: Pubkey::default(),
    //         access_metadata: AccessMetadata::new(owner, params.program_owner, params.forester),
    //         rollover_metadata: RolloverMetadata::new(
    //             params.index,
    //             rollover_fee,
    //             params.rollover_threshold,
    //             params.network_fee.unwrap_or_default(),
    //             params.close_threshold,
    //             Some(params.additional_bytes),
    //         ),
    //         queue_type: QueueType::Output as u64,
    //         associated_merkle_tree: mt_pubkey,
    //     };
    //     // let mut account_info_data = ctx.accounts.queue.to_account_info();
    //     // let mut output_queue_account_loader =
    //     //     AccountLoader::<'info, BatchedQueueAccount>::try_from(&account_info_data)?;
    //     // let mut output_queue_account = output_queue_account_loader.load_init()?;
    //     // let data = account_info_data.try_borrow_data()?;
    //     // output_queue_account.queue.init(
    //     //     metadata,
    //     //     num_batches_output_queue,
    //     //     params.output_queue_batch_size,
    //     //     params.output_queue_zkp_batch_size,
    //     // );
    //     let mut data = output_queue_account_data.try_borrow_mut_data()?;
    //     let mut output_queue_account = bytes_to_struct(&mut data);
    //     // let queue_account = ZeroCopyBatchedQueueAccount::init_from_account(
    //     //     &mut output_queue_account,
    //     //     &mut data,
    //     //     0,
    //     //     0,
    //     // )?;
    // }
    Ok(())
}

fn bytes_to_struct<T: Clone + Copy + Discriminator, const INIT: bool>(bytes: &mut [u8]) -> T {
    // Ensure the slice has at least as many bytes as needed for MyStruct
    assert!(bytes.len() >= std::mem::size_of::<T>());

    if INIT {
        bytes[0..8].copy_from_slice(&T::DISCRIMINATOR);
    } else if T::DISCRIMINATOR != bytes[0..8] {
        msg!("discriminator: {:?}", T::DISCRIMINATOR);
        msg!("bytes: {:?}", bytes[0..128].to_vec());
        panic!("Discriminator mismatch");
    } else {
        unreachable!("bytes_to_struct");
    }

    // Cast the first N bytes (size of MyStruct) to MyStruct
    unsafe { *(bytes[8..].as_mut_ptr() as *mut T) }
}
pub fn init_batched_state_merkle_tree_accounts<'a>(
    owner: Pubkey,
    params: InitStateTreeAccountsInstructionData,
    output_queue_account: &'a mut BatchedQueueAccount,
    output_queue_account_data: &mut [u8],
    output_queue_pubkey: Pubkey,
    queue_rent: u64,
    mt_account: &'a mut BatchedMerkleTreeAccount,
    mt_account_data: &mut [u8],
    mt_pubkey: Pubkey,
    merkle_tree_rent: u64,
    additional_bytes_rent: u64,
) -> Result<()> {
    if params.bloom_filter_capacity % 8 != 0 {
        println!(
            "params.bloom_filter_capacity: {}",
            params.bloom_filter_capacity
        );
        println!("Blooms must be divisible by 8 or it will create unaligned memory.");
        return err!(AccountCompressionErrorCode::InvalidBloomFilterCapacity);
    }

    let num_batches_input_queue = 4;
    let num_batches_output_queue = 2;
    let height = 26;

    // Output queue
    {
        let rollover_fee = match params.rollover_threshold {
            Some(rollover_threshold) => {
                let rent = merkle_tree_rent + additional_bytes_rent + queue_rent;
                let rollover_fee = compute_rollover_fee(rollover_threshold, height, rent)
                    .map_err(ProgramError::from)?;
                check_rollover_fee_sufficient(rollover_fee, 0, rent, rollover_threshold, height)?;
                rollover_fee
            }
            None => 0,
        };
        let metadata = QueueMetadata {
            next_queue: Pubkey::default(),
            access_metadata: AccessMetadata::new(owner, params.program_owner, params.forester),
            rollover_metadata: RolloverMetadata::new(
                params.index,
                rollover_fee,
                params.rollover_threshold,
                params.network_fee.unwrap_or_default(),
                params.close_threshold,
                Some(params.additional_bytes),
            ),
            queue_type: QueueType::Output as u64,
            associated_merkle_tree: mt_pubkey,
        };
        output_queue_account.init(
            metadata,
            num_batches_output_queue,
            params.output_queue_batch_size,
            params.output_queue_zkp_batch_size,
        );
        ZeroCopyBatchedQueueAccount::init_from_account(
            output_queue_account,
            output_queue_account_data,
            0,
            0,
        )?;
    }
    let metadata = MerkleTreeMetadata {
        next_merkle_tree: Pubkey::default(),
        access_metadata: crate::AccessMetadata::new(owner, params.program_owner, params.forester),
        rollover_metadata: crate::RolloverMetadata::new(
            params.index,
            // Complete rollover fee is charged when creating an output
            // compressed account by inserting it into the output queue.
            0,
            params.rollover_threshold,
            params.network_fee.unwrap_or_default(),
            params.close_threshold,
            None,
        ),
        associated_queue: output_queue_pubkey,
    };
    mt_account.metadata = metadata;
    mt_account.root_history_capacity = params.root_history_capacity;
    mt_account.height = height;
    mt_account.tree_type = BatchedTreeType::State as u64;
    mt_account.subtree_hash = HEIGHT_26_SUBTREE_ZERO_HASH;
    mt_account.queue.init(
        num_batches_input_queue,
        params.input_queue_batch_size,
        params.input_queue_zkp_batch_size,
    );
    mt_account.queue.bloom_filter_capacity = params.bloom_filter_capacity;
    println!("initing mt_account: ");
    ZeroCopyBatchedMerkleTreeAccount::init_from_account(
        mt_account,
        mt_account_data,
        params.bloomfilter_num_iters,
        params.bloom_filter_capacity,
    )?;
    Ok(())
}
pub fn assert_mt_zero_copy_inited(
    account: &mut BatchedMerkleTreeAccount,
    account_data: &mut [u8],
    ref_account: BatchedMerkleTreeAccount,
    num_iters: u64,
) {
    let queue = account.queue.clone();
    let ref_queue = ref_account.queue.clone();
    let queue_type = QueueType::Input as u64;
    let num_batches = ref_queue.num_batches as usize;

    let mut zero_copy_account =
        ZeroCopyBatchedMerkleTreeAccount::from_account(account, account_data)
            .expect("from_account failed");
    assert_eq!(*zero_copy_account.account, ref_account, "metadata mismatch");

    assert_eq!(
        zero_copy_account.root_history.capacity(),
        ref_account.root_history_capacity as usize,
        "root_history_capacity mismatch"
    );

    assert!(
        zero_copy_account.root_history.is_empty(),
        "root_history not empty"
    );
    assert_eq!(
        zero_copy_account.account.subtree_hash,
        ref_account.subtree_hash
    );

    assert_queue_inited(
        queue,
        ref_queue,
        queue_type,
        &mut zero_copy_account.value_vecs,
        &mut zero_copy_account.bloomfilter_stores,
        &mut zero_copy_account.batches,
        num_batches,
        num_iters,
    );
}

#[cfg(test)]
pub mod tests {

    use rand::{rngs::StdRng, Rng};

    use crate::{
        batched_merkle_tree::{
            get_merkle_tree_account_size, get_merkle_tree_account_size_default,
            ZeroCopyBatchedMerkleTreeAccount,
        },
        batched_queue::{
            assert_queue_zero_copy_inited, get_output_queue_account_size,
            get_output_queue_account_size_default,
        },
        QueueType,
    };

    use super::*;

    #[test]
    fn test_account_init() {
        let owner = Pubkey::new_unique();

        let queue_account_size = get_output_queue_account_size_default();

        let mut output_queue_account = BatchedQueueAccount::default();
        let mut output_queue_account_data = vec![0; queue_account_size];
        let output_queue_pubkey = Pubkey::new_unique();

        let mt_account_size = get_merkle_tree_account_size_default();
        let mut mt_account = BatchedMerkleTreeAccount::default();
        let mut mt_account_data = vec![0; mt_account_size];
        let mt_pubkey = Pubkey::new_unique();

        let params = InitStateTreeAccountsInstructionData::default();

        let merkle_tree_rent = 1_000_000_000;
        let queue_rent = 1_000_000_000;
        let additional_bytes_rent = 1000;
        init_batched_state_merkle_tree_accounts(
            owner,
            params.clone(),
            &mut output_queue_account,
            &mut output_queue_account_data,
            output_queue_pubkey,
            queue_rent,
            &mut mt_account,
            &mut mt_account_data,
            mt_pubkey,
            merkle_tree_rent,
            additional_bytes_rent,
        )
        .unwrap();
        let ref_output_queue_account = BatchedQueueAccount::get_output_queue_default(
            owner,
            None,
            None,
            params.rollover_threshold,
            0,
            params.output_queue_batch_size,
            params.output_queue_zkp_batch_size,
            params.additional_bytes,
            merkle_tree_rent + additional_bytes_rent + queue_rent,
            mt_pubkey,
        );
        assert_queue_zero_copy_inited(
            &mut output_queue_account,
            output_queue_account_data.as_mut_slice(),
            ref_output_queue_account,
            0,
        );
        let ref_mt_account = BatchedMerkleTreeAccount::get_state_tree_default(
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
            output_queue_pubkey,
        );
        assert_mt_zero_copy_inited(
            &mut mt_account,
            &mut mt_account_data,
            ref_mt_account,
            params.bloomfilter_num_iters,
        );
    }

    #[test]
    fn test_rnd_account_init() {
        use rand::SeedableRng;
        let mut rng = StdRng::seed_from_u64(0);
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
            let output_queue_zkp_batch_size = rng.gen_range(1..1000);

            let params = InitStateTreeAccountsInstructionData {
                index: rng.gen_range(0..1000),
                program_owner,
                forester,
                additional_bytes: rng.gen_range(0..1000),
                bloomfilter_num_iters: rng.gen_range(0..1000),
                input_queue_batch_size: rng.gen_range(1..1000) * input_queue_zkp_batch_size,
                output_queue_batch_size: rng.gen_range(1..1000) * output_queue_zkp_batch_size,
                input_queue_zkp_batch_size, //TODO: randomize 100,500,1000
                output_queue_zkp_batch_size,
                // 8 bits per byte, divisible by 8 for aligned memory
                bloom_filter_capacity: rng.gen_range(0..1000) * 8 * 8,
                network_fee: Some(rng.gen_range(0..1000)),
                rollover_threshold: Some(rng.gen_range(0..100)),
                close_threshold: None,
                root_history_capacity: rng.gen_range(0..1000),
            };
            let queue_account_size = get_output_queue_account_size(
                params.output_queue_batch_size,
                params.output_queue_zkp_batch_size,
            );

            let mut output_queue_account = BatchedQueueAccount::default();
            let mut output_queue_account_data = vec![0; queue_account_size];
            let output_queue_pubkey = Pubkey::new_unique();

            let mt_account_size = get_merkle_tree_account_size(
                params.input_queue_batch_size,
                params.bloom_filter_capacity,
                params.input_queue_zkp_batch_size,
                params.root_history_capacity,
            );
            let mut mt_account = BatchedMerkleTreeAccount::default();
            let mut mt_account_data = vec![0; mt_account_size];
            let mt_pubkey = Pubkey::new_unique();

            let merkle_tree_rent = rng.gen_range(0..10000000);
            let queue_rent = rng.gen_range(0..10000000);
            let additional_bytes_rent = rng.gen_range(0..10000000);
            init_batched_state_merkle_tree_accounts(
                owner,
                params.clone(),
                &mut output_queue_account,
                &mut output_queue_account_data,
                output_queue_pubkey,
                queue_rent,
                &mut mt_account,
                &mut mt_account_data,
                mt_pubkey,
                merkle_tree_rent,
                additional_bytes_rent,
            )
            .unwrap();
            let ref_output_queue_account = BatchedQueueAccount::get_output_queue(
                owner,
                program_owner,
                forester,
                params.rollover_threshold,
                params.index,
                params.output_queue_batch_size,
                params.output_queue_zkp_batch_size,
                params.additional_bytes,
                merkle_tree_rent + additional_bytes_rent + queue_rent,
                mt_pubkey,
                params.network_fee.unwrap_or_default(),
            );
            assert_queue_zero_copy_inited(
                &mut output_queue_account,
                output_queue_account_data.as_mut_slice(),
                ref_output_queue_account,
                0,
            );
            let ref_mt_account = BatchedMerkleTreeAccount::get_state_tree_default(
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
                output_queue_pubkey,
            );
            assert_mt_zero_copy_inited(
                &mut mt_account,
                &mut mt_account_data,
                ref_mt_account,
                params.bloomfilter_num_iters,
            );
        }
    }
}
