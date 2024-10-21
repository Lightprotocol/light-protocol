use crate::assert_address_merkle_tree::assert_address_merkle_tree_initialized;
use crate::assert_queue::assert_address_queue_initialized;
use crate::env_accounts;
use crate::rpc::test_rpc::ProgramTestRpcConnection;
use account_compression::sdk::create_initialize_address_merkle_tree_and_queue_instruction;
use account_compression::utils::constants::GROUP_AUTHORITY_SEED;
use account_compression::{
    sdk::create_initialize_merkle_tree_instruction, GroupAuthority, RegisteredProgram,
};
use account_compression::{AddressMerkleTreeConfig, AddressQueueConfig, QueueType};
use account_compression::{NullifierQueueConfig, StateMerkleTreeConfig};
use forester_utils::forester_epoch::{Epoch, TreeAccounts, TreeType};
use forester_utils::registry::register_test_forester;
use forester_utils::{airdrop_lamports, create_account_instruction};
use light_client::rpc::errors::RpcError;
use light_client::rpc::solana_rpc::SolanaRpcUrl;
use light_client::rpc::{RpcConnection, SolanaRpcConnection};
use light_hasher::Poseidon;
use light_macros::pubkey;
use light_registry::account_compression_cpi::sdk::get_registered_program_pda;
use light_registry::protocol_config::state::ProtocolConfig;
use light_registry::sdk::{
    create_deregister_program_instruction, create_finalize_registration_instruction,
    create_initialize_governance_authority_instruction,
    create_initialize_group_authority_instruction, create_register_program_instruction,
    create_update_protocol_config_instruction,
};
use light_registry::utils::{
    get_cpi_authority_pda, get_forester_pda, get_protocol_config_pda_address,
};
use light_registry::ForesterConfig;
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_sdk::signature::{read_keypair_file, Signature};
use solana_sdk::{
    pubkey::Pubkey, signature::Keypair, signature::Signer, system_instruction,
    transaction::Transaction,
};
use std::cmp;
use std::path::PathBuf;

pub const CPI_CONTEXT_ACCOUNT_RENT: u64 = 143487360; // lamports of the cpi context account
pub const NOOP_PROGRAM_ID: Pubkey = pubkey!("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV");

/// Setup test programs
/// deploys:
/// 1. light_registry program
/// 2. account_compression program
/// 3. light_compressed_token program
/// 4. light_system_program program
pub async fn setup_test_programs(
    additional_programs: Option<Vec<(String, Pubkey)>>,
) -> ProgramTestContext {
    let mut program_test = ProgramTest::default();
    let sbf_path = std::env::var("SBF_OUT_DIR").unwrap();
    // find path to bin where light cli stores program binaries.
    let path = find_light_bin().unwrap();
    std::env::set_var("SBF_OUT_DIR", path.to_str().unwrap());
    program_test.add_program("light_registry", light_registry::ID, None);
    program_test.add_program("account_compression", account_compression::ID, None);
    program_test.add_program("light_compressed_token", light_compressed_token::ID, None);
    program_test.add_program("light_system_program", light_system_program::ID, None);
    program_test.add_program("spl_noop", NOOP_PROGRAM_ID, None);
    std::env::set_var("SBF_OUT_DIR", sbf_path);
    let registered_program = env_accounts::get_registered_program_pda();
    program_test.add_account(
        get_registered_program_pda(&light_system_program::ID),
        registered_program,
    );
    let registered_program = env_accounts::get_registered_registry_program_pda();
    program_test.add_account(
        get_registered_program_pda(&light_registry::ID),
        registered_program,
    );
    if let Some(programs) = additional_programs {
        for (name, id) in programs {
            program_test.add_program(&name, id, None);
        }
    }
    program_test.set_compute_max_units(1_400_000u64);
    program_test.start_with_context().await
}

