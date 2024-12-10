use crate::{
    batched_merkle_tree::{BatchedMerkleTreeAccount, ZeroCopyBatchedMerkleTreeAccount},
    batched_queue::{
        assert_queue_zero_copy_inited, BatchedQueueAccount, ZeroCopyBatchedQueueAccount,
    },
    init_batched_state_merkle_tree_accounts,
    utils::{
        check_account::check_account_balance_is_rent_exempt,
        check_signer_is_registered_or_authority::{
            check_signer_is_registered_or_authority, GroupAccounts,
        },
        if_equals_none,
        transfer_lamports::transfer_lamports,
    },
    InitStateTreeAccountsInstructionData, RegisteredProgram,
};
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

use super::assert_state_mt_zero_copy_inited;

#[derive(Accounts)]
pub struct RolloverBatchStateMerkleTree<'info> {
    #[account(mut)]
    /// Signer used to receive rollover accounts rentexemption reimbursement.
    pub fee_payer: Signer<'info>,
    pub authority: Signer<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
    /// CHECK: is initialized in this instruction.
    #[account(zero)]
    pub new_state_merkle_tree: AccountInfo<'info>,
    /// CHECK: checked in manual deserialization.
    #[account(mut)]
    pub old_state_merkle_tree: AccountInfo<'info>,
    /// CHECK: is initialized in this instruction.
    #[account(zero)]
    pub new_output_queue: AccountInfo<'info>,
    /// CHECK: checked in manual deserialization.
    #[account(mut)]
    pub old_output_queue: AccountInfo<'info>,
}

impl<'info> GroupAccounts<'info> for RolloverBatchStateMerkleTree<'info> {
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
/// 2. initialize new Merkle tree and output queue with the same parameters
pub fn process_rollover_batch_state_merkle_tree<'a, 'b, 'c: 'info, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, RolloverBatchStateMerkleTree<'info>>,
    additional_bytes: u64,
    network_fee: Option<u64>,
) -> Result<()> {
    let old_merkle_tree_account =
        &mut ZeroCopyBatchedMerkleTreeAccount::state_tree_from_account_info_mut(
            &ctx.accounts.old_state_merkle_tree,
        )?;
    let old_output_queue = &mut ZeroCopyBatchedQueueAccount::output_queue_from_account_info_mut(
        &ctx.accounts.old_output_queue,
    )?;
    check_signer_is_registered_or_authority::<
        RolloverBatchStateMerkleTree,
        ZeroCopyBatchedMerkleTreeAccount,
    >(&ctx, old_merkle_tree_account)?;

    let merkle_tree_rent = check_account_balance_is_rent_exempt(
        &ctx.accounts.new_state_merkle_tree.to_account_info(),
        ctx.accounts
            .old_state_merkle_tree
            .to_account_info()
            .data_len(),
    )?;
    let queue_rent = check_account_balance_is_rent_exempt(
        &ctx.accounts.new_output_queue.to_account_info(),
        ctx.accounts.old_output_queue.to_account_info().data_len(),
    )?;
    let additional_bytes_rent = Rent::get()?.minimum_balance(additional_bytes as usize);
    let new_mt_data = &mut ctx.accounts.new_state_merkle_tree.try_borrow_mut_data()?;
    rollover_batch_state_tree(
        old_merkle_tree_account,
        ctx.accounts.old_state_merkle_tree.key(),
        new_mt_data,
        merkle_tree_rent,
        ctx.accounts.new_state_merkle_tree.key(),
        old_output_queue,
        ctx.accounts.old_output_queue.key(),
        &mut ctx.accounts.new_output_queue.try_borrow_mut_data()?,
        queue_rent,
        ctx.accounts.new_output_queue.key(),
        additional_bytes_rent,
        additional_bytes,
        network_fee,
    )?;

    transfer_lamports(
        &ctx.accounts.old_output_queue.to_account_info(),
        &ctx.accounts.fee_payer.to_account_info(),
        merkle_tree_rent + queue_rent + additional_bytes_rent,
    )?;

    Ok(())
}

