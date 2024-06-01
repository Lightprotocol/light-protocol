use crate::assert_address_merkle_tree::assert_address_merkle_tree_initialized;
use crate::assert_queue::assert_address_queue_initialized;
use crate::create_account_instruction;
use crate::rpc::rpc_connection::RpcConnection;
use crate::rpc::test_rpc::ProgramTestRpcConnection;
use account_compression::utils::constants::GROUP_AUTHORITY_SEED;
use account_compression::QueueType;
use account_compression::{
    sdk::create_initialize_merkle_tree_instruction, GroupAuthority, RegisteredProgram,
};
use account_compression::{NullifierQueueConfig, StateMerkleTreeConfig};
use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use light_macros::pubkey;
use light_registry::sdk::{
    create_initialize_governance_authority_instruction,
    create_initialize_group_authority_instruction, create_register_program_instruction,
    get_cpi_authority_pda, get_governance_authority_pda, get_group_pda,
};
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_sdk::{
    pubkey::Pubkey, signature::Keypair, signature::Signer, system_instruction,
    transaction::Transaction,
};

pub const CPI_CONTEXT_ACCOUNT_RENT: u64 = 143487360; // lamports of the cpi context account
pub const LIGHT_ID: Pubkey = pubkey!("5WzvRtu7LABotw1SUEpguJiKU27LRGsiCnF5FH6VV7yP");
pub const ACCOUNT_COMPRESSION_ID: Pubkey = pubkey!("5QPEJ5zDsVou9FQS3KCauKswM3VwBEBu4dpL9xTqkWwN");
pub const PDA_PROGRAM_ID: Pubkey = pubkey!("6UqiSPd2mRCTTwkzhcs1M6DGYsqHWd5jiPueX3LwDMXQ");
pub const COMPRESSED_TOKEN_PROGRAM_PROGRAM_ID: Pubkey =
    pubkey!("9sixVEthz2kMSKfeApZXHwuboT6DZuT6crAYJTciUCqE");
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
    program_test.add_program("light_registry", LIGHT_ID, None);
    program_test.add_program("account_compression", ACCOUNT_COMPRESSION_ID, None);
    program_test.add_program(
        "light_compressed_token",
        COMPRESSED_TOKEN_PROGRAM_PROGRAM_ID,
        None,
    );
    program_test.add_program("light_system_program", PDA_PROGRAM_ID, None);
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
    pub registered_program_pda: Pubkey,
    pub address_merkle_tree_pubkey: Pubkey,
    pub address_merkle_tree_queue_pubkey: Pubkey,
    pub cpi_context_account_pubkey: Pubkey,
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
    let cpi_authority_pda = get_cpi_authority_pda();
    let authority_pda = get_governance_authority_pda();
    let payer = Keypair::from_bytes(&PAYER_KEYPAIR).unwrap();
    let _ = airdrop_lamports(&mut context, &payer.pubkey(), 100_000_000_000).await;

    let instruction =
        create_initialize_governance_authority_instruction(payer.pubkey(), payer.pubkey());
    context
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    let group_seed_keypair = Keypair::from_bytes(&GROUP_PDA_SEED_TEST_KEYPAIR).unwrap();
    let group_pda = initialize_new_group(
        &group_seed_keypair,
        &payer,
        &mut context,
        cpi_authority_pda.0,
    )
    .await;

    let gov_authority = context
        .get_anchor_account::<GroupAuthority>(&authority_pda.0)
        .await;
    assert_eq!(gov_authority.authority, payer.pubkey());
    println!("gov authority: {:?}", gov_authority);
    let (instruction, registered_program_pda) = create_register_program_instruction(
        payer.pubkey(),
        authority_pda,
        group_pda,
        PDA_PROGRAM_ID,
    );

    let transfer_instruction = system_instruction::transfer(
        &payer.pubkey(),
        &cpi_authority_pda.0,
        context
            .get_minimum_balance_for_rent_exemption(RegisteredProgram::LEN)
            .await
            .unwrap(),
    );

    context
        .create_and_send_transaction(
            &[transfer_instruction, instruction],
            &payer.pubkey(),
            &[&payer],
        )
        .await
        .unwrap();
    let merkle_tree_keypair = Keypair::from_bytes(&MERKLE_TREE_TEST_KEYPAIR).unwrap();
    let merkle_tree_pubkey = merkle_tree_keypair.pubkey();
    let nullifier_queue_keypair = Keypair::from_bytes(&NULLIFIER_QUEUE_TEST_KEYPAIR).unwrap();
    let nullifier_queue_pubkey = nullifier_queue_keypair.pubkey();

    create_state_merkle_tree_and_queue_account(
        &payer,
        &group_pda,
        &mut context,
        &merkle_tree_keypair,
        &nullifier_queue_keypair,
        None,
        1,
    )
    .await;

    let address_merkle_tree_keypair =
        Keypair::from_bytes(&ADDRESS_MERKLE_TREE_TEST_KEYPAIR).unwrap();

    let address_merkle_tree_queue_keypair =
        Keypair::from_bytes(&ADDRESS_MERKLE_TREE_QUEUE_TEST_KEYPAIR).unwrap();

    create_address_merkle_tree_and_queue_account(
        &payer,
        &group_pda,
        &mut context,
        &address_merkle_tree_keypair,
        &address_merkle_tree_queue_keypair,
        None,
        1,
    )
    .await;
    let cpi_signature_keypair = Keypair::from_bytes(&SIGNATURE_CPI_TEST_KEYPAIR).unwrap();

    init_cpi_context_account(
        &mut context,
        &merkle_tree_pubkey,
        &cpi_signature_keypair,
        &payer,
    )
    .await;
    (
        context,
        EnvAccounts {
            merkle_tree_pubkey,
            nullifier_queue_pubkey,
            group_pda,
            governance_authority: payer,
            governance_authority_pda: authority_pda.0,
            registered_program_pda,
            address_merkle_tree_pubkey: address_merkle_tree_keypair.pubkey(),
            address_merkle_tree_queue_pubkey: address_merkle_tree_queue_keypair.pubkey(),
            cpi_context_account_pubkey: cpi_signature_keypair.pubkey(),
        },
    )
}

