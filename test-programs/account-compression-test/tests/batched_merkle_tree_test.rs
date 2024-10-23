#![cfg(feature = "test-sbf")]

use account_compression::assert_mt_zero_copy_inited;
use account_compression::batched_merkle_tree::get_merkle_tree_account_size;
use account_compression::batched_queue::{
    assert_queue_zero_copy_inited, get_output_queue_account_size, BatchedQueueAccount,
};
use account_compression::{
    batched_merkle_tree::BatchedMerkleTreeAccount, InitStateTreeAccountsInstructionData, ID,
};
use anchor_lang::{InstructionData, ToAccountMetas};
use light_test_utils::{create_account_instruction, RpcConnection};
use light_test_utils::{rpc::ProgramTestRpcConnection, AccountZeroCopy};
use solana_program_test::ProgramTest;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::{
    instruction::Instruction,
    signature::{Keypair, Signer},
};

#[tokio::test]
async fn test_init_state_merkle_tree() {
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
    let payer = context.get_payer().insecure_clone();

    let params = InitStateTreeAccountsInstructionData::default();

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
    let queue_rent = context
        .get_minimum_balance_for_rent_exemption(queue_account_size)
        .await
        .unwrap();
    let create_queue_account_ix = create_account_instruction(
        &payer_pubkey,
        queue_account_size,
        queue_rent,
        &ID,
        Some(&nullifier_queue_keypair),
    );
    let mt_rent = context
        .get_minimum_balance_for_rent_exemption(mt_account_size)
        .await
        .unwrap();
    let create_mt_account_ix = create_account_instruction(
        &payer_pubkey,
        mt_account_size,
        mt_rent,
        &ID,
        Some(&merkle_tree_keypair),
    );

    let instruction = account_compression::instruction::InitializeBatchedStateMerkleTree { params };
    let accounts = account_compression::accounts::InitializeBatchedStateMerkleTreeAndQueue {
        authority: context.get_payer().pubkey(),
        merkle_tree: merkle_tree_pubkey,
        queue: nullifier_queue_pubkey,
        registered_program_pda: None,
    };

    let instruction = Instruction {
        program_id: ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction.data(),
    };
    context
        .create_and_send_transaction(
            &[create_queue_account_ix, create_mt_account_ix, instruction],
            &payer_pubkey,
            &[&payer, &nullifier_queue_keypair, &merkle_tree_keypair],
        )
        .await
        .unwrap();
    let mut merkle_tree =
        AccountZeroCopy::<BatchedMerkleTreeAccount>::new(&mut context, merkle_tree_pubkey).await;

    let mut queue =
        AccountZeroCopy::<BatchedQueueAccount>::new(&mut context, nullifier_queue_pubkey).await;
    let owner = context.get_payer().pubkey();

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
        nullifier_queue_pubkey,
    );
    println!(
        "merkle_tree.deserialized().clone() {:?}",
        merkle_tree.deserialized().clone()
    );
    println!(
        "queue.deserialized().clone() {:?}",
        queue.deserialized().clone()
    );
    assert_mt_zero_copy_inited(
        &mut merkle_tree.deserialized().clone(),
        &mut merkle_tree.account.data.as_mut_slice(),
        ref_mt_account,
        params.bloomfilter_num_iters,
    );

    let ref_output_queue_account = BatchedQueueAccount::get_output_queue_default(
        owner,
        None,
        None,
        params.rollover_threshold,
        0,
        params.output_queue_batch_size,
        params.output_queue_zkp_batch_size,
        params.additional_bytes,
        0, //merkle_tree_rent + additional_bytes_rent + queue_rent,
        merkle_tree_pubkey,
    );
    assert_queue_zero_copy_inited(
        &mut queue.deserialized().clone(),
        &mut queue.account.data.as_mut_slice(),
        ref_output_queue_account,
        0,
    );
}