pub fn rollover_batch_state_tree(
    old_merkle_tree: &mut ZeroCopyBatchedMerkleTreeAccount,
    old_mt_pubkey: Pubkey,
    new_mt_data: &mut [u8],
    new_mt_rent: u64,
    new_mt_pubkey: Pubkey,
    old_output_queue: &mut ZeroCopyBatchedQueueAccount,
    old_queue_pubkey: Pubkey,
    new_output_queue_data: &mut [u8],
    new_output_queue_rent: u64,
    new_output_queue_pubkey: Pubkey,
    additional_bytes_rent: u64,
    additional_bytes: u64,
    network_fee: Option<u64>,
) -> Result<()> {
    old_merkle_tree
        .get_account_mut()
        .metadata
        .rollover(old_queue_pubkey, new_mt_pubkey)?;

    old_output_queue
        .get_account_mut()
        .metadata
        .rollover(old_mt_pubkey, new_output_queue_pubkey)?;
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

    let params = InitStateTreeAccountsInstructionData {
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
        additional_bytes,
        output_queue_batch_size: old_output_queue.get_account().queue.batch_size,
        output_queue_zkp_batch_size: old_output_queue.get_account().queue.zkp_batch_size,
        output_queue_num_batches: old_output_queue.batches.len() as u64,
    };

    init_batched_state_merkle_tree_accounts(
        old_merkle_tree_account.metadata.access_metadata.owner,
        params,
        new_output_queue_data,
        new_output_queue_pubkey,
        new_output_queue_rent,
        new_mt_data,
        new_mt_pubkey,
        new_mt_rent,
        additional_bytes_rent,
    )
}

#[cfg(test)]
mod batch_state_tree_rollover_tests {
    use rand::{rngs::StdRng, Rng};
    use solana_sdk::pubkey::Pubkey;

    use crate::{
        assert_state_mt_zero_copy_inited,
        batched_merkle_tree::{
            get_merkle_tree_account_size, get_merkle_tree_account_size_default,
            BatchedMerkleTreeAccount, ZeroCopyBatchedMerkleTreeAccount,
        },
        batched_queue::{
            assert_queue_zero_copy_inited, get_output_queue_account_size,
            get_output_queue_account_size_default, ZeroCopyBatchedQueueAccount,
        },
        get_output_queue_account_default, init_batched_state_merkle_tree_accounts,
        rollover_batch_state_tree,
        tests::get_output_queue,
        InitStateTreeAccountsInstructionData,
    };

