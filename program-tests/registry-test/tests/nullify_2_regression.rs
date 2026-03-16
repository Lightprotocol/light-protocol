use account_compression::state::QueueAccount;
use anchor_lang::InstructionData;
use forester_utils::account_zero_copy::{get_concurrent_merkle_tree, get_hash_set};
use light_compressed_account::TreeType;
use light_hasher::Poseidon;
use light_program_test::{
    program_test::LightProgramTest, utils::assert::assert_rpc_error, ProgramTestConfig,
};
use light_registry::{
    account_compression_cpi::sdk::{
        create_nullify_2_instruction, create_nullify_instruction, CreateNullify2InstructionInputs,
        CreateNullifyInstructionInputs,
    },
    errors::RegistryError,
};
use light_test_utils::{e2e_test_env::init_program_test_env, Rpc};
use serial_test::serial;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

#[serial]
#[tokio::test]
async fn test_nullify_2_validation_and_success() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::default_with_batched_trees(true))
        .await
        .unwrap();
    rpc.indexer = None;
    let env = rpc.test_accounts.clone();
    let forester = Keypair::new();
    rpc.airdrop_lamports(&forester.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let merkle_tree_keypair = Keypair::new();
    let nullifier_queue_keypair = Keypair::new();
    let cpi_context_keypair = Keypair::new();
    let (mut rpc, state_tree_bundle) = {
        let mut e2e_env = init_program_test_env(rpc, &env, 50).await;
        e2e_env.indexer.state_merkle_trees.clear();
        e2e_env.keypair_action_config.fee_assert = false;
        e2e_env
            .indexer
            .add_state_merkle_tree(
                &mut e2e_env.rpc,
                &merkle_tree_keypair,
                &nullifier_queue_keypair,
                &cpi_context_keypair,
                None,
                Some(forester.pubkey()),
                TreeType::StateV1,
            )
            .await;
        e2e_env
            .compress_sol_deterministic(&forester, 1_000_000, None)
            .await;
        e2e_env
            .transfer_sol_deterministic(&forester, &Pubkey::new_unique(), None)
            .await
            .unwrap();
        (e2e_env.rpc, e2e_env.indexer.state_merkle_trees[0].clone())
    };

    let nullifier_queue = unsafe {
        get_hash_set::<QueueAccount, _>(&mut rpc, state_tree_bundle.accounts.nullifier_queue).await
    }
    .unwrap();
    let mut queue_index = None;
    let mut account_hash = None;
    for i in 0..nullifier_queue.get_capacity() {
        let bucket = nullifier_queue.get_bucket(i).unwrap();
        if let Some(bucket) = bucket {
            if bucket.sequence_number.is_none() {
                queue_index = Some(i as u16);
                account_hash = Some(bucket.value_bytes());
                break;
            }
        }
    }
    let queue_index = queue_index.unwrap();
    let account_hash = account_hash.unwrap();
    let leaf_index = state_tree_bundle
        .merkle_tree
        .get_leaf_index(&account_hash)
        .unwrap() as u64;
    let proof = state_tree_bundle
        .merkle_tree
        .get_proof_of_leaf(leaf_index as usize, false)
        .unwrap();
    let proof_depth = proof.len();
    let onchain_tree =
        get_concurrent_merkle_tree::<account_compression::StateMerkleTreeAccount, _, Poseidon, 26>(
            &mut rpc,
            state_tree_bundle.accounts.merkle_tree,
        )
        .await
        .unwrap();
    let change_log_index = onchain_tree.changelog_index() as u64;

    let valid_ix = create_nullify_2_instruction(
        CreateNullify2InstructionInputs {
            authority: forester.pubkey(),
            nullifier_queue: state_tree_bundle.accounts.nullifier_queue,
            merkle_tree: state_tree_bundle.accounts.merkle_tree,
            change_log_index,
            leaves_queue_index: queue_index,
            index: leaf_index,
            proof: proof.try_into().unwrap(),
            derivation: forester.pubkey(),
            is_metadata_forester: true,
        },
        0,
    );

    let mut empty_proof_accounts_ix = valid_ix.clone();
    empty_proof_accounts_ix
        .accounts
        .truncate(empty_proof_accounts_ix.accounts.len() - proof_depth);
    let result = rpc
        .create_and_send_transaction(&[empty_proof_accounts_ix], &forester.pubkey(), &[&forester])
        .await;
    assert_rpc_error(result, 0, RegistryError::InvalidProofAccountsLength.into()).unwrap();

    let malformed_ix = Instruction {
        program_id: light_registry::ID,
        accounts: valid_ix.accounts.clone(),
        data: light_registry::instruction::Nullify2 {
            bump: 255,
            change_log_indices: vec![change_log_index, change_log_index + 1],
            leaves_queue_indices: vec![queue_index],
            indices: vec![leaf_index],
        }
        .data(),
    };
    let result = rpc
        .create_and_send_transaction(&[malformed_ix], &forester.pubkey(), &[&forester])
        .await;
    assert_rpc_error(result, 0, RegistryError::InvalidNullify2Inputs.into()).unwrap();

    rpc.create_and_send_transaction(&[valid_ix], &forester.pubkey(), &[&forester])
        .await
        .unwrap();
}

