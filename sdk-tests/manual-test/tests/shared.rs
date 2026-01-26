//! Shared test helpers for manual-test integration tests.

use light_client::interface::InitializeRentFreeConfig;
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest},
    ProgramTestConfig, Rpc,
};
use light_token::instruction::RENT_SPONSOR;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Setup test environment with Light Protocol and compression config.
/// Returns (rpc, payer, config_pda).
pub async fn setup_test_env() -> (LightProgramTest, Keypair, Pubkey) {
    let program_id = manual_test::ID;
    let mut config = ProgramTestConfig::new_v2(true, Some(vec![("manual_test", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    let (init_config_ix, config_pda) = InitializeRentFreeConfig::new(
        &program_id,
        &payer.pubkey(),
        &program_data_pda,
        RENT_SPONSOR,
        payer.pubkey(),
    )
    .build();

    rpc.create_and_send_transaction(&[init_config_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("Initialize config should succeed");

    (rpc, payer, config_pda)
}
