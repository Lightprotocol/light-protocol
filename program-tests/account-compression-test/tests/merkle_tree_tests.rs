#![cfg(feature = "test-sbf")]
use std::{collections::HashMap, mem};

use account_compression::{
    self,
    errors::AccountCompressionErrorCode,
    queue_from_bytes_copy,
    state::{queue_from_bytes_zero_copy_mut, QueueAccount},
    utils::constants::{STATE_MERKLE_TREE_CANOPY_DEPTH, STATE_MERKLE_TREE_HEIGHT},
    AddressMerkleTreeConfig, AddressQueueConfig, NullifierQueueConfig, StateMerkleTreeAccount,
    StateMerkleTreeConfig, ID, SAFETY_MARGIN,
};
use anchor_lang::{InstructionData, ToAccountMetas};
use light_account_checks::error::AccountError;
use light_compressed_account::instruction_data::insert_into_queues::InsertIntoQueuesInstructionDataMut;
use light_concurrent_merkle_tree::{
    errors::ConcurrentMerkleTreeError, event::MerkleTreeEvent,
    zero_copy::ConcurrentMerkleTreeZeroCopyMut,
};
use light_hash_set::HashSetError;
use light_hasher::{
    bigint::bigint_to_be_bytes_array, zero_bytes::poseidon::ZERO_BYTES, Hasher, Poseidon,
};
use light_merkle_tree_metadata::{errors::MerkleTreeMetadataError, QueueType};
use light_merkle_tree_reference::MerkleTree;
use light_program_test::{
    accounts::state_tree::{
        create_initialize_merkle_tree_instruction, create_insert_leaves_instruction,
    },
    program_test::{LightProgramTest, TestRpc},
    utils::assert::assert_rpc_error,
    ProgramTestConfig,
};
use light_test_utils::{
    airdrop_lamports,
    assert_merkle_tree::assert_merkle_tree_initialized,
    assert_queue::assert_nullifier_queue_initialized,
    create_account_instruction, create_address_merkle_tree_and_queue_account_with_assert,
    get_concurrent_merkle_tree, get_hash_set,
    pack::pack_pubkey,
    state_tree_rollover::{
        assert_rolled_over_pair, perform_state_merkle_tree_roll_over,
        set_state_merkle_tree_next_index, StateMerkleTreeRolloverMode,
    },
    Rpc, RpcError,
};
use num_bigint::{BigUint, ToBigUint};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};

/// Tests:
/// 1. Functional: Initialize nullifier queue
/// 2. Functional: Insert into nullifier queue
/// 3. Failing: Insert the same elements into nullifier queue again (3 and 1 element(s))
/// 4. Failing: Insert into nullifier queue with invalid authority
/// 5. Functional: Insert one element into nullifier queue
async fn test_init_and_insert_into_nullifier_queue(
    merkle_tree_config: &StateMerkleTreeConfig,
    queue_config: &NullifierQueueConfig,
) {
    let config = ProgramTestConfig {
        skip_protocol_init: true,
        with_prover: false,
        ..Default::default()
    };
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let merkle_tree_keypair = Keypair::new();
    let merkle_tree_pubkey = merkle_tree_keypair.pubkey();
    let nullifier_queue_keypair = Keypair::new();
    let nullifier_queue_pubkey = nullifier_queue_keypair.pubkey();
    let payer_pubkey = rpc.get_payer().pubkey();
    fail_initialize_state_merkle_tree_and_nullifier_queue_invalid_sizes(
        &mut rpc,
        &payer_pubkey,
        &merkle_tree_keypair,
        &nullifier_queue_keypair,
        merkle_tree_config,
        queue_config,
    )
    .await;
    fail_initialize_state_merkle_tree_and_nullifier_queue_invalid_config(
        &mut rpc,
        &payer_pubkey,
        &merkle_tree_keypair,
        &nullifier_queue_keypair,
        merkle_tree_config,
        queue_config,
    )
    .await;
    functional_1_initialize_state_merkle_tree_and_nullifier_queue(
        &mut rpc,
        &payer_pubkey,
        &merkle_tree_keypair,
        &nullifier_queue_keypair,
        merkle_tree_config,
        queue_config,
    )
    .await;
    let merkle_tree_keypair_2 = Keypair::new();
    let nullifier_queue_keypair_2 = Keypair::new();
    functional_1_initialize_state_merkle_tree_and_nullifier_queue(
        &mut rpc,
        &payer_pubkey,
        &merkle_tree_keypair_2,
        &nullifier_queue_keypair_2,
        merkle_tree_config,
        queue_config,
    )
    .await;
    functional_2_test_insert_into_nullifier_queues(
        &mut rpc,
        &nullifier_queue_pubkey,
        &merkle_tree_pubkey,
    )
    .await;

    fail_3_insert_same_elements_into_nullifier_queue(
        &mut rpc,
        &nullifier_queue_pubkey,
        &merkle_tree_pubkey,
        vec![[3u8; 32], [1u8; 32], [1u8; 32]],
    )
    .await;
    fail_3_insert_same_elements_into_nullifier_queue(
        &mut rpc,
        &nullifier_queue_pubkey,
        &merkle_tree_pubkey,
        vec![[1u8; 32]],
    )
    .await;
    fail_4_insert_with_invalid_signer(
        &mut rpc,
        &nullifier_queue_pubkey,
        &merkle_tree_pubkey,
        vec![[3u8; 32]],
    )
    .await;

    functional_5_test_insert_into_nullifier_queue(
        &mut rpc,
        &nullifier_queue_pubkey,
        &merkle_tree_pubkey,
    )
    .await;
    let queue_tree_pair = (nullifier_queue_pubkey, merkle_tree_pubkey);
    let queue_tree_pair_2 = (
        nullifier_queue_keypair_2.pubkey(),
        merkle_tree_keypair_2.pubkey(),
    );
    let nullifier_1 = [10u8; 32];
    let nullifier_2 = [20u8; 32];
    // CHECK: nullifiers inserted into correct queue with 2 queues
    functional_6_test_insert_into_two_nullifier_queues(
        &mut rpc,
        &[nullifier_1, nullifier_2],
        &[queue_tree_pair, queue_tree_pair_2],
    )
    .await;

    let nullifier_1 = [11u8; 32];
    let nullifier_2 = [21u8; 32];
    let nullifier_3 = [31u8; 32];
    let nullifier_4 = [41u8; 32];
    // CHECK: nullifiers inserted into correct queue with 2 queues and not ordered
    functional_7_test_insert_into_two_nullifier_queues_not_ordered(
        &mut rpc,
        &[nullifier_1, nullifier_2, nullifier_3, nullifier_4],
        &[
            queue_tree_pair,
            queue_tree_pair_2,
            queue_tree_pair,
            queue_tree_pair_2,
        ],
    )
    .await;
}

#[tokio::test]
async fn test_init_and_insert_into_nullifier_queue_default() {
    test_init_and_insert_into_nullifier_queue(
        &StateMerkleTreeConfig::default(),
        &NullifierQueueConfig::default(),
    )
    .await
}

#[tokio::test]
async fn test_init_and_insert_into_nullifier_queue_custom() {
    for changelog_size in [1, 1000, 2000] {
        for roots_size in [1, 1000, 2000] {
            if roots_size < changelog_size {
                continue;
            }
            for queue_capacity in [5003, 6857, 7901] {
                test_init_and_insert_into_nullifier_queue(
                    &StateMerkleTreeConfig {
                        height: STATE_MERKLE_TREE_HEIGHT as u32,
                        changelog_size,
                        roots_size,
                        canopy_depth: STATE_MERKLE_TREE_CANOPY_DEPTH,
                        network_fee: Some(5000),
                        rollover_threshold: Some(95),
                        close_threshold: None,
                    },
                    &NullifierQueueConfig {
                        capacity: queue_capacity,
                        sequence_threshold: roots_size + SAFETY_MARGIN,
                        network_fee: None,
                    },
                )
                .await;
            }
        }
    }
}