#[serial]
#[tokio::test]
async fn test_legacy_nullify_still_succeeds() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::default_with_batched_trees(true))
        .await
        .unwrap();
    rpc.indexer = None;
    let env = rpc.test_accounts.clone();
    let forester = Keypair::new();
    rpc.airdrop_lamports(&forester.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let merkle_tree_keypair = Keypair::new();
    let nullifier_queue_keypair = Keypair::new();
    let cpi_context_keypair = Keypair::new();
    let (mut rpc, state_tree_bundle) = {
        let mut e2e_env = init_program_test_env(rpc, &env, 50).await;
        e2e_env.indexer.state_merkle_trees.clear();
        e2e_env.keypair_action_config.fee_assert = false;
        e2e_env
            .indexer
            .add_state_merkle_tree(
                &mut e2e_env.rpc,
                &merkle_tree_keypair,
                &nullifier_queue_keypair,
                &cpi_context_keypair,
                None,
                Some(forester.pubkey()),
                TreeType::StateV1,
            )
            .await;
        e2e_env
            .compress_sol_deterministic(&forester, 1_000_000, None)
            .await;
        e2e_env
            .transfer_sol_deterministic(&forester, &Pubkey::new_unique(), None)
            .await
            .unwrap();
        (e2e_env.rpc, e2e_env.indexer.state_merkle_trees[0].clone())
    };
    let nullifier_queue = unsafe {
        get_hash_set::<QueueAccount, _>(&mut rpc, state_tree_bundle.accounts.nullifier_queue).await
    }
    .unwrap();
    let mut queue_index = None;
    let mut account_hash = None;
    for i in 0..nullifier_queue.get_capacity() {
        let bucket = nullifier_queue.get_bucket(i).unwrap();
        if let Some(bucket) = bucket {
            if bucket.sequence_number.is_none() {
                queue_index = Some(i as u16);
                account_hash = Some(bucket.value_bytes());
                break;
            }
        }
    }
    let queue_index = queue_index.unwrap();
    let account_hash = account_hash.unwrap();
    let leaf_index = state_tree_bundle
        .merkle_tree
        .get_leaf_index(&account_hash)
        .unwrap() as u64;
    let proof = state_tree_bundle
        .merkle_tree
        .get_proof_of_leaf(leaf_index as usize, false)
        .unwrap();
    let onchain_tree =
        get_concurrent_merkle_tree::<account_compression::StateMerkleTreeAccount, _, Poseidon, 26>(
            &mut rpc,
            state_tree_bundle.accounts.merkle_tree,
        )
        .await
        .unwrap();
    let change_log_index = onchain_tree.changelog_index() as u64;

    let legacy_ix = create_nullify_instruction(
        CreateNullifyInstructionInputs {
            authority: forester.pubkey(),
            nullifier_queue: state_tree_bundle.accounts.nullifier_queue,
            merkle_tree: state_tree_bundle.accounts.merkle_tree,
            change_log_indices: vec![change_log_index],
            leaves_queue_indices: vec![queue_index],
            indices: vec![leaf_index],
            proofs: vec![proof],
            derivation: forester.pubkey(),
            is_metadata_forester: true,
        },
        0,
    );
    rpc.create_and_send_transaction(&[legacy_ix], &forester.pubkey(), &[&forester])
        .await
        .unwrap();
}
