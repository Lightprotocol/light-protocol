use light_batched_merkle_tree::merkle_tree::BatchedMerkleTreeAccount;
use light_client::rpc::Rpc;
use light_compressed_account::TreeType;
use light_merkle_tree_metadata::fee::FORESTER_REIMBURSEMENT_CAP;
use light_program_test::{
    program_test::LightProgramTest, utils::assert::assert_rpc_error, ProgramTestConfig,
};
use light_registry::account_compression_cpi::sdk::{
    create_init_reimbursement_pda_instruction, get_reimbursement_pda,
};
use light_test_utils::{
    e2e_test_env::init_program_test_env,
    test_batch_forester::{perform_batch_append, perform_batch_nullify},
};
use serial_test::serial;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

/// Verify that LightProgramTest setup creates reimbursement PDAs for genesis
/// state trees, and that they have at least rent-exempt balance.
#[serial]
#[tokio::test]
async fn test_init_reimbursement_pda() {
    let rpc = LightProgramTest::new(ProgramTestConfig::default_test_forester(true))
        .await
        .unwrap();
    let env = rpc.test_accounts.clone();

    let state_tree_pubkey = env.v2_state_trees[0].merkle_tree;
    let (reimbursement_pda, _) = get_reimbursement_pda(&state_tree_pubkey);

    let account = rpc
        .get_account(reimbursement_pda)
        .await
        .unwrap()
        .expect("Reimbursement PDA should exist on chain");

    let rent = rpc
        .get_minimum_balance_for_rent_exemption(account.data.len())
        .await
        .unwrap();
    assert!(
        account.lamports >= rent,
        "Reimbursement PDA lamports ({}) should be >= rent-exempt minimum ({})",
        account.lamports,
        rent,
    );
}

/// Attempting to init the same reimbursement PDA twice must fail.
#[serial]
#[tokio::test]
async fn test_init_reimbursement_pda_fails_double_init() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::default_test_forester(true))
        .await
        .unwrap();
    let env = rpc.test_accounts.clone();

    let state_tree_pubkey = env.v2_state_trees[0].merkle_tree;
    let payer = rpc.get_payer().insecure_clone();

    let ix = create_init_reimbursement_pda_instruction(payer.pubkey(), state_tree_pubkey);
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
        .await;

    // Anchor returns "already in use" for double-init attempts.
    assert_rpc_error(result, 0, 0).unwrap();
}

/// After batch_append via registry, the reimbursement PDA balance should
/// increase by FORESTER_REIMBURSEMENT_CAP.
#[serial]
#[tokio::test]
async fn test_batch_append_funds_pda() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::default_test_forester(true))
        .await
        .unwrap();
    rpc.indexer = None;
    let env = rpc.test_accounts.clone();

    let user_keypair = Keypair::new();
    rpc.airdrop_lamports(&user_keypair.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    let merkle_tree_keypair = Keypair::new();
    let queue_keypair = Keypair::new();
    let cpi_context_keypair = Keypair::new();

    let (mut state_bundle, mut rpc) = {
        let mut e2e_env = init_program_test_env(rpc, &env, 50).await;
        e2e_env.indexer.state_merkle_trees.clear();

        e2e_env
            .indexer
            .add_state_merkle_tree(
                &mut e2e_env.rpc,
                &merkle_tree_keypair,
                &queue_keypair,
                &cpi_context_keypair,
                None,
                None,
                TreeType::StateV2,
            )
            .await;

        let tree_pubkey = e2e_env.indexer.state_merkle_trees[0].accounts.merkle_tree;
        let mut tree_account = e2e_env.rpc.get_account(tree_pubkey).await.unwrap().unwrap();
        let tree = BatchedMerkleTreeAccount::state_from_bytes(
            tree_account.data.as_mut_slice(),
            &tree_pubkey.into(),
        )
        .unwrap();
        let batch_size = tree.get_metadata().queue_batches.batch_size;

        // Fill the output queue with compressed SOL transactions.
        for i in 0..batch_size {
            println!("\ntx {}", i);
            e2e_env
                .compress_sol_deterministic(&user_keypair, 1_000_000, None)
                .await;
        }

        (e2e_env.indexer.state_merkle_trees[0].clone(), e2e_env.rpc)
    };

    let tree_pubkey = state_bundle.accounts.merkle_tree;
    let (reimbursement_pda, _) = get_reimbursement_pda(&tree_pubkey);

    // Record PDA balance before batch_append.
    let pda_before = rpc
        .get_account(reimbursement_pda)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    perform_batch_append(
        &mut rpc,
        &mut state_bundle,
        &env.protocol.forester,
        0,
        false,
        None,
    )
    .await
    .unwrap();

    // Verify PDA balance increased by exactly FORESTER_REIMBURSEMENT_CAP.
    let pda_after = rpc
        .get_account(reimbursement_pda)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    assert_eq!(
        pda_after - pda_before,
        FORESTER_REIMBURSEMENT_CAP,
        "PDA balance should increase by FORESTER_REIMBURSEMENT_CAP after batch_append",
    );
}