/// Tests:
/// (Since nullifier queue and address queue use the same code, we only need to test one)
/// Show that we cannot insert into a full queue.
/// 1. try to insert into queue to generate the full error
/// 2. nullify one
/// 3. try to insert again it should still generate the full error
/// 4. advance Merkle tree seq until one before it would work check that it still fails
/// 5. advance Merkle tree seq by one and check that inserting works now
///    6.try inserting again it should fail with full error
async fn test_full_nullifier_queue(
    merkle_tree_config: &StateMerkleTreeConfig,
    queue_config: &NullifierQueueConfig,
) {
    let config = ProgramTestConfig {
        skip_protocol_init: true,
        with_prover: false,
        ..Default::default()
    };
    let mut rpc = LightProgramTest::new(config).await.unwrap();

    let merkle_tree_keypair = Keypair::new();
    let merkle_tree_pubkey = merkle_tree_keypair.pubkey();
    let nullifier_queue_keypair = Keypair::new();
    let nullifier_queue_pubkey = nullifier_queue_keypair.pubkey();

    let payer_pubkey = rpc.get_payer().pubkey();
    functional_1_initialize_state_merkle_tree_and_nullifier_queue(
        &mut rpc,
        &payer_pubkey,
        &merkle_tree_keypair,
        &nullifier_queue_keypair,
        merkle_tree_config,
        queue_config,
    )
    .await;
    let leaf: [u8; 32] = bigint_to_be_bytes_array(&1.to_biguint().unwrap()).unwrap();
    // append a leaf so that we have a leaf to nullify
    let mut reference_merkle_tree_1 = MerkleTree::<Poseidon>::new(
        STATE_MERKLE_TREE_HEIGHT as usize,
        STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
    );
    functional_3_append_leaves_to_merkle_tree(
        &mut rpc,
        &mut [&mut reference_merkle_tree_1],
        &vec![merkle_tree_pubkey],
        &vec![(0u8, leaf)],
    )
    .await;
    let lamports_queue_accounts = rpc
        .get_account(nullifier_queue_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports
        + rpc
            .get_account(merkle_tree_pubkey)
            .await
            .unwrap()
            .unwrap()
            .lamports
            * 2;
    // fills queue with increasing values starting from 0
    // -> in this process inserts leaf with value 1 into queue
    // all elements are marked with sequence number 2400
    set_nullifier_queue_to_full(
        &mut rpc,
        &nullifier_queue_pubkey,
        0,
        lamports_queue_accounts,
    )
    .await;

    let initial_value = 309005;
    let element: [u8; 32] = bigint_to_be_bytes_array(&initial_value.to_biguint().unwrap()).unwrap();
    // CHECK 1
    fail_insert_into_full_queue(
        &mut rpc,
        &nullifier_queue_pubkey,
        &merkle_tree_pubkey,
        vec![element],
    )
    .await;
    let mut reference_merkle_tree = MerkleTree::<Poseidon>::new(26, 10);
    reference_merkle_tree.append(&leaf).unwrap();

    let merkle_tree =
        get_concurrent_merkle_tree::<StateMerkleTreeAccount, LightProgramTest, Poseidon, 26>(
            &mut rpc,
            merkle_tree_pubkey,
        )
        .await;
    assert_eq!(merkle_tree.root(), reference_merkle_tree.root());
    let leaf_index = reference_merkle_tree.get_leaf_index(&leaf).unwrap() as u64;
    let element_index = unsafe {
        get_hash_set::<QueueAccount, LightProgramTest>(&mut rpc, nullifier_queue_pubkey)
            .await
            .find_element_index(&BigUint::from_bytes_be(&leaf), None)
            .unwrap()
    };
    // CHECK 2
    nullify(
        &mut rpc,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        queue_config,
        &mut reference_merkle_tree,
        &leaf,
        merkle_tree.changelog_index() as u64,
        element_index.unwrap() as u16,
        leaf_index,
    )
    .await
    .unwrap();
    // CHECK 3
    fail_insert_into_full_queue(
        &mut rpc,
        &nullifier_queue_pubkey,
        &merkle_tree_pubkey,
        vec![element],
    )
    .await;
    // Advance to sequence threshold + 1 (expected sequence number of the last
    // element - 1).
    set_state_merkle_tree_sequence(
        &mut rpc,
        &merkle_tree_pubkey,
        queue_config.sequence_threshold + 1,
        lamports_queue_accounts,
    )
    .await;
    // CHECK 4
    fail_insert_into_full_queue(
        &mut rpc,
        &nullifier_queue_pubkey,
        &merkle_tree_pubkey,
        vec![element],
    )
    .await;
    // TODO: add e2e test in compressed pda program for this
    // Advance to sequence threshold + 2 (expected sequence number of the last
    // element).
    set_state_merkle_tree_sequence(
        &mut rpc,
        &merkle_tree_pubkey,
        queue_config.sequence_threshold + 2,
        lamports_queue_accounts,
    )
    .await;
    let payer = rpc.get_payer().insecure_clone();
    let account = rpc
        .get_account(nullifier_queue_pubkey)
        .await
        .unwrap()
        .unwrap();

    let mut data = account.data.clone();
    let nullifier_queue = &mut unsafe { queue_from_bytes_zero_copy_mut(&mut data).unwrap() };
    let replacement_start_value = 606;
    let replacement_value = find_overlapping_probe_index(
        1,
        replacement_start_value,
        nullifier_queue.hash_set.get_capacity(),
    );
    // CHECK: 5
    let element: [u8; 32] =
        bigint_to_be_bytes_array(&replacement_value.to_biguint().unwrap()).unwrap();
    insert_into_single_nullifier_queue(
        &[element],
        &payer,
        &payer,
        &nullifier_queue_pubkey,
        &merkle_tree_pubkey,
        &mut rpc,
    )
    .await
    .unwrap();
    // CHECK: 6
    let element: [u8; 32] = bigint_to_be_bytes_array(&30000.to_biguint().unwrap()).unwrap();
    fail_insert_into_full_queue(
        &mut rpc,
        &nullifier_queue_pubkey,
        &merkle_tree_pubkey,
        vec![element],
    )
    .await;
}

#[tokio::test]
async fn test_full_nullifier_queue_default() {
    test_full_nullifier_queue(
        &StateMerkleTreeConfig::default(),
        &NullifierQueueConfig::default(),
    )
    .await
}

/// Insert nullifiers failing tests
/// Test:
/// 1. no nullifiers
/// 2. mismatch remaining accounts and addresses (removed error)
/// 3. invalid queue accounts:
///    3.1 pass non queue account as queue account
///    3.2 pass address queue account
///    3.3 pass non associated queue account
/// 4. invalid Merkle tree accounts:
///    4.1 pass non Merkle tree account as Merkle tree account
///    4.2 pass non associated Merkle tree account
async fn failing_queue(
    merkle_tree_config: &StateMerkleTreeConfig,
    queue_config: &NullifierQueueConfig,
) {
    let config = ProgramTestConfig {
        skip_protocol_init: true,
        with_prover: false,
        ..Default::default()
    };
    let mut rpc = LightProgramTest::new(config).await.unwrap();

    let merkle_tree_keypair = Keypair::new();
    let merkle_tree_pubkey = merkle_tree_keypair.pubkey();
    let nullifier_queue_keypair = Keypair::new();
    let nullifier_queue_pubkey = nullifier_queue_keypair.pubkey();

    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = rpc.get_payer().pubkey();
    functional_1_initialize_state_merkle_tree_and_nullifier_queue(
        &mut rpc,
        &payer_pubkey,
        &merkle_tree_keypair,
        &nullifier_queue_keypair,
        merkle_tree_config,
        queue_config,
    )
    .await;
    let merkle_tree_keypair_2 = Keypair::new();
    let nullifier_queue_keypair_2 = Keypair::new();
    functional_1_initialize_state_merkle_tree_and_nullifier_queue(
        &mut rpc,
        &payer_pubkey,
        &merkle_tree_keypair_2,
        &nullifier_queue_keypair_2,
        merkle_tree_config,
        queue_config,
    )
    .await;

    let address_merkle_tree_keypair = Keypair::new();
    let address_queue_keypair = Keypair::new();
    create_address_merkle_tree_and_queue_account_with_assert(
        &payer,
        false,
        &mut rpc,
        &address_merkle_tree_keypair,
        &address_queue_keypair,
        None,
        None,
        &AddressMerkleTreeConfig::default(),
        &AddressQueueConfig::default(),
        1,
    )
    .await
    .unwrap();

    let queue_tree_pair = (nullifier_queue_pubkey, merkle_tree_pubkey);
    // CHECK 1: no nullifiers as input
    let result =
        insert_into_nullifier_queues(&[], &payer, &payer, &[queue_tree_pair], &mut rpc).await;
    assert_rpc_error(
        result,
        0,
        AccountCompressionErrorCode::InputElementsEmpty.into(),
    )
    .unwrap();
    let nullifier_1 = [1u8; 32];

    // CHECK 3.1: pass non queue account as queue account
    let result = insert_into_nullifier_queues(
        &[nullifier_1],
        &payer,
        &payer,
        &[(merkle_tree_pubkey, nullifier_queue_pubkey)],
        &mut rpc,
    )
    .await;
    assert_rpc_error(
        result,
        0,
        AccountCompressionErrorCode::InvalidAccount.into(),
    )
    .unwrap();

    // CHECK 3.2: pass address queue account instead of nullifier queue account
    let result = insert_into_nullifier_queues(
        &[nullifier_1],
        &payer,
        &payer,
        &[(address_queue_keypair.pubkey(), merkle_tree_pubkey)],
        &mut rpc,
    )
    .await;
    assert_rpc_error(
        result,
        0,
        AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated.into(),
    )
    .unwrap();
    let nullifier_2 = [2u8; 32];

    // CHECK 3.3: pass non associated queue account
    let result = insert_into_nullifier_queues(
        &[nullifier_2],
        &payer,
        &payer,
        &[(nullifier_queue_keypair_2.pubkey(), merkle_tree_pubkey)],
        &mut rpc,
    )
    .await;
    assert_rpc_error(
        result,
        0,
        AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated.into(),
    )
    .unwrap();
    // CHECK 4.1: pass non Merkle tree account
    // // Triggering a discriminator mismatch error is not possibly
    // // by passing an invalid Merkle tree account.
    // // A non Merkle tree account cannot be associated with a queue account.
    // // Hence the instruction fails with MerkleTreeAndQueueNotAssociated.
    // // The Merkle tree account will not be deserialized.
    // let result = insert_into_nullifier_queues(
    //     &[nullifier_1],
    //     &payer,
    //     &payer,
    //     &[(
    //         nullifier_queue_keypair.pubkey(),
    //         nullifier_queue_keypair.pubkey(),
    //     )],
    //     &mut rpc,
    // )
    // .await;
    // assert_rpc_error(
    //     result,
    //     0,
    //     AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated.into(),
    // )
    // .unwrap();
    // CHECK 4.2: pass non associated Merkle tree account
    let result = insert_into_nullifier_queues(
        &[nullifier_1],
        &payer,
        &payer,
        &[(
            nullifier_queue_keypair.pubkey(),
            merkle_tree_keypair_2.pubkey(),
        )],
        &mut rpc,
    )
    .await;
    assert_rpc_error(
        result,
        0,
        AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated.into(),
    )
    .unwrap();
}

#[tokio::test]
async fn test_failing_queue_default() {
    failing_queue(
        &StateMerkleTreeConfig::default(),
        &NullifierQueueConfig::default(),
    )
    .await
}

/// Tests:
/// 1. Should fail: not ready for rollover
/// 2. Should fail: merkle tree and queue not associated (invalid tree)
/// 3. Should fail: merkle tree and queue not associated (invalid queue)
/// 4. Should succeed: rollover state merkle tree
/// 5. Should fail: merkle tree already rolled over
async fn test_init_and_rollover_state_merkle_tree(
    merkle_tree_config: &StateMerkleTreeConfig,
    queue_config: &NullifierQueueConfig,
) {
    let config = ProgramTestConfig {
        skip_protocol_init: true,
        with_prover: false,
        ..Default::default()
    };
    let mut context = LightProgramTest::new(config).await.unwrap();
    let payer_pubkey = context.get_payer().pubkey();
    let merkle_tree_keypair = Keypair::new();
    let merkle_tree_pubkey = merkle_tree_keypair.pubkey();
    let nullifier_queue_keypair = Keypair::new();
    let nullifier_queue_pubkey = nullifier_queue_keypair.pubkey();
    functional_1_initialize_state_merkle_tree_and_nullifier_queue(
        &mut context,
        &payer_pubkey,
        &merkle_tree_keypair,
        &nullifier_queue_keypair,
        merkle_tree_config,
        queue_config,
    )
    .await;

    let merkle_tree_keypair_2 = Keypair::new();
    let merkle_tree_pubkey_2 = merkle_tree_keypair_2.pubkey();
    let nullifier_queue_keypair_2 = Keypair::new();
    functional_1_initialize_state_merkle_tree_and_nullifier_queue(
        &mut context,
        &payer_pubkey,
        &merkle_tree_keypair_2,
        &nullifier_queue_keypair_2,
        merkle_tree_config,
        queue_config,
    )
    .await;

    let required_next_index = 2u64.pow(26) * merkle_tree_config.rollover_threshold.unwrap() / 100;
    let failing_next_index = required_next_index - 1;
    let lamports_queue_accounts = context
        .get_account(nullifier_queue_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports
        + context
            .get_account(merkle_tree_pubkey)
            .await
            .unwrap()
            .unwrap()
            .lamports
            * 2;
    set_state_merkle_tree_next_index(
        &mut context,
        &merkle_tree_pubkey,
        failing_next_index,
        lamports_queue_accounts,
    )
    .await;

    let new_nullifier_queue_keypair = Keypair::new();
    let new_state_merkle_tree_keypair = Keypair::new();

    let result = perform_state_merkle_tree_roll_over(
        &mut context,
        &new_nullifier_queue_keypair,
        &new_state_merkle_tree_keypair,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        merkle_tree_config,
        queue_config,
        None,
    )
    .await;

    assert_rpc_error(
        result,
        2,
        AccountCompressionErrorCode::NotReadyForRollover.into(),
    )
    .unwrap();

    let result = perform_state_merkle_tree_roll_over(
        &mut context,
        &new_nullifier_queue_keypair,
        &new_state_merkle_tree_keypair,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        merkle_tree_config,
        queue_config,
        Some(StateMerkleTreeRolloverMode::QueueInvalidSize),
    )
    .await;

    assert_rpc_error(result, 2, AccountError::InvalidAccountSize.into()).unwrap();
    let result = perform_state_merkle_tree_roll_over(
        &mut context,
        &new_nullifier_queue_keypair,
        &new_state_merkle_tree_keypair,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        merkle_tree_config,
        queue_config,
        Some(StateMerkleTreeRolloverMode::TreeInvalidSize),
    )
    .await;

    assert_rpc_error(result, 2, AccountError::InvalidAccountSize.into()).unwrap();

    set_state_merkle_tree_next_index(
        &mut context,
        &merkle_tree_pubkey,
        required_next_index,
        lamports_queue_accounts,
    )
    .await;
    let result = perform_state_merkle_tree_roll_over(
        &mut context,
        &new_nullifier_queue_keypair,
        &new_state_merkle_tree_keypair,
        &merkle_tree_pubkey,
        &nullifier_queue_keypair_2.pubkey(),
        merkle_tree_config,
        queue_config,
        None,
    )
    .await;

    assert_rpc_error(
        result,
        2,
        MerkleTreeMetadataError::MerkleTreeAndQueueNotAssociated.into(),
    )
    .unwrap();

    let result = perform_state_merkle_tree_roll_over(
        &mut context,
        &new_nullifier_queue_keypair,
        &new_state_merkle_tree_keypair,
        &merkle_tree_pubkey_2,
        &nullifier_queue_keypair.pubkey(),
        merkle_tree_config,
        queue_config,
        None,
    )
    .await;

    assert_rpc_error(
        result,
        2,
        MerkleTreeMetadataError::MerkleTreeAndQueueNotAssociated.into(),
    )
    .unwrap();

    let signer_prior_balance = context
        .get_account(payer_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    let rollover_signature_and_slot = perform_state_merkle_tree_roll_over(
        &mut context,
        &new_nullifier_queue_keypair,
        &new_state_merkle_tree_keypair,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        merkle_tree_config,
        queue_config,
        None,
    )
    .await
    .unwrap();
    let payer: Keypair = context.get_payer().insecure_clone();
    assert_rolled_over_pair(
        &payer.pubkey(),
        &mut context,
        &signer_prior_balance,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &new_state_merkle_tree_keypair.pubkey(),
        &new_nullifier_queue_keypair.pubkey(),
        rollover_signature_and_slot.1,
        0,
        3,
    )
    .await;

    let failing_new_nullifier_queue_keypair = Keypair::new();
    let failing_new_state_merkle_tree_keypair = Keypair::new();

    let result = perform_state_merkle_tree_roll_over(
        &mut context,
        &failing_new_nullifier_queue_keypair,
        &failing_new_state_merkle_tree_keypair,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        merkle_tree_config,
        queue_config,
        None,
    )
    .await;

    assert_rpc_error(
        result,
        2,
        MerkleTreeMetadataError::MerkleTreeAlreadyRolledOver.into(),
    )
    .unwrap();
}

#[tokio::test]
async fn test_init_and_rollover_state_merkle_tree_default() {
    test_init_and_rollover_state_merkle_tree(
        &StateMerkleTreeConfig::default(),
        &NullifierQueueConfig::default(),
    )
    .await
}

#[tokio::test]
async fn test_init_and_rollover_state_merkle_tree_custom() {
    for changelog_size in [1, 1000, 2000] {
        for roots_size in [1, 1000, 2000] {
            if roots_size < changelog_size {
                continue;
            }
            for queue_capacity in [5003, 6857, 7901] {
                test_init_and_rollover_state_merkle_tree(
                    &StateMerkleTreeConfig {
                        height: STATE_MERKLE_TREE_HEIGHT as u32,
                        changelog_size,
                        roots_size,
                        canopy_depth: STATE_MERKLE_TREE_CANOPY_DEPTH,
                        network_fee: Some(5000),
                        rollover_threshold: Some(95),
                        close_threshold: None,
                    },
                    &NullifierQueueConfig {
                        capacity: queue_capacity,
                        sequence_threshold: roots_size + SAFETY_MARGIN,
                        network_fee: None,
                    },
                )
                .await;
            }
        }
    }
}

/// Tests:
/// 1. Functional: Initialize merkle tree
/// 2. Failing: mismatching leaf and merkle tree accounts number
/// 3. Failing: pass invalid Merkle tree account
/// 4. Functional: Append leaves to merkle tree
/// 5. Functional: Append leaves to multiple merkle trees not-ordered
/// 6. Failing: Append leaves with invalid authority
async fn test_append_functional_and_failing(
    merkle_tree_config: &StateMerkleTreeConfig,
    queue_config: &NullifierQueueConfig,
) {
    let config = ProgramTestConfig {
        skip_protocol_init: true,
        with_prover: false,
        ..Default::default()
    };
    let mut context = LightProgramTest::new(config).await.unwrap();
    let payer_pubkey = context.get_payer().pubkey();
    let merkle_tree_keypair = Keypair::new();
    let queue_keypair = Keypair::new();
    // CHECK 1
    let merkle_tree_pubkey = functional_1_initialize_state_merkle_tree_and_nullifier_queue(
        &mut context,
        &payer_pubkey,
        &merkle_tree_keypair,
        &queue_keypair,
        merkle_tree_config,
        queue_config,
    )
    .await;
    let merkle_tree_keypair_2 = Keypair::new();
    let queue_keypair_2 = Keypair::new();
    let merkle_tree_pubkey_2 = functional_1_initialize_state_merkle_tree_and_nullifier_queue(
        &mut context,
        &payer_pubkey,
        &merkle_tree_keypair_2,
        &queue_keypair_2,
        merkle_tree_config,
        queue_config,
    )
    .await;

    // CHECK: 2 fail append with invalid inputs (mismatching leaf and merkle tree accounts)
    fail_2_append_leaves_with_invalid_inputs(
        &mut context,
        &[merkle_tree_pubkey],
        vec![(0, [1u8; 32]), (1, [2u8; 32])],
        AccountCompressionErrorCode::NotAllLeavesProcessed.into(),
    )
    .await
    .unwrap();
    // CHECK: 3 fail append with invalid inputs (pass invalid Merkle tree account)
    fail_2_append_leaves_with_invalid_inputs(
        &mut context,
        &[queue_keypair.pubkey()],
        vec![(0, [1u8; 32])],
        AccountCompressionErrorCode::StateMerkleTreeAccountDiscriminatorMismatch.into(),
    )
    .await
    .unwrap();

    // CHECK: 4 append leaves to merkle tree
    let leaves = (0u8..=50)
        .map(|i| {
            (
                0,
                [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, i,
                ],
            )
        })
        .collect::<Vec<(u8, [u8; 32])>>();
    let mut reference_merkle_tree_1 = MerkleTree::<Poseidon>::new(
        STATE_MERKLE_TREE_HEIGHT as usize,
        STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
    );
    functional_3_append_leaves_to_merkle_tree(
        &mut context,
        &mut [&mut reference_merkle_tree_1],
        &vec![merkle_tree_pubkey],
        &leaves,
    )
    .await;

    let leaves = vec![
        (0, [1u8; 32]),
        (1, [2u8; 32]),
        (2, [3u8; 32]),
        (3, [4u8; 32]),
    ];
    let mut reference_merkle_tree_2 = MerkleTree::<Poseidon>::new(
        STATE_MERKLE_TREE_HEIGHT as usize,
        STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
    );
    // CHECK: 5 append leaves to multiple merkle trees not-ordered
    functional_3_append_leaves_to_merkle_tree(
        &mut context,
        &mut [&mut reference_merkle_tree_1, &mut reference_merkle_tree_2],
        &vec![
            merkle_tree_pubkey,
            merkle_tree_pubkey_2,
            merkle_tree_pubkey,
            merkle_tree_pubkey_2,
        ],
        &leaves,
    )
    .await;

    // CHECK 6: fail append with invalid authority
    fail_4_append_leaves_with_invalid_authority(&mut context, &merkle_tree_pubkey).await;
}

#[tokio::test]
async fn test_append_functional_and_failing_default() {
    test_append_functional_and_failing(
        &StateMerkleTreeConfig::default(),
        &NullifierQueueConfig::default(),
    )
    .await
}

/// Tests:
/// 1. Functional: nullify leaf
/// 2. Failing: nullify leaf with invalid leaf index
/// 3. Failing: nullify leaf with invalid leaf queue index
/// 4. Failing: nullify leaf with invalid change log index
/// 5. Functional: nullify other leaf
/// 6. Failing: nullify leaf with nullifier queue that is not associated with the merkle tree
async fn test_nullify_leaves(
    merkle_tree_config: &StateMerkleTreeConfig,
    queue_config: &NullifierQueueConfig,
) {
    let config = ProgramTestConfig {
        skip_protocol_init: true,
        ..Default::default()
    };
    let mut context = LightProgramTest::new(config).await.unwrap();
    let merkle_tree_keypair = Keypair::new();
    let merkle_tree_pubkey = merkle_tree_keypair.pubkey();
    let nullifier_queue_keypair = Keypair::new();
    let nullifier_queue_pubkey = nullifier_queue_keypair.pubkey();
    let payer = context.get_payer().insecure_clone();
    let payer_pubkey = context.get_payer().pubkey();
    functional_1_initialize_state_merkle_tree_and_nullifier_queue(
        &mut context,
        &payer_pubkey,
        &merkle_tree_keypair,
        &nullifier_queue_keypair,
        merkle_tree_config,
        queue_config,
    )
    .await;

    let other_merkle_tree_keypair = Keypair::new();
    let invalid_nullifier_queue_keypair = Keypair::new();
    let invalid_nullifier_queue_pubkey = invalid_nullifier_queue_keypair.pubkey();
    functional_1_initialize_state_merkle_tree_and_nullifier_queue(
        &mut context,
        &payer_pubkey,
        &other_merkle_tree_keypair,
        &invalid_nullifier_queue_keypair,
        merkle_tree_config,
        queue_config,
    )
    .await;

    let elements = vec![(0, [1u8; 32]), (0, [2u8; 32])];
    let mut reference_merkle_tree = MerkleTree::<Poseidon>::new(
        merkle_tree_config.height as usize,
        merkle_tree_config.canopy_depth as usize,
    );
    functional_3_append_leaves_to_merkle_tree(
        &mut context,
        &mut [&mut reference_merkle_tree],
        &vec![merkle_tree_pubkey],
        &elements,
    )
    .await;

    insert_into_single_nullifier_queue(
        &elements
            .iter()
            .map(|element| element.1)
            .collect::<Vec<[u8; 32]>>(),
        &payer,
        &payer,
        &nullifier_queue_pubkey,
        &merkle_tree_pubkey,
        &mut context,
    )
    .await
    .unwrap();

    let mut reference_merkle_tree = MerkleTree::<Poseidon>::new(
        merkle_tree_config.height as usize,
        merkle_tree_config.canopy_depth as usize,
    );
    reference_merkle_tree.append(&elements[0].1).unwrap();
    reference_merkle_tree.append(&elements[1].1).unwrap();

    let leaf_queue_index = {
        let account = context
            .get_account(nullifier_queue_pubkey)
            .await
            .unwrap()
            .unwrap();
        let mut data = account.data.clone();
        let nullifier_queue = &mut unsafe { queue_from_bytes_copy(&mut data).unwrap() };
        let (_, index) = nullifier_queue
            .find_element(&BigUint::from_bytes_be(&elements[0].1), None)
            .unwrap()
            .unwrap();
        index
    };

    let element_index = reference_merkle_tree
        .get_leaf_index(&elements[0].1)
        .unwrap() as u64;
    let element_one_index = reference_merkle_tree
        .get_leaf_index(&elements[1].1)
        .unwrap() as u64;
    nullify(
        &mut context,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        queue_config,
        &mut reference_merkle_tree,
        &elements[0].1,
        2,
        leaf_queue_index as u16,
        element_index,
    )
    .await
    .unwrap();

    // 2. nullify with invalid leaf index
    let invalid_element_index = 0;
    let valid_changelog_index = 3;
    let valid_leaf_queue_index = {
        let account = context
            .get_account(nullifier_queue_pubkey)
            .await
            .unwrap()
            .unwrap();
        let mut data = account.data.clone();
        let nullifier_queue = &mut unsafe { queue_from_bytes_copy(&mut data).unwrap() };
        let (_, index) = nullifier_queue
            .find_element(&BigUint::from_bytes_be(&elements[1].1), None)
            .unwrap()
            .unwrap();
        index as u16
    };
    let result = nullify(
        &mut context,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        queue_config,
        &mut reference_merkle_tree,
        &elements[1].1,
        valid_changelog_index,
        valid_leaf_queue_index,
        invalid_element_index,
    )
    .await;
    assert_rpc_error(
        result,
        0,
        ConcurrentMerkleTreeError::InvalidProof([0; 32], [0; 32]).into(),
    )
    .unwrap();

    // 3. nullify with invalid leaf queue index
    let valid_element_index = 1;
    let invalid_leaf_prove_by_index = 0;
    let result = nullify(
        &mut context,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        queue_config,
        &mut reference_merkle_tree,
        &elements[1].1,
        valid_changelog_index,
        invalid_leaf_prove_by_index,
        valid_element_index,
    )
    .await;
    assert_rpc_error(result, 0, AccountCompressionErrorCode::LeafNotFound.into()).unwrap();

    // 4. nullify with invalid change log index
    let invalid_changelog_index = 0;
    let result = nullify(
        &mut context,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        queue_config,
        &mut reference_merkle_tree,
        &elements[1].1,
        invalid_changelog_index,
        valid_leaf_queue_index,
        element_one_index,
    )
    .await;
    // returns LeafNotFound why?
    assert_rpc_error(
        result,
        0,
        ConcurrentMerkleTreeError::CannotUpdateLeaf.into(),
    )
    .unwrap();
    // 5. nullify other leaf
    nullify(
        &mut context,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        queue_config,
        &mut reference_merkle_tree,
        &elements[1].1,
        valid_changelog_index,
        valid_leaf_queue_index,
        valid_element_index,
    )
    .await
    .unwrap();

    // 6. nullify leaf with nullifier queue that is not associated with the
    // merkle tree
    let result = nullify(
        &mut context,
        &merkle_tree_pubkey,
        &invalid_nullifier_queue_pubkey,
        queue_config,
        &mut reference_merkle_tree,
        &elements[0].1,
        2,
        valid_leaf_queue_index,
        element_index,
    )
    .await;
    assert_rpc_error(
        result,
        0,
        AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated.into(),
    )
    .unwrap();
}

#[tokio::test]
async fn test_nullify_leaves_default() {
    test_nullify_leaves(
        &StateMerkleTreeConfig::default(),
        &NullifierQueueConfig::default(),
    )
    .await
}

async fn functional_2_test_insert_into_nullifier_queues<R: Rpc>(
    rpc: &mut R,
    nullifier_queue_pubkey: &Pubkey,
    merkle_tree_pubkey: &Pubkey,
) {
    let payer = rpc.get_payer().insecure_clone();
    let elements = vec![[1_u8; 32], [2_u8; 32]];
    insert_into_single_nullifier_queue(
        &elements,
        &payer,
        &payer,
        nullifier_queue_pubkey,
        merkle_tree_pubkey,
        rpc,
    )
    .await
    .unwrap();
    let array = unsafe { get_hash_set::<QueueAccount, R>(rpc, *nullifier_queue_pubkey).await };
    let element_0 = BigUint::from_bytes_be(&elements[0]);
    let (array_element_0, _) = array.find_element(&element_0, None).unwrap().unwrap();
    assert_eq!(array_element_0.value_bytes(), [1u8; 32]);
    assert_eq!(array_element_0.sequence_number(), None);
    let element_1 = BigUint::from_bytes_be(&elements[1]);
    let (array_element_1, _) = array.find_element(&element_1, None).unwrap().unwrap();
    assert_eq!(array_element_1.value_bytes(), [2u8; 32]);
    assert_eq!(array_element_1.sequence_number(), None);
}

async fn fail_3_insert_same_elements_into_nullifier_queue<R: Rpc>(
    context: &mut R,
    nullifier_queue_pubkey: &Pubkey,
    merkle_tree_pubkey: &Pubkey,
    elements: Vec<[u8; 32]>,
) {
    let payer = context.get_payer().insecure_clone();

    let result = insert_into_single_nullifier_queue(
        &elements,
        &payer,
        &payer,
        nullifier_queue_pubkey,
        merkle_tree_pubkey,
        context,
    )
    .await;
    assert_rpc_error(
        result,
        0,
        HashSetError::ElementAlreadyExists.into(), // Invalid proof
    )
    .unwrap();
}

async fn fail_4_insert_with_invalid_signer<R: Rpc>(
    rpc: &mut R,
    nullifier_queue_pubkey: &Pubkey,
    merkle_tree_pubkey: &Pubkey,
    elements: Vec<[u8; 32]>,
) {
    let invalid_signer = Keypair::new();
    airdrop_lamports(rpc, &invalid_signer.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let result = insert_into_single_nullifier_queue(
        &elements,
        &invalid_signer,
        &invalid_signer,
        nullifier_queue_pubkey,
        merkle_tree_pubkey,
        rpc,
    )
    .await;
    assert_rpc_error(
        result,
        0,
        AccountCompressionErrorCode::InvalidAuthority.into(),
    )
    .unwrap();
}

async fn functional_5_test_insert_into_nullifier_queue<R: Rpc>(
    rpc: &mut R,
    nullifier_queue_pubkey: &Pubkey,
    merkle_tree_pubkey: &Pubkey,
) {
    let payer = rpc.get_payer().insecure_clone();
    let element = 3_u32.to_biguint().unwrap();
    let elements = vec![bigint_to_be_bytes_array(&element).unwrap()];
    insert_into_single_nullifier_queue(
        &elements,
        &payer,
        &payer,
        nullifier_queue_pubkey,
        merkle_tree_pubkey,
        rpc,
    )
    .await
    .unwrap();
    let array = unsafe { get_hash_set::<QueueAccount, R>(rpc, *nullifier_queue_pubkey).await };

    let (array_element, _) = array.find_element(&element, None).unwrap().unwrap();
    assert_eq!(array_element.value_biguint(), element);
    assert_eq!(array_element.sequence_number(), None);
}

async fn insert_into_single_nullifier_queue<R: Rpc>(
    elements: &[[u8; 32]],
    fee_payer: &Keypair,
    payer: &Keypair,
    nullifier_queue_pubkey: &Pubkey,
    merkle_tree_pubkey: &Pubkey,
    context: &mut R,
) -> Result<Signature, RpcError> {
    let mut bytes = vec![
        0u8;
        InsertIntoQueuesInstructionDataMut::required_size_for_capacity(
            0,
            elements.len() as u8,
            0,
            0,
            0,
            0
        )
    ];
    let (mut ix_data, _) =
        InsertIntoQueuesInstructionDataMut::new_at(&mut bytes, 0, elements.len() as u8, 0, 0, 0, 0)
            .unwrap();
    ix_data.num_queues = 1;
    for (i, ix_nf) in ix_data.nullifiers.iter_mut().enumerate() {
        ix_nf.account_hash = elements[i];
        ix_nf.queue_index = 0;
        ix_nf.tree_index = 1;
    }

    let instruction_data = account_compression::instruction::InsertIntoQueues { bytes };
    let accounts = account_compression::accounts::GenericInstruction {
        authority: payer.pubkey(),
    };

    let remaining_accounts = vec![
        AccountMeta::new(*nullifier_queue_pubkey, false),
        AccountMeta::new(*merkle_tree_pubkey, false),
    ];
    let instruction = Instruction {
        program_id: ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    };
    let latest_blockhash = context.get_latest_blockhash().await.unwrap().0;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&fee_payer.pubkey()),
        &vec![fee_payer, payer],
        latest_blockhash,
    );
    context.process_transaction(transaction.clone()).await
}

async fn insert_into_nullifier_queues<R: Rpc>(
    elements: &[[u8; 32]],
    fee_payer: &Keypair,
    payer: &Keypair,
    pubkeys: &[(Pubkey, Pubkey)],
    context: &mut R,
) -> Result<Signature, RpcError> {
    let mut hash_set = HashMap::<Pubkey, u8>::new();
    let mut bytes = vec![
        0u8;
        InsertIntoQueuesInstructionDataMut::required_size_for_capacity(
            0,
            elements.len() as u8,
            0,
            0,
            0,
            0
        )
    ];
    let (mut ix_data, _) =
        InsertIntoQueuesInstructionDataMut::new_at(&mut bytes, 0, elements.len() as u8, 0, 0, 0, 0)
            .unwrap();

    for (i, ix_nf) in ix_data.nullifiers.iter_mut().enumerate() {
        ix_nf.account_hash = elements[i];
        ix_nf.queue_index = pack_pubkey(&pubkeys[i].0, &mut hash_set);
        ix_nf.tree_index = pack_pubkey(&pubkeys[i].1, &mut hash_set);
    }
    ix_data.num_queues = if hash_set.len() == 1 {
        1
    } else {
        hash_set.len() as u8 / 2
    };

    let instruction_data = account_compression::instruction::InsertIntoQueues { bytes };
    let accounts = account_compression::accounts::GenericInstruction {
        authority: payer.pubkey(),
    };

    let mut remaining_accounts = hash_set
        .iter()
        .map(|(pubkey, index)| (*pubkey, *index))
        .collect::<Vec<(Pubkey, u8)>>();
    remaining_accounts.sort_by_key(|(_, idx)| *idx);
    let remaining_accounts = remaining_accounts
        .iter()
        .map(|(pubkey, _)| AccountMeta::new(*pubkey, false))
        .collect::<Vec<AccountMeta>>();

    let instruction = Instruction {
        program_id: ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    };
    let latest_blockhash = context.get_latest_blockhash().await.unwrap().0;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&fee_payer.pubkey()),
        &vec![fee_payer, payer],
        latest_blockhash,
    );
    context.process_transaction(transaction.clone()).await
}

