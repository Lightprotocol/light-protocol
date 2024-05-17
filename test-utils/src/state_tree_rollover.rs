#![allow(clippy::await_holding_refcell_ref)]

use crate::{create_account_instruction, get_hash_set};
use account_compression::{
    self,
    initialize_address_merkle_tree::AccountLoader,
    initialize_nullifier_queue::NullifierQueueAccount,
    utils::constants::{
        STATE_MERKLE_TREE_HEIGHT, STATE_NULLIFIER_QUEUE_INDICES, STATE_NULLIFIER_QUEUE_VALUES,
    },
    StateMerkleTreeAccount, ID,
};
use anchor_lang::{InstructionData, Lamports, ToAccountMetas};
use light_concurrent_merkle_tree::{ConcurrentMerkleTree, ConcurrentMerkleTree26};
use light_hasher::Poseidon;
use memoffset::offset_of;
use solana_program_test::{
    BanksClientError, BanksTransactionResultWithMetadata, ProgramTestContext,
};
use solana_sdk::{
    account::AccountSharedData,
    account_info::AccountInfo,
    instruction::{AccountMeta, Instruction},
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solana_sdk::{account::WritableAccount, pubkey::Pubkey};

pub async fn perform_state_merkle_tree_roll_over(
    context: &mut ProgramTestContext,
    new_nullifier_queue_keypair: &Keypair,
    new_state_merkle_tree_keypair: &Keypair,
    merkle_tree_pubkey: &Pubkey,
    nullifier_queue_pubkey: &Pubkey,
) -> Result<BanksTransactionResultWithMetadata, BanksClientError> {
    let payer_pubkey = context.payer.pubkey();
    let size = NullifierQueueAccount::size(
        STATE_NULLIFIER_QUEUE_INDICES as usize,
        STATE_NULLIFIER_QUEUE_VALUES as usize,
    )
    .unwrap();
    let create_nullifier_queue_instruction = create_account_instruction(
        &payer_pubkey,
        size,
        context
            .banks_client
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(size),
        &ID,
        Some(new_nullifier_queue_keypair),
    );
    let create_state_merkle_tree_instruction = create_account_instruction(
        &payer_pubkey,
        account_compression::StateMerkleTreeAccount::LEN,
        context
            .banks_client
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(account_compression::StateMerkleTreeAccount::LEN),
        &ID,
        Some(new_state_merkle_tree_keypair),
    );
    let instruction_data =
        account_compression::instruction::RolloverStateMerkleTreeAndNullifierQueue {};
    let accounts = account_compression::accounts::RolloverStateMerkleTreeAndNullifierQueue {
        fee_payer: context.payer.pubkey(),
        authority: context.payer.pubkey(),
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
    let blockhash = context.get_new_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[
            create_nullifier_queue_instruction,
            create_state_merkle_tree_instruction,
            instruction,
        ],
        Some(&context.payer.pubkey()),
        &vec![
            &context.payer,
            &new_nullifier_queue_keypair,
            &new_state_merkle_tree_keypair,
        ],
        blockhash,
    );
    context
        .banks_client
        .process_transaction_with_metadata(transaction)
        .await
}

pub async fn set_state_merkle_tree_next_index(
    context: &mut ProgramTestContext,
    merkle_tree_pubkey: &Pubkey,
    next_index: u64,
    lamports: u64,
) {
    // is in range 8 -9 in concurrent mt
    // offset for next index

    let offset_start = 8
        + offset_of!(StateMerkleTreeAccount, state_merkle_tree_struct)
        + offset_of!(ConcurrentMerkleTree26<Poseidon>, next_index);
    let offset_end = offset_start + 8;
    let mut merkle_tree = context
        .banks_client
        .get_account(*merkle_tree_pubkey)
        .await
        .unwrap()
        .unwrap();
    merkle_tree.data[offset_start..offset_end].copy_from_slice(&next_index.to_le_bytes());
    let mut account_share_data = AccountSharedData::from(merkle_tree);
    account_share_data.set_lamports(lamports);
    context.set_account(merkle_tree_pubkey, &account_share_data);
    let merkle_tree = context
        .banks_client
        .get_account(*merkle_tree_pubkey)
        .await
        .unwrap()
        .unwrap();
    let data_in_offset = u64::from_le_bytes(
        merkle_tree.data[offset_start..offset_end]
            .try_into()
            .unwrap(),
    );
    assert_eq!(data_in_offset, next_index);
}

pub async fn assert_rolled_over_pair(
    context: &mut ProgramTestContext,
    fee_payer_prior_balance: &u64,
    old_merkle_tree_pubkey: &Pubkey,
    old_nullifier_queue_pubkey: &Pubkey,
    new_merkle_tree_pubkey: &Pubkey,
    new_nullifier_queue_pubkey: &Pubkey,
) {
    let mut new_mt_account = context
        .banks_client
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
    let new_mt_account = AccountLoader::<StateMerkleTreeAccount>::try_from(&account_info).unwrap();
    let new_loaded_mt_account = new_mt_account.load().unwrap();

    let mut old_mt_account = context
        .banks_client
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
    let old_mt_account = AccountLoader::<StateMerkleTreeAccount>::try_from(&account_info).unwrap();
    let old_loaded_mt_account = old_mt_account.load().unwrap();
    let current_slot = context.banks_client.get_root_slot().await.unwrap();
    // Old Merkle tree
    // 1. rolled over slot is set to current slot
    // 2. next Merkle tree is set to the new Merkle tree
    assert_eq!(old_loaded_mt_account.rolledover_slot, current_slot);
    assert_eq!(
        old_loaded_mt_account.next_merkle_tree,
        *new_merkle_tree_pubkey
    );
    // New Merkle tree
    // 1. index is equal to the old Merkle tree index
    // 2. rollover fee is equal to the old Merkle tree rollover fee (the fee is calculated onchain in case rent should change the fee might be different)
    // 3. tip is equal to the old Merkle tree tip
    // 4. rollover threshold is equal to the old Merkle tree rollover threshold
    // 5. rolled over slot is set to u64::MAX (not rolled over)
    // 6. close threshold is equal to the old Merkle tree close threshold
    // 7. associated queue is equal to the new queue
    // 7. next merkle tree is set to Pubkey::default() (not set)
    // 8. owner is equal to the old Merkle tree owner
    // 9. delegate is equal to the old Merkle tree delegate

    assert_eq!(old_loaded_mt_account.index, new_loaded_mt_account.index);
    assert_eq!(
        old_loaded_mt_account.rollover_fee,
        new_loaded_mt_account.rollover_fee
    );
    assert_eq!(old_loaded_mt_account.tip, new_loaded_mt_account.tip);
    assert_eq!(
        old_loaded_mt_account.rollover_threshold,
        new_loaded_mt_account.rollover_threshold
    );
    assert_eq!(u64::MAX, new_loaded_mt_account.rolledover_slot);

    assert_eq!(
        old_loaded_mt_account.close_threshold,
        new_loaded_mt_account.close_threshold
    );
    assert_eq!(
        new_loaded_mt_account.associated_queue,
        *new_nullifier_queue_pubkey
    );
    assert_eq!(new_loaded_mt_account.next_merkle_tree, Pubkey::default());

    assert_eq!(old_loaded_mt_account.owner, new_loaded_mt_account.owner);
    assert_eq!(
        old_loaded_mt_account.delegate,
        new_loaded_mt_account.delegate
    );

    let struct_old: *mut ConcurrentMerkleTree<Poseidon, { STATE_MERKLE_TREE_HEIGHT as usize }> =
        old_loaded_mt_account.state_merkle_tree_struct.as_ptr() as _;
    let struct_new: *mut ConcurrentMerkleTree<Poseidon, { STATE_MERKLE_TREE_HEIGHT as usize }> =
        new_loaded_mt_account.state_merkle_tree_struct.as_ptr() as _;
    assert_eq!(unsafe { (*struct_old).height }, unsafe {
        (*struct_new).height
    });

    assert_eq!(unsafe { (*struct_old).changelog_capacity }, unsafe {
        (*struct_new).changelog_capacity
    });

    assert_eq!(unsafe { (*struct_old).changelog_length }, unsafe {
        (*struct_new).changelog_length
    });

    assert_eq!(unsafe { (*struct_old).current_changelog_index }, unsafe {
        (*struct_new).current_changelog_index
    });

    assert_eq!(unsafe { (*struct_old).roots_capacity }, unsafe {
        (*struct_new).roots_capacity
    });

    assert_eq!(unsafe { (*struct_old).roots_length }, unsafe {
        (*struct_new).roots_length
    });

    assert_eq!(unsafe { (*struct_old).canopy_depth }, unsafe {
        (*struct_new).canopy_depth
    });

    {
        let mut new_queue_account = context
            .banks_client
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
        let new_queue_account =
            AccountLoader::<NullifierQueueAccount>::try_from(&account_info).unwrap();
        let new_loaded_queue_account = new_queue_account.load().unwrap();
        let mut old_queue_account = context
            .banks_client
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
        let old_queue_account =
            AccountLoader::<NullifierQueueAccount>::try_from(&account_info).unwrap();
        let old_loaded_queue_account = old_queue_account.load().unwrap();

        assert_eq!(old_loaded_queue_account.rolledover_slot, current_slot);
        assert_eq!(
            old_loaded_queue_account.index,
            new_loaded_queue_account.index
        );
        assert_eq!(
            old_loaded_queue_account.rollover_fee,
            new_loaded_queue_account.rollover_fee
        );
        assert_eq!(old_loaded_queue_account.tip, new_loaded_queue_account.tip);
        assert_eq!(u64::MAX, new_loaded_queue_account.rolledover_slot);

        assert_eq!(
            old_loaded_queue_account.owner,
            new_loaded_queue_account.owner
        );

        assert_eq!(
            old_loaded_queue_account.delegate,
            new_loaded_queue_account.delegate
        );
        assert_eq!(
            new_loaded_queue_account.associated_merkle_tree,
            *new_merkle_tree_pubkey
        );
        assert_eq!(
            old_loaded_queue_account.next_queue,
            *new_nullifier_queue_pubkey
        );
        assert_eq!(
            old_mt_account.get_lamports(),
            new_mt_account.get_lamports()
                + new_queue_account.get_lamports()
                + old_mt_account.get_lamports()
        );
    }
    let fee_payer_post_balance = context
        .banks_client
        .get_account(context.payer.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;
    // rent is reimbursed, 3 signatures cost 3 x 5000 lamports
    assert_eq!(*fee_payer_prior_balance, fee_payer_post_balance + 15000);
    let old_address_queue = unsafe {
        get_hash_set::<u16, NullifierQueueAccount>(context, *old_nullifier_queue_pubkey).await
    };
    let new_address_queue = unsafe {
        get_hash_set::<u16, NullifierQueueAccount>(context, *new_nullifier_queue_pubkey).await
    };

    assert_eq!(
        old_address_queue.capacity_indices,
        new_address_queue.capacity_indices,
    );

    assert_eq!(
        old_address_queue.capacity_values,
        new_address_queue.capacity_values,
    );

    assert_eq!(
        old_address_queue.sequence_threshold,
        new_address_queue.sequence_threshold,
    );
}
