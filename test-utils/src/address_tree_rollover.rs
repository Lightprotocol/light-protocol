#![allow(clippy::await_holding_refcell_ref)]

use crate::rpc::errors::RpcError;
use crate::rpc::rpc_connection::RpcConnection;
use crate::{
    assert_rollover::{
        assert_rolledover_merkle_trees, assert_rolledover_merkle_trees_metadata,
        assert_rolledover_queues_metadata,
    },
    get_hash_set,
};
use account_compression::{
    accounts, initialize_address_merkle_tree::AccountLoader, instruction, state::QueueAccount,
    utils::constants::ADDRESS_MERKLE_TREE_HEIGHT, AddressMerkleTreeAccount,
};
use anchor_lang::{InstructionData, Lamports, ToAccountMetas};
use light_concurrent_merkle_tree::{ConcurrentMerkleTree, ConcurrentMerkleTree26};
use light_hasher::Poseidon;
use memoffset::offset_of;
use solana_sdk::{
    account::{AccountSharedData, WritableAccount},
    account_info::AccountInfo,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};

pub async fn set_address_merkle_tree_next_index<R: RpcConnection>(
    rpc: &mut R,
    merkle_tree_pubkey: &Pubkey,
    next_index: u64,
    lamports: u64,
) {
    // is in range 8 -9 in concurrent mt
    // offset for next index
    let offset_start = 8
        + offset_of!(AddressMerkleTreeAccount, merkle_tree_struct)
        + offset_of!(ConcurrentMerkleTree26<Poseidon>, next_index); // 6 * 8 + 8 + 4 * 32 + 8 * 8;
    let offset_end = offset_start + 8;
    let mut merkle_tree = rpc.get_account(*merkle_tree_pubkey).await.unwrap().unwrap();
    merkle_tree.data[offset_start..offset_end].copy_from_slice(&next_index.to_le_bytes());
    let mut account_share_data = AccountSharedData::from(merkle_tree);
    account_share_data.set_lamports(lamports);
    rpc.set_account(merkle_tree_pubkey, &account_share_data);
    let merkle_tree = rpc.get_account(*merkle_tree_pubkey).await.unwrap().unwrap();
    let data_in_offset = u64::from_le_bytes(
        merkle_tree.data[offset_start..offset_end]
            .try_into()
            .unwrap(),
    );
    assert_eq!(data_in_offset, next_index);
}

pub async fn perform_address_merkle_tree_roll_over<R: RpcConnection>(
    context: &mut R,
    new_queue_keypair: &Keypair,
    new_address_merkle_tree_keypair: &Keypair,
    old_merkle_tree_pubkey: &Pubkey,
    old_queue_pubkey: &Pubkey,
) -> Result<(), RpcError> {
    let payer = context.get_payer().insecure_clone();
    let size =
        QueueAccount::size(account_compression::utils::constants::ADDRESS_QUEUE_VALUES as usize)
            .unwrap();
    let account_create_ix = crate::create_account_instruction(
        &payer.pubkey(),
        size,
        context
            .get_minimum_balance_for_rent_exemption(size)
            .await
            .unwrap(),
        &account_compression::ID,
        Some(new_queue_keypair),
    );

    let mt_account_create_ix = crate::create_account_instruction(
        &payer.pubkey(),
        account_compression::AddressMerkleTreeAccount::LEN,
        context
            .get_minimum_balance_for_rent_exemption(
                account_compression::AddressMerkleTreeAccount::LEN,
            )
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
    let blockhash = context.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[account_create_ix, mt_account_create_ix, instruction],
        Some(&context.get_payer().pubkey()),
        &vec![
            &context.get_payer(),
            &new_queue_keypair,
            &new_address_merkle_tree_keypair,
        ],
        blockhash,
    );
    context.process_transaction(transaction).await
}

pub async fn assert_rolled_over_address_merkle_tree_and_queue<R: RpcConnection>(
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

    assert_rolledover_merkle_trees_metadata(
        &old_loaded_mt_account.metadata,
        &new_loaded_mt_account.metadata,
        current_slot,
        new_queue_pubkey,
    );

    let struct_old = unsafe {
        &*(old_loaded_mt_account.merkle_tree_struct.as_ptr()
            as *mut ConcurrentMerkleTree<Poseidon, { ADDRESS_MERKLE_TREE_HEIGHT as usize }>)
    };
    let struct_new = unsafe {
        &*(new_loaded_mt_account.merkle_tree_struct.as_ptr()
            as *mut ConcurrentMerkleTree<Poseidon, { ADDRESS_MERKLE_TREE_HEIGHT as usize }>)
    };
    assert_rolledover_merkle_trees(struct_old, struct_new);

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
    let fee_payer_post_balance = rpc
        .get_account(rpc.get_payer().pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;
    // rent is reimbursed, 3 signatures cost 3 x 5000 lamports
    assert_eq!(*fee_payer_prior_balance, fee_payer_post_balance + 15000);
    {
        let old_address_queue =
            unsafe { get_hash_set::<QueueAccount, R>(rpc, *old_queue_pubkey).await };
        let new_address_queue =
            unsafe { get_hash_set::<QueueAccount, R>(rpc, *new_queue_pubkey).await };

        assert_eq!(old_address_queue.capacity, new_address_queue.capacity);

        assert_eq!(
            old_address_queue.sequence_threshold,
            new_address_queue.sequence_threshold,
        );
    }
}