#[allow(clippy::too_many_arguments)]
async fn initialize_state_merkle_tree_and_nullifier_queue<R: Rpc>(
    rpc: &mut R,
    payer_pubkey: &Pubkey,
    merkle_tree_keypair: &Keypair,
    queue_keypair: &Keypair,
    merkle_tree_config: &StateMerkleTreeConfig,
    queue_config: &NullifierQueueConfig,
    merkle_tree_size: usize,
    queue_size: usize,
    forester: Option<Pubkey>,
) -> Result<Signature, RpcError> {
    let merkle_tree_account_create_ix = create_account_instruction(
        &rpc.get_payer().pubkey(),
        merkle_tree_size,
        rpc.get_minimum_balance_for_rent_exemption(merkle_tree_size)
            .await
            .unwrap(),
        &ID,
        Some(merkle_tree_keypair),
    );

    let nullifier_queue_account_create_ix = create_account_instruction(
        payer_pubkey,
        queue_size,
        rpc.get_minimum_balance_for_rent_exemption(queue_size)
            .await
            .unwrap(),
        &ID,
        Some(queue_keypair),
    );
    let merkle_tree_pubkey = merkle_tree_keypair.pubkey();

    let instruction = create_initialize_merkle_tree_instruction(
        rpc.get_payer().pubkey(),
        None,
        merkle_tree_pubkey,
        queue_keypair.pubkey(),
        merkle_tree_config.clone(),
        queue_config.clone(),
        None,
        forester,
        1,
    );

    let latest_blockhash = rpc.get_latest_blockhash().await.unwrap().0;
    let transaction = Transaction::new_signed_with_payer(
        &[
            merkle_tree_account_create_ix,
            nullifier_queue_account_create_ix,
            instruction,
        ],
        Some(&rpc.get_payer().pubkey()),
        &vec![&rpc.get_payer(), &merkle_tree_keypair, queue_keypair],
        latest_blockhash,
    );
    rpc.process_transaction(transaction.clone()).await
}

