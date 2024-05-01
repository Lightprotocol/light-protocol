#![cfg(feature = "test-sbf")]
use account_compression::{
    self,
    errors::AccountCompressionErrorCode,
    initialize_nullifier_queue::{nullifier_queue_from_bytes_zero_copy_mut, NullifierQueueAccount},
    sdk::{create_initialize_merkle_tree_instruction, create_insert_leaves_instruction},
    utils::constants::{
        STATE_MERKLE_TREE_CANOPY_DEPTH, STATE_MERKLE_TREE_CHANGELOG, STATE_MERKLE_TREE_HEIGHT,
        STATE_MERKLE_TREE_ROOTS, STATE_NULLIFIER_QUEUE_INDICES, STATE_NULLIFIER_QUEUE_VALUES,
    },
    NullifierQueueConfig, StateMerkleTreeAccount, StateMerkleTreeConfig, ID,
};
use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use light_concurrent_merkle_tree::{event::MerkleTreeEvent, ConcurrentMerkleTree26};
use light_hash_set::HashSetError;
use light_hasher::{zero_bytes::poseidon::ZERO_BYTES, Hasher, Poseidon};
use light_merkle_tree_reference::MerkleTree;
use light_test_utils::rpc::errors::{assert_rpc_error, RpcError};
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::test_rpc::ProgramTestRpcConnection;
use light_test_utils::{
    airdrop_lamports, create_account_instruction, get_hash_set,
    merkle_tree::assert_merkle_tree_initialized,
    state_tree_rollover::{
        assert_rolled_over_pair, perform_state_merkle_tree_roll_over,
        set_state_merkle_tree_next_index,
    },
    AccountZeroCopy,
};
use light_utils::bigint::bigint_to_be_bytes_array;
use memoffset::offset_of;
use num_bigint::ToBigUint;
use solana_program_test::ProgramTest;
use solana_sdk::{
    account::AccountSharedData,
    instruction::{AccountMeta, Instruction},
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solana_sdk::{account::WritableAccount, pubkey::Pubkey};

/// Tests:
/// Show that we cannot insert into a full queue.
/// 1. try to insert into queue to generate the full error
/// 2. nullify one
/// 3. try to insert again it should still generate the full error
/// 4. advance Merkle tree seq until one before it would work check that it still fails
/// 5. advance Merkle tree seq by one and check that inserting works now
/// 6.try inserting again it should fail with full error
#[tokio::test]
async fn test_nullifier_queue_security() {
    let mut program_test = ProgramTest::default();
    program_test.add_program("account_compression", ID, None);
    program_test.add_program(
        "spl_noop",
        Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
        None,
    );
    let merkle_tree_keypair = Keypair::new();
    let merkle_tree_pubkey = merkle_tree_keypair.pubkey();
    let nullifier_queue_keypair = Keypair::new();
    let nullifier_queue_pubkey = nullifier_queue_keypair.pubkey();
    program_test.set_compute_max_units(1_400_000u64);
    let context = program_test.start_with_context().await;
    let mut rpc = ProgramTestRpcConnection { context };
    let payer_pubkey = rpc.get_payer().pubkey();
    let tip = 123;
    let rollover_threshold = Some(95);
    let close_threshold = Some(100);
    functional_1_initialize_state_merkle_tree_and_nullifier_queue(
        &mut rpc,
        &payer_pubkey,
        &merkle_tree_keypair,
        &nullifier_queue_keypair,
        tip,
        rollover_threshold.clone(),
        close_threshold,
    )
    .await;
    let leaf: [u8; 32] = bigint_to_be_bytes_array(&1.to_biguint().unwrap()).unwrap();

    functional_3_append_leaves_to_merkle_tree(&mut rpc, &merkle_tree_pubkey, &vec![(0u8, leaf)])
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
    set_nullifier_queue_to_full(
        &mut rpc,
        &nullifier_queue_pubkey,
        0,
        lamports_queue_accounts,
    )
    .await;

    let initial_value = 6005;
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
    let onchain_merkle_tree =
        AccountZeroCopy::<StateMerkleTreeAccount>::new(&mut rpc, merkle_tree_pubkey).await;
    let deserialized = onchain_merkle_tree.deserialized();
    let merkle_tree = deserialized.copy_merkle_tree().unwrap();
    assert_eq!(merkle_tree.root(), reference_merkle_tree.root());
    let leaf_index = reference_merkle_tree.get_leaf_index(&leaf).unwrap() as u64;
    // CHECK 2
    nullify(
        &mut rpc,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &mut reference_merkle_tree,
        &leaf,
        merkle_tree.changelog_index() as u64,
        1,
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
    // advance to sequence number minus one
    set_state_merkle_tree_sequence(&mut rpc, &merkle_tree_pubkey, 2402, lamports_queue_accounts)
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
    set_state_merkle_tree_sequence(&mut rpc, &merkle_tree_pubkey, 2403, lamports_queue_accounts)
        .await;
    let payer = rpc.get_payer().insecure_clone();
    let account = rpc
        .get_account(nullifier_queue_pubkey)
        .await
        .unwrap()
        .unwrap();

    let mut data = account.data.clone();
    let nullifier_queue =
        &mut unsafe { nullifier_queue_from_bytes_zero_copy_mut(&mut data).unwrap() };
    let replacement_start_value = 606;
    let replacement_value = find_overlapping_probe_index(
        1,
        replacement_start_value,
        nullifier_queue.hash_set.capacity_values,
    );
    // CHECK: 5
    let element: [u8; 32] =
        bigint_to_be_bytes_array(&replacement_value.to_biguint().unwrap()).unwrap();
    insert_into_nullifier_queues(
        &vec![element],
        &payer,
        &payer,
        &nullifier_queue_pubkey,
        &merkle_tree_pubkey,
        &mut rpc,
    )
    .await
    .unwrap();
    // CHECK: 6
    let element: [u8; 32] = bigint_to_be_bytes_array(&12000.to_biguint().unwrap()).unwrap();
    fail_insert_into_full_queue(
        &mut rpc,
        &nullifier_queue_pubkey,
        &merkle_tree_pubkey,
        vec![element],
    )
    .await;
}

pub async fn set_nullifier_queue_to_full<R: RpcConnection>(
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
    let current_index;
    let capacity;
    {
        let hash_set = &mut unsafe { nullifier_queue_from_bytes_zero_copy_mut(&mut data).unwrap() };
        current_index = unsafe { *hash_set.hash_set.next_value_index };

        capacity = hash_set.hash_set.capacity_values - left_over_indices;
        for i in current_index..capacity {
            hash_set.insert(&(i).to_biguint().unwrap(), 2400).unwrap();
        }
    }
    assert_ne!(account.data, data);
    account.data = data;
    let mut account_share_data = AccountSharedData::from(account);
    account_share_data.set_lamports(lamports);
    rpc.set_account(nullifier_queue_pubkey, &account_share_data);
    let account = rpc
        .get_account(*nullifier_queue_pubkey)
        .await
        .unwrap()
        .unwrap();
    let mut data = account.data.clone();
    let nullifier_queue =
        &mut unsafe { nullifier_queue_from_bytes_zero_copy_mut(&mut data).unwrap() };
    for i in current_index..capacity {
        let array_element = nullifier_queue.by_value_index(i, None).unwrap();
        assert_eq!(array_element.value_biguint(), i.to_biguint().unwrap());
    }
}

fn find_overlapping_probe_index(
    initial_value: usize,
    start_replacement_value: usize,
    capacity_values: usize,
) -> usize {
    for salt in 0..10000 {
        let replacement_value = start_replacement_value + salt;

        for i in 0..20 {
            let probe_index = (initial_value.clone()
                + i.to_biguint().unwrap() * i.to_biguint().unwrap())
                % capacity_values.to_biguint().unwrap();
            let replacement_probe_index = (replacement_value.clone()
                + i.to_biguint().unwrap() * i.to_biguint().unwrap())
                % capacity_values.to_biguint().unwrap();
            if probe_index == replacement_probe_index {
                return replacement_value;
            }
        }
    }
    panic!("No value with overlapping probe index found!");
}
async fn fail_insert_into_full_queue<R: RpcConnection>(
    context: &mut R,
    nullifier_queue_pubkey: &Pubkey,
    merkle_tree_pubkey: &Pubkey,
    elements: Vec<[u8; 32]>,
) {
    let payer = context.get_payer().insecure_clone();

    let result = insert_into_nullifier_queues(
        &elements,
        &payer,
        &payer,
        nullifier_queue_pubkey,
        merkle_tree_pubkey,
        context,
    )
    .await;

    assert_rpc_error(result, 0, HashSetError::Full.into());
}

pub async fn set_state_merkle_tree_sequence<R: RpcConnection>(
    rpc: &mut R,
    merkle_tree_pubkey: &Pubkey,
    sequence_number: u64,
    lamports: u64,
) {
    // is in range 9 - 10 in concurrent mt
    // offset for sequence number
    // let offset_start = 6 * 8 + 8 + 4 * 32 + 8 * 9;
    // let offset_end = offset_start + 8;
    let offset_start = 8
        + offset_of!(StateMerkleTreeAccount, state_merkle_tree_struct)
        + offset_of!(ConcurrentMerkleTree26<Poseidon>, sequence_number);
    let offset_end = offset_start + 8;
    let mut merkle_tree = rpc.get_account(*merkle_tree_pubkey).await.unwrap().unwrap();
    merkle_tree.data[offset_start..offset_end].copy_from_slice(&sequence_number.to_le_bytes());
    let mut account_share_data = AccountSharedData::from(merkle_tree);
    account_share_data.set_lamports(lamports);
    rpc.set_account(merkle_tree_pubkey, &account_share_data);
    let merkle_tree = rpc.get_account(*merkle_tree_pubkey).await.unwrap().unwrap();
    let data_in_offset = u64::from_le_bytes(
        merkle_tree.data[offset_start..offset_end]
            .try_into()
            .unwrap(),
    );
    assert_eq!(data_in_offset, sequence_number);
}

/// Tests:
/// 1. Should fail: not ready for rollover
/// 2. Should fail: merkle tree and queue not associated (invalid tree)
/// 3. Should fail: merkle tree and queue not associated (invalid queue)
/// 4. Should succeed: rollover state merkle tree
/// 5. Should fail: merkle tree already rolled over
#[tokio::test]
async fn test_init_and_rollover_state_merkle_tree() {
    let mut program_test = ProgramTest::default();
    program_test.add_program("account_compression", ID, None);
    program_test.add_program(
        "spl_noop",
        Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
        None,
    );
    let merkle_tree_keypair = Keypair::new();
    let merkle_tree_pubkey = merkle_tree_keypair.pubkey();
    let nullifier_queue_keypair = Keypair::new();
    let nullifier_queue_pubkey = nullifier_queue_keypair.pubkey();
    program_test.set_compute_max_units(1_400_000u64);
    let context = program_test.start_with_context().await;
    let mut context = ProgramTestRpcConnection { context };
    let payer_pubkey = context.get_payer().pubkey();
    let tip = 123;
    let rollover_threshold = Some(95);
    let close_threshold = Some(100);
    functional_1_initialize_state_merkle_tree_and_nullifier_queue(
        &mut context,
        &payer_pubkey,
        &merkle_tree_keypair,
        &nullifier_queue_keypair,
        tip,
        rollover_threshold,
        close_threshold,
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
        tip,
        rollover_threshold,
        close_threshold,
    )
    .await;

    let required_next_index = 2u64.pow(26) * rollover_threshold.unwrap() / 100;
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
    )
    .await;

    assert_rpc_error(
        result,
        2,
        AccountCompressionErrorCode::NotReadyForRollover.into(),
    );

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
    )
    .await;

    assert_rpc_error(
        result,
        2,
        AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated.into(),
    );

    let result = perform_state_merkle_tree_roll_over(
        &mut context,
        &new_nullifier_queue_keypair,
        &new_state_merkle_tree_keypair,
        &merkle_tree_pubkey_2,
        &nullifier_queue_keypair.pubkey(),
    )
    .await;

    assert_rpc_error(
        result,
        2,
        AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated.into(),
    );

    let signer_prior_balance = context
        .get_account(payer_pubkey)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    perform_state_merkle_tree_roll_over(
        &mut context,
        &new_nullifier_queue_keypair,
        &new_state_merkle_tree_keypair,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
    )
    .await
    .unwrap();

    assert_rolled_over_pair(
        &mut context,
        &signer_prior_balance,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &new_state_merkle_tree_keypair.pubkey(),
        &new_nullifier_queue_keypair.pubkey(),
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
    )
    .await;

    assert_rpc_error(
        result,
        2,
        AccountCompressionErrorCode::MerkleTreeAlreadyRolledOver.into(),
    );
}

