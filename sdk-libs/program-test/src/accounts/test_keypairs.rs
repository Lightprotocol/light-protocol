use solana_sdk::signature::Keypair;

#[derive(Debug)]
pub struct TestKeypairs {
    pub state_merkle_tree: Keypair,
    pub nullifier_queue: Keypair,
    pub governance_authority: Keypair,
    pub forester: Keypair,
    pub address_merkle_tree: Keypair,
    pub address_merkle_tree_queue: Keypair,
    pub cpi_context_account: Keypair,
    pub system_program: Keypair,
    pub registry_program: Keypair,
    pub batched_state_merkle_tree: Keypair,
    pub batched_output_queue: Keypair,
    pub batched_cpi_context: Keypair,
    pub batch_address_merkle_tree: Keypair,
    pub state_merkle_tree_2: Keypair,
    pub nullifier_queue_2: Keypair,
    pub cpi_context_2: Keypair,
    pub group_pda_seed: Keypair,
}

impl TestKeypairs {
    pub fn program_test_default() -> TestKeypairs {
        TestKeypairs {
            state_merkle_tree: Keypair::from_bytes(&MERKLE_TREE_TEST_KEYPAIR).unwrap(),
            nullifier_queue: Keypair::from_bytes(&NULLIFIER_QUEUE_TEST_KEYPAIR).unwrap(),
            governance_authority: Keypair::from_bytes(&PAYER_KEYPAIR).unwrap(),
            forester: Keypair::from_bytes(&FORESTER_TEST_KEYPAIR).unwrap(),
            address_merkle_tree: Keypair::from_bytes(&ADDRESS_MERKLE_TREE_TEST_KEYPAIR).unwrap(),
            address_merkle_tree_queue: Keypair::from_bytes(&ADDRESS_MERKLE_TREE_QUEUE_TEST_KEYPAIR)
                .unwrap(),
            cpi_context_account: Keypair::from_bytes(&SIGNATURE_CPI_TEST_KEYPAIR).unwrap(),
            system_program: Keypair::from_bytes(&OLD_SYSTEM_PROGRAM_ID_TEST_KEYPAIR).unwrap(),
            registry_program: Keypair::from_bytes(&OLD_REGISTRY_ID_TEST_KEYPAIR).unwrap(),
            batched_state_merkle_tree: Keypair::from_bytes(&BATCHED_STATE_MERKLE_TREE_TEST_KEYPAIR)
                .unwrap(),
            batched_output_queue: Keypair::from_bytes(&BATCHED_OUTPUT_QUEUE_TEST_KEYPAIR).unwrap(),
            batched_cpi_context: Keypair::from_bytes(&BATCHED_CPI_CONTEXT_TEST_KEYPAIR).unwrap(),
            batch_address_merkle_tree: Keypair::from_bytes(
                &BATCHED_ADDRESS_MERKLE_TREE_TEST_KEYPAIR,
            )
            .unwrap(),
            state_merkle_tree_2: Keypair::new(),
            nullifier_queue_2: Keypair::new(),
            cpi_context_2: Keypair::new(),
            group_pda_seed: Keypair::from_bytes(&GROUP_PDA_SEED_TEST_KEYPAIR).unwrap(),
        }
    }
}