fn find_light_bin() -> Option<PathBuf> {
    // Run the 'which light' command to find the location of 'light' binary

    #[cfg(not(feature = "devenv"))]
    {
        use std::process::Command;
        let output = Command::new("which")
            .arg("light")
            .output()
            .expect("Failed to execute 'which light'");

        if !output.status.success() {
            return None;
        }
        // Convert the output into a string (removing any trailing newline)
        let light_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // Get the parent directory of the 'light' binary
        let mut light_bin_path = PathBuf::from(light_path);
        light_bin_path.pop(); // Remove the 'light' binary itself

        // Assuming the node_modules path starts from '/lib/node_modules/...'
        let node_modules_bin =
            light_bin_path.join("../lib/node_modules/@lightprotocol/zk-compression-cli/bin");

        Some(node_modules_bin.canonicalize().unwrap_or(node_modules_bin))
    }
    #[cfg(feature = "devenv")]
    {
        let light_protocol_toplevel = String::from_utf8_lossy(
            &std::process::Command::new("git")
                .arg("rev-parse")
                .arg("--show-toplevel")
                .output()
                .expect("Failed to get top-level directory")
                .stdout,
        )
        .trim()
        .to_string();
        let light_path = PathBuf::from(format!("{}/target/deploy/", light_protocol_toplevel));
        Some(light_path)
    }
}

#[derive(Debug)]
pub struct EnvAccounts {
    pub merkle_tree_pubkey: Pubkey,
    pub nullifier_queue_pubkey: Pubkey,
    pub governance_authority: Keypair,
    pub governance_authority_pda: Pubkey,
    pub group_pda: Pubkey,
    pub forester: Keypair,
    pub registered_program_pda: Pubkey,
    pub registered_registry_program_pda: Pubkey,
    pub address_merkle_tree_pubkey: Pubkey,
    pub address_merkle_tree_queue_pubkey: Pubkey,
    pub cpi_context_account_pubkey: Pubkey,
    pub registered_forester_pda: Pubkey,
    pub forester_epoch: Option<Epoch>,
}

impl EnvAccounts {
    pub fn get_local_test_validator_accounts() -> EnvAccounts {
        EnvAccounts {
            merkle_tree_pubkey: pubkey!("smt1NamzXdq4AMqS2fS2F1i5KTYPZRhoHgWx38d8WsT"),
            nullifier_queue_pubkey: pubkey!("nfq1NvQDJ2GEgnS8zt9prAe8rjjpAW1zFkrvZoBR148"),
            governance_authority: Keypair::from_bytes(&PAYER_KEYPAIR).unwrap(),
            governance_authority_pda: Pubkey::default(),
            group_pda: Pubkey::default(),
            forester: Keypair::new(),
            registered_program_pda: get_registered_program_pda(&light_system_program::ID),
            registered_registry_program_pda: get_registered_program_pda(&light_registry::ID),
            address_merkle_tree_pubkey: pubkey!("amt1Ayt45jfbdw5YSo7iz6WZxUmnZsQTYXy82hVwyC2"),
            address_merkle_tree_queue_pubkey: pubkey!(
                "aq1S9z4reTSQAdgWHGD2zDaS39sjGrAxbR31vxJ2F4F"
            ),
            cpi_context_account_pubkey: pubkey!("cpi1uHzrEhBG733DoEJNgHCyRS3XmmyVNZx5fonubE4"),
            registered_forester_pda: Pubkey::default(),
            forester_epoch: None, // Set to None or to an appropriate Epoch value if needed
        }
    }
}

#[derive(Debug)]
pub struct EnvAccountKeypairs {
    pub state_merkle_tree: Keypair,
    pub nullifier_queue: Keypair,
    pub governance_authority: Keypair,
    pub forester: Keypair,
    pub address_merkle_tree: Keypair,
    pub address_merkle_tree_queue: Keypair,
    pub cpi_context_account: Keypair,
    pub system_program: Keypair,
    pub registry_program: Keypair,
}

impl EnvAccountKeypairs {
    pub fn program_test_default() -> EnvAccountKeypairs {
        EnvAccountKeypairs {
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
        }
    }

