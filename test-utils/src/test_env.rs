use std::cmp;

use crate::assert_address_merkle_tree::assert_address_merkle_tree_initialized;
use crate::assert_queue::assert_address_queue_initialized;
use crate::create_account_instruction;
use crate::registry::register_test_forester;
use crate::rpc::rpc_connection::RpcConnection;
use crate::rpc::solana_rpc::SolanaRpcUrl;
use crate::rpc::test_rpc::ProgramTestRpcConnection;
use crate::rpc::SolanaRpcConnection;
use account_compression::sdk::create_initialize_address_merkle_tree_and_queue_instruction;
use account_compression::utils::constants::GROUP_AUTHORITY_SEED;
use account_compression::{
    sdk::create_initialize_merkle_tree_instruction, GroupAuthority, RegisteredProgram,
};
use account_compression::{AddressMerkleTreeConfig, AddressQueueConfig, QueueType};
use account_compression::{NullifierQueueConfig, StateMerkleTreeConfig};
use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use light_hasher::Poseidon;
use light_macros::pubkey;
use light_registry::get_forester_epoch_pda_address;
use light_registry::sdk::{
    create_initialize_governance_authority_instruction,
    create_initialize_group_authority_instruction, create_register_program_instruction,
    get_cpi_authority_pda, get_governance_authority_pda, get_group_pda,
};
use light_system_program::utils::get_registered_program_pda;
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_sdk::{
    pubkey::Pubkey, signature::Keypair, signature::Signer, system_instruction,
    transaction::Transaction,
};

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
    program_test.add_program("light_registry", light_registry::ID, None);
    program_test.add_program("account_compression", account_compression::ID, None);
    program_test.add_program("light_compressed_token", light_compressed_token::ID, None);
    program_test.add_program("light_system_program", light_system_program::ID, None);
    program_test.add_program("spl_noop", NOOP_PROGRAM_ID, None);
    if let Some(programs) = additional_programs {
        for (name, id) in programs {
            program_test.add_program(&name, id, None);
        }
    }
    program_test.set_compute_max_units(1_400_000u64);
    program_test.start_with_context().await
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
    pub registered_forester_epoch_pda: Pubkey,
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
pub const SYSTEM_PROGRAM_ID_TEST_KEYPAIR: [u8; 64] = [
    10, 62, 81, 156, 201, 11, 242, 85, 89, 182, 145, 223, 214, 144, 53, 147, 242, 197, 41, 55, 203,
    212, 70, 178, 225, 209, 4, 211, 43, 153, 222, 21, 238, 250, 35, 216, 163, 90, 82, 72, 167, 209,
    196, 227, 210, 173, 89, 255, 142, 20, 199, 150, 144, 215, 61, 164, 34, 47, 181, 228, 226, 153,
    208, 17,
];
// Pubkey: 7Z9Yuy3HkBCc2Wf3xzMGnz6qpV4n7ciwcoEMGKqhAnj1
pub const REGISTRY_ID_TEST_KEYPAIR: [u8; 64] = [
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

pub async fn setup_test_programs_with_accounts(
    additional_programs: Option<Vec<(String, Pubkey)>>,
) -> (ProgramTestRpcConnection, EnvAccounts) {
    use crate::airdrop_lamports;

    let context = setup_test_programs(additional_programs).await;
    let mut context = ProgramTestRpcConnection { context };
    let payer = Keypair::from_bytes(&PAYER_KEYPAIR).unwrap();
    let _ = airdrop_lamports(&mut context, &payer.pubkey(), 100_000_000_000).await;
    let forester = Keypair::from_bytes(&FORESTER_TEST_KEYPAIR).unwrap();
    airdrop_lamports(&mut context, &forester.pubkey(), 10_000_000_000)
        .await
        .unwrap();
    let env_accounts = initialize_accounts(&mut context, &payer, &forester).await;
    (context, env_accounts)
}

pub async fn setup_accounts_devnet(payer: &Keypair, forester: &Keypair) -> EnvAccounts {
    let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::Devnet, None);

    initialize_accounts(&mut rpc, payer, forester).await
}