pub async fn fail_initialize_state_merkle_tree_and_nullifier_queue_invalid_sizes<R: Rpc>(
    rpc: &mut R,
    payer_pubkey: &Pubkey,
    merkle_tree_keypair: &Keypair,
    queue_keypair: &Keypair,
    merkle_tree_config: &StateMerkleTreeConfig,
    queue_config: &NullifierQueueConfig,
) {
    let valid_tree_size = StateMerkleTreeAccount::size(
        merkle_tree_config.height as usize,
        merkle_tree_config.changelog_size as usize,
        merkle_tree_config.roots_size as usize,
        merkle_tree_config.canopy_depth as usize,
    );
    let valid_queue_size = QueueAccount::size(queue_config.capacity as usize).unwrap();

    // NOTE: Starting from 0 to the account struct size triggers a panic in Anchor
    // macros (sadly, not assertable...), which happens earlier than our
    // serialization error.
    // Our recoverable error is thrown for ranges from the struct size
    // (+ discriminator) up to the expected account size.
    for invalid_tree_size in
        (8 + mem::size_of::<StateMerkleTreeAccount>()..valid_tree_size).step_by(200_000)
    {
        for invalid_queue_size in
            (8 + mem::size_of::<QueueAccount>()..valid_queue_size).step_by(50_000)
        {
            let result = initialize_state_merkle_tree_and_nullifier_queue(
                rpc,
                payer_pubkey,
                merkle_tree_keypair,
                queue_keypair,
                merkle_tree_config,
                queue_config,
                invalid_tree_size,
                invalid_queue_size,
                None,
            )
            .await;
            assert_rpc_error(result, 2, AccountError::InvalidAccountSize.into()).unwrap();
        }
    }
}