    pub fn from_target_folder() -> EnvAccountKeypairs {
        let prefix = String::from("../../../light-keypairs/");
        let target_prefix = String::from("../../target/");
        let state_merkle_tree = read_keypair_file(format!(
            "{}smt1NamzXdq4AMqS2fS2F1i5KTYPZRhoHgWx38d8WsT.json",
            prefix
        ))
        .unwrap();
        let nullifier_queue = read_keypair_file(
            "../../../light-keypairs/nfq1NvQDJ2GEgnS8zt9prAe8rjjpAW1zFkrvZoBR148.json",
        )
        .unwrap();
        let governance_authority = read_keypair_file(format!(
            "{}governance-authority-keypair.json",
            target_prefix
        ))
        .unwrap();
        let forester =
            read_keypair_file(format!("{}forester-keypair.json", target_prefix)).unwrap();
        let address_merkle_tree = read_keypair_file(format!(
            "{}amt1Ayt45jfbdw5YSo7iz6WZxUmnZsQTYXy82hVwyC2.json",
            prefix
        ))
        .unwrap();
        let address_merkle_tree_queue = read_keypair_file(format!(
            "{}aq1S9z4reTSQAdgWHGD2zDaS39sjGrAxbR31vxJ2F4F.json",
            prefix
        ))
        .unwrap();
        let cpi_context_account = read_keypair_file(format!(
            "{}cpi1uHzrEhBG733DoEJNgHCyRS3XmmyVNZx5fonubE4.json",
            prefix
        ))
        .unwrap();
        let system_program = read_keypair_file(format!(
            "{}SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7.json",
            prefix
        ))
        .unwrap();
        let registry_program = read_keypair_file(format!(
            "{}Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX.json",
            prefix
        ))
        .unwrap();
        EnvAccountKeypairs {
            state_merkle_tree,
            nullifier_queue,
            governance_authority,
            forester,
            address_merkle_tree,
            address_merkle_tree_queue,
            cpi_context_account,
            system_program,
            registry_program,
        }
    }
}

// Hardcoded keypairs for deterministic pubkeys for testing
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

/// Setup test programs with accounts
/// deploys:
/// 1. light program
/// 2. account_compression program
/// 3. light_compressed_token program
/// 4. light_system_program program
///
/// Sets up the following accounts:
/// 5. creates and initializes governance authority
/// 6. creates and initializes group authority
/// 7. registers the light_system_program program with the group authority
/// 8. initializes Merkle tree owned by
/// Note:
/// - registers a forester
/// - advances to the active phase slot 2
/// - active phase doesn't end
// TODO(vadorovsky): Remove this function...
pub async fn setup_test_programs_with_accounts(
    additional_programs: Option<Vec<(String, Pubkey)>>,
) -> (ProgramTestRpcConnection, EnvAccounts) {
    setup_test_programs_with_accounts_with_protocol_config(
        additional_programs,
        ProtocolConfig {
            // Init with an active epoch which doesn't end
            active_phase_length: 1_000_000_000,
            slot_length: 1_000_000_000 - 1,
            genesis_slot: 0,
            registration_phase_length: 2,
            ..Default::default()
        },
        true,
    )
    .await
}

/// Setup test programs with accounts
/// deploys:
/// 1. light program
/// 2. account_compression program
/// 3. light_compressed_token program
/// 4. light_system_program program
///
/// Sets up the following accounts:
/// 5. creates and initializes governance authority
/// 6. creates and initializes group authority
/// 7. registers the light_system_program program with the group authority
/// 8. initializes Merkle tree owned by
/// Note:
/// - registers a forester
/// - advances to the active phase slot 2
/// - active phase doesn't end
// TODO(vadorovsky): ...in favor of this one.
pub async fn setup_test_programs_with_accounts_v2(
    additional_programs: Option<Vec<(String, Pubkey)>>,
) -> (
    light_client::rpc::test_rpc::ProgramTestRpcConnection,
    EnvAccounts,
) {
    setup_test_programs_with_accounts_with_protocol_config_v2(
        additional_programs,
        ProtocolConfig {
            // Init with an active epoch which doesn't end
            active_phase_length: 1_000_000_000,
            slot_length: 1_000_000_000 - 1,
            genesis_slot: 0,
            registration_phase_length: 2,
            ..Default::default()
        },
        true,
    )
    .await
}

// TODO(vadorovsky): Remote this function...
pub async fn setup_test_programs_with_accounts_with_protocol_config(
    additional_programs: Option<Vec<(String, Pubkey)>>,
    protocol_config: ProtocolConfig,
    register_forester_and_advance_to_active_phase: bool,
) -> (ProgramTestRpcConnection, EnvAccounts) {
    let context = setup_test_programs(additional_programs).await;
    let mut context = ProgramTestRpcConnection { context };
    let keypairs = EnvAccountKeypairs::program_test_default();
    airdrop_lamports(
        &mut context,
        &keypairs.governance_authority.pubkey(),
        100_000_000_000,
    )
    .await
    .unwrap();
    airdrop_lamports(&mut context, &keypairs.forester.pubkey(), 10_000_000_000)
        .await
        .unwrap();
    let env_accounts = initialize_accounts(
        &mut context,
        keypairs,
        protocol_config,
        register_forester_and_advance_to_active_phase,
        true,
    )
    .await;
    (context, env_accounts)
}