pub async fn initialize_new_group(
    group_seed_keypair: &Keypair,
    payer: &Keypair,
    context: &mut ProgramTestRpcConnection,
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
        .await;
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
        PDA_PROGRAM_ID,
    );

    let address_merkle_tree_keypair =
        Keypair::from_bytes(&ADDRESS_MERKLE_TREE_TEST_KEYPAIR).unwrap();

    let address_merkle_tree_queue_keypair =
        Keypair::from_bytes(&ADDRESS_MERKLE_TREE_QUEUE_TEST_KEYPAIR).unwrap();

    let cpi_signature_keypair = Keypair::from_bytes(&SIGNATURE_CPI_TEST_KEYPAIR).unwrap();

    EnvAccounts {
        merkle_tree_pubkey,
        nullifier_queue_pubkey,
        group_pda,
        governance_authority: payer,
        governance_authority_pda: authority_pda.0,
        registered_program_pda,
        address_merkle_tree_pubkey: address_merkle_tree_keypair.pubkey(),
        address_merkle_tree_queue_pubkey: address_merkle_tree_queue_keypair.pubkey(),
        cpi_context_account_pubkey: cpi_signature_keypair.pubkey(),
    }
}

pub async fn create_state_merkle_tree_and_queue_account<R: RpcConnection>(
    payer: &Keypair,
    owner: &Pubkey,
    rpc: &mut R,
    merkle_tree_keypair: &Keypair,
    nullifier_queue_keypair: &Keypair,
    program_owner: Option<Pubkey>,
    index: u64,
) {
    let merkle_tree_account_create_ix = crate::create_account_instruction(
        &payer.pubkey(),
        account_compression::state::StateMerkleTreeAccount::LEN,
        rpc.get_minimum_balance_for_rent_exemption(
            account_compression::StateMerkleTreeAccount::LEN,
        )
        .await
        .unwrap(),
        &ACCOUNT_COMPRESSION_ID,
        Some(merkle_tree_keypair),
    );
    let size = account_compression::state::queue::QueueAccount::size(
        account_compression::utils::constants::STATE_NULLIFIER_QUEUE_VALUES as usize,
    )
    .unwrap();
    let nullifier_queue_account_create_ix = crate::create_account_instruction(
        &payer.pubkey(),
        size,
        rpc.get_minimum_balance_for_rent_exemption(size)
            .await
            .unwrap(),
        &ACCOUNT_COMPRESSION_ID,
        Some(nullifier_queue_keypair),
    );

    let instruction = create_initialize_merkle_tree_instruction(
        payer.pubkey(),
        *owner,
        merkle_tree_keypair.pubkey(),
        nullifier_queue_keypair.pubkey(),
        StateMerkleTreeConfig::default(),
        NullifierQueueConfig::default(),
        program_owner,
        index,
        CPI_CONTEXT_ACCOUNT_RENT,
    );

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

#[inline(never)]
pub async fn create_address_merkle_tree_and_queue_account<R: RpcConnection>(
    payer: &Keypair,
    owner: &Pubkey,
    context: &mut R,
    address_merkle_tree_keypair: &Keypair,
    address_queue_keypair: &Keypair,
    program_owner: Option<Pubkey>,
    index: u64,
) {
    use account_compression::{
        sdk::create_initialize_address_merkle_tree_and_queue_instruction, AddressMerkleTreeConfig,
        AddressQueueConfig,
    };

    let size = account_compression::state::QueueAccount::size(
        account_compression::utils::constants::ADDRESS_QUEUE_VALUES as usize,
    )
    .unwrap();
    let account_create_ix = create_account_instruction(
        &payer.pubkey(),
        size,
        context
            .get_minimum_balance_for_rent_exemption(size)
            .await
            .unwrap(),
        &ACCOUNT_COMPRESSION_ID,
        Some(address_queue_keypair),
    );

    let mt_account_create_ix = create_account_instruction(
        &payer.pubkey(),
        account_compression::AddressMerkleTreeAccount::LEN,
        context
            .get_minimum_balance_for_rent_exemption(
                account_compression::AddressMerkleTreeAccount::LEN,
            )
            .await
            .unwrap(),
        &ACCOUNT_COMPRESSION_ID,
        Some(address_merkle_tree_keypair),
    );
    let tree_config = AddressMerkleTreeConfig::default();
    let queue_config = AddressQueueConfig::default();
    let instruction = create_initialize_address_merkle_tree_and_queue_instruction(
        index,
        payer.pubkey(),
        *owner,
        program_owner,
        address_merkle_tree_keypair.pubkey(),
        address_queue_keypair.pubkey(),
        tree_config.clone(),
        queue_config.clone(),
    );
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
    let expected_change_log_length = 4;
    let expected_roots_length = 4;
    let expected_next_index = 2;
    let expected_right_most_leaf = [
        44, 52, 197, 107, 177, 35, 22, 127, 107, 74, 169, 250, 166, 2, 228, 145, 220, 126, 40, 237,
        196, 176, 110, 41, 32, 173, 159, 91, 143, 216, 66, 32,
    ];
    assert_address_merkle_tree_initialized(
        context,
        &address_merkle_tree_keypair.pubkey(),
        &address_queue_keypair.pubkey(),
        &tree_config,
        index,
        program_owner,
        expected_change_log_length,
        expected_roots_length,
        expected_next_index,
        &expected_right_most_leaf,
        owner,
    )
    .await;

    assert_address_queue_initialized(
        context,
        &address_queue_keypair.pubkey(),
        &queue_config,
        &address_merkle_tree_keypair.pubkey(),
        &tree_config,
        QueueType::AddressQueue,
        index,
        program_owner,
        owner,
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
pub async fn register_program_with_registry_program(
    rpc: &mut ProgramTestRpcConnection,
    env: &EnvAccounts,
    program_id: &Pubkey,
) -> Result<Pubkey, crate::rpc::errors::RpcError> {
    let governance_authority_pda = get_governance_authority_pda();
    let (instruction, token_program_registered_program_pda) =
        light_registry::sdk::create_register_program_instruction(
            env.governance_authority.pubkey(),
            governance_authority_pda,
            env.group_pda,
            *program_id,
        );
    let cpi_authority_pda = get_cpi_authority_pda();
    let transfer_instruction = system_instruction::transfer(
        &env.governance_authority.pubkey(),
        &cpi_authority_pda.0,
        rpc.get_minimum_balance_for_rent_exemption(RegisteredProgram::LEN)
            .await
            .unwrap(),
    );

    rpc.create_and_send_transaction(
        &[transfer_instruction, instruction],
        &env.governance_authority.pubkey(),
        &[&env.governance_authority],
    )
    .await?;
    Ok(token_program_registered_program_pda)
}
