#![allow(clippy::await_holding_refcell_ref)]

use account_compression::{
    accounts, instruction, state::QueueAccount, AddressMerkleTreeAccount, AddressMerkleTreeConfig,
    AddressQueueConfig,
};
use anchor_lang::{
    prelude::AccountLoader, InstructionData, Key, Lamports, ToAccountInfo, ToAccountMetas,
};
use forester_utils::{
    account_zero_copy::{get_hash_set, get_indexed_merkle_tree},
    instructions::create_account_instruction,
    registry::{
        create_rollover_address_merkle_tree_instructions,
        create_rollover_state_merkle_tree_instructions,
    },
};
use light_client::rpc::{Rpc, RpcError};
use light_hasher::Poseidon;
use light_indexed_merkle_tree::zero_copy::IndexedMerkleTreeZeroCopyMut;
use light_program_test::{program_test::TestRpc, Indexer};
use solana_sdk::{
    account::WritableAccount, account_info::AccountInfo, clock::Slot, instruction::Instruction,
    pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction,
};

use crate::assert_rollover::{
    assert_rolledover_merkle_trees, assert_rolledover_merkle_trees_metadata,
    assert_rolledover_queues_metadata,
};

pub async fn set_address_merkle_tree_next_index<R: Rpc + TestRpc + Indexer>(
    rpc: &mut R,
    merkle_tree_pubkey: &Pubkey,
    next_index: u64,
    lamports: u64,
) {
    let mut merkle_tree = rpc.get_account(*merkle_tree_pubkey).await.unwrap().unwrap();
    let merkle_tree_deserialized =
        &mut IndexedMerkleTreeZeroCopyMut::<Poseidon, usize, 26, 16>::from_bytes_zero_copy_mut(
            &mut merkle_tree.data[8 + std::mem::size_of::<AddressMerkleTreeAccount>()..],
        )
        .unwrap();
    while merkle_tree_deserialized.next_index() < next_index as usize {
        merkle_tree_deserialized.inc_next_index().unwrap();
    }
    let mut account_share_data = merkle_tree;
    account_share_data.set_lamports(lamports);
    rpc.set_account(*merkle_tree_pubkey, account_share_data);
    let mut merkle_tree = rpc.get_account(*merkle_tree_pubkey).await.unwrap().unwrap();
    let merkle_tree_deserialized =
        IndexedMerkleTreeZeroCopyMut::<Poseidon, usize, 26, 16>::from_bytes_zero_copy_mut(
            &mut merkle_tree.data[8 + std::mem::size_of::<AddressMerkleTreeAccount>()..],
        )
        .unwrap();
    assert_eq!(merkle_tree_deserialized.next_index() as u64, next_index);
}

