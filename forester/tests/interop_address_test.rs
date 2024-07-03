use forester::indexer::PhotonIndexer;
use light_hasher::Poseidon;
use light_test_utils::get_indexed_merkle_tree;
use light_test_utils::indexer::{Indexer};
use light_test_utils::rpc::SolanaRpcConnection;
use light_test_utils::test_env::get_test_env_accounts;
use log::{info, LevelFilter};
use rand::Rng;
use solana_sdk::pubkey::Pubkey;
use account_compression::AddressMerkleTreeAccount;
use account_compression::utils::constants::{ADDRESS_MERKLE_TREE_CHANGELOG, ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG};

async fn init() {
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(LevelFilter::Info.to_string()),
    )
    .is_test(true)
    .try_init();

    // let validator_config = LightValidatorConfig {
    //     enable_forester: true,
    //     enable_prover: true,
    //     enable_indexer: true,
    //     wait_time: 25,
    //     ..LightValidatorConfig::default()
    // };
    // spawn_validator(validator_config).await;
}

// truncate to <254 bit
pub fn generate_pubkey_254() -> Pubkey {
    let mock_address: Pubkey = Pubkey::new_unique();
    let mut mock_address_less_than_254_bit: [u8; 32] = mock_address.to_bytes().try_into().unwrap();
    mock_address_less_than_254_bit[0] = 0;
    Pubkey::from(mock_address_less_than_254_bit)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_photon_onchain_roots() {
    init().await;
    info!("Starting test_photon_onchain_roots");

    let env_accounts = get_test_env_accounts();

    let rpc_url = "https://zk-testnet.helius.dev:8899".to_string();
    let indexer_url = "https://zk-testnet.helius.dev:8784".to_string();

    let photon_indexer = PhotonIndexer::new(indexer_url.to_string());

    let mut address = [0u8; 32];
    rand::thread_rng().fill(&mut address[..]);
    info!("Address: {:?}", address);

    let proof = photon_indexer.get_multiple_new_address_proofs(
        [0u8; 32],
        address
    ).await.unwrap();
    info!("Photon proof: {:?}", proof);

    let mut rpc = SolanaRpcConnection::new(rpc_url, None);

    let merkle_tree = get_indexed_merkle_tree::<
        AddressMerkleTreeAccount,
        SolanaRpcConnection,
        Poseidon,
        usize,
        26,
    >(&mut rpc, env_accounts.address_merkle_tree_pubkey)
        .await;

    let changelog_index = merkle_tree.changelog_index();
    info!("changelog_index: {:?}", changelog_index);
    info!("merkle_tree.next_index: {:?}", merkle_tree.next_index());
    info!("merkle_tree.sequence_number: {:?}", merkle_tree.sequence_number());
    info!("photon proof: {:?}", proof.root);

    let indexer_changelog = proof.root_seq % ADDRESS_MERKLE_TREE_CHANGELOG;
    let indexer_index_changelog = (proof.root_seq - 1) % ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG;

    info!("photon changelog: {:?}", indexer_changelog);
    info!("photon index_changelog: {:?}", indexer_index_changelog);

    for (i, root) in merkle_tree.roots.iter().enumerate() {
        info!("{} {:?}", i, root);
    }
}