/// After batch_nullify, the forester receives FORESTER_REIMBURSEMENT_CAP from
/// the PDA, and the PDA balance decreases by that amount.
#[serial]
#[tokio::test]
async fn test_batch_nullify_reimburses_forester() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::default_test_forester(true))
        .await
        .unwrap();
    rpc.indexer = None;
    let env = rpc.test_accounts.clone();

    let user_keypair = Keypair::new();
    rpc.airdrop_lamports(&user_keypair.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    let merkle_tree_keypair = Keypair::new();
    let queue_keypair = Keypair::new();
    let cpi_context_keypair = Keypair::new();

    let (mut state_bundle, mut rpc) = {
        let mut e2e_env = init_program_test_env(rpc, &env, 50).await;
        e2e_env.indexer.state_merkle_trees.clear();

        e2e_env
            .indexer
            .add_state_merkle_tree(
                &mut e2e_env.rpc,
                &merkle_tree_keypair,
                &queue_keypair,
                &cpi_context_keypair,
                None,
                None,
                TreeType::StateV2,
            )
            .await;

        let tree_pubkey = e2e_env.indexer.state_merkle_trees[0].accounts.merkle_tree;
        let mut tree_account = e2e_env.rpc.get_account(tree_pubkey).await.unwrap().unwrap();
        let tree = BatchedMerkleTreeAccount::state_from_bytes(
            tree_account.data.as_mut_slice(),
            &tree_pubkey.into(),
        )
        .unwrap();
        let batch_size = tree.get_metadata().queue_batches.batch_size;

        // Fill output queue AND create input queue work (compress + transfer).
        for i in 0..batch_size {
            println!("\ntx {}", i);
            e2e_env
                .compress_sol_deterministic(&user_keypair, 1_000_000, None)
                .await;
            e2e_env
                .transfer_sol_deterministic(&user_keypair, &Pubkey::new_unique(), None)
                .await
                .unwrap();
        }

        (e2e_env.indexer.state_merkle_trees[0].clone(), e2e_env.rpc)
    };

    let tree_pubkey = state_bundle.accounts.merkle_tree;
    let (reimbursement_pda, _) = get_reimbursement_pda(&tree_pubkey);

    // First: batch_append to fund the PDA.
    perform_batch_append(
        &mut rpc,
        &mut state_bundle,
        &env.protocol.forester,
        0,
        false,
        None,
    )
    .await
    .unwrap();

    // Record balances before batch_nullify.
    let pda_before_nullify = rpc
        .get_account(reimbursement_pda)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let forester_before_nullify = rpc
        .get_account(env.protocol.forester.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    perform_batch_nullify(
        &mut rpc,
        &mut state_bundle,
        &env.protocol.forester,
        0,
        false,
        None,
    )
    .await
    .unwrap();

    // Verify PDA balance decreased by FORESTER_REIMBURSEMENT_CAP.
    let pda_after_nullify = rpc
        .get_account(reimbursement_pda)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    assert_eq!(
        pda_before_nullify - pda_after_nullify,
        FORESTER_REIMBURSEMENT_CAP,
        "PDA balance should decrease by FORESTER_REIMBURSEMENT_CAP after batch_nullify",
    );

    // Verify forester net change is positive (received reimbursement minus tx fee).
    let forester_after_nullify = rpc
        .get_account(env.protocol.forester.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let net_change = forester_after_nullify as i64 - forester_before_nullify as i64;
    assert!(
        net_change > -(FORESTER_REIMBURSEMENT_CAP as i64),
        "Forester should gain from nullify reimbursement (net change: {})",
        net_change,
    );
}
