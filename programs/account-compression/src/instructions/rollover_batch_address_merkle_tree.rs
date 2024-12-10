use crate::{
    batched_merkle_tree::{BatchedMerkleTreeAccount, ZeroCopyBatchedMerkleTreeAccount},
    init_batched_address_merkle_tree_account,
    utils::{
        check_account::check_account_balance_is_rent_exempt,
        check_signer_is_registered_or_authority::{
            check_signer_is_registered_or_authority, GroupAccounts,
        },
        if_equals_none,
        transfer_lamports::transfer_lamports,
    },
    InitAddressTreeAccountsInstructionData, RegisteredProgram,
};
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

use super::assert_address_mt_zero_copy_inited;

#[derive(Accounts)]
pub struct RolloverBatchAddressMerkleTree<'info> {
    #[account(mut)]
    /// Signer used to receive rollover accounts rentexemption reimbursement.
    pub fee_payer: Signer<'info>,
    pub authority: Signer<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
    /// CHECK:  in account compression program.
    #[account(zero)]
    pub new_address_merkle_tree: AccountInfo<'info>,
    /// CHECK: cecked in manual deserialization.
    #[account(mut)]
    pub old_address_merkle_tree: AccountInfo<'info>,
}

impl<'info> GroupAccounts<'info> for RolloverBatchAddressMerkleTree<'info> {
    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

/// Checks:
/// 1. Merkle tree is ready to be rolled over
/// 2. Merkle tree is not already rolled over
/// 3. Rollover threshold is configured, if not tree cannot be rolled over
///
/// Actions:
/// 1. mark Merkle tree as rolled over in this slot
/// 2. initialize new Merkle tree and nullifier queue with the same parameters
pub fn process_rollover_batch_address_merkle_tree<'a, 'b, 'c: 'info, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, RolloverBatchAddressMerkleTree<'info>>,
    network_fee: Option<u64>,
) -> Result<()> {
    let old_merkle_tree_account =
        &mut ZeroCopyBatchedMerkleTreeAccount::address_tree_from_account_info_mut(
            &ctx.accounts.old_address_merkle_tree,
        )?;
    check_signer_is_registered_or_authority::<
        RolloverBatchAddressMerkleTree,
        ZeroCopyBatchedMerkleTreeAccount,
    >(&ctx, old_merkle_tree_account)?;

    let merkle_tree_rent = check_account_balance_is_rent_exempt(
        &ctx.accounts.new_address_merkle_tree.to_account_info(),
        ctx.accounts
            .old_address_merkle_tree
            .to_account_info()
            .data_len(),
    )?;
    let new_mt_data = &mut ctx.accounts.new_address_merkle_tree.try_borrow_mut_data()?;
    rollover_batch_address_tree(
        old_merkle_tree_account,
        new_mt_data,
        merkle_tree_rent,
        ctx.accounts.new_address_merkle_tree.key(),
        network_fee,
    )?;

    transfer_lamports(
        &ctx.accounts.old_address_merkle_tree.to_account_info(),
        &ctx.accounts.fee_payer.to_account_info(),
        merkle_tree_rent,
    )?;

    Ok(())
}

pub fn rollover_batch_address_tree(
    old_merkle_tree: &mut ZeroCopyBatchedMerkleTreeAccount,
    new_mt_data: &mut [u8],
    new_mt_rent: u64,
    new_mt_pubkey: Pubkey,
    network_fee: Option<u64>,
) -> Result<()> {
    old_merkle_tree
        .get_account_mut()
        .metadata
        .rollover(Pubkey::default(), new_mt_pubkey)?;
    let old_merkle_tree_account = old_merkle_tree.get_account();

    if old_merkle_tree_account.next_index
        < ((1 << old_merkle_tree_account.height)
            * old_merkle_tree_account
                .metadata
                .rollover_metadata
                .rollover_threshold
            / 100)
    {
        return err!(crate::errors::AccountCompressionErrorCode::NotReadyForRollover);
    }
    if old_merkle_tree_account
        .metadata
        .rollover_metadata
        .network_fee
        == 0
        && network_fee.is_some()
    {
        msg!("Network fee must be 0 for manually forested trees.");
        return err!(crate::errors::AccountCompressionErrorCode::InvalidNetworkFee);
    }

    let params = InitAddressTreeAccountsInstructionData {
        index: old_merkle_tree_account.metadata.rollover_metadata.index,
        program_owner: if_equals_none(
            old_merkle_tree_account
                .metadata
                .access_metadata
                .program_owner,
            Pubkey::default(),
        ),
        forester: if_equals_none(
            old_merkle_tree_account.metadata.access_metadata.forester,
            Pubkey::default(),
        ),
        height: old_merkle_tree_account.height,
        input_queue_batch_size: old_merkle_tree_account.queue.batch_size,
        input_queue_zkp_batch_size: old_merkle_tree_account.queue.zkp_batch_size,
        bloom_filter_capacity: old_merkle_tree_account.queue.bloom_filter_capacity,
        bloom_filter_num_iters: old_merkle_tree.batches[0].num_iters,
        root_history_capacity: old_merkle_tree_account.root_history_capacity,
        network_fee,
        rollover_threshold: if_equals_none(
            old_merkle_tree_account
                .metadata
                .rollover_metadata
                .rollover_threshold,
            u64::MAX,
        ),
        close_threshold: if_equals_none(
            old_merkle_tree_account
                .metadata
                .rollover_metadata
                .close_threshold,
            u64::MAX,
        ),
        input_queue_num_batches: old_merkle_tree_account.queue.num_batches,
    };

    init_batched_address_merkle_tree_account(
        old_merkle_tree_account.metadata.access_metadata.owner,
        params,
        new_mt_data,
        new_mt_rent,
    )
}