/// Tests:
/// 1. Functional: Initialize merkle tree
/// 2. Failing: Append with invalid inputs
/// 3. Functional: Append leaves to merkle tree
/// 4. Failing: Append leaves with invalid authority
#[tokio::test]
async fn test_init_and_insert_leaves_into_merkle_tree() {
    let mut program_test = ProgramTest::default();
    program_test.add_program("account_compression", ID, None);
    program_test.add_program(
        "spl_noop",
        Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
        None,
    );

    program_test.set_compute_max_units(1_400_000u64);
    let context = program_test.start_with_context().await;
    let mut context = ProgramTestRpcConnection { context };
    let payer_pubkey = context.get_payer().pubkey();
    let merkle_tree_keypair = Keypair::new();
    let queue_keypair = Keypair::new();
    let merkle_tree_pubkey = functional_1_initialize_state_merkle_tree_and_nullifier_queue(
        &mut context,
        &payer_pubkey,
        &merkle_tree_keypair,
        &queue_keypair,
        0,
        None,
        None,
    )
    .await;

    fail_2_append_leaves_with_invalid_inputs(&mut context, &merkle_tree_pubkey).await;

    // We should always support appending 60 leaves at once.
    let leaves = (0u8..=60)
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
    functional_3_append_leaves_to_merkle_tree(&mut context, &merkle_tree_pubkey, &leaves).await;

    fail_4_append_leaves_with_invalid_authority(&mut context, &merkle_tree_pubkey).await;
}