pub async fn perform_address_merkle_tree_roll_over<R: Rpc>(
    context: &mut R,
    new_queue_keypair: &Keypair,
    new_address_merkle_tree_keypair: &Keypair,
    old_merkle_tree_pubkey: &Pubkey,
    old_queue_pubkey: &Pubkey,
    merkle_tree_config: &AddressMerkleTreeConfig,
    queue_config: &AddressQueueConfig,
) -> Result<solana_sdk::signature::Signature, RpcError> {
    let payer = context.get_payer().insecure_clone();
    let size = QueueAccount::size(queue_config.capacity as usize).unwrap();
    let account_create_ix = create_account_instruction(
        &payer.pubkey(),
        size,
        context
            .get_minimum_balance_for_rent_exemption(size)
            .await
            .unwrap(),
        &account_compression::ID,
        Some(new_queue_keypair),
    );

    let size = AddressMerkleTreeAccount::size(
        merkle_tree_config.height as usize,
        merkle_tree_config.changelog_size as usize,
        merkle_tree_config.roots_size as usize,
        merkle_tree_config.canopy_depth as usize,
        merkle_tree_config.address_changelog_size as usize,
    );
    let mt_account_create_ix = create_account_instruction(
        &payer.pubkey(),
        size,
        context
            .get_minimum_balance_for_rent_exemption(size)
            .await
            .unwrap(),
        &account_compression::ID,
        Some(new_address_merkle_tree_keypair),
    );
    let instruction_data = instruction::RolloverAddressMerkleTreeAndQueue {};
    let accounts = accounts::RolloverAddressMerkleTreeAndQueue {
        fee_payer: context.get_payer().pubkey(),
        authority: context.get_payer().pubkey(),
        registered_program_pda: None,
        new_address_merkle_tree: new_address_merkle_tree_keypair.pubkey(),
        new_queue: new_queue_keypair.pubkey(),
        old_address_merkle_tree: *old_merkle_tree_pubkey,
        old_queue: *old_queue_pubkey,
    };
    let instruction = Instruction {
        program_id: account_compression::ID,
        accounts: [accounts.to_account_metas(Some(true))].concat(),
        data: instruction_data.data(),
    };
    let blockhash = context.get_latest_blockhash().await?;
    let transaction = Transaction::new_signed_with_payer(
        &[account_create_ix, mt_account_create_ix, instruction],
        Some(&context.get_payer().pubkey()),
        &vec![
            &context.get_payer(),
            &new_queue_keypair,
            &new_address_merkle_tree_keypair,
        ],
        blockhash.0,
    );
    context.process_transaction(transaction).await
}

