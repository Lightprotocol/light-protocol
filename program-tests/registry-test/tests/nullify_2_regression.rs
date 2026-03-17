use account_compression::state::QueueAccount;
use forester_utils::account_zero_copy::{get_concurrent_merkle_tree, get_hash_set};
use light_compressed_account::TreeType;
use light_hasher::Poseidon;
use light_program_test::{
    indexer::state_tree::StateMerkleTreeBundle, program_test::LightProgramTest,
    utils::assert::assert_rpc_error, ProgramTestConfig,
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
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

/// Queue item data extracted from the nullifier queue.
struct QueueEntry {
    queue_index: u16,
    leaf_index: u64,
    proof: Vec<[u8; 32]>,
    change_log_index: u64,
}

/// Shared test environment: creates a state tree, compresses SOL, and transfers
/// to populate the nullifier queue.
async fn setup_tree_with_nullifier_queue_entries(
    num_entries: usize,
) -> (LightProgramTest, StateMerkleTreeBundle, Keypair) {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::default_with_batched_trees(true))
        .await
        .unwrap();
    rpc.indexer = None;
    let env = rpc.test_accounts.clone();
    let forester = Keypair::new();
    rpc.airdrop_lamports(&forester.pubkey(), 10_000_000_000)
        .await
        .unwrap();
    let merkle_tree_keypair = Keypair::new();
    let nullifier_queue_keypair = Keypair::new();
    let cpi_context_keypair = Keypair::new();

    let (rpc, state_tree_bundle) = {
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

        for _ in 0..num_entries {
            e2e_env
                .compress_sol_deterministic(&forester, 1_000_000, None)
                .await;
            e2e_env
                .transfer_sol_deterministic(&forester, &Pubkey::new_unique(), None)
                .await
                .unwrap();
        }

        (e2e_env.rpc, e2e_env.indexer.state_merkle_trees[0].clone())
    };

    (rpc, state_tree_bundle, forester)
}

/// Read pending queue entries from the nullifier queue.
async fn read_queue_entries(
    rpc: &mut LightProgramTest,
    state_tree_bundle: &StateMerkleTreeBundle,
    max_entries: usize,
) -> Vec<QueueEntry> {
    let nullifier_queue = unsafe {
        get_hash_set::<QueueAccount, _>(rpc, state_tree_bundle.accounts.nullifier_queue).await
    }
    .unwrap();

    let onchain_tree =
        get_concurrent_merkle_tree::<account_compression::StateMerkleTreeAccount, _, Poseidon, 26>(
            rpc,
            state_tree_bundle.accounts.merkle_tree,
        )
        .await
        .unwrap();
    let change_log_index = onchain_tree.changelog_index() as u64;

    let mut entries = Vec::new();
    for i in 0..nullifier_queue.get_capacity() {
        if entries.len() >= max_entries {
            break;
        }
        let bucket = nullifier_queue.get_bucket(i).unwrap();
        if let Some(bucket) = bucket {
            if bucket.sequence_number.is_none() {
                let account_hash = bucket.value_bytes();
                let leaf_index = state_tree_bundle
                    .merkle_tree
                    .get_leaf_index(&account_hash)
                    .unwrap() as u64;
                let proof = state_tree_bundle
                    .merkle_tree
                    .get_proof_of_leaf(leaf_index as usize, false)
                    .unwrap();

                entries.push(QueueEntry {
                    queue_index: i as u16,
                    leaf_index,
                    proof,
                    change_log_index,
                });
            }
        }
    }
    entries
}

