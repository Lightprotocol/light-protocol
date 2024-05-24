#![cfg(feature = "test-sbf")]

use solana_sdk::signature::{Keypair, Signer};

use light_test_utils::test_env::{
    register_program_with_registry_program, setup_test_programs_with_accounts,
};

#[tokio::test]
async fn test_e2e() {
    let (mut rpc, env) = setup_test_programs_with_accounts(None).await;
    let random_program_id = Keypair::new();
    register_program_with_registry_program(&mut rpc, &env, &random_program_id.pubkey())
        .await
        .unwrap();
}