async fn functional_1_initialize_state_merkle_tree_and_nullifier_queue<R: RpcConnection>(
    rpc: &mut R,
    payer_pubkey: &Pubkey,
    merkle_tree_keypair: &Keypair,
    queue_keypair: &Keypair,
    network_fee: u64,
    rollover_threshold: Option<u64>,
    close_threshold: Option<u64>,
) -> Pubkey {
    let merkle_tree_account_create_ix = create_account_instruction(
        &rpc.get_payer().pubkey(),
        StateMerkleTreeAccount::LEN,
        rpc.get_rent()
            .await
            .unwrap()
            .minimum_balance(account_compression::StateMerkleTreeAccount::LEN),
        &ID,
        Some(merkle_tree_keypair),
    );

    let size = NullifierQueueAccount::size(
        STATE_NULLIFIER_QUEUE_INDICES as usize,
        STATE_NULLIFIER_QUEUE_VALUES as usize,
    )
    .unwrap();
    let nullifier_queue_account_create_ix = create_account_instruction(
        payer_pubkey,
        size,
        rpc.get_rent().await.unwrap().minimum_balance(size),
        &ID,
        Some(queue_keypair),
    );
    let merkle_tree_pubkey = merkle_tree_keypair.pubkey();

    let state_merkle_tree_config = StateMerkleTreeConfig {
        rollover_threshold,
        close_threshold,
        network_fee: Some(network_fee),
        ..Default::default()
    };

    let instruction = create_initialize_merkle_tree_instruction(
        rpc.get_payer().pubkey(),
        merkle_tree_pubkey,
        queue_keypair.pubkey(),
        state_merkle_tree_config.clone(),
        NullifierQueueConfig::default(),
        None,
        1,
        0,
    );

    let latest_blockhash = rpc.get_latest_blockhash().await.unwrap();
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
    rpc.process_transaction(transaction.clone()).await.unwrap();
    assert_merkle_tree_initialized(
        rpc,
        &merkle_tree_pubkey,
        &queue_keypair.pubkey(),
        STATE_MERKLE_TREE_HEIGHT as usize,
        STATE_MERKLE_TREE_CHANGELOG as usize,
        STATE_MERKLE_TREE_ROOTS as usize,
        STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
        1,
        1,
        0,
        &Poseidon::zero_bytes()[0],
        rollover_threshold,
        close_threshold,
        network_fee,
        payer_pubkey,
    )
    .await;

    merkle_tree_keypair.pubkey()
}