/// Tries to initzalize Merkle tree and queue with unsupported configuration
/// parameters:
///
/// 1. Merkle tree height (different than 26).
/// 2. Merkle tree canopy depth (different than 10).
/// 3. Merkle tree changelog size (zero).
/// 4. Merkle tree roots size (zero).
/// 5. Merkle tree close threshold (any).
/// 6. Queue sequence threshold (lower than roots + safety margin).
pub async fn fail_initialize_state_merkle_tree_and_nullifier_queue_invalid_config<R: Rpc>(
    rpc: &mut R,
    payer_pubkey: &Pubkey,
    merkle_tree_keypair: &Keypair,
    queue_keypair: &Keypair,
    merkle_tree_config: &StateMerkleTreeConfig,
    queue_config: &NullifierQueueConfig,
) {
    let merkle_tree_size = StateMerkleTreeAccount::size(
        merkle_tree_config.height as usize,
        merkle_tree_config.changelog_size as usize,
        merkle_tree_config.roots_size as usize,
        merkle_tree_config.canopy_depth as usize,
    );
    let queue_size = QueueAccount::size(queue_config.capacity as usize).unwrap();

    for invalid_height in (0..26).step_by(5) {
        let mut merkle_tree_config = merkle_tree_config.clone();
        merkle_tree_config.height = invalid_height;
        let result = initialize_state_merkle_tree_and_nullifier_queue(
            rpc,
            payer_pubkey,
            merkle_tree_keypair,
            queue_keypair,
            &merkle_tree_config,
            queue_config,
            merkle_tree_size,
            queue_size,
            None,
        )
        .await;
        assert_rpc_error(
            result,
            2,
            AccountCompressionErrorCode::UnsupportedHeight.into(),
        )
        .unwrap();
    }
    for invalid_height in (27..50).step_by(5) {
        let mut merkle_tree_config = merkle_tree_config.clone();
        merkle_tree_config.height = invalid_height;
        let result = initialize_state_merkle_tree_and_nullifier_queue(
            rpc,
            payer_pubkey,
            merkle_tree_keypair,
            queue_keypair,
            &merkle_tree_config,
            queue_config,
            merkle_tree_size,
            queue_size,
            None,
        )
        .await;
        assert_rpc_error(
            result,
            2,
            AccountCompressionErrorCode::UnsupportedHeight.into(),
        )
        .unwrap();
    }
    for invalid_canopy_depth in (0..10).step_by(3) {
        let mut merkle_tree_config = merkle_tree_config.clone();
        merkle_tree_config.canopy_depth = invalid_canopy_depth;
        let result = initialize_state_merkle_tree_and_nullifier_queue(
            rpc,
            payer_pubkey,
            merkle_tree_keypair,
            queue_keypair,
            &merkle_tree_config,
            queue_config,
            merkle_tree_size,
            queue_size,
            None,
        )
        .await;
        assert_rpc_error(
            result,
            2,
            AccountCompressionErrorCode::UnsupportedCanopyDepth.into(),
        )
        .unwrap();
    }
    {
        let mut merkle_tree_config = merkle_tree_config.clone();
        merkle_tree_config.changelog_size = 0;
        let merkle_tree_size = StateMerkleTreeAccount::size(
            merkle_tree_config.height as usize,
            merkle_tree_config.changelog_size as usize,
            merkle_tree_config.roots_size as usize,
            merkle_tree_config.canopy_depth as usize,
        );
        let result = initialize_state_merkle_tree_and_nullifier_queue(
            rpc,
            payer_pubkey,
            merkle_tree_keypair,
            queue_keypair,
            &merkle_tree_config,
            queue_config,
            merkle_tree_size,
            queue_size,
            None,
        )
        .await;
        assert_rpc_error(result, 2, ConcurrentMerkleTreeError::ChangelogZero.into()).unwrap();
    }
    {
        let mut merkle_tree_config = merkle_tree_config.clone();
        merkle_tree_config.roots_size = 0;
        let merkle_tree_size = StateMerkleTreeAccount::size(
            merkle_tree_config.height as usize,
            merkle_tree_config.changelog_size as usize,
            merkle_tree_config.roots_size as usize,
            merkle_tree_config.canopy_depth as usize,
        );
        let result = initialize_state_merkle_tree_and_nullifier_queue(
            rpc,
            payer_pubkey,
            merkle_tree_keypair,
            queue_keypair,
            &merkle_tree_config,
            queue_config,
            merkle_tree_size,
            queue_size,
            None,
        )
        .await;
        assert_rpc_error(result, 2, ConcurrentMerkleTreeError::RootsZero.into()).unwrap();
    }
    for invalid_close_threshold in (0..100).step_by(20) {
        let mut merkle_tree_config = merkle_tree_config.clone();
        merkle_tree_config.close_threshold = Some(invalid_close_threshold);
        let result = initialize_state_merkle_tree_and_nullifier_queue(
            rpc,
            payer_pubkey,
            merkle_tree_keypair,
            queue_keypair,
            &merkle_tree_config,
            queue_config,
            merkle_tree_size,
            queue_size,
            None,
        )
        .await;
        assert_rpc_error(
            result,
            2,
            AccountCompressionErrorCode::UnsupportedCloseThreshold.into(),
        )
        .unwrap();
    }
    for invalid_sequence_threshold in
        (0..merkle_tree_config.roots_size + SAFETY_MARGIN).step_by(200)
    {
        let mut queue_config = queue_config.clone();
        queue_config.sequence_threshold = invalid_sequence_threshold;
        let result = initialize_state_merkle_tree_and_nullifier_queue(
            rpc,
            payer_pubkey,
            merkle_tree_keypair,
            queue_keypair,
            merkle_tree_config,
            &queue_config,
            merkle_tree_size,
            queue_size,
            None,
        )
        .await;
        assert_rpc_error(
            result,
            2,
            AccountCompressionErrorCode::InvalidSequenceThreshold.into(),
        )
        .unwrap();
    }
}

