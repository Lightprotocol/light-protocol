use light_client::rpc::{client::RpcUrl, Rpc};
use light_program_test::{
    accounts::{
        initialize::initialize_accounts, test_accounts::TestAccounts, test_keypairs::TestKeypairs,
    },
    ProgramTestConfig, RpcError,
};

pub async fn setup_accounts(keypairs: TestKeypairs, url: RpcUrl) -> Result<TestAccounts, RpcError> {
    use light_client::rpc::LightClientConfig;
    use solana_sdk::commitment_config::CommitmentConfig;

    let mut rpc = light_client::rpc::LightClient::new(LightClientConfig {
        commitment_config: Some(CommitmentConfig::confirmed()),
        url: url.to_string(),
        photon_url: None,
        api_key: None,
        fetch_active_tree: false,
    })
    .await
    .unwrap();

    initialize_accounts(
        &mut rpc,
        &ProgramTestConfig::default_with_batched_trees(false),
        &keypairs,
    )
    .await
}