pub async fn fail_2_append_leaves_with_invalid_inputs<R: RpcConnection>(
    context: &mut R,
    merkle_tree_pubkey: &Pubkey,
) {
    let instruction_data = account_compression::instruction::AppendLeavesToMerkleTrees {
        leaves: vec![(0, [1u8; 32]), (1, [2u8; 32])],
    };

    let accounts = account_compression::accounts::AppendLeaves {
        fee_payer: context.get_payer().pubkey(),
        authority: context.get_payer().pubkey(),
        registered_program_pda: None,
        log_wrapper: Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
        system_program: system_program::ID,
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

    let latest_blockhash = context.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&context.get_payer().pubkey()),
        &vec![&context.get_payer()],
        latest_blockhash,
    );
    let remaining_accounts_mismatch_error = context.process_transaction(transaction).await;
    assert!(remaining_accounts_mismatch_error.is_err());
}

pub async fn functional_3_append_leaves_to_merkle_tree<R: RpcConnection>(
    context: &mut R,
    merkle_tree_pubkey: &Pubkey,
    leaves: &Vec<(u8, [u8; 32])>,
) {
    let payer = context.get_payer().insecure_clone();
    let pre_account_mt = context
        .get_account(*merkle_tree_pubkey)
        .await
        .unwrap()
        .unwrap();
    let old_merkle_tree =
        AccountZeroCopy::<StateMerkleTreeAccount>::new(context, *merkle_tree_pubkey).await;
    let old_merkle_tree = old_merkle_tree.deserialized().copy_merkle_tree().unwrap();
    let instruction = [create_insert_leaves_instruction(
        leaves.clone(),
        context.get_payer().pubkey(),
        context.get_payer().pubkey(),
        vec![*merkle_tree_pubkey],
    )];

    context
        .create_and_send_transaction(&instruction, &payer.pubkey(), &[&payer, &payer])
        .await
        .unwrap();
    let post_account_mt = context
        .get_account(*merkle_tree_pubkey)
        .await
        .unwrap()
        .unwrap();
    let merkle_tree =
        AccountZeroCopy::<StateMerkleTreeAccount>::new(context, *merkle_tree_pubkey).await;
    let merkle_tree_deserialized = merkle_tree.deserialized();
    let roll_over_fee = (merkle_tree_deserialized
        .metadata
        .rollover_metadata
        .rollover_fee
        * (leaves.len() as u64))
        + merkle_tree_deserialized
            .metadata
            .rollover_metadata
            .network_fee;
    let merkle_tree = merkle_tree_deserialized.copy_merkle_tree().unwrap();
    assert_eq!(
        merkle_tree.next_index,
        old_merkle_tree.next_index + leaves.len()
    );

    let mut reference_merkle_tree = ConcurrentMerkleTree26::<Poseidon>::new(
        STATE_MERKLE_TREE_HEIGHT as usize,
        STATE_MERKLE_TREE_CHANGELOG as usize,
        STATE_MERKLE_TREE_ROOTS as usize,
        STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
    )
    .unwrap();
    reference_merkle_tree.init().unwrap();
    let leaves: Vec<&[u8; 32]> = leaves.iter().map(|leaf| &leaf.1).collect();
    reference_merkle_tree.append_batch(&leaves).unwrap();
    assert_eq!(merkle_tree.root(), reference_merkle_tree.root());
    assert_eq!(
        pre_account_mt.lamports + roll_over_fee,
        post_account_mt.lamports
    );
}