#[cfg(test)]
mod address_tree_rollover_tests {
    use light_bounded_vec::{BoundedVecMetadata, CyclicBoundedVecMetadata};
    use rand::thread_rng;
    use solana_sdk::pubkey::Pubkey;

    use crate::{
        assert_address_mt_zero_copy_inited,
        batch::Batch,
        batched_merkle_tree::{
            get_merkle_tree_account_size, get_merkle_tree_account_size_default,
            BatchedMerkleTreeAccount, ZeroCopyBatchedMerkleTreeAccount,
        },
        init_batched_address_merkle_tree_account, InitAddressTreeAccountsInstructionData,
    };

    use super::{assert_address_mt_roll_over, rollover_batch_address_tree};

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

        let mut new_mt_account_data = vec![0; mt_account_size];
        let new_mt_pubkey = Pubkey::new_unique();

        // 1. Failing: not ready for rollover
        {
            let mut mt_account_data = mt_account_data.clone();
            let result = rollover_batch_address_tree(
                &mut ZeroCopyBatchedMerkleTreeAccount::address_tree_from_bytes_mut(
                    &mut mt_account_data,
                )
                .unwrap(),
                &mut new_mt_account_data,
                merkle_tree_rent,
                new_mt_pubkey,
                params.network_fee,
            );
            assert_eq!(
                result,
                Err(crate::errors::AccountCompressionErrorCode::NotReadyForRollover.into())
            );
        }
        // 2. Failing rollover threshold not set
        {
            let mut mt_account_data = mt_account_data.clone();
            let merkle_tree = &mut ZeroCopyBatchedMerkleTreeAccount::address_tree_from_bytes_mut(
                &mut mt_account_data,
            )
            .unwrap();
            merkle_tree
                .get_account_mut()
                .metadata
                .rollover_metadata
                .rollover_threshold = u64::MAX;
            let result = rollover_batch_address_tree(
                merkle_tree,
                &mut new_mt_account_data,
                merkle_tree_rent,
                new_mt_pubkey,
                params.network_fee,
            );
            assert_eq!(
                result,
                Err(crate::errors::AccountCompressionErrorCode::RolloverNotConfigured.into())
            );
        }
        // 3. Functional: rollover address tree
        {
            let merkle_tree = &mut ZeroCopyBatchedMerkleTreeAccount::address_tree_from_bytes_mut(
                &mut mt_account_data,
            )
            .unwrap();
            merkle_tree.get_account_mut().next_index = 1 << merkle_tree.get_account().height;

            rollover_batch_address_tree(
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

            let result = rollover_batch_address_tree(
                &mut ZeroCopyBatchedMerkleTreeAccount::address_tree_from_bytes_mut(
                    &mut mt_account_data,
                )
                .unwrap(),
                &mut new_mt_account_data,
                merkle_tree_rent,
                new_mt_pubkey,
                params.network_fee,
            );
            assert_eq!(
                result,
                Err(crate::errors::AccountCompressionErrorCode::MerkleTreeAlreadyRolledOver.into())
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
            let mut new_mt_data = vec![0; mt_account_size];
            let new_mt_rent = merkle_tree_rent;
            let network_fee = params.network_fee;
            let new_mt_pubkey = Pubkey::new_unique();
            let mut zero_copy_old_mt =
                ZeroCopyBatchedMerkleTreeAccount::address_tree_from_bytes_mut(&mut mt_account_data)
                    .unwrap();
            zero_copy_old_mt.get_account_mut().next_index = 1 << params.height;
            rollover_batch_address_tree(
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
}

// TODO: assert that remainder of old_mt_account_data is not changed
pub fn assert_address_mt_roll_over(
    mut old_mt_account_data: Vec<u8>,
    mut old_ref_mt_account: BatchedMerkleTreeAccount,
    mut new_mt_account_data: Vec<u8>,
    new_ref_mt_account: BatchedMerkleTreeAccount,
    new_mt_pubkey: Pubkey,
    bloom_filter_num_iters: u64,
) {
    old_ref_mt_account
        .metadata
        .rollover(Pubkey::default(), new_mt_pubkey)
        .unwrap();
    let old_mt_account =
        ZeroCopyBatchedMerkleTreeAccount::address_tree_from_bytes_mut(&mut old_mt_account_data)
            .unwrap();
    assert_eq!(old_mt_account.get_account(), &old_ref_mt_account);

    assert_address_mt_zero_copy_inited(
        &mut new_mt_account_data,
        new_ref_mt_account,
        bloom_filter_num_iters,
    );
}