pub async fn initialize_accounts<R: RpcConnection>(
    context: &mut R,
    payer: &Keypair,
    forester: &Keypair,
) -> EnvAccounts {
    let cpi_authority_pda = get_cpi_authority_pda();
    let authority_pda = get_governance_authority_pda();

    let instruction =
        create_initialize_governance_authority_instruction(payer.pubkey(), payer.pubkey());
    context
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
        .unwrap();

    let group_seed_keypair = Keypair::from_bytes(&GROUP_PDA_SEED_TEST_KEYPAIR).unwrap();
    let group_pda =
        initialize_new_group(&group_seed_keypair, payer, context, cpi_authority_pda.0).await;

    let gov_authority = context
        .get_anchor_account::<GroupAuthority>(&authority_pda.0)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(gov_authority.authority, payer.pubkey());

    println!("forester: {:?}", forester.pubkey());
    register_test_forester(context, payer, &forester.pubkey())
        .await
        .unwrap();
    let system_program_id_test_keypair =
        Keypair::from_bytes(&SYSTEM_PROGRAM_ID_TEST_KEYPAIR).unwrap();
    register_program_with_registry_program(
        context,
        payer,
        &group_pda,
        &system_program_id_test_keypair,
    )
    .await
    .unwrap();
    let registry_id_test_keypair = Keypair::from_bytes(&REGISTRY_ID_TEST_KEYPAIR).unwrap();
    register_program_with_registry_program(context, payer, &group_pda, &registry_id_test_keypair)
        .await
        .unwrap();

    let merkle_tree_keypair = Keypair::from_bytes(&MERKLE_TREE_TEST_KEYPAIR).unwrap();
    let merkle_tree_pubkey = merkle_tree_keypair.pubkey();
    let nullifier_queue_keypair = Keypair::from_bytes(&NULLIFIER_QUEUE_TEST_KEYPAIR).unwrap();
    let nullifier_queue_pubkey = nullifier_queue_keypair.pubkey();

    create_state_merkle_tree_and_queue_account(
        payer,
        true,
        context,
        &merkle_tree_keypair,
        &nullifier_queue_keypair,
        None,
        1,
        &StateMerkleTreeConfig::default(),
        &NullifierQueueConfig::default(),
    )
    .await;

    let address_merkle_tree_keypair =
        Keypair::from_bytes(&ADDRESS_MERKLE_TREE_TEST_KEYPAIR).unwrap();

    let address_merkle_tree_queue_keypair =
        Keypair::from_bytes(&ADDRESS_MERKLE_TREE_QUEUE_TEST_KEYPAIR).unwrap();

    create_address_merkle_tree_and_queue_account(
        payer,
        true,
        context,
        &address_merkle_tree_keypair,
        &address_merkle_tree_queue_keypair,
        None,
        &AddressMerkleTreeConfig::default(),
        &AddressQueueConfig::default(),
        1,
    )
    .await;
    let cpi_signature_keypair = Keypair::from_bytes(&SIGNATURE_CPI_TEST_KEYPAIR).unwrap();

    init_cpi_context_account(context, &merkle_tree_pubkey, &cpi_signature_keypair, payer).await;
    let registered_system_program_pda = get_registered_program_pda(&light_system_program::ID);
    let registered_registry_program_pda = get_registered_program_pda(&light_registry::ID);
    EnvAccounts {
        merkle_tree_pubkey,
        nullifier_queue_pubkey,
        group_pda,
        governance_authority: payer.insecure_clone(),
        governance_authority_pda: authority_pda.0,
        forester: forester.insecure_clone(),
        registered_program_pda: registered_system_program_pda,
        address_merkle_tree_pubkey: address_merkle_tree_keypair.pubkey(),
        address_merkle_tree_queue_pubkey: address_merkle_tree_queue_keypair.pubkey(),
        cpi_context_account_pubkey: cpi_signature_keypair.pubkey(),
        registered_registry_program_pda,
        registered_forester_epoch_pda: get_forester_epoch_pda_address(&forester.pubkey()).0,
    }
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
    let authority_pda = get_governance_authority_pda();
    let (_, registered_program_pda) = create_register_program_instruction(
        payer.pubkey(),
        authority_pda,
        group_pda,
        light_system_program::ID,
    );

    let address_merkle_tree_keypair =
        Keypair::from_bytes(&ADDRESS_MERKLE_TREE_TEST_KEYPAIR).unwrap();

    let address_merkle_tree_queue_keypair =
        Keypair::from_bytes(&ADDRESS_MERKLE_TREE_QUEUE_TEST_KEYPAIR).unwrap();

    let cpi_signature_keypair = Keypair::from_bytes(&SIGNATURE_CPI_TEST_KEYPAIR).unwrap();
    let registered_registry_program_pda = get_registered_program_pda(&light_registry::ID);
    let forester = Keypair::from_bytes(&FORESTER_TEST_KEYPAIR).unwrap();
    EnvAccounts {
        merkle_tree_pubkey,
        nullifier_queue_pubkey,
        group_pda,
        governance_authority: payer,
        governance_authority_pda: authority_pda.0,
        registered_forester_epoch_pda: get_forester_epoch_pda_address(&forester.pubkey()).0,
        forester,
        registered_program_pda,
        address_merkle_tree_pubkey: address_merkle_tree_keypair.pubkey(),
        address_merkle_tree_queue_pubkey: address_merkle_tree_queue_keypair.pubkey(),
        cpi_context_account_pubkey: cpi_signature_keypair.pubkey(),
        registered_registry_program_pda,
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn create_state_merkle_tree_and_queue_account<R: RpcConnection>(
    payer: &Keypair,
    registry: bool,
    rpc: &mut R,
    merkle_tree_keypair: &Keypair,
    nullifier_queue_keypair: &Keypair,
    program_owner: Option<Pubkey>,
    index: u64,
    merkle_tree_config: &StateMerkleTreeConfig,
    queue_config: &NullifierQueueConfig,
) {
    use light_registry::sdk::create_initialize_merkle_tree_instruction as create_initialize_merkle_tree_instruction_registry;
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

    let instruction = if registry {
        create_initialize_merkle_tree_instruction_registry(
            payer.pubkey(),
            merkle_tree_keypair.pubkey(),
            nullifier_queue_keypair.pubkey(),
            merkle_tree_config.clone(),
            queue_config.clone(),
            program_owner,
            index,
            0, // TODO: replace with CPI_CONTEXT_ACCOUNT_RENT
        )
    } else {
        create_initialize_merkle_tree_instruction(
            payer.pubkey(),
            None,
            merkle_tree_keypair.pubkey(),
            nullifier_queue_keypair.pubkey(),
            merkle_tree_config.clone(),
            queue_config.clone(),
            program_owner,
            index,
            0, // TODO: replace with CPI_CONTEXT_ACCOUNT_RENT
        )
    };

    let transaction = Transaction::new_signed_with_payer(
        &[
            merkle_tree_account_create_ix,
            nullifier_queue_account_create_ix,
            instruction,
        ],
        Some(&payer.pubkey()),
        &vec![payer, merkle_tree_keypair, nullifier_queue_keypair],
        rpc.get_latest_blockhash().await.unwrap(),
    );
    rpc.process_transaction(transaction.clone()).await.unwrap();
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
    merkle_tree_config: &AddressMerkleTreeConfig,
    queue_config: &AddressQueueConfig,
    index: u64,
) {
    use light_registry::sdk::create_initialize_address_merkle_tree_and_queue_instruction as create_initialize_address_merkle_tree_and_queue_instruction_registry;

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
            index,
            payer.pubkey(),
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
    context
        .process_transaction(transaction.clone())
        .await
        .unwrap();

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
        &owner,
    )
    .await;
}

pub async fn init_cpi_context_account<R: RpcConnection>(
    rpc: &mut R,
    merkle_tree_pubkey: &Pubkey,
    cpi_account_keypair: &Keypair,
    payer: &Keypair,
) -> Pubkey {
    use solana_sdk::instruction::Instruction;

    use crate::create_account_instruction;
    let account_size: usize = 20 * 1024 + 8;
    let account_create_ix = create_account_instruction(
        &payer.pubkey(),
        account_size,
        rpc.get_minimum_balance_for_rent_exemption(account_size)
            .await
            .unwrap(),
        &light_system_program::ID,
        Some(cpi_account_keypair),
    );
    let data = light_system_program::instruction::InitCpiContextAccount {};
    let accounts = light_system_program::accounts::InitializeCpiContextAccount {
        fee_payer: payer.pubkey(),
        cpi_context_account: cpi_account_keypair.pubkey(),
        system_program: system_program::ID,
        associated_merkle_tree: *merkle_tree_pubkey,
    };
    let instruction = Instruction {
        program_id: light_system_program::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: data.data(),
    };
    rpc.create_and_send_transaction(
        &[account_create_ix, instruction],
        &payer.pubkey(),
        &[payer, cpi_account_keypair],
    )
    .await
    .unwrap();
    cpi_account_keypair.pubkey()
}

pub async fn register_program_with_registry_program<R: RpcConnection>(
    rpc: &mut R,
    governance_authority: &Keypair,
    group_pda: &Pubkey,
    program_id_keypair: &Keypair,
) -> Result<Pubkey, crate::rpc::errors::RpcError> {
    let governance_authority_pda = get_governance_authority_pda();
    let (instruction, token_program_registered_program_pda) = create_register_program_instruction(
        governance_authority.pubkey(),
        governance_authority_pda,
        *group_pda,
        program_id_keypair.pubkey(),
    );
    let cpi_authority_pda = get_cpi_authority_pda();
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