pub async fn assert_rolled_over_address_merkle_tree_and_queue<R: Rpc>(
    payer: &Pubkey,
    rpc: &mut R,
    fee_payer_prior_balance: &u64,
    old_merkle_tree_pubkey: &Pubkey,
    old_queue_pubkey: &Pubkey,
    new_merkle_tree_pubkey: &Pubkey,
    new_queue_pubkey: &Pubkey,
) {
    let current_slot = rpc.get_slot().await.unwrap();

    let mut new_mt_account = rpc
        .get_account(*new_merkle_tree_pubkey)
        .await
        .unwrap()
        .unwrap();
    let mut new_mt_lamports = 0u64;
    let account_info = AccountInfo::new(
        new_merkle_tree_pubkey,
        false,
        false,
        &mut new_mt_lamports,
        &mut new_mt_account.data,
        &account_compression::ID,
        false,
        0u64,
    );
    let new_mt_account =
        AccountLoader::<AddressMerkleTreeAccount>::try_from(&account_info).unwrap();
    let new_loaded_mt_account = new_mt_account.load().unwrap();

    let mut old_mt_account = rpc
        .get_account(*old_merkle_tree_pubkey)
        .await
        .unwrap()
        .unwrap();

    let mut old_mt_lamports = 0u64;
    let account_info = AccountInfo::new(
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
        AccountLoader::<AddressMerkleTreeAccount>::try_from(&account_info).unwrap();
    let old_loaded_mt_account = old_mt_account.load().unwrap();
    assert_eq!(
        new_mt_account.to_account_info().data.borrow().len(),
        old_mt_account.to_account_info().data.borrow().len()
    );
    assert_rolledover_merkle_trees_metadata(
        &old_loaded_mt_account.metadata,
        &new_loaded_mt_account.metadata,
        current_slot,
        new_queue_pubkey,
    );

    drop(new_loaded_mt_account);
    drop(old_loaded_mt_account);

    let struct_old =
        get_indexed_merkle_tree::<AddressMerkleTreeAccount, R, Poseidon, usize, 26, 16>(
            rpc,
            old_mt_account.key(),
        )
        .await;
    let struct_new =
        get_indexed_merkle_tree::<AddressMerkleTreeAccount, R, Poseidon, usize, 26, 16>(
            rpc,
            new_mt_account.key(),
        )
        .await;
    assert_rolledover_merkle_trees(&struct_old.merkle_tree, &struct_new.merkle_tree);
    assert_eq!(
        struct_old.merkle_tree.changelog.capacity(),
        struct_new.merkle_tree.changelog.capacity()
    );

    {
        let mut new_queue_account = rpc.get_account(*new_queue_pubkey).await.unwrap().unwrap();
        let mut new_mt_lamports = 0u64;
        let account_info = AccountInfo::new(
            new_queue_pubkey,
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
        let mut old_queue_account = rpc.get_account(*old_queue_pubkey).await.unwrap().unwrap();

        let mut old_mt_lamports = 0u64;
        let account_info = AccountInfo::new(
            old_queue_pubkey,
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
        assert_eq!(
            old_queue_account.to_account_info().data.borrow().len(),
            new_queue_account.to_account_info().data.borrow().len(),
        );
        assert_rolledover_queues_metadata(
            &old_loaded_queue_account.metadata,
            &new_loaded_queue_account.metadata,
            current_slot,
            new_merkle_tree_pubkey,
            new_queue_pubkey,
            old_mt_account.get_lamports(),
            new_mt_account.get_lamports(),
            new_queue_account.get_lamports(),
        );
    }
    let fee_payer_post_balance = rpc.get_account(*payer).await.unwrap().unwrap().lamports;
    // rent is reimbursed, 3 signatures cost 3 x 5000 lamports
    assert_eq!(*fee_payer_prior_balance, fee_payer_post_balance + 15000);
    {
        let old_address_queue =
            unsafe { get_hash_set::<QueueAccount, R>(rpc, *old_queue_pubkey).await };
        let new_address_queue =
            unsafe { get_hash_set::<QueueAccount, R>(rpc, *new_queue_pubkey).await };

        assert_eq!(
            old_address_queue.get_capacity(),
            new_address_queue.get_capacity()
        );

        assert_eq!(
            old_address_queue.sequence_threshold,
            new_address_queue.sequence_threshold,
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn perform_address_merkle_tree_roll_over_forester<R: Rpc>(
    payer: &Keypair,
    context: &mut R,
    new_queue_keypair: &Keypair,
    new_address_merkle_tree_keypair: &Keypair,
    old_merkle_tree_pubkey: &Pubkey,
    old_queue_pubkey: &Pubkey,
    epoch: u64,
    is_metadata_forester: bool,
) -> Result<solana_sdk::signature::Signature, RpcError> {
    let instructions = create_rollover_address_merkle_tree_instructions(
        context,
        &payer.pubkey(),
        &payer.pubkey(),
        new_queue_keypair,
        new_address_merkle_tree_keypair,
        old_merkle_tree_pubkey,
        old_queue_pubkey,
        epoch,
        is_metadata_forester,
    )
    .await;
    let blockhash = context.get_latest_blockhash().await?;
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &vec![&payer, &new_queue_keypair, &new_address_merkle_tree_keypair],
        blockhash.0,
    );
    context.process_transaction(transaction).await
}

#[allow(clippy::too_many_arguments)]
pub async fn perform_state_merkle_tree_roll_over_forester<R: Rpc>(
    payer: &Keypair,
    context: &mut R,
    new_queue_keypair: &Keypair,
    new_address_merkle_tree_keypair: &Keypair,
    new_cpi_signature_keypair: &Keypair,
    old_merkle_tree_pubkey: &Pubkey,
    old_queue_pubkey: &Pubkey,
    epoch: u64,
    is_metadata_forester: bool,
) -> Result<(solana_sdk::signature::Signature, Slot), RpcError> {
    let instructions = create_rollover_state_merkle_tree_instructions(
        context,
        &payer.pubkey(),
        &payer.pubkey(),
        new_queue_keypair,
        new_address_merkle_tree_keypair,
        new_cpi_signature_keypair,
        old_merkle_tree_pubkey,
        old_queue_pubkey,
        epoch,
        is_metadata_forester,
    )
    .await;
    let blockhash = context.get_latest_blockhash().await?;
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &vec![
            &payer,
            &new_queue_keypair,
            &new_address_merkle_tree_keypair,
            &new_cpi_signature_keypair,
        ],
        blockhash.0,
    );
    context.process_transaction_with_context(transaction).await
}