    use super::assert_state_mt_roll_over;

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
                params.clone(),
                &mut queue_account_data,
                queue_pubkey,
                queue_rent,
                &mut mt_account_data,
                mt_pubkey,
                merkle_tree_rent,
                additional_bytes_rent,
            )
            .unwrap();

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
                queue_pubkey,
                params.height,
                params.input_queue_num_batches,
            );
            assert_state_mt_zero_copy_inited(
                &mut mt_account_data,
                ref_mt_account,
                params.bloom_filter_num_iters,
            );

            let ref_output_queue_account = get_output_queue_account_default(
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
                params.height,
                params.output_queue_num_batches,
                params.network_fee.unwrap_or_default(),
            );
            assert_queue_zero_copy_inited(
                queue_account_data.as_mut_slice(),
                ref_output_queue_account,
                0,
            );
            let mut new_mt_account_data = vec![0; mt_account_size];
            let new_mt_pubkey = Pubkey::new_unique();

            let mut new_queue_account_data = vec![0; queue_account_size];
            let new_queue_pubkey = Pubkey::new_unique();

            // 1. Failing: not ready for rollover
            {
                let mut mt_account_data = mt_account_data.clone();
                let mut queue_account_data = queue_account_data.clone();
                let result = rollover_batch_state_tree(
                    &mut ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                        &mut mt_account_data,
                    )
                    .unwrap(),
                    mt_pubkey,
                    &mut new_mt_account_data,
                    merkle_tree_rent,
                    new_mt_pubkey,
                    &mut ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut queue_account_data)
                        .unwrap(),
                    queue_pubkey,
                    &mut new_queue_account_data,
                    queue_rent,
                    new_queue_pubkey,
                    additional_bytes_rent,
                    additional_bytes,
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
                let merkle_tree = &mut ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                    &mut mt_account_data,
                )
                .unwrap();
                merkle_tree
                    .get_account_mut()
                    .metadata
                    .rollover_metadata
                    .rollover_threshold = u64::MAX;
                let mut queue_account_data = queue_account_data.clone();
                let result = rollover_batch_state_tree(
                    &mut ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                        &mut mt_account_data,
                    )
                    .unwrap(),
                    mt_pubkey,
                    &mut new_mt_account_data,
                    merkle_tree_rent,
                    new_mt_pubkey,
                    &mut ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut queue_account_data)
                        .unwrap(),
                    queue_pubkey,
                    &mut new_queue_account_data,
                    queue_rent,
                    new_queue_pubkey,
                    additional_bytes_rent,
                    additional_bytes,
                    params.network_fee,
                );
                assert_eq!(
                    result,
                    Err(crate::errors::AccountCompressionErrorCode::RolloverNotConfigured.into())
                );
            }
            // 3. Failing: invalid mt size
            {
                let mut mt_account_data = mt_account_data.clone();
                let mut queue_account_data = queue_account_data.clone();
                let merkle_tree = &mut ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                    &mut mt_account_data,
                )
                .unwrap();
                merkle_tree.get_account_mut().next_index = 1 << merkle_tree.get_account().height;
                let mut new_mt_account_data = vec![0; mt_account_size - 1];
                let mut new_queue_account_data = vec![0; queue_account_size];

                let result = rollover_batch_state_tree(
                    merkle_tree,
                    mt_pubkey,
                    &mut new_mt_account_data,
                    merkle_tree_rent,
                    new_mt_pubkey,
                    &mut ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut queue_account_data)
                        .unwrap(),
                    queue_pubkey,
                    &mut new_queue_account_data,
                    queue_rent,
                    new_queue_pubkey,
                    additional_bytes_rent,
                    additional_bytes,
                    params.network_fee,
                );
                assert_eq!(
                    result,
                    Err(crate::errors::AccountCompressionErrorCode::SizeMismatch.into())
                );
            }
            // 4. Failing: invalid queue size
            {
                let mut mt_account_data = mt_account_data.clone();
                let mut queue_account_data = queue_account_data.clone();
                let merkle_tree = &mut ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                    &mut mt_account_data,
                )
                .unwrap();
                merkle_tree.get_account_mut().next_index = 1 << merkle_tree.get_account().height;
                let mut new_mt_account_data = vec![0; mt_account_size];
                let mut new_queue_account_data = vec![0; queue_account_size - 1];

                let result = rollover_batch_state_tree(
                    merkle_tree,
                    mt_pubkey,
                    &mut new_mt_account_data,
                    merkle_tree_rent,
                    new_mt_pubkey,
                    &mut ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut queue_account_data)
                        .unwrap(),
                    queue_pubkey,
                    &mut new_queue_account_data,
                    queue_rent,
                    new_queue_pubkey,
                    additional_bytes_rent,
                    additional_bytes,
                    params.network_fee,
                );
                assert_eq!(
                    result,
                    Err(crate::errors::AccountCompressionErrorCode::SizeMismatch.into())
                );
            }
            // 5. Functional: rollover address tree
            {
                let merkle_tree = &mut ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                    &mut mt_account_data,
                )
                .unwrap();
                merkle_tree.get_account_mut().next_index = 1 << merkle_tree.get_account().height;
                println!("new_mt_pubkey {:?}", new_mt_pubkey);
                println!("new_queue_pubkey {:?}", new_queue_pubkey);
                rollover_batch_state_tree(
                    merkle_tree,
                    mt_pubkey,
                    &mut new_mt_account_data,
                    merkle_tree_rent,
                    new_mt_pubkey,
                    &mut ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut queue_account_data)
                        .unwrap(),
                    queue_pubkey,
                    &mut new_queue_account_data,
                    queue_rent,
                    new_queue_pubkey,
                    additional_bytes_rent,
                    additional_bytes,
                    params.network_fee,
                )
                .unwrap();

                let mut ref_rolledover_mt = ref_mt_account.clone();
                ref_rolledover_mt.next_index = 1 << merkle_tree.get_account().height;
                let mut new_ref_output_queue_account = ref_output_queue_account.clone();
                new_ref_output_queue_account
                    .metadata
                    .rollover_metadata
                    .additional_bytes = additional_bytes;
                new_ref_output_queue_account.metadata.associated_merkle_tree = new_mt_pubkey;
                let mut new_ref_merkle_tree_account = ref_mt_account.clone();
                new_ref_merkle_tree_account.metadata.associated_queue = new_queue_pubkey;

                assert_state_mt_roll_over(
                    mt_account_data.to_vec(),
                    new_ref_merkle_tree_account,
                    new_mt_account_data.to_vec(),
                    mt_pubkey,
                    new_mt_pubkey,
                    params.bloom_filter_num_iters,
                    ref_rolledover_mt,
                    queue_account_data.to_vec(),
                    new_ref_output_queue_account,
                    new_queue_account_data.to_vec(),
                    new_queue_pubkey,
                    ref_output_queue_account,
                    queue_pubkey,
                    1,
                );
            }
            // 6. Failing: already rolled over
            {
                let mut mt_account_data = mt_account_data.clone();
                let mut queue_account_data = queue_account_data.clone();

                let mut new_mt_account_data = vec![0; mt_account_size];
                let mut new_queue_account_data = vec![0; queue_account_size];

                let result = rollover_batch_state_tree(
                    &mut ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                        &mut mt_account_data,
                    )
                    .unwrap(),
                    mt_pubkey,
                    &mut new_mt_account_data,
                    merkle_tree_rent,
                    new_mt_pubkey,
                    &mut ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut queue_account_data)
                        .unwrap(),
                    queue_pubkey,
                    &mut new_queue_account_data,
                    queue_rent,
                    new_queue_pubkey,
                    additional_bytes_rent,
                    additional_bytes,
                    params.network_fee,
                );
                assert_eq!(
                    result,
                    Err(
                        crate::errors::AccountCompressionErrorCode::MerkleTreeAlreadyRolledOver
                            .into()
                    )
                );
            }
        }
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
                params.clone(),
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
            let new_queue_pubkey = Pubkey::new_unique();
            // 7. failing Invalid network fee
            {
                let mut mt_account_data = mt_account_data.clone();
                let mut queue_account_data = queue_account_data.clone();
                let merkle_tree = &mut ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                    &mut mt_account_data,
                )
                .unwrap();
                merkle_tree.get_account_mut().next_index = 1 << merkle_tree.get_account().height;
                let mut new_mt_account_data = vec![0; mt_account_size];
                let mut new_queue_account_data = vec![0; queue_account_size];

                let result = rollover_batch_state_tree(
                    merkle_tree,
                    mt_pubkey,
                    &mut new_mt_account_data,
                    merkle_tree_rent,
                    new_mt_pubkey,
                    &mut ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut queue_account_data)
                        .unwrap(),
                    queue_pubkey,
                    &mut new_queue_account_data,
                    queue_rent,
                    new_queue_pubkey,
                    additional_bytes_rent,
                    additional_bytes,
                    Some(1),
                );
                assert_eq!(
                    result,
                    Err(crate::errors::AccountCompressionErrorCode::InvalidNetworkFee.into())
                );
            }
            let mut new_mt_account_data = vec![0; mt_account_size];
            let mut new_queue_account_data = vec![0; queue_account_size];
            let mut ref_mt_account = BatchedMerkleTreeAccount::get_state_tree_default(
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
                queue_pubkey,
                params.height,
                params.input_queue_num_batches,
            );
            ref_mt_account.metadata.access_metadata.forester = forester;
            let mut ref_output_queue_account = get_output_queue_account_default(
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
                params.height,
                params.output_queue_num_batches,
                params.network_fee.unwrap_or_default(),
            );
            ref_output_queue_account
                .metadata
                .rollover_metadata
                .network_fee = 0;
            ref_output_queue_account.metadata.access_metadata.forester = forester;
            assert_queue_zero_copy_inited(
                queue_account_data.as_mut_slice(),
                ref_output_queue_account,
                0,
            );
            // 8. Functional: rollover address tree with network fee 0 additional bytes 0
            {
                let merkle_tree = &mut ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                    &mut mt_account_data,
                )
                .unwrap();
                merkle_tree.get_account_mut().next_index = 1 << merkle_tree.get_account().height;
                println!("new_mt_pubkey {:?}", new_mt_pubkey);
                println!("new_queue_pubkey {:?}", new_queue_pubkey);
                rollover_batch_state_tree(
                    merkle_tree,
                    mt_pubkey,
                    &mut new_mt_account_data,
                    merkle_tree_rent,
                    new_mt_pubkey,
                    &mut ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut queue_account_data)
                        .unwrap(),
                    queue_pubkey,
                    &mut new_queue_account_data,
                    queue_rent,
                    new_queue_pubkey,
                    additional_bytes_rent,
                    additional_bytes,
                    params.network_fee,
                )
                .unwrap();

                let mut ref_rolledover_mt = ref_mt_account.clone();
                ref_rolledover_mt.next_index = 1 << merkle_tree.get_account().height;
                let mut new_ref_output_queue_account = ref_output_queue_account.clone();
                new_ref_output_queue_account
                    .metadata
                    .rollover_metadata
                    .additional_bytes = additional_bytes;
                new_ref_output_queue_account.metadata.associated_merkle_tree = new_mt_pubkey;
                let mut new_ref_merkle_tree_account = ref_mt_account.clone();
                new_ref_merkle_tree_account.metadata.associated_queue = new_queue_pubkey;

                assert_state_mt_roll_over(
                    mt_account_data.to_vec(),
                    new_ref_merkle_tree_account,
                    new_mt_account_data.to_vec(),
                    mt_pubkey,
                    new_mt_pubkey,
                    params.bloom_filter_num_iters,
                    ref_rolledover_mt,
                    queue_account_data.to_vec(),
                    new_ref_output_queue_account,
                    new_queue_account_data.to_vec(),
                    new_queue_pubkey,
                    ref_output_queue_account,
                    queue_pubkey,
                    1,
                );
            }
        }
    }

    #[test]
    fn test_rnd_rollover() {
        use rand::SeedableRng;
        let mut rng = StdRng::seed_from_u64(0);
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
                Some(rng.gen_range(0..1000))
            } else {
                None
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
                input_queue_num_batches: rng.gen_range(1..4),
                output_queue_num_batches: rng.gen_range(1..4),
                height: rng.gen_range(1..32),
            };

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

            let merkle_tree_rent = rng.gen_range(0..10000000);
            let queue_rent = rng.gen_range(0..10000000);
            let additional_bytes_rent = rng.gen_range(0..10000000);
            let additional_bytes = rng.gen_range(0..1000);
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
            let ref_output_queue_account = get_output_queue(
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
                params.output_queue_num_batches,
                params.height,
            );
            assert_queue_zero_copy_inited(
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
                params.height,
                params.input_queue_num_batches,
            );
            assert_state_mt_zero_copy_inited(
                &mut mt_account_data,
                ref_mt_account,
                params.bloom_filter_num_iters,
            );

            let mut new_mt_account_data = vec![0; mt_account_size];
            let new_mt_pubkey = Pubkey::new_unique();

            let mut new_queue_account_data = vec![0; queue_account_size];
            let new_queue_pubkey = Pubkey::new_unique();

            let merkle_tree = &mut ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(
                &mut mt_account_data,
            )
            .unwrap();
            merkle_tree.get_account_mut().next_index = 1 << merkle_tree.get_account().height;
            rollover_batch_state_tree(
                merkle_tree,
                mt_pubkey,
                &mut new_mt_account_data,
                merkle_tree_rent,
                new_mt_pubkey,
                &mut ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut output_queue_account_data)
                    .unwrap(),
                output_queue_pubkey,
                &mut new_queue_account_data,
                queue_rent,
                new_queue_pubkey,
                additional_bytes_rent,
                additional_bytes,
                params.network_fee,
            )
            .unwrap();

            let mut ref_rolledover_mt = ref_mt_account.clone();
            ref_rolledover_mt.next_index = 1 << merkle_tree.get_account().height;
            let mut new_ref_output_queue_account = ref_output_queue_account.clone();
            new_ref_output_queue_account
                .metadata
                .rollover_metadata
                .additional_bytes = additional_bytes;
            new_ref_output_queue_account.metadata.associated_merkle_tree = new_mt_pubkey;
            let mut new_ref_merkle_tree_account = ref_mt_account.clone();
            new_ref_merkle_tree_account.metadata.associated_queue = new_queue_pubkey;

            assert_state_mt_roll_over(
                mt_account_data.to_vec(),
                new_ref_merkle_tree_account,
                new_mt_account_data.to_vec(),
                mt_pubkey,
                new_mt_pubkey,
                params.bloom_filter_num_iters,
                ref_rolledover_mt,
                output_queue_account_data.to_vec(),
                new_ref_output_queue_account,
                new_queue_account_data.to_vec(),
                new_queue_pubkey,
                ref_output_queue_account,
                output_queue_pubkey,
                1,
            );
        }
    }
}