pub async fn fail_4_append_leaves_with_invalid_authority<R: RpcConnection>(
    rpc: &mut R,
    merkle_tree_pubkey: &Pubkey,
) {
    let invalid_autority = Keypair::new();
    airdrop_lamports(rpc, &invalid_autority.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let instruction_data = account_compression::instruction::AppendLeavesToMerkleTrees {
        leaves: vec![(0, [1u8; 32])],
    };

    let accounts = account_compression::accounts::AppendLeaves {
        fee_payer: rpc.get_payer().pubkey(),
        authority: invalid_autority.pubkey(),
        registered_program_pda: None,
        log_wrapper: Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
        system_program: system_program::ID,
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
    let latest_blockhash = rpc.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&invalid_autority.pubkey()),
        &vec![&rpc.get_payer(), &invalid_autority],
        latest_blockhash,
    );
    let remaining_accounts_mismatch_error = rpc.process_transaction(transaction).await;
    assert!(remaining_accounts_mismatch_error.is_err());
}

/// Tests:
/// 1. Functional: nullify leaf
/// 2. Failing: nullify leaf with invalid leaf index
/// 3. Failing: nullify leaf with invalid leaf queue index
/// 4. Failing: nullify leaf with invalid change log index
/// 5. Functional: nullify other leaf
/// 6. Failing: nullify leaf with nullifier queue that is not associated with the merkle tree
#[tokio::test]
async fn test_nullify_leaves() {
    let mut program_test = ProgramTest::default();
    program_test.add_program("account_compression", ID, None);
    program_test.add_program(
        "spl_noop",
        Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
        None,
    );
    let merkle_tree_keypair = Keypair::new();
    let merkle_tree_pubkey = merkle_tree_keypair.pubkey();
    let nullifier_queue_keypair = Keypair::new();
    let nullifier_queue_pubkey = nullifier_queue_keypair.pubkey();
    program_test.set_compute_max_units(1_400_000u64);
    let context = program_test.start_with_context().await;
    let mut context = ProgramTestRpcConnection { context };
    let payer = context.get_payer().insecure_clone();
    let payer_pubkey = context.get_payer().pubkey();
    let tip = 123;
    let rollover_threshold = Some(95);
    let close_threshold = Some(100);
    functional_1_initialize_state_merkle_tree_and_nullifier_queue(
        &mut context,
        &payer_pubkey,
        &merkle_tree_keypair,
        &nullifier_queue_keypair,
        tip,
        rollover_threshold,
        close_threshold,
    )
    .await;

    let other_merkle_tree_keypair = Keypair::new();
    let invalid_nullifier_queue_keypair = Keypair::new();
    let invalid_nullifier_queue_pubkey = nullifier_queue_keypair.pubkey();
    functional_1_initialize_state_merkle_tree_and_nullifier_queue(
        &mut context,
        &payer_pubkey,
        &other_merkle_tree_keypair,
        &invalid_nullifier_queue_keypair,
        tip,
        rollover_threshold,
        close_threshold,
    )
    .await;

    let elements = vec![(0, [1u8; 32]), (0, [2u8; 32])];

    functional_3_append_leaves_to_merkle_tree(&mut context, &merkle_tree_pubkey, &elements).await;

    insert_into_nullifier_queues(
        &elements.iter().map(|element| element.1).collect(),
        &payer,
        &payer,
        &nullifier_queue_pubkey,
        &merkle_tree_pubkey,
        &mut context,
    )
    .await
    .unwrap();

    let mut reference_merkle_tree = MerkleTree::<Poseidon>::new(
        STATE_MERKLE_TREE_HEIGHT as usize,
        STATE_MERKLE_TREE_CANOPY_DEPTH as usize,
    );
    reference_merkle_tree.append(&elements[0].1).unwrap();
    reference_merkle_tree.append(&elements[1].1).unwrap();

    let element_index = reference_merkle_tree
        .get_leaf_index(&elements[0].1)
        .unwrap() as u64;
    nullify(
        &mut context,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &mut reference_merkle_tree,
        &elements[0].1,
        2,
        0,
        element_index,
    )
    .await
    .unwrap();

    // nullify with invalid leaf index
    let invalid_element_index = 0;
    let valid_changelog_index = 3;
    let valid_leaf_queue_index = 1;
    nullify(
        &mut context,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &mut reference_merkle_tree,
        &elements[1].1,
        valid_changelog_index,
        valid_leaf_queue_index,
        invalid_element_index,
    )
    .await
    .unwrap_err();
    let valid_element_index = 1;
    let invalid_leaf_queue_index = 0;
    nullify(
        &mut context,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &mut reference_merkle_tree,
        &elements[1].1,
        valid_changelog_index,
        invalid_leaf_queue_index,
        valid_element_index,
    )
    .await
    .unwrap_err();
    nullify(
        &mut context,
        &merkle_tree_pubkey,
        &nullifier_queue_pubkey,
        &mut reference_merkle_tree,
        &elements[1].1,
        valid_changelog_index,
        valid_leaf_queue_index,
        valid_element_index,
    )
    .await
    .unwrap();

    nullify(
        &mut context,
        &merkle_tree_pubkey,
        &invalid_nullifier_queue_pubkey,
        &mut reference_merkle_tree,
        &elements[0].1,
        2,
        0,
        element_index,
    )
    .await
    .unwrap_err();
}

