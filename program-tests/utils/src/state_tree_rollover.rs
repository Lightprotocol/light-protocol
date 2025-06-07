#![allow(clippy::await_holding_refcell_ref)]

use std::mem;

use account_compression::{
    self, state::QueueAccount, NullifierQueueConfig, StateMerkleTreeAccount, StateMerkleTreeConfig,
    ID,
};
use anchor_lang::{
    prelude::AccountLoader, Discriminator, InstructionData, Lamports, ToAccountMetas,
};
use forester_utils::{account_zero_copy::get_hash_set, instructions::create_account_instruction};
use light_account_checks::discriminator::Discriminator as LightDiscriminator;
use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use light_client::rpc::{errors::RpcError, Rpc};
use light_concurrent_merkle_tree::{
    copy::ConcurrentMerkleTreeCopy, zero_copy::ConcurrentMerkleTreeZeroCopyMut,
};
use light_hasher::Poseidon;
use light_program_test::{program_test::test_rpc::TestRpc, Indexer};
use solana_sdk::{
    account::WritableAccount,
    account_info::AccountInfo,
    clock::Slot,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

use crate::assert_rollover::{
    assert_rolledover_merkle_trees, assert_rolledover_merkle_trees_metadata,
    assert_rolledover_queues_metadata,
};

pub enum StateMerkleTreeRolloverMode {
    QueueInvalidSize,
    TreeInvalidSize,
}

#[allow(clippy::too_many_arguments)]
pub async fn perform_state_merkle_tree_roll_over<R: Rpc>(
    rpc: &mut R,
    new_nullifier_queue_keypair: &Keypair,
    new_state_merkle_tree_keypair: &Keypair,
    merkle_tree_pubkey: &Pubkey,
    nullifier_queue_pubkey: &Pubkey,
    merkle_tree_config: &StateMerkleTreeConfig,
    queue_config: &NullifierQueueConfig,
    mode: Option<StateMerkleTreeRolloverMode>,
) -> Result<(solana_sdk::signature::Signature, Slot), RpcError> {
    let payer_pubkey = rpc.get_payer().pubkey();
    let mut size = QueueAccount::size(queue_config.capacity as usize).unwrap();
    if let Some(StateMerkleTreeRolloverMode::QueueInvalidSize) = mode {
        size += 1;
    }
    let create_nullifier_queue_instruction = create_account_instruction(
        &payer_pubkey,
        size,
        rpc.get_minimum_balance_for_rent_exemption(size)
            .await
            .unwrap(),
        &ID,
        Some(new_nullifier_queue_keypair),
    );
    let mut state_tree_size = account_compression::state::StateMerkleTreeAccount::size(
        merkle_tree_config.height as usize,
        merkle_tree_config.changelog_size as usize,
        merkle_tree_config.roots_size as usize,
        merkle_tree_config.canopy_depth as usize,
    );
    if let Some(StateMerkleTreeRolloverMode::TreeInvalidSize) = mode {
        state_tree_size += 1;
    }
    let create_state_merkle_tree_instruction = create_account_instruction(
        &payer_pubkey,
        state_tree_size,
        rpc.get_minimum_balance_for_rent_exemption(state_tree_size)
            .await
            .unwrap(),
        &ID,
        Some(new_state_merkle_tree_keypair),
    );
    let instruction_data =
        account_compression::instruction::RolloverStateMerkleTreeAndNullifierQueue {};
    let accounts = account_compression::accounts::RolloverStateMerkleTreeAndNullifierQueue {
        fee_payer: rpc.get_payer().pubkey(),
        authority: rpc.get_payer().pubkey(),
        registered_program_pda: None,
        new_state_merkle_tree: new_state_merkle_tree_keypair.pubkey(),
        new_nullifier_queue: new_nullifier_queue_keypair.pubkey(),
        old_state_merkle_tree: *merkle_tree_pubkey,
        old_nullifier_queue: *nullifier_queue_pubkey,
    };
    let instruction = Instruction {
        program_id: account_compression::ID,
        accounts: [
            accounts.to_account_metas(Some(true)),
            vec![AccountMeta::new(*merkle_tree_pubkey, false)],
        ]
        .concat(),
        data: instruction_data.data(),
    };
    let blockhash = rpc.get_latest_blockhash().await?;
    let transaction = Transaction::new_signed_with_payer(
        &[
            create_nullifier_queue_instruction,
            create_state_merkle_tree_instruction,
            instruction,
        ],
        Some(&rpc.get_payer().pubkey()),
        &vec![
            &rpc.get_payer(),
            &new_nullifier_queue_keypair,
            &new_state_merkle_tree_keypair,
        ],
        blockhash.0,
    );
    rpc.process_transaction_with_context(transaction).await
}

pub async fn set_state_merkle_tree_next_index<R: Rpc + TestRpc + Indexer>(
    rpc: &mut R,
    merkle_tree_pubkey: &Pubkey,
    next_index: u64,
    lamports: u64,
) {
    let mut merkle_tree = rpc.get_account(*merkle_tree_pubkey).await.unwrap().unwrap();
    let discriminator = &merkle_tree.data[0..8];
    match discriminator {
        StateMerkleTreeAccount::DISCRIMINATOR => {
            {
                let merkle_tree_deserialized =
                    &mut ConcurrentMerkleTreeZeroCopyMut::<Poseidon, 26>::from_bytes_zero_copy_mut(
                        &mut merkle_tree.data[8 + std::mem::size_of::<StateMerkleTreeAccount>()..],
                    )
                    .unwrap();
                while merkle_tree_deserialized.next_index() < next_index as usize {
                    merkle_tree_deserialized.inc_next_index().unwrap();
                }
            }
            merkle_tree.set_lamports(lamports);

            rpc.set_account(*merkle_tree_pubkey, merkle_tree);
            let mut merkle_tree = rpc.get_account(*merkle_tree_pubkey).await.unwrap().unwrap();
            let merkle_tree_deserialized =
                ConcurrentMerkleTreeZeroCopyMut::<Poseidon, 26>::from_bytes_zero_copy_mut(
                    &mut merkle_tree.data[8 + std::mem::size_of::<StateMerkleTreeAccount>()..],
                )
                .unwrap();
            assert_eq!(merkle_tree_deserialized.next_index() as u64, next_index);
        }
        BatchedMerkleTreeAccount::LIGHT_DISCRIMINATOR_SLICE => {}
        _ => panic!("Invalid discriminator"),
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn assert_rolled_over_pair<R: Rpc>(
    payer: &Pubkey,
    rpc: &mut R,
    fee_payer_prior_balance: &u64,
    old_merkle_tree_pubkey: &Pubkey,
    old_nullifier_queue_pubkey: &Pubkey,
    new_merkle_tree_pubkey: &Pubkey,
    new_nullifier_queue_pubkey: &Pubkey,
    current_slot: u64,
    additional_rent: u64,
    num_signatures: u64,
) {
    let mut new_mt_account = rpc
        .get_account(*new_merkle_tree_pubkey)
        .await
        .unwrap()
        .unwrap();
    let mut new_mt_lamports = 0u64;
    let old_account_info = AccountInfo::new(
        new_merkle_tree_pubkey,
        false,
        false,
        &mut new_mt_lamports,
        &mut new_mt_account.data,
        &ID,
        false,
        0u64,
    );
    let new_mt_account =
        AccountLoader::<StateMerkleTreeAccount>::try_from(&old_account_info).unwrap();
    let new_loaded_mt_account = new_mt_account.load().unwrap();

    let mut old_mt_account = rpc
        .get_account(*old_merkle_tree_pubkey)
        .await
        .unwrap()
        .unwrap();

    let mut old_mt_lamports = 0u64;
    let new_account_info = AccountInfo::new(
        old_merkle_tree_pubkey,
        false,
        false,
        &mut old_mt_lamports,
        &mut old_mt_account.data,
        &account_compression::ID,
        false,
        0u64,
    );
    let old_mt_account =
        AccountLoader::<StateMerkleTreeAccount>::try_from(&new_account_info).unwrap();
    let old_loaded_mt_account = old_mt_account.load().unwrap();

    assert_rolledover_merkle_trees_metadata(
        &old_loaded_mt_account.metadata,
        &new_loaded_mt_account.metadata,
        current_slot,
        new_nullifier_queue_pubkey,
    );

    let old_mt_data = old_account_info.try_borrow_data().unwrap();
    let old_mt = ConcurrentMerkleTreeCopy::<Poseidon, 26>::from_bytes_copy(
        &old_mt_data[8 + mem::size_of::<StateMerkleTreeAccount>()..],
    )
    .unwrap();
    let new_mt_data = new_account_info.try_borrow_data().unwrap();
    let new_mt = ConcurrentMerkleTreeCopy::<Poseidon, 26>::from_bytes_copy(
        &new_mt_data[8 + mem::size_of::<StateMerkleTreeAccount>()..],
    )
    .unwrap();
    assert_rolledover_merkle_trees(&old_mt, &new_mt);

    {
        let mut new_queue_account = rpc
            .get_account(*new_nullifier_queue_pubkey)
            .await
            .unwrap()
            .unwrap();
        let mut new_mt_lamports = 0u64;
        let account_info = AccountInfo::new(
            new_nullifier_queue_pubkey,
            false,
            false,
            &mut new_mt_lamports,
            &mut new_queue_account.data,
            &account_compression::ID,
            false,
            0u64,
        );
        let new_queue_account = AccountLoader::<QueueAccount>::try_from(&account_info).unwrap();
        let new_loaded_queue_account = new_queue_account.load().unwrap();
        let mut old_queue_account = rpc
            .get_account(*old_nullifier_queue_pubkey)
            .await
            .unwrap()
            .unwrap();
        let mut old_mt_lamports = 0u64;
        let account_info = AccountInfo::new(
            old_nullifier_queue_pubkey,
            false,
            false,
            &mut old_mt_lamports,
            &mut old_queue_account.data,
            &account_compression::ID,
            false,
            0u64,
        );
        let old_queue_account = AccountLoader::<QueueAccount>::try_from(&account_info).unwrap();
        let old_loaded_queue_account = old_queue_account.load().unwrap();

        assert_rolledover_queues_metadata(
            &old_loaded_queue_account.metadata,
            &new_loaded_queue_account.metadata,
            current_slot,
            new_merkle_tree_pubkey,
            new_nullifier_queue_pubkey,
            old_mt_account.get_lamports(),
            new_mt_account.get_lamports(),
            new_queue_account.get_lamports(),
        );
    }
    let fee_payer_post_balance = rpc.get_account(*payer).await.unwrap().unwrap().lamports;
    // rent is reimbursed, 3 signatures cost 3 x 5000 lamports
    assert_eq!(
        *fee_payer_prior_balance,
        fee_payer_post_balance + 5000 * num_signatures + additional_rent
    );
    let old_address_queue =
        unsafe { get_hash_set::<QueueAccount, R>(rpc, *old_nullifier_queue_pubkey).await };
    let new_address_queue =
        unsafe { get_hash_set::<QueueAccount, R>(rpc, *new_nullifier_queue_pubkey).await };

    assert_eq!(
        old_address_queue.get_capacity(),
        new_address_queue.get_capacity()
    );

    assert_eq!(
        old_address_queue.sequence_threshold,
        new_address_queue.sequence_threshold,
    );
}