#[serial]
#[tokio::test]
async fn test_nullify_2_validation_and_success() {
    let (mut rpc, state_tree_bundle, forester) = setup_tree_with_nullifier_queue_entries(1).await;
    let entries = read_queue_entries(&mut rpc, &state_tree_bundle, 1).await;
    let entry = &entries[0];

    let valid_ix = create_nullify_2_instruction(
        CreateNullify2InstructionInputs {
            authority: forester.pubkey(),
            nullifier_queue: state_tree_bundle.accounts.nullifier_queue,
            merkle_tree: state_tree_bundle.accounts.merkle_tree,
            change_log_index: entry.change_log_index,
            leaves_queue_index: entry.queue_index,
            index: entry.leaf_index,
            proof: entry.proof.clone().try_into().unwrap(),
            derivation: forester.pubkey(),
            is_metadata_forester: true,
        },
        0,
    );

    // Test: empty proof accounts → InvalidProofAccountsLength.
    let mut empty_proof_accounts_ix = valid_ix.clone();
    empty_proof_accounts_ix
        .accounts
        .truncate(empty_proof_accounts_ix.accounts.len() - entry.proof.len());
    let result = rpc
        .create_and_send_transaction(&[empty_proof_accounts_ix], &forester.pubkey(), &[&forester])
        .await;
    assert_rpc_error(result, 0, RegistryError::InvalidProofAccountsLength.into()).unwrap();

    // Test: success with valid instruction.
    rpc.create_and_send_transaction(&[valid_ix], &forester.pubkey(), &[&forester])
        .await
        .unwrap();
}

#[serial]
#[tokio::test]
async fn test_legacy_nullify_still_succeeds() {
    let (mut rpc, state_tree_bundle, forester) = setup_tree_with_nullifier_queue_entries(1).await;
    let entries = read_queue_entries(&mut rpc, &state_tree_bundle, 1).await;
    let entry = &entries[0];

    let legacy_ix = create_nullify_instruction(
        CreateNullifyInstructionInputs {
            authority: forester.pubkey(),
            nullifier_queue: state_tree_bundle.accounts.nullifier_queue,
            merkle_tree: state_tree_bundle.accounts.merkle_tree,
            change_log_indices: vec![entry.change_log_index],
            leaves_queue_indices: vec![entry.queue_index],
            indices: vec![entry.leaf_index],
            proofs: vec![entry.proof.clone()],
            derivation: forester.pubkey(),
            is_metadata_forester: true,
        },
        0,
    );
    rpc.create_and_send_transaction(&[legacy_ix], &forester.pubkey(), &[&forester])
        .await
        .unwrap();
}

#[serial]
#[tokio::test]
async fn test_paired_nullify_2_in_single_transaction() {
    let (mut rpc, state_tree_bundle, forester) = setup_tree_with_nullifier_queue_entries(2).await;
    let entries = read_queue_entries(&mut rpc, &state_tree_bundle, 2).await;
    assert!(
        entries.len() >= 2,
        "need at least 2 queue entries, got {}",
        entries.len()
    );

    let ix_0 = create_nullify_2_instruction(
        CreateNullify2InstructionInputs {
            authority: forester.pubkey(),
            nullifier_queue: state_tree_bundle.accounts.nullifier_queue,
            merkle_tree: state_tree_bundle.accounts.merkle_tree,
            change_log_index: entries[0].change_log_index,
            leaves_queue_index: entries[0].queue_index,
            index: entries[0].leaf_index,
            proof: entries[0].proof.clone().try_into().unwrap(),
            derivation: forester.pubkey(),
            is_metadata_forester: true,
        },
        0,
    );
    let ix_1 = create_nullify_2_instruction(
        CreateNullify2InstructionInputs {
            authority: forester.pubkey(),
            nullifier_queue: state_tree_bundle.accounts.nullifier_queue,
            merkle_tree: state_tree_bundle.accounts.merkle_tree,
            change_log_index: entries[1].change_log_index,
            leaves_queue_index: entries[1].queue_index,
            index: entries[1].leaf_index,
            proof: entries[1].proof.clone().try_into().unwrap(),
            derivation: forester.pubkey(),
            is_metadata_forester: true,
        },
        0,
    );

    // Both nullify_2 instructions in a single transaction (the core pairing use case).
    rpc.create_and_send_transaction(&[ix_0, ix_1], &forester.pubkey(), &[&forester])
        .await
        .unwrap();
}