#[allow(clippy::too_many_arguments)]
pub async fn nullify<R: RpcConnection>(
    rpc: &mut R,
    merkle_tree_pubkey: &Pubkey,
    nullifier_queue_pubkey: &Pubkey,
    reference_merkle_tree: &mut MerkleTree<Poseidon>,
    element: &[u8; 32],
    change_log_index: u64,
    leaf_queue_index: u16,
    element_index: u64,
) -> Result<(), RpcError> {
    let payer = rpc.get_payer().insecure_clone();
    let proof: Vec<[u8; 32]> = reference_merkle_tree
        .get_proof_of_leaf(element_index as usize, false)
        .unwrap()
        .to_array::<16>()
        .unwrap()
        .to_vec();

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
            None,
        )
        .await?;

    let merkle_tree =
        AccountZeroCopy::<StateMerkleTreeAccount>::new(rpc, *merkle_tree_pubkey).await;
    reference_merkle_tree
        .update(&ZERO_BYTES[0], element_index as usize)
        .unwrap();
    assert_eq!(
        merkle_tree
            .deserialized()
            .copy_merkle_tree()
            .unwrap()
            .root(),
        reference_merkle_tree.root()
    );

    let account = rpc
        .get_account(*nullifier_queue_pubkey)
        .await
        .unwrap()
        .unwrap();
    let mut data = account.data.clone();

    let nullifier_queue =
        &mut unsafe { nullifier_queue_from_bytes_zero_copy_mut(&mut data).unwrap() };

    let array_element = nullifier_queue
        .by_value_index(
            leaf_queue_index.into(),
            Some(
                merkle_tree
                    .deserialized()
                    .copy_merkle_tree()
                    .unwrap()
                    .sequence_number,
            ),
        )
        .unwrap();
    assert_eq!(&array_element.value_bytes(), element);
    assert_eq!(
        array_element.sequence_number(),
        Some(
            merkle_tree
                .deserialized()
                .load_merkle_tree()
                .unwrap()
                .sequence_number
                + STATE_MERKLE_TREE_ROOTS as usize
        )
    );
    let event = event.as_ref().unwrap();
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