async fn functional_1_initialize_state_merkle_tree_and_nullifier_queue<R: Rpc>(
    rpc: &mut R,
    payer_pubkey: &Pubkey,
    merkle_tree_keypair: &Keypair,
    queue_keypair: &Keypair,
    merkle_tree_config: &StateMerkleTreeConfig,
    queue_config: &NullifierQueueConfig,
) -> Pubkey {
    let merkle_tree_size = StateMerkleTreeAccount::size(
        merkle_tree_config.height as usize,
        merkle_tree_config.changelog_size as usize,
        merkle_tree_config.roots_size as usize,
        merkle_tree_config.canopy_depth as usize,
    );
    let queue_size = QueueAccount::size(queue_config.capacity as usize).unwrap();
    let forester = Pubkey::new_unique();
    initialize_state_merkle_tree_and_nullifier_queue(
        rpc,
        payer_pubkey,
        merkle_tree_keypair,
        queue_keypair,
        merkle_tree_config,
        queue_config,
        merkle_tree_size,
        queue_size,
        Some(forester),
    )
    .await
    .unwrap();

    assert_merkle_tree_initialized(
        rpc,
        &merkle_tree_keypair.pubkey(),
        &queue_keypair.pubkey(),
        merkle_tree_config.height as usize,
        merkle_tree_config.changelog_size as usize,
        merkle_tree_config.roots_size as usize,
        merkle_tree_config.canopy_depth as usize,
        1,
        1,
        0,
        &Poseidon::zero_bytes()[0],
        merkle_tree_config.rollover_threshold,
        merkle_tree_config.close_threshold,
        merkle_tree_config.network_fee.unwrap(),
        payer_pubkey,
    )
    .await;
    assert_nullifier_queue_initialized(
        rpc,
        &queue_keypair.pubkey(),
        queue_config,
        &merkle_tree_keypair.pubkey(),
        merkle_tree_config,
        QueueType::NullifierV1,
        1,
        None,
        Some(forester),
        payer_pubkey,
    )
    .await;
    merkle_tree_keypair.pubkey()
}