// Hardcoded keypairs for deterministic pubkeys for testing
// TODO: derive keypairs from deterministic seed and salt
// so that we can init test env with dynamic numbers of deterministic keypairs
// (eg multiple trees, foresters)
pub const MERKLE_TREE_TEST_KEYPAIR: [u8; 64] = [
    146, 193, 80, 51, 114, 21, 221, 27, 228, 203, 43, 26, 211, 158, 183, 129, 254, 206, 249, 89,
    121, 99, 123, 196, 106, 29, 91, 144, 50, 161, 42, 139, 68, 77, 125, 32, 76, 128, 61, 180, 1,
    207, 69, 44, 121, 118, 153, 17, 179, 183, 115, 34, 163, 127, 102, 214, 1, 87, 175, 177, 95, 49,
    65, 69,
];
pub const NULLIFIER_QUEUE_TEST_KEYPAIR: [u8; 64] = [
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

pub const ADDRESS_MERKLE_TREE_TEST_KEYPAIR: [u8; 64] = [
    145, 184, 150, 187, 7, 48, 33, 191, 136, 115, 127, 243, 135, 119, 163, 99, 186, 21, 67, 161,
    22, 211, 102, 149, 158, 51, 182, 231, 97, 28, 77, 118, 165, 62, 148, 222, 135, 123, 222, 189,
    109, 46, 57, 112, 159, 209, 86, 59, 62, 139, 159, 208, 193, 206, 130, 48, 119, 195, 103, 235,
    231, 94, 83, 227,
];

pub const ADDRESS_MERKLE_TREE_QUEUE_TEST_KEYPAIR: [u8; 64] = [
    177, 80, 56, 144, 179, 178, 209, 143, 125, 134, 80, 75, 74, 156, 241, 156, 228, 50, 210, 35,
    149, 0, 28, 198, 132, 157, 54, 197, 173, 200, 104, 156, 243, 76, 173, 207, 166, 74, 210, 59,
    59, 211, 75, 180, 111, 40, 13, 151, 57, 237, 103, 145, 136, 105, 65, 143, 250, 50, 64, 94, 214,
    184, 217, 99,
];

pub const SIGNATURE_CPI_TEST_KEYPAIR: [u8; 64] = [
    189, 58, 29, 111, 77, 118, 218, 228, 64, 122, 227, 119, 148, 83, 245, 92, 107, 168, 153, 61,
    221, 100, 243, 106, 228, 231, 147, 200, 195, 156, 14, 10, 162, 100, 133, 197, 231, 125, 178,
    71, 33, 62, 223, 145, 136, 210, 160, 96, 75, 148, 143, 30, 41, 89, 205, 141, 248, 204, 48, 157,
    195, 216, 81, 204,
];

pub const GROUP_PDA_SEED_TEST_KEYPAIR: [u8; 64] = [
    97, 41, 77, 16, 152, 43, 140, 41, 11, 146, 82, 50, 38, 162, 216, 34, 95, 6, 237, 11, 74, 227,
    221, 137, 26, 136, 52, 144, 74, 212, 215, 155, 216, 47, 98, 199, 9, 61, 213, 72, 205, 237, 76,
    74, 119, 253, 96, 1, 140, 92, 149, 148, 250, 32, 53, 54, 186, 15, 48, 130, 222, 205, 3, 98,
];
// The test program id keypairs are necessary because the program id keypair needs to sign
// to register the program to the security group.
// The program ids should only be used for localnet testing.
// Pubkey: H5sFv8VwWmjxHYS2GB4fTDsK7uTtnRT4WiixtHrET3bN
pub const OLD_SYSTEM_PROGRAM_ID_TEST_KEYPAIR: [u8; 64] = [
    10, 62, 81, 156, 201, 11, 242, 85, 89, 182, 145, 223, 214, 144, 53, 147, 242, 197, 41, 55, 203,
    212, 70, 178, 225, 209, 4, 211, 43, 153, 222, 21, 238, 250, 35, 216, 163, 90, 82, 72, 167, 209,
    196, 227, 210, 173, 89, 255, 142, 20, 199, 150, 144, 215, 61, 164, 34, 47, 181, 228, 226, 153,
    208, 17,
];
// Pubkey: 7Z9Yuy3HkBCc2Wf3xzMGnz6qpV4n7ciwcoEMGKqhAnj1
pub const OLD_REGISTRY_ID_TEST_KEYPAIR: [u8; 64] = [
    43, 149, 192, 218, 153, 35, 206, 182, 230, 102, 193, 208, 163, 11, 195, 46, 228, 116, 113, 62,
    161, 102, 207, 139, 128, 8, 120, 150, 30, 119, 150, 140, 97, 98, 96, 14, 138, 90, 82, 76, 254,
    197, 232, 33, 204, 67, 237, 139, 100, 115, 187, 164, 115, 31, 164, 21, 246, 9, 162, 211, 227,
    20, 96, 192,
];

pub const FORESTER_TEST_KEYPAIR: [u8; 64] = [
    81, 4, 133, 152, 100, 67, 157, 52, 66, 70, 150, 214, 242, 90, 65, 199, 143, 192, 96, 172, 214,
    44, 250, 77, 224, 55, 104, 35, 168, 1, 92, 200, 204, 184, 194, 21, 117, 231, 90, 62, 117, 179,
    162, 181, 71, 36, 34, 47, 49, 195, 215, 90, 115, 3, 69, 74, 210, 75, 162, 191, 63, 51, 170,
    204,
];

// HLKs5NJ8FXkJg8BrzJt56adFYYuwg5etzDtBbQYTsixu
pub const BATCHED_STATE_MERKLE_TREE_TEST_KEYPAIR: [u8; 64] = [
    85, 82, 64, 221, 4, 69, 191, 4, 64, 56, 29, 32, 145, 68, 117, 157, 130, 83, 228, 58, 142, 48,
    130, 43, 101, 149, 140, 82, 123, 102, 108, 148, 242, 174, 90, 229, 244, 60, 225, 10, 207, 196,
    201, 136, 192, 35, 58, 9, 149, 215, 40, 149, 244, 9, 184, 209, 113, 234, 101, 91, 227, 243, 41,
    254,
];
// 6L7SzhYB3anwEQ9cphpJ1U7Scwj57bx2xueReg7R9cKU
pub const BATCHED_OUTPUT_QUEUE_TEST_KEYPAIR: [u8; 64] = [
    56, 183, 128, 249, 154, 184, 81, 219, 6, 98, 1, 79, 56, 253, 134, 198, 170, 16, 43, 112, 170,
    206, 203, 48, 49, 119, 115, 11, 192, 208, 67, 107, 79, 47, 194, 208, 90, 252, 43, 18, 216, 76,
    41, 113, 8, 161, 113, 18, 188, 202, 207, 115, 125, 235, 151, 110, 167, 166, 249, 78, 75, 221,
    38, 219,
];
// 7Hp52chxaew8bW1ApR4fck2bh6Y8qA1pu3qwH6N9zaLj
pub const BATCHED_CPI_CONTEXT_TEST_KEYPAIR: [u8; 64] = [
    152, 98, 187, 34, 35, 31, 202, 218, 11, 86, 181, 144, 29, 208, 167, 201, 77, 12, 104, 170, 95,
    53, 115, 33, 244, 179, 187, 255, 246, 100, 43, 203, 93, 116, 162, 215, 36, 226, 217, 56, 215,
    240, 198, 198, 253, 195, 107, 230, 122, 63, 116, 163, 105, 167, 18, 188, 161, 63, 146, 7, 238,
    3, 12, 228,
];

// EzKE84aVTkCUhDHLELqyJaq1Y7UVVmqxXqZjVHwHY3rK (just used to init but is reinited in light program test to amt2)
pub const BATCHED_ADDRESS_MERKLE_TREE_TEST_KEYPAIR: [u8; 64] = [
    39, 24, 219, 214, 174, 34, 141, 22, 238, 96, 128, 5, 244, 12, 239, 3, 45, 61, 42, 53, 92, 87,
    28, 24, 35, 87, 72, 11, 158, 224, 210, 70, 207, 214, 165, 6, 152, 46, 60, 129, 118, 32, 27,
    128, 68, 73, 71, 250, 6, 83, 176, 199, 153, 140, 237, 11, 55, 237, 3, 179, 242, 138, 37, 12,
];