/// Tests:
/// 1. Functional: Initialize nullifier queue
/// 2. Functional: Insert into nullifier queue
/// 3. Failing: Insert the same elements into nullifier queue again (3 and 1 element(s))
/// 4. Failing: Insert into nullifier queue with invalid authority
/// 5. Functional: Insert one element into nullifier queue
#[tokio::test]
async fn test_init_and_insert_into_nullifier_queue() {
    let mut program_test = ProgramTest::default();
    program_test.add_program("account_compression", ID, None);
    program_test.add_program(
        "spl_noop",
        Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
        None,
    );
    let merkle_tree_keypair = Keypair::new();
    let merkle_tree_pubkey = merkle_tree_keypair.pubkey();
    let nullifier_queue_keypair = Keypair::new();
    let nullifier_queue_pubkey = nullifier_queue_keypair.pubkey();
    program_test.set_compute_max_units(1_400_000u64);
    let context = program_test.start_with_context().await;
    let mut rpc = ProgramTestRpcConnection { context };
    let payer_pubkey = rpc.get_payer().pubkey();
    let tip = 123;
    let rollover_threshold = Some(95);
    let close_threshold = Some(100);
    functional_1_initialize_state_merkle_tree_and_nullifier_queue(
        &mut rpc,
        &payer_pubkey,
        &merkle_tree_keypair,
        &nullifier_queue_keypair,
        tip,
        rollover_threshold,
        close_threshold,
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

    functional_5_test_insert_into_nullifier_queues(
        &mut rpc,
        &nullifier_queue_pubkey,
        &merkle_tree_pubkey,
    )
    .await;
}

async fn functional_2_test_insert_into_nullifier_queues<R: RpcConnection>(
    rpc: &mut R,
    nullifier_queue_pubkey: &Pubkey,
    merkle_tree_pubkey: &Pubkey,
) {
    let payer = rpc.get_payer().insecure_clone();
    let elements = vec![[1_u8; 32], [2_u8; 32]];
    insert_into_nullifier_queues(
        &elements,
        &payer,
        &payer,
        nullifier_queue_pubkey,
        merkle_tree_pubkey,
        rpc,
    )
    .await
    .unwrap();
    let array = unsafe {
        get_hash_set::<u16, NullifierQueueAccount, R>(rpc, *nullifier_queue_pubkey).await
    };
    let array_element_0 = array.by_value_index(0, None).unwrap();
    assert_eq!(array_element_0.value_bytes(), [1u8; 32]);
    assert_eq!(array_element_0.sequence_number(), None);
    let array_element_1 = array.by_value_index(1, None).unwrap();
    assert_eq!(array_element_1.value_bytes(), [2u8; 32]);
    assert_eq!(array_element_1.sequence_number(), None);
}

async fn fail_3_insert_same_elements_into_nullifier_queue<R: RpcConnection>(
    context: &mut R,
    nullifier_queue_pubkey: &Pubkey,
    merkle_tree_pubkey: &Pubkey,
    elements: Vec<[u8; 32]>,
) {
    let payer = context.get_payer().insecure_clone();

    insert_into_nullifier_queues(
        &elements,
        &payer,
        &payer,
        nullifier_queue_pubkey,
        merkle_tree_pubkey,
        context,
    )
    .await
    .unwrap_err();
}

async fn fail_4_insert_with_invalid_signer<R: RpcConnection>(
    rpc: &mut R,
    nullifier_queue_pubkey: &Pubkey,
    merkle_tree_pubkey: &Pubkey,
    elements: Vec<[u8; 32]>,
) {
    let invalid_signer = Keypair::new();
    airdrop_lamports(rpc, &invalid_signer.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    insert_into_nullifier_queues(
        &elements,
        &invalid_signer,
        &invalid_signer,
        nullifier_queue_pubkey,
        merkle_tree_pubkey,
        rpc,
    )
    .await
    .unwrap_err();
}

async fn functional_5_test_insert_into_nullifier_queues<R: RpcConnection>(
    rpc: &mut R,
    nullifier_queue_pubkey: &Pubkey,
    merkle_tree_pubkey: &Pubkey,
) {
    let payer = rpc.get_payer().insecure_clone();
    let element = 3_u32.to_biguint().unwrap();
    let elements = vec![bigint_to_be_bytes_array(&element).unwrap()];
    insert_into_nullifier_queues(
        &elements,
        &payer,
        &payer,
        nullifier_queue_pubkey,
        merkle_tree_pubkey,
        rpc,
    )
    .await
    .unwrap();
    let array = unsafe {
        get_hash_set::<u16, NullifierQueueAccount, R>(rpc, *nullifier_queue_pubkey).await
    };
    let array_element = array.by_value_index(2, None).unwrap();
    assert_eq!(array_element.value_biguint(), element);
    assert_eq!(array_element.sequence_number(), None);
}

async fn insert_into_nullifier_queues<R: RpcConnection>(
    elements: &Vec<[u8; 32]>,
    fee_payer: &Keypair,
    payer: &Keypair,
    nullifier_queue_pubkey: &Pubkey,
    merkle_tree_pubkey: &Pubkey,
    context: &mut R,
) -> Result<(), RpcError> {
    let instruction_data = account_compression::instruction::InsertIntoNullifierQueues {
        elements: elements.to_vec(),
        charge_network_fee: true,
    };
    let accounts = account_compression::accounts::InsertIntoNullifierQueues {
        fee_payer: fee_payer.pubkey(),
        authority: payer.pubkey(),
        registered_program_pda: None,
        system_program: system_program::ID,
    };
    let mut remaining_accounts = Vec::with_capacity(elements.len() * 2);
    remaining_accounts.extend(vec![
        AccountMeta::new(*nullifier_queue_pubkey, false);
        elements.len()
    ]);
    remaining_accounts.extend(vec![
        AccountMeta::new(*merkle_tree_pubkey, false);
        elements.len()
    ]);

    let instruction = Instruction {
        program_id: ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    };
    let latest_blockhash = context.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&fee_payer.pubkey()),
        &vec![fee_payer, payer],
        latest_blockhash,
    );
    context.process_transaction(transaction.clone()).await
}