pub async fn fail_2_append_leaves_with_invalid_inputs<R: Rpc>(
    context: &mut R,
    merkle_tree_pubkeys: &[Pubkey],
    leaves: Vec<(u8, [u8; 32])>,
    expected_error: u32,
) -> Result<(), RpcError> {
    let mut bytes = vec![
        0u8;
        InsertIntoQueuesInstructionDataMut::required_size_for_capacity(
            leaves.len() as u8,
            0,
            0,
            merkle_tree_pubkeys.len() as u8,
            0,
            0,
        )
    ];
    let (mut ix_data, _) = InsertIntoQueuesInstructionDataMut::new_at(
        &mut bytes,
        leaves.len() as u8,
        0,
        0,
        merkle_tree_pubkeys.len() as u8,
        0,
        0,
    )
    .unwrap();
    ix_data.num_output_queues = merkle_tree_pubkeys.len() as u8;

    for (i, (index, leaf)) in leaves.iter().enumerate() {
        ix_data.leaves[i].leaf = *leaf;
        ix_data.leaves[i].account_index = *index;
    }
    let instruction_data = account_compression::instruction::InsertIntoQueues { bytes };

    let accounts = account_compression::accounts::GenericInstruction {
        authority: context.get_payer().pubkey(),
    };

    let instruction = Instruction {
        program_id: ID,
        accounts: [
            accounts.to_account_metas(Some(true)),
            merkle_tree_pubkeys
                .iter()
                .map(|merkle_tree_pubkey| AccountMeta::new(*merkle_tree_pubkey, false))
                .collect::<Vec<AccountMeta>>(),
        ]
        .concat(),
        data: instruction_data.data(),
    };

    let latest_blockhash = context.get_latest_blockhash().await.unwrap().0;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.get_payer().pubkey()),
        &vec![&context.get_payer()],
        latest_blockhash,
    );
    let result = context.process_transaction(transaction).await;
    assert_rpc_error(result, 0, expected_error)
}

pub async fn functional_3_append_leaves_to_merkle_tree<R: Rpc>(
    context: &mut R,
    reference_merkle_trees: &mut [&mut MerkleTree<Poseidon>],
    merkle_tree_pubkeys: &Vec<Pubkey>,
    leaves: &Vec<(u8, [u8; 32])>,
) {
    let payer = context.get_payer().insecure_clone();
    let mut hash_map = HashMap::<Pubkey, (Vec<[u8; 32]>, u64, usize, usize)>::new();
    for (i, leaf) in leaves {
        let pre_account_mt = context
            .get_account(merkle_tree_pubkeys[(*i) as usize])
            .await
            .unwrap()
            .unwrap();
        let old_merkle_tree =
            get_concurrent_merkle_tree::<StateMerkleTreeAccount, R, Poseidon, 26>(
                context,
                merkle_tree_pubkeys[(*i) as usize],
            )
            .await;
        hash_map
            .entry(merkle_tree_pubkeys[(*i) as usize])
            .or_insert_with(|| {
                (
                    Vec::<[u8; 32]>::new(),
                    pre_account_mt.lamports,
                    old_merkle_tree.next_index(),
                    *i as usize,
                )
            })
            .0
            .push(*leaf);
    }
    let instruction = [create_insert_leaves_instruction(
        leaves.clone(),
        context.get_payer().pubkey(),
        (*merkle_tree_pubkeys).clone(),
    )];

    context
        .create_and_send_transaction(&instruction, &payer.pubkey(), &[&payer, &payer])
        .await
        .unwrap();

    for (pubkey, (leaves, lamports, next_index, mt_index)) in hash_map.iter() {
        let num_leaves = leaves.len();
        let post_account_mt = context.get_account(*pubkey).await.unwrap().unwrap();

        let merkle_tree =
            get_concurrent_merkle_tree::<StateMerkleTreeAccount, R, Poseidon, 26>(context, *pubkey)
                .await;
        assert_eq!(merkle_tree.next_index(), next_index + num_leaves);
        let leaves: Vec<&[u8; 32]> = leaves.iter().collect();

        let reference_merkle_tree = &mut reference_merkle_trees[*mt_index];
        reference_merkle_tree.append_batch(&leaves).unwrap();

        assert_eq!(merkle_tree.root(), reference_merkle_tree.root());
        assert_eq!(*lamports, post_account_mt.lamports);

        let changelog_entry = merkle_tree
            .changelog
            .get(merkle_tree.changelog_index())
            .unwrap();
        let path = reference_merkle_tree
            .get_path_of_leaf(merkle_tree.current_index(), true)
            .unwrap();
        assert!(changelog_entry.path.eq_to_vec(path));
    }
}

pub async fn fail_4_append_leaves_with_invalid_authority<R: Rpc>(
    rpc: &mut R,
    merkle_tree_pubkey: &Pubkey,
) {
    let invalid_autority = Keypair::new();
    airdrop_lamports(rpc, &invalid_autority.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let mut bytes =
        vec![
            0u8;
            InsertIntoQueuesInstructionDataMut::required_size_for_capacity(1, 0, 0, 1, 0, 0,)
        ];
    let (mut ix_data, _) =
        InsertIntoQueuesInstructionDataMut::new_at(&mut bytes, 1, 0, 0, 1, 0, 0).unwrap();
    ix_data.num_output_queues = 1;
    ix_data.leaves[0].leaf = [1; 32];
    ix_data.leaves[0].account_index = 0;

    let instruction_data = account_compression::instruction::InsertIntoQueues { bytes };
    let accounts = account_compression::accounts::GenericInstruction {
        authority: invalid_autority.pubkey(),
    };

    let instruction = Instruction {
        program_id: ID,
        accounts: [
            accounts.to_account_metas(Some(true)),
            vec![AccountMeta::new(*merkle_tree_pubkey, false)],
        ]
        .concat(),
        data: instruction_data.data(),
    };
    let latest_blockhash = rpc.get_latest_blockhash().await.unwrap().0;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&invalid_autority.pubkey()),
        &vec![&invalid_autority],
        latest_blockhash,
    );
    let remaining_accounts_mismatch_error = rpc.process_transaction(transaction).await;
    assert!(remaining_accounts_mismatch_error.is_err());
}

