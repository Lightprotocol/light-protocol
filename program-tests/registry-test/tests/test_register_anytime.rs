use forester_utils::forester_epoch::Epoch;
use light_program_test::{
    program_test::{LightProgramTest, TestRpc},
    ProgramTestConfig,
};
use light_registry::{
    protocol_config::state::{ProtocolConfig, ProtocolConfigPda},
    ForesterConfig, ForesterEpochPda,
};
use light_test_utils::{
    assert_epoch::{assert_epoch_pda, assert_registered_forester_pda},
    register_test_forester, Rpc,
};
use serial_test::serial;
use solana_sdk::{signature::Keypair, signer::Signer};

/// Verifies that foresters can register for an epoch outside the original
/// registration window (slots 0-99 with default config). Previously this
/// would fail with `NotInRegistrationPeriod`.
#[serial]
#[tokio::test]
async fn test_forester_register_outside_registration_phase() {
    let config = ProgramTestConfig {
        protocol_config: ProtocolConfig::default(),
        with_prover: false,
        with_forester: false,
        ..Default::default()
    };
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    rpc.indexer = None;
    let env = rpc.test_accounts.clone();

    let forester_keypair = Keypair::new();
    rpc.airdrop_lamports(&forester_keypair.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    // Register the base forester PDA.
    register_test_forester(
        &mut rpc,
        &env.protocol.governance_authority,
        &forester_keypair.pubkey(),
        ForesterConfig { fee: 1 },
    )
    .await
    .unwrap();

    let protocol_config = rpc
        .get_anchor_account::<ProtocolConfigPda>(&env.protocol.governance_authority_pda)
        .await
        .unwrap()
        .unwrap()
        .config;

    // Warp to slot 500 -- well past the old registration window (0-99).
    // With ProtocolConfig::default() the active phase starts at slot 100.
    rpc.warp_to_slot(500).unwrap();

    // Register for epoch 0 with explicit epoch to bypass client-side
    // auto-detection (which would skip if not in registration phase).
    let registered_epoch = Epoch::register(
        &mut rpc,
        &protocol_config,
        &forester_keypair,
        &forester_keypair.pubkey(),
        Some(0),
    )
    .await
    .unwrap();

    assert!(
        registered_epoch.is_some(),
        "Forester should be able to register outside the original registration window"
    );
    let registered_epoch = registered_epoch.unwrap();

    let forester_epoch_pda = rpc
        .get_anchor_account::<ForesterEpochPda>(&registered_epoch.forester_epoch_pda)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(forester_epoch_pda.epoch, 0);
    assert!(forester_epoch_pda.total_epoch_weight.is_none());

    assert_epoch_pda(&mut rpc, 0, 1).await;
    assert_registered_forester_pda(
        &mut rpc,
        &registered_epoch.forester_epoch_pda,
        &forester_keypair.pubkey(),
        0,
    )
    .await;
}
