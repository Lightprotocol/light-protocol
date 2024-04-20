use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::{Keypair, read_keypair_file};
use solana_sdk::signer::Signer;
use crank::nullifier::nullify_compressed_accounts;
use crank::constants::SERVER_URL;

#[test]
fn test_nullify_leaves() {
    let client = RpcClient::new(SERVER_URL);
    
    let indexed_array_keypair = Keypair::from_bytes(&INDEXED_ARRAY_TEST_KEYPAIR).unwrap();
    let indexed_array_pubkey = indexed_array_keypair.pubkey();

    let merkle_tree_keypair = Keypair::from_bytes(&MERKLE_TREE_TEST_KEYPAIR).unwrap();
    let merkle_tree_pubkey = merkle_tree_keypair.pubkey();

    let payer_keypair = read_keypair_file("keypair.json").unwrap();

    println!("Nullify compressed accounts for indexed array: {} and merkle tree: {}", indexed_array_pubkey, merkle_tree_pubkey);

   nullify_compressed_accounts(&indexed_array_pubkey, &merkle_tree_pubkey, &payer_keypair, &client);
}

// Hardcoded keypairs for deterministic pubkeys for testing
pub const MERKLE_TREE_TEST_KEYPAIR: [u8; 64] = [
    146, 193, 80, 51, 114, 21, 221, 27, 228, 203, 43, 26, 211, 158, 183, 129, 254, 206, 249, 89,
    121, 99, 123, 196, 106, 29, 91, 144, 50, 161, 42, 139, 68, 77, 125, 32, 76, 128, 61, 180, 1,
    207, 69, 44, 121, 118, 153, 17, 179, 183, 115, 34, 163, 127, 102, 214, 1, 87, 175, 177, 95, 49,
    65, 69,
];
pub const INDEXED_ARRAY_TEST_KEYPAIR: [u8; 64] = [
    222, 130, 14, 179, 120, 234, 200, 231, 112, 214, 179, 171, 214, 95, 225, 61, 71, 61, 96, 214,
    47, 253, 213, 178, 11, 77, 16, 2, 7, 24, 106, 218, 45, 107, 25, 100, 70, 71, 137, 47, 210, 248,
    220, 223, 11, 204, 205, 89, 248, 48, 211, 168, 11, 25, 219, 158, 99, 47, 127, 248, 142, 107,
    196, 110,
];
pub const PAYER_KEYPAIR: [u8; 64] = [
    17, 34, 231, 31, 83, 147, 93, 173, 61, 164, 25, 0, 204, 82, 234, 91, 202, 187, 228, 110, 146,
    97, 112, 131, 180, 164, 96, 220, 57, 207, 65, 107, 2, 99, 226, 251, 88, 66, 92, 33, 25, 216,
    211, 185, 112, 203, 212, 238, 105, 144, 72, 121, 176, 253, 106, 168, 115, 158, 154, 188, 62,
    255, 166, 81,
];
