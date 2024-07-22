use forester::indexer::PhotonIndexer;
use light_bounded_vec::BoundedVec;
use light_concurrent_merkle_tree::hash::compute_root;
use light_hasher::Poseidon;
use light_test_utils::get_indexed_merkle_tree;
use light_test_utils::indexer::Indexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::SolanaRpcConnection;
use light_test_utils::test_env::get_test_env_accounts;
use log::{info, LevelFilter};
use rand::Rng;
use solana_sdk::pubkey::Pubkey;
use account_compression::{utils::constants::{ADDRESS_MERKLE_TREE_CHANGELOG, ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG}, StateMerkleTreeAccount};
use light_test_utils::get_concurrent_merkle_tree;

mod test_utils;
use test_utils::*;

async fn init() {
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(LevelFilter::Info.to_string()),
    )
    .is_test(true)
    .try_init();
}

// truncate to <254 bit
pub fn generate_pubkey_254() -> Pubkey {
    let mock_address: Pubkey = Pubkey::new_unique();
    let mut mock_address_less_than_254_bit: [u8; 32] = mock_address.to_bytes().try_into().unwrap();
    mock_address_less_than_254_bit[0] = 0;
    Pubkey::from(mock_address_less_than_254_bit)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn roots_test() {
    init().await;
    info!("Starting test_photon_onchain_roots");

    let env_accounts = get_test_env_accounts();
    let forester_config = forester_config();
    
    let rpc_url = "https://devnet.helius-rpc.com?api-key=<HELIUS_API_KEY>".to_string();
    let indexer_url = "https://devnet.helius-rpc.com".to_string();
    
    let indexer_rpc = SolanaRpcConnection::new(forester_config.external_services.rpc_url, None);
    let photon_indexer = PhotonIndexer::new(indexer_url.to_string(), indexer_rpc);

    let mut address = [0u8; 32];
    rand::thread_rng().fill(&mut address[..]);
    info!("Address: {:?}", address);

    let proof = photon_indexer.get_multiple_new_address_proofs(
        [0u8; 32],
        vec![address]
    ).await.unwrap();
    info!("Photon proof: {:?}", proof);

    let mut rpc = SolanaRpcConnection::new(rpc_url, None);

 
    // let merkle_tree =
    //     get_concurrent_merkle_tree::<StateMerkleTreeAccount, SolanaRpcConnection, Poseidon, usize, 26, 16>(
    //         &mut rpc,
    //         env_accounts.merkle_tree_pubkey
    //     )
    //     .await;

    let merkle_tree = get_concurrent_merkle_tree::<
        StateMerkleTreeAccount,
        SolanaRpcConnection,
        Poseidon,
        26,
    >(&mut rpc, env_accounts.merkle_tree_pubkey)
    .await;


    // merkle_tree.validate_proof(leaf, leaf_index, proof)

    let changelog_index = merkle_tree.changelog_index();
    info!("changelog_index: {:?}", changelog_index);
    info!("merkle_tree.next_index: {:?}", merkle_tree.next_index());
    info!("merkle_tree.sequence_number: {:?}", merkle_tree.sequence_number());
    info!("photon proof: {:?}", proof.first().unwrap().root);


    // let indexer_changelog = proof.first().unwrap().root_seq % ADDRESS_MERKLE_TREE_CHANGELOG;
    // let indexer_index_changelog = (proof.first().unwrap().root_seq - 1) % ADDRESS_MERKLE_TREE_INDEXED_CHANGELOG;

    // info!("photon changelog: {:?}", indexer_changelog);
    // info!("photon index_changelog: {:?}", indexer_index_changelog);

    for (i, root) in merkle_tree.roots.iter().enumerate() {
        info!("{} {:?}", i, root);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn verify_proof_test() {
    let proof_path =[
        [
          0, 0, 0, 0, 0, 0, 0, 0, 0,
          0, 0, 0, 0, 0, 0, 0, 0, 0,
          0, 0, 0, 0, 0, 0, 0, 0, 0,
          0, 0, 0, 0, 0
        ],
        [
           32, 152, 245, 251, 158,  35, 158,
          171,  60, 234, 195, 242, 123, 129,
          228, 129, 220,  49,  36, 213,  95,
          254, 213,  35, 168,  57, 238, 132,
           70, 182,  72, 100
        ],
        [
           33, 249,  58, 218, 202, 211, 186, 142,
           49,  87,  65,  23, 124,  15,   8,  45,
           95,   5, 210, 213,  99, 184, 103,  27,
          247, 117, 159,   8,  21, 215, 194,  93
        ],
        [
           29, 147, 208, 165, 141, 236, 182, 114,
          177,  10,  47,  87, 117,  48, 158, 129,
          181, 219,  11,   6, 150,  14, 217,  55,
           67, 195, 226,  66,  71, 254, 174,  64
        ],
        [
            7, 249, 216,  55, 203,  23, 176, 211,
           99,  32, 255, 233,  59, 165,  35,  69,
          241, 183,  40,  87,  26,  86, 130, 101,
          202, 172, 151,  85, 157, 188, 149,  42
        ],
        [
           43, 148, 207,  94, 135,  70, 179, 245,
          201,  99,  31,  76,  93, 243,  41,   7,
          166, 153, 197, 140, 148, 178, 173,  77,
          123,  92, 236,  22,  57,  24,  63,  85
        ],
        [
           27, 255,  99, 216, 244,  81, 125, 240,
           86,  23, 106, 220, 152,  82, 144,  37,
          228,  24, 114, 180,  40,  12, 214, 243,
          102,  16, 223,  94,  97, 115, 200,  76
        ],
        [
            7, 130, 149, 229, 162,  43, 132, 233,
          130, 207,  96,  30, 182,  57,  89, 123,
          139,   5,  21, 168, 140, 181, 172, 127,
          168, 164, 170, 190,  60, 135,  52, 157
        ],
        [
          47, 165, 229, 241, 143,  96,  39, 166,
          80,  27, 236, 134,  69, 100,  71,  42,
          97, 107,  46,  39,  74,  65,  33,  26,
          68,  76, 190,  58, 153, 243, 204,  97
        ],
        [
           14, 136,  67, 118, 208, 216, 253,  33,
          236, 183, 128,  56, 158, 148,  31, 102,
          228,  94, 122, 204, 227, 226,  40, 171,
           62,  33,  86, 166,  20, 252, 215,  71
        ],
        [
           27, 114,   1, 218, 114,  73,  79,  30,
           40, 113, 122, 209, 165,  46, 180, 105,
          249,  88, 146, 249,  87, 113,  53,  51,
          222,  97, 117, 229, 218,  25,  10, 242
        ],
        [
           31, 141, 136,  34, 114,  94,  54,  56,
           82,   0, 192, 178,   1,  36, 152,  25,
          166, 230, 225, 228, 101,   8,   8, 181,
          190, 188, 107, 250, 206, 125, 118,  54
        ],
        [
           44,  93, 130, 246, 108, 145,  75,
          175, 185, 112,  21, 137, 186, 140,
          252, 251,  97,  98, 176, 161,  42,
          207, 136, 168, 208, 135, 154,   4,
          113, 181, 248,  90
        ],
        [
           20, 197,  65,  72, 160, 148,  11, 184,
           32, 149, 127,  90, 223,  63, 161,  19,
           78, 245, 196, 170, 161,  19, 244, 100,
          100,  88, 242, 112, 224, 191, 191, 208
        ],
        [
          25,  13,  51, 177,  47, 152, 111, 150,
          30,  16, 192, 238,  68, 216, 185, 175,
          17, 190,  37,  88, 140, 173, 137, 212,
          22,  17, 142,  75, 244, 235, 232,  12
        ],
        [
           34, 249, 138, 169, 206, 112,  65,  82,
          172,  23,  53,  73,  20, 173, 115, 237,
           17, 103, 174, 101, 150, 175,  81,  10,
          165, 179, 100, 147,  37, 224, 108, 146
        ],
        [
           42, 124, 124, 155, 108, 229, 136,
           11, 159, 111,  34, 141, 114, 191,
          106,  87,  90,  82, 111,  41, 198,
          110, 204, 238, 248, 183,  83, 211,
          139, 186, 115,  35
        ],
        [
           46, 129, 134, 229,  88, 105, 142, 193,
          198, 122, 249, 193,  77,  70,  63, 252,
           71,   0,  67, 201, 194, 152, 139, 149,
           77, 117, 221, 100,  63,  54, 185, 146
        ],
        [
           15,  87, 197,  87,  30, 154,  78, 171,
           73, 226, 200, 207,   5,  13, 174, 148,
          138, 239, 110, 173, 100, 115, 146,  39,
           53,  70,  36, 157,  28,  31, 241,  15
        ],
        [
           24,  48, 238, 103, 181, 251, 85,  74,
          213, 246,  61,  67, 136, 128, 14,  28,
          254, 120, 227,  16, 105, 125, 70, 228,
           60, 156, 227,  97,  52, 247, 44, 202
        ],
        [
           33,  52, 231, 106, 197, 210,  26, 171,
           24, 108,  43, 225, 221, 143, 132, 238,
          136,  10,  30,  70, 234, 247,  18, 249,
          211, 113, 182, 223,  34,  25,  31,  62
        ],
        [
           25, 223, 144, 236, 132,  78, 188,
           79, 254, 235, 216, 102, 243,  56,
           89, 176, 192,  81, 216, 201,  88,
          238,  58, 168, 143, 143, 141, 243,
          219, 145, 165, 177
        ],
        [
           24, 204, 162, 166, 107,  92,   7, 135,
          152,  30, 105, 174, 253, 132, 133,  45,
          116, 175,  14, 147, 239,  73,  18, 180,
          100, 140,   5, 247,  34, 239, 229,  43
        ],
        [
           35, 136, 144, 148,  21,  35,  13,  27,
           77,  19,   4, 210, 213,  79,  71,  58,
           98, 131,  56, 242, 239, 173, 131, 250,
          223,   5, 100,  69,  73, 210,  83, 141
        ],
        [
           39,  23,  31, 180, 169, 123, 108, 192,
          233, 232, 245,  67, 181,  41,  77, 232,
          102, 162, 175,  44, 156, 141,  11,  29,
          150, 230, 115, 228,  82, 158, 213,  64
        ],
        [
           47, 246, 101,   5,  64, 246,  41, 253,
           87,  17, 160, 188, 116, 252,  13,  40,
          220, 178,  48, 185,  57,  37, 131, 229,
          248, 213, 150, 150, 221, 230, 174,  33
        ]
      ];

    let proof_vec = BoundedVec::<[u8; 32]>::from_slice(&proof_path);

    let leaf = [
      7, 144, 209, 225, 131,  17, 247,  82,
    137, 251,  78, 211, 121, 215,  66,  33,
    196, 227,  89, 141, 218, 115, 117,  46,
     91,  82, 166, 147, 236,  66, 164, 200
  ];

    let leaf_index = 76; 

    let computed_root = compute_root::<Poseidon>(&leaf, leaf_index, &proof_vec).unwrap();

    println!("Computed root: {:?}", computed_root);
    
    // let mut rpc = SolanaRpcConnection::new(rpc_url, None);
    // let env_accounts = get_test_env_accounts();

    // let merkle_tree = get_concurrent_merkle_tree::<
    //         StateMerkleTreeAccount,
    //         SolanaRpcConnection,
    //         Poseidon,
    //         26,
    //     >(&mut rpc, env_accounts.merkle_tree_pubkey)
    //     .await;

    // merkle_tree.validate_proof(leaf, leaf_index, proof)

}