// TODO(vadorovsky): ...in favor of this one.
pub async fn setup_test_programs_with_accounts_with_protocol_config_v2(
    additional_programs: Option<Vec<(String, Pubkey)>>,
    protocol_config: ProtocolConfig,
    register_forester_and_advance_to_active_phase: bool,
) -> (
    light_client::rpc::test_rpc::ProgramTestRpcConnection,
    EnvAccounts,
) {
    let context = setup_test_programs(additional_programs).await;
    let mut context = light_client::rpc::test_rpc::ProgramTestRpcConnection { context };
    let keypairs = EnvAccountKeypairs::program_test_default();
    airdrop_lamports(
        &mut context,
        &keypairs.governance_authority.pubkey(),
        100_000_000_000,
    )
    .await
    .unwrap();
    airdrop_lamports(&mut context, &keypairs.forester.pubkey(), 10_000_000_000)
        .await
        .unwrap();
    let env_accounts = initialize_accounts(
        &mut context,
        keypairs,
        protocol_config,
        register_forester_and_advance_to_active_phase,
        true,
    )
    .await;
    (context, env_accounts)
}

pub async fn setup_accounts(keypairs: EnvAccountKeypairs, url: SolanaRpcUrl) -> EnvAccounts {
    let mut rpc = SolanaRpcConnection::new(url, None);

    initialize_accounts(&mut rpc, keypairs, ProtocolConfig::default(), false, false).await
}