#[allow(clippy::too_many_arguments)]
pub async fn nullify<R: Rpc>(
    rpc: &mut R,
    merkle_tree_pubkey: &Pubkey,
    nullifier_queue_pubkey: &Pubkey,
    nullifier_queue_config: &NullifierQueueConfig,
    reference_merkle_tree: &mut MerkleTree<Poseidon>,
    element: &[u8; 32],
    change_log_index: u64,
    leaf_queue_index: u16,
    element_index: u64,
) -> Result<(), RpcError> {
    let payer = rpc.get_payer().insecure_clone();
    let proof: Vec<[u8; 32]> = reference_merkle_tree
        .get_proof_of_leaf(element_index as usize, false)
        .unwrap();

    let instructions = [
        account_compression::nullify_leaves::sdk_nullify::create_nullify_instruction(
            vec![change_log_index].as_slice(),
            vec![leaf_queue_index].as_slice(),
            vec![element_index].as_slice(),
            vec![proof].as_slice(),
            &rpc.get_payer().pubkey(),
            merkle_tree_pubkey,
            nullifier_queue_pubkey,
        ),
    ];

    let event = rpc
        .create_and_send_transaction_with_event::<MerkleTreeEvent>(
            &instructions,
            &payer.pubkey(),
            &[&payer],
        )
        .await?;

    let merkle_tree = get_concurrent_merkle_tree::<StateMerkleTreeAccount, R, Poseidon, 26>(
        rpc,
        *merkle_tree_pubkey,
    )
    .await;
    reference_merkle_tree
        .update(&ZERO_BYTES[0], element_index as usize)
        .unwrap();
    assert_eq!(merkle_tree.root(), reference_merkle_tree.root());

    let account = rpc
        .get_account(*nullifier_queue_pubkey)
        .await
        .unwrap()
        .unwrap();
    let mut data = account.data.clone();

    let nullifier_queue = &mut unsafe { queue_from_bytes_zero_copy_mut(&mut data).unwrap() };

    let array_element = nullifier_queue
        .get_bucket(leaf_queue_index.into())
        .unwrap()
        .unwrap();
    assert_eq!(&array_element.value_bytes(), element);
    assert_eq!(
        array_element.sequence_number(),
        Some(merkle_tree.sequence_number() + nullifier_queue_config.sequence_threshold as usize)
    );
    let event = event.unwrap().0;
    match event {
        MerkleTreeEvent::V1(_) => panic!("Expected V2 event"),
        MerkleTreeEvent::V2(event_v1) => {
            assert_eq!(event_v1.id, merkle_tree_pubkey.to_bytes());
            assert_eq!(event_v1.nullified_leaves_indices[0], element_index);
        }
        MerkleTreeEvent::V3(_) => panic!("Expected V2 event"),
    }
    Ok(())
}

pub async fn set_nullifier_queue_to_full<R: Rpc + TestRpc>(
    rpc: &mut R,
    nullifier_queue_pubkey: &Pubkey,
    left_over_indices: usize,
    lamports: u64,
) {
    let mut account = rpc
        .get_account(*nullifier_queue_pubkey)
        .await
        .unwrap()
        .unwrap();
    let mut data = account.data.clone();
    let capacity;
    {
        let hash_set = &mut unsafe { queue_from_bytes_zero_copy_mut(&mut data).unwrap() };
        capacity = hash_set.hash_set.get_capacity() - left_over_indices;
        println!("capacity: {}", capacity);
        let arbitrary_sequence_number = 0;
        for i in 0..capacity {
            hash_set
                .insert(&i.to_biguint().unwrap(), arbitrary_sequence_number)
                .unwrap();
        }
    }
    assert_ne!(account.data, data);
    account.data = data;
    account.lamports = lamports;

    rpc.set_account(*nullifier_queue_pubkey, account.clone());
    let new_data = account.data.clone();
    let account = rpc
        .get_account(*nullifier_queue_pubkey)
        .await
        .unwrap()
        .unwrap();
    let mut data = account.data.clone();
    assert_eq!(new_data, data);
    let nullifier_queue = &mut unsafe { queue_from_bytes_zero_copy_mut(&mut data).unwrap() };
    for i in 0..capacity {
        assert!(nullifier_queue
            .contains(&i.to_biguint().unwrap(), None)
            .unwrap());
    }
}

fn find_overlapping_probe_index(
    initial_value: usize,
    start_replacement_value: usize,
    capacity_values: usize,
) -> usize {
    for salt in 0..capacity_values {
        let replacement_value = start_replacement_value + salt;

        for i in 0..20 {
            let probe_index = (initial_value + i.to_biguint().unwrap() * i.to_biguint().unwrap())
                % capacity_values.to_biguint().unwrap();
            let replacement_probe_index = (replacement_value
                + i.to_biguint().unwrap() * i.to_biguint().unwrap())
                % capacity_values.to_biguint().unwrap();
            if probe_index == replacement_probe_index {
                return replacement_value;
            }
        }
    }
    panic!("No value with overlapping probe index found!");
}
async fn fail_insert_into_full_queue<R: Rpc>(
    context: &mut R,
    nullifier_queue_pubkey: &Pubkey,
    merkle_tree_pubkey: &Pubkey,
    elements: Vec<[u8; 32]>,
) {
    let payer = context.get_payer().insecure_clone();

    let result = insert_into_single_nullifier_queue(
        &elements,
        &payer,
        &payer,
        nullifier_queue_pubkey,
        merkle_tree_pubkey,
        context,
    )
    .await;

    assert_rpc_error(result, 0, HashSetError::Full.into()).unwrap();
}

pub async fn set_state_merkle_tree_sequence<R: Rpc + TestRpc>(
    rpc: &mut R,
    merkle_tree_pubkey: &Pubkey,
    sequence_number: u64,
    lamports: u64,
) {
    let mut merkle_tree = rpc.get_account(*merkle_tree_pubkey).await.unwrap().unwrap();
    {
        let merkle_tree_deserialized =
            &mut ConcurrentMerkleTreeZeroCopyMut::<Poseidon, 26>::from_bytes_zero_copy_mut(
                &mut merkle_tree.data[8 + mem::size_of::<StateMerkleTreeAccount>()..],
            )
            .unwrap();
        while merkle_tree_deserialized.sequence_number() < sequence_number as usize {
            merkle_tree_deserialized.inc_sequence_number().unwrap();
        }
    }
    merkle_tree.lamports = lamports;
    rpc.set_account(*merkle_tree_pubkey, merkle_tree);
    let mut merkle_tree = rpc.get_account(*merkle_tree_pubkey).await.unwrap().unwrap();
    let merkle_tree_deserialized =
        ConcurrentMerkleTreeZeroCopyMut::<Poseidon, 26>::from_bytes_zero_copy_mut(
            &mut merkle_tree.data[8 + mem::size_of::<StateMerkleTreeAccount>()..],
        )
        .unwrap();
    assert_eq!(
        merkle_tree_deserialized.sequence_number() as u64,
        sequence_number
    );
}
pub async fn assert_element_inserted_in_nullifier_queue(
    rpc: &mut LightProgramTest,
    nullifier_queue_pubkey: &Pubkey,
    nullifier: [u8; 32],
) {
    let array = unsafe {
        get_hash_set::<QueueAccount, LightProgramTest>(rpc, *nullifier_queue_pubkey).await
    };
    let nullifier_bn = BigUint::from_bytes_be(&nullifier);
    let (array_element, _) = array.find_element(&nullifier_bn, None).unwrap().unwrap();
    assert_eq!(array_element.value_bytes(), nullifier);
    assert_eq!(array_element.sequence_number(), None);
}

async fn functional_6_test_insert_into_two_nullifier_queues(
    rpc: &mut LightProgramTest,
    nullifiers: &[[u8; 32]],
    queue_tree_pairs: &[(Pubkey, Pubkey)],
) {
    let payer = rpc.get_payer().insecure_clone();
    insert_into_nullifier_queues(nullifiers, &payer, &payer, queue_tree_pairs, rpc)
        .await
        .unwrap();
    assert_element_inserted_in_nullifier_queue(rpc, &queue_tree_pairs[0].0, nullifiers[0]).await;
    assert_element_inserted_in_nullifier_queue(rpc, &queue_tree_pairs[1].0, nullifiers[1]).await;
}

async fn functional_7_test_insert_into_two_nullifier_queues_not_ordered(
    rpc: &mut LightProgramTest,
    nullifiers: &[[u8; 32]],
    queue_tree_pairs: &[(Pubkey, Pubkey)],
) {
    let payer = rpc.get_payer().insecure_clone();
    insert_into_nullifier_queues(nullifiers, &payer, &payer, queue_tree_pairs, rpc)
        .await
        .unwrap();
    assert_element_inserted_in_nullifier_queue(rpc, &queue_tree_pairs[0].0, nullifiers[0]).await;
    assert_element_inserted_in_nullifier_queue(rpc, &queue_tree_pairs[0].0, nullifiers[2]).await;
    assert_element_inserted_in_nullifier_queue(rpc, &queue_tree_pairs[1].0, nullifiers[1]).await;
    assert_element_inserted_in_nullifier_queue(rpc, &queue_tree_pairs[1].0, nullifiers[3]).await;
}
