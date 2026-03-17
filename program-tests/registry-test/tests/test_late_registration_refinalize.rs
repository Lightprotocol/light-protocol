use forester_utils::forester_epoch::Epoch;
use light_program_test::{
    program_test::{LightProgramTest, TestRpc},
    ProgramTestConfig,
};
use light_registry::{
    protocol_config::state::{ProtocolConfig, ProtocolConfigPda},
    sdk::create_finalize_registration_instruction,
    utils::get_epoch_pda_address,
    EpochPda, ForesterConfig, ForesterEpochPda,
};
use light_test_utils::{register_test_forester, Rpc};
use serial_test::serial;
use solana_sdk::{signature::Keypair, signer::Signer};

/// Test that a forester registering after finalization can be included
/// via re-finalization. Two foresters:
///   - Forester A registers early, finalizes at active phase start
///   - Forester B registers during active phase (late)
///   - Both re-finalize → total_epoch_weight reflects both
#[serial]
#[tokio::test]
async fn test_late_registration_refinalize() {
    let config = ProgramTestConfig {
        protocol_config: ProtocolConfig::default(),
        with_prover: false,
        with_forester: false,
        ..Default::default()
    };
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    rpc.indexer = None;
    let env = rpc.test_accounts.clone();

    let protocol_config = rpc
        .get_anchor_account::<ProtocolConfigPda>(&env.protocol.governance_authority_pda)
        .await
        .unwrap()
        .unwrap()
        .config;

    // --- Create and register forester A ---
    let forester_a = Keypair::new();
    rpc.airdrop_lamports(&forester_a.pubkey(), 10_000_000_000)
        .await
        .unwrap();
    register_test_forester(
        &mut rpc,
        &env.protocol.governance_authority,
        &forester_a.pubkey(),
        ForesterConfig { fee: 1 },
    )
    .await
    .unwrap();

    // Register forester A for epoch 0 (during registration phase).
    let epoch_a = Epoch::register(
        &mut rpc,
        &protocol_config,
        &forester_a,
        &forester_a.pubkey(),
        Some(0),
    )
    .await
    .unwrap()
    .expect("Forester A should register for epoch 0");

    // Verify epoch weight = 1 (only A).
    let epoch_pda_pubkey = get_epoch_pda_address(0);
    let epoch_pda: EpochPda = rpc
        .get_anchor_account(&epoch_pda_pubkey)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(epoch_pda.registered_weight, 1, "Only forester A registered");

    // --- Warp to active phase and finalize A ---
    rpc.warp_to_slot(protocol_config.registration_phase_length + protocol_config.genesis_slot)
        .unwrap();

    let ix_a =
        create_finalize_registration_instruction(&forester_a.pubkey(), &forester_a.pubkey(), 0);
    rpc.create_and_send_transaction(&[ix_a], &forester_a.pubkey(), &[&forester_a])
        .await
        .unwrap();

    // Verify A's total_epoch_weight = 1 (snapshot of registered_weight when only A was registered).
    let forester_a_pda: ForesterEpochPda = rpc
        .get_anchor_account(&epoch_a.forester_epoch_pda)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        forester_a_pda.total_epoch_weight,
        Some(1),
        "After first finalize, total_epoch_weight should be 1"
    );
    assert_eq!(forester_a_pda.finalize_counter, 1);

    // --- Register forester B late (during active phase) ---
    let forester_b = Keypair::new();
    rpc.airdrop_lamports(&forester_b.pubkey(), 10_000_000_000)
        .await
        .unwrap();
    register_test_forester(
        &mut rpc,
        &env.protocol.governance_authority,
        &forester_b.pubkey(),
        ForesterConfig { fee: 1 },
    )
    .await
    .unwrap();

    // Warp a few more slots into active phase.
    rpc.warp_to_slot(protocol_config.registration_phase_length + protocol_config.genesis_slot + 50)
        .unwrap();

    let epoch_b = Epoch::register(
        &mut rpc,
        &protocol_config,
        &forester_b,
        &forester_b.pubkey(),
        Some(0),
    )
    .await
    .unwrap()
    .expect("Forester B should register for epoch 0 during active phase");

    // Verify epoch weight = 2 (both A and B).
    let epoch_pda: EpochPda = rpc
        .get_anchor_account(&epoch_pda_pubkey)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(epoch_pda.registered_weight, 2, "Both foresters registered");

    // B has not finalized yet.
    let forester_b_pda: ForesterEpochPda = rpc
        .get_anchor_account(&epoch_b.forester_epoch_pda)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        forester_b_pda.total_epoch_weight, None,
        "Forester B has not finalized yet"
    );

    // --- Re-finalize both foresters ---
    // Forester A re-finalizes (updates snapshot to include B).
    let ix_a2 =
        create_finalize_registration_instruction(&forester_a.pubkey(), &forester_a.pubkey(), 0);
    rpc.create_and_send_transaction(&[ix_a2], &forester_a.pubkey(), &[&forester_a])
        .await
        .unwrap();

    let forester_a_pda: ForesterEpochPda = rpc
        .get_anchor_account(&epoch_a.forester_epoch_pda)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        forester_a_pda.total_epoch_weight,
        Some(2),
        "After re-finalize, A's total_epoch_weight should be 2"
    );
    assert_eq!(forester_a_pda.finalize_counter, 2, "A finalized twice");

    // Forester B finalizes for the first time.
    let ix_b =
        create_finalize_registration_instruction(&forester_b.pubkey(), &forester_b.pubkey(), 0);
    rpc.create_and_send_transaction(&[ix_b], &forester_b.pubkey(), &[&forester_b])
        .await
        .unwrap();

    let forester_b_pda: ForesterEpochPda = rpc
        .get_anchor_account(&epoch_b.forester_epoch_pda)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        forester_b_pda.total_epoch_weight,
        Some(2),
        "B's total_epoch_weight should also be 2"
    );
    assert_eq!(forester_b_pda.finalize_counter, 1);

    // Both foresters now agree on total_epoch_weight = 2.
    assert_eq!(
        forester_a_pda.total_epoch_weight, forester_b_pda.total_epoch_weight,
        "Both foresters should have the same total_epoch_weight"
    );

    // Verify both foresters have correct indices:
    // A registered first → index 0, B registered second → index 1.
    assert_eq!(forester_a_pda.forester_index, 0);
    assert_eq!(forester_b_pda.forester_index, 1);
}