pub async fn initialize_accounts<R: RpcConnection>(
    context: &mut R,
    keypairs: EnvAccountKeypairs,
    protocol_config: ProtocolConfig,
    register_forester_and_advance_to_active_phase: bool,
    skip_register_programs: bool,
) -> EnvAccounts {
    let cpi_authority_pda = get_cpi_authority_pda();
    let protocol_config_pda = get_protocol_config_pda_address();
    let instruction = create_initialize_governance_authority_instruction(
        keypairs.governance_authority.pubkey(),
        keypairs.governance_authority.pubkey(),
        protocol_config,
    );
    let update_instruction = create_update_protocol_config_instruction(
        keypairs.governance_authority.pubkey(),
        Some(keypairs.governance_authority.pubkey()),
        None,
    );
    context
        .create_and_send_transaction(
            &[instruction, update_instruction],
            &keypairs.governance_authority.pubkey(),
            &[&keypairs.governance_authority],
        )
        .await
        .unwrap();

    let group_seed_keypair = Keypair::from_bytes(&GROUP_PDA_SEED_TEST_KEYPAIR).unwrap();
    let group_pda = initialize_new_group(
        &group_seed_keypair,
        &keypairs.governance_authority,
        context,
        cpi_authority_pda.0,
    )
    .await;

    let gov_authority = context
        .get_anchor_account::<GroupAuthority>(&protocol_config_pda.0)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        gov_authority.authority,
        keypairs.governance_authority.pubkey()
    );

    println!("forester: {:?}", keypairs.forester.pubkey());
    register_test_forester(
        context,
        &keypairs.governance_authority,
        &keypairs.forester.pubkey(),
        ForesterConfig::default(),
    )
    .await
    .unwrap();
    println!("Registered register_test_forester ");

    if !skip_register_programs {
        register_program_with_registry_program(
            context,
            &keypairs.governance_authority,
            &group_pda,
            &keypairs.system_program,
        )
        .await
        .unwrap();
        register_program_with_registry_program(
            context,
            &keypairs.governance_authority,
            &group_pda,
            &keypairs.registry_program,
        )
        .await
        .unwrap();
    }
    println!("Registered system program");
    let merkle_tree_pubkey = keypairs.state_merkle_tree.pubkey();
    let nullifier_queue_pubkey = keypairs.nullifier_queue.pubkey();
    create_state_merkle_tree_and_queue_account(
        &keypairs.governance_authority,
        true,
        context,
        &keypairs.state_merkle_tree,
        &keypairs.nullifier_queue,
        Some(&keypairs.cpi_context_account),
        None,
        None,
        1,
        &StateMerkleTreeConfig::default(),
        &NullifierQueueConfig::default(),
    )
    .await
    .unwrap();

    create_address_merkle_tree_and_queue_account(
        &keypairs.governance_authority,
        true,
        context,
        &keypairs.address_merkle_tree,
        &keypairs.address_merkle_tree_queue,
        None,
        None,
        &AddressMerkleTreeConfig::default(),
        &AddressQueueConfig::default(),
        0,
    )
    .await
    .unwrap();

    let registered_system_program_pda = get_registered_program_pda(&light_system_program::ID);
    let registered_registry_program_pda = get_registered_program_pda(&light_registry::ID);
    let forester_epoch = if register_forester_and_advance_to_active_phase {
        let mut registered_epoch = Epoch::register(
            context,
            &protocol_config,
            &keypairs.forester,
            &keypairs.forester.pubkey(),
        )
        .await
        .unwrap()
        .unwrap();
        context
            .warp_to_slot(registered_epoch.phases.active.start)
            .await
            .unwrap();
        let tree_accounts = vec![
            TreeAccounts {
                tree_type: TreeType::State,
                merkle_tree: merkle_tree_pubkey,
                queue: nullifier_queue_pubkey,
                is_rolledover: false,
            },
            TreeAccounts {
                tree_type: TreeType::Address,
                merkle_tree: keypairs.address_merkle_tree.pubkey(),
                queue: keypairs.address_merkle_tree_queue.pubkey(),
                is_rolledover: false,
            },
        ];

        registered_epoch
            .fetch_account_and_add_trees_with_schedule(context, &tree_accounts)
            .await
            .unwrap();
        let ix = create_finalize_registration_instruction(
            &keypairs.forester.pubkey(),
            &keypairs.forester.pubkey(),
            0,
        );
        context
            .create_and_send_transaction(&[ix], &keypairs.forester.pubkey(), &[&keypairs.forester])
            .await
            .unwrap();
        Some(registered_epoch)
    } else {
        None
    };
    EnvAccounts {
        merkle_tree_pubkey,
        nullifier_queue_pubkey,
        group_pda,
        governance_authority: keypairs.governance_authority.insecure_clone(),
        governance_authority_pda: protocol_config_pda.0,
        forester: keypairs.forester.insecure_clone(),
        registered_program_pda: registered_system_program_pda,
        address_merkle_tree_pubkey: keypairs.address_merkle_tree.pubkey(),
        address_merkle_tree_queue_pubkey: keypairs.address_merkle_tree_queue.pubkey(),
        cpi_context_account_pubkey: keypairs.cpi_context_account.pubkey(),
        registered_registry_program_pda,
        registered_forester_pda: get_forester_pda(&keypairs.forester.pubkey()).0,
        forester_epoch,
    }
}
pub fn get_group_pda(seed: Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[GROUP_AUTHORITY_SEED, seed.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0
}

pub async fn initialize_new_group<R: RpcConnection>(
    group_seed_keypair: &Keypair,
    payer: &Keypair,
    context: &mut R,
    authority: Pubkey,
) -> Pubkey {
    let group_pda = Pubkey::find_program_address(
        &[
            GROUP_AUTHORITY_SEED,
            group_seed_keypair.pubkey().to_bytes().as_slice(),
        ],
        &account_compression::ID,
    )
    .0;

    let instruction = create_initialize_group_authority_instruction(
        payer.pubkey(),
        group_pda,
        group_seed_keypair.pubkey(),
        authority,
    );

    context
        .create_and_send_transaction(
            &[instruction],
            &payer.pubkey(),
            &[payer, group_seed_keypair],
        )
        .await
        .unwrap();
    let group_authority = context
        .get_anchor_account::<GroupAuthority>(&group_pda)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(group_authority.authority, authority);
    assert_eq!(group_authority.seed, group_seed_keypair.pubkey());
    group_pda
}

pub fn get_test_env_accounts() -> EnvAccounts {
    let merkle_tree_keypair = Keypair::from_bytes(&MERKLE_TREE_TEST_KEYPAIR).unwrap();
    let merkle_tree_pubkey = merkle_tree_keypair.pubkey();
    let nullifier_queue_keypair = Keypair::from_bytes(&NULLIFIER_QUEUE_TEST_KEYPAIR).unwrap();
    let nullifier_queue_pubkey = nullifier_queue_keypair.pubkey();
    let group_seed_keypair = Keypair::from_bytes(&GROUP_PDA_SEED_TEST_KEYPAIR).unwrap();
    let group_pda = get_group_pda(group_seed_keypair.pubkey());

    let payer = Keypair::from_bytes(&PAYER_KEYPAIR).unwrap();
    let protocol_config_pda = get_protocol_config_pda_address();
    let (_, registered_program_pda) = create_register_program_instruction(
        payer.pubkey(),
        protocol_config_pda,
        group_pda,
        light_system_program::ID,
    );

    let address_merkle_tree_keypair =
        Keypair::from_bytes(&ADDRESS_MERKLE_TREE_TEST_KEYPAIR).unwrap();

    let address_merkle_tree_queue_keypair =
        Keypair::from_bytes(&ADDRESS_MERKLE_TREE_QUEUE_TEST_KEYPAIR).unwrap();

    let cpi_context_keypair = Keypair::from_bytes(&SIGNATURE_CPI_TEST_KEYPAIR).unwrap();
    let registered_registry_program_pda = get_registered_program_pda(&light_registry::ID);
    let forester = Keypair::from_bytes(&FORESTER_TEST_KEYPAIR).unwrap();
    EnvAccounts {
        merkle_tree_pubkey,
        nullifier_queue_pubkey,
        group_pda,
        governance_authority: payer,
        governance_authority_pda: protocol_config_pda.0,
        registered_forester_pda: get_forester_pda(&forester.pubkey()).0,
        forester,
        registered_program_pda,
        address_merkle_tree_pubkey: address_merkle_tree_keypair.pubkey(),
        address_merkle_tree_queue_pubkey: address_merkle_tree_queue_keypair.pubkey(),
        cpi_context_account_pubkey: cpi_context_keypair.pubkey(),
        registered_registry_program_pda,
        forester_epoch: None,
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn create_state_merkle_tree_and_queue_account<R: RpcConnection>(
    payer: &Keypair,
    registry: bool,
    rpc: &mut R,
    merkle_tree_keypair: &Keypair,
    nullifier_queue_keypair: &Keypair,
    cpi_context_keypair: Option<&Keypair>,
    program_owner: Option<Pubkey>,
    forester: Option<Pubkey>,
    index: u64,
    merkle_tree_config: &StateMerkleTreeConfig,
    queue_config: &NullifierQueueConfig,
) -> Result<Signature, RpcError> {
    use light_registry::account_compression_cpi::sdk::create_initialize_merkle_tree_instruction as create_initialize_merkle_tree_instruction_registry;
    let size = account_compression::state::StateMerkleTreeAccount::size(
        merkle_tree_config.height as usize,
        merkle_tree_config.changelog_size as usize,
        merkle_tree_config.roots_size as usize,
        merkle_tree_config.canopy_depth as usize,
    );

    let merkle_tree_account_create_ix = create_account_instruction(
        &payer.pubkey(),
        size,
        rpc.get_minimum_balance_for_rent_exemption(size)
            .await
            .unwrap(),
        &account_compression::ID,
        Some(merkle_tree_keypair),
    );
    let size =
        account_compression::state::queue::QueueAccount::size(queue_config.capacity as usize)
            .unwrap();
    let nullifier_queue_account_create_ix = create_account_instruction(
        &payer.pubkey(),
        size,
        rpc.get_minimum_balance_for_rent_exemption(size)
            .await
            .unwrap(),
        &account_compression::ID,
        Some(nullifier_queue_keypair),
    );

    let transaction = if registry {
        let cpi_context_keypair = cpi_context_keypair.unwrap();
        let rent_cpi_config = rpc
            .get_minimum_balance_for_rent_exemption(
                ProtocolConfig::default().cpi_context_size as usize,
            )
            .await
            .unwrap();
        let create_cpi_context_instruction = create_account_instruction(
            &payer.pubkey(),
            ProtocolConfig::default().cpi_context_size as usize,
            rent_cpi_config,
            &light_system_program::ID,
            Some(cpi_context_keypair),
        );

        let instruction = create_initialize_merkle_tree_instruction_registry(
            payer.pubkey(),
            merkle_tree_keypair.pubkey(),
            nullifier_queue_keypair.pubkey(),
            cpi_context_keypair.pubkey(),
            merkle_tree_config.clone(),
            queue_config.clone(),
            program_owner,
            forester,
        );
        Transaction::new_signed_with_payer(
            &[
                create_cpi_context_instruction,
                merkle_tree_account_create_ix,
                nullifier_queue_account_create_ix,
                instruction,
            ],
            Some(&payer.pubkey()),
            &vec![
                payer,
                merkle_tree_keypair,
                nullifier_queue_keypair,
                cpi_context_keypair,
            ],
            rpc.get_latest_blockhash().await.unwrap(),
        )
    } else {
        let instruction = create_initialize_merkle_tree_instruction(
            payer.pubkey(),
            None,
            merkle_tree_keypair.pubkey(),
            nullifier_queue_keypair.pubkey(),
            merkle_tree_config.clone(),
            queue_config.clone(),
            program_owner,
            forester,
            index,
        );
        Transaction::new_signed_with_payer(
            &[
                merkle_tree_account_create_ix,
                nullifier_queue_account_create_ix,
                instruction,
            ],
            Some(&payer.pubkey()),
            &vec![payer, merkle_tree_keypair, nullifier_queue_keypair],
            rpc.get_latest_blockhash().await.unwrap(),
        )
    };

    rpc.process_transaction(transaction.clone()).await
}

#[allow(clippy::too_many_arguments)]
#[inline(never)]
pub async fn create_address_merkle_tree_and_queue_account<R: RpcConnection>(
    payer: &Keypair,
    registry: bool,
    context: &mut R,
    address_merkle_tree_keypair: &Keypair,
    address_queue_keypair: &Keypair,
    program_owner: Option<Pubkey>,
    forester: Option<Pubkey>,
    merkle_tree_config: &AddressMerkleTreeConfig,
    queue_config: &AddressQueueConfig,
    index: u64,
) -> Result<Signature, RpcError> {
    use light_registry::account_compression_cpi::sdk::create_initialize_address_merkle_tree_and_queue_instruction as create_initialize_address_merkle_tree_and_queue_instruction_registry;

    let size =
        account_compression::state::QueueAccount::size(queue_config.capacity as usize).unwrap();
    let account_create_ix = create_account_instruction(
        &payer.pubkey(),
        size,
        context
            .get_minimum_balance_for_rent_exemption(size)
            .await
            .unwrap(),
        &account_compression::ID,
        Some(address_queue_keypair),
    );

    let size = account_compression::state::AddressMerkleTreeAccount::size(
        merkle_tree_config.height as usize,
        merkle_tree_config.changelog_size as usize,
        merkle_tree_config.roots_size as usize,
        merkle_tree_config.canopy_depth as usize,
        merkle_tree_config.address_changelog_size as usize,
    );
    let mt_account_create_ix = create_account_instruction(
        &payer.pubkey(),
        size,
        context
            .get_minimum_balance_for_rent_exemption(size)
            .await
            .unwrap(),
        &account_compression::ID,
        Some(address_merkle_tree_keypair),
    );
    let instruction = if registry {
        create_initialize_address_merkle_tree_and_queue_instruction_registry(
            payer.pubkey(),
            forester,
            program_owner,
            address_merkle_tree_keypair.pubkey(),
            address_queue_keypair.pubkey(),
            merkle_tree_config.clone(),
            queue_config.clone(),
        )
    } else {
        create_initialize_address_merkle_tree_and_queue_instruction(
            index,
            payer.pubkey(),
            None,
            program_owner,
            forester,
            address_merkle_tree_keypair.pubkey(),
            address_queue_keypair.pubkey(),
            merkle_tree_config.clone(),
            queue_config.clone(),
        )
    };
    let transaction = Transaction::new_signed_with_payer(
        &[account_create_ix, mt_account_create_ix, instruction],
        Some(&payer.pubkey()),
        &vec![&payer, &address_queue_keypair, &address_merkle_tree_keypair],
        context.get_latest_blockhash().await.unwrap(),
    );
    let result = context.process_transaction(transaction.clone()).await;
    #[allow(clippy::question_mark)]
    if let Err(e) = result {
        return Err(e);
    }

    // To initialize the indexed tree we do 4 operations:
    // 1. insert 0 append 0 and update 0
    // 2. insert 1 append BN254_FIELD_SIZE -1 and update 0
    // we appended two values this the expected next index is 2;
    // The right most leaf is the hash of the indexed array element with value FIELD_SIZE - 1
    // index 1, next_index: 0
    let expected_change_log_length = cmp::min(4, merkle_tree_config.changelog_size as usize);
    let expected_roots_length = cmp::min(4, merkle_tree_config.roots_size as usize);
    let expected_next_index = 2;
    let expected_indexed_change_log_length =
        cmp::min(4, merkle_tree_config.address_changelog_size as usize);
    let mut reference_tree =
        light_indexed_merkle_tree::reference::IndexedMerkleTree::<Poseidon, usize>::new(
            account_compression::utils::constants::ADDRESS_MERKLE_TREE_HEIGHT as usize,
            account_compression::utils::constants::ADDRESS_MERKLE_TREE_CANOPY_DEPTH as usize,
        )
        .unwrap();
    reference_tree.init().unwrap();

    let expected_right_most_leaf = reference_tree
        .merkle_tree
        .leaf(reference_tree.merkle_tree.rightmost_index - 1);

    let _expected_right_most_leaf = [
        30, 164, 22, 238, 180, 2, 24, 181, 64, 193, 207, 184, 219, 233, 31, 109, 84, 232, 162, 158,
        220, 48, 163, 158, 50, 107, 64, 87, 167, 217, 99, 245,
    ];
    assert_eq!(expected_right_most_leaf, _expected_right_most_leaf);
    let owner = if registry {
        let registered_program = get_registered_program_pda(&light_registry::ID);
        let registered_program_account = context
            .get_anchor_account::<RegisteredProgram>(&registered_program)
            .await
            .unwrap()
            .unwrap();
        registered_program_account.group_authority_pda
    } else {
        payer.pubkey()
    };
    assert_address_merkle_tree_initialized(
        context,
        &address_merkle_tree_keypair.pubkey(),
        &address_queue_keypair.pubkey(),
        merkle_tree_config,
        index,
        program_owner,
        forester,
        expected_change_log_length,
        expected_roots_length,
        expected_next_index,
        &expected_right_most_leaf,
        &owner,
        expected_indexed_change_log_length,
    )
    .await;

    assert_address_queue_initialized(
        context,
        &address_queue_keypair.pubkey(),
        queue_config,
        &address_merkle_tree_keypair.pubkey(),
        merkle_tree_config,
        QueueType::AddressQueue,
        index,
        program_owner,
        forester,
        &owner,
    )
    .await;
    result
}

pub async fn register_program_with_registry_program<R: RpcConnection>(
    rpc: &mut R,
    governance_authority: &Keypair,
    group_pda: &Pubkey,
    program_id_keypair: &Keypair,
) -> Result<Pubkey, RpcError> {
    let governance_authority_pda = get_protocol_config_pda_address();
    let (instruction, token_program_registered_program_pda) = create_register_program_instruction(
        governance_authority.pubkey(),
        governance_authority_pda,
        *group_pda,
        program_id_keypair.pubkey(),
    );
    let cpi_authority_pda = light_registry::utils::get_cpi_authority_pda();
    let transfer_instruction = system_instruction::transfer(
        &governance_authority.pubkey(),
        &cpi_authority_pda.0,
        rpc.get_minimum_balance_for_rent_exemption(RegisteredProgram::LEN)
            .await
            .unwrap(),
    );

    rpc.create_and_send_transaction(
        &[transfer_instruction, instruction],
        &governance_authority.pubkey(),
        &[governance_authority, program_id_keypair],
    )
    .await?;
    Ok(token_program_registered_program_pda)
}

pub async fn deregister_program_with_registry_program<R: RpcConnection>(
    rpc: &mut R,
    governance_authority: &Keypair,
    group_pda: &Pubkey,
    program_id_keypair: &Keypair,
) -> Result<Pubkey, light_client::rpc::errors::RpcError> {
    let governance_authority_pda = get_protocol_config_pda_address();
    let (instruction, token_program_registered_program_pda) = create_deregister_program_instruction(
        governance_authority.pubkey(),
        governance_authority_pda,
        *group_pda,
        program_id_keypair.pubkey(),
    );
    let cpi_authority_pda = light_registry::utils::get_cpi_authority_pda();
    let transfer_instruction = system_instruction::transfer(
        &governance_authority.pubkey(),
        &cpi_authority_pda.0,
        rpc.get_minimum_balance_for_rent_exemption(RegisteredProgram::LEN)
            .await
            .unwrap(),
    );

    rpc.create_and_send_transaction(
        &[transfer_instruction, instruction],
        &governance_authority.pubkey(),
        &[governance_authority],
    )
    .await?;
    Ok(token_program_registered_program_pda)
}