pub fn assert_state_mt_roll_over(
    mt_account_data: Vec<u8>,
    ref_mt_account: BatchedMerkleTreeAccount,
    new_mt_account_data: Vec<u8>,
    old_mt_pubkey: Pubkey,
    new_mt_pubkey: Pubkey,
    bloom_filter_num_iters: u64,
    ref_rolledover_mt: BatchedMerkleTreeAccount,
    mut queue_account_data: Vec<u8>,
    ref_queue_account: BatchedQueueAccount,
    mut new_queue_account_data: Vec<u8>,
    new_queue_pubkey: Pubkey,
    mut ref_rolledover_queue: BatchedQueueAccount,
    old_queue_pubkey: Pubkey,
    slot: u64,
) {
    ref_rolledover_queue
        .metadata
        .rollover(old_mt_pubkey, new_queue_pubkey)
        .unwrap();
    ref_rolledover_queue
        .metadata
        .rollover_metadata
        .rolledover_slot = slot;

    assert_queue_zero_copy_inited(&mut new_queue_account_data, ref_queue_account, 0);
    println!("asserted queue roll over");

    let zero_copy_queue =
        ZeroCopyBatchedQueueAccount::from_bytes_mut(&mut queue_account_data).unwrap();
    assert_eq!(
        zero_copy_queue.get_account().metadata,
        ref_rolledover_queue.metadata
    );
    assert_mt_roll_over(
        mt_account_data,
        ref_mt_account,
        new_mt_account_data,
        new_mt_pubkey,
        bloom_filter_num_iters,
        ref_rolledover_mt,
        old_queue_pubkey,
        slot,
    )
}

// TODO: assert that the rest of the rolled over account didn't change
pub fn assert_mt_roll_over(
    mut mt_account_data: Vec<u8>,
    ref_mt_account: BatchedMerkleTreeAccount,
    mut new_mt_account_data: Vec<u8>,
    new_mt_pubkey: Pubkey,
    bloom_filter_num_iters: u64,
    mut ref_rolledover_mt: BatchedMerkleTreeAccount,
    old_queue_pubkey: Pubkey,
    slot: u64,
) {
    ref_rolledover_mt
        .metadata
        .rollover(old_queue_pubkey, new_mt_pubkey)
        .unwrap();
    ref_rolledover_mt.metadata.rollover_metadata.rolledover_slot = slot;
    let zero_copy_mt =
        ZeroCopyBatchedMerkleTreeAccount::state_tree_from_bytes_mut(&mut mt_account_data).unwrap();
    assert_eq!(*zero_copy_mt.get_account(), ref_rolledover_mt);

    assert_state_mt_zero_copy_inited(
        &mut new_mt_account_data,
        ref_mt_account,
        bloom_filter_num_iters,
    );
}
