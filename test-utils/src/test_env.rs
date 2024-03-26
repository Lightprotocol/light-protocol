#[cfg(feature = "light_program")]
use account_compression::indexed_array_sdk::create_initialize_indexed_array_instruction;
#[cfg(feature = "light_program")]
use account_compression::{
    instruction::InitializeAddressMerkleTree, state::AddressMerkleTreeAccount, GroupAuthority,
    RegisteredProgram,
};
use anchor_lang::{system_program, InstructionData};
#[cfg(feature = "light_program")]
use light::sdk::{
    create_initialize_governance_authority_instruction,
    create_initiatialize_group_authority_instruction, create_register_program_instruction,
    get_cpi_authority_pda, get_governance_authority_pda, get_group_account,
};
use light_macros::pubkey;
use solana_program_test::{BanksClientError, ProgramTest, ProgramTestContext};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    transaction::Transaction,
};
#[cfg(feature = "light_program")]
use solana_sdk::{signature::Signer, system_instruction};

use crate::create_account_instruction;
#[cfg(feature = "light_program")]
use crate::{create_and_send_transaction, get_account};

pub const LIGHT_ID: Pubkey = pubkey!("5WzvRtu7LABotw1SUEpguJiKU27LRGsiCnF5FH6VV7yP");
pub const ACCOUNT_COMPRESSION_ID: Pubkey = pubkey!("5QPEJ5zDsVou9FQS3KCauKswM3VwBEBu4dpL9xTqkWwN");
pub const PDA_PROGRAM_ID: Pubkey = pubkey!("6UqiSPd2mRCTTwkzhcs1M6DGYsqHWd5jiPueX3LwDMXQ");
pub const COMPRESSED_TOKEN_PROGRAM_PROGRAM_ID: Pubkey =
    pubkey!("9sixVEthz2kMSKfeApZXHwuboT6DZuT6crAYJTciUCqE");
pub const NOOP_PROGRAM_ID: Pubkey = pubkey!("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV");
/// Setup test programs
/// deploys:
/// 1. light program
/// 2. account_compression program
/// 3. psp_compressed_token program
/// 4. psp_compressed_pda program
pub async fn setup_test_programs() -> ProgramTestContext {
    let mut program_test = ProgramTest::default();
    program_test.add_program("light", LIGHT_ID, None);
    program_test.add_program("account_compression", ACCOUNT_COMPRESSION_ID, None);
    program_test.add_program(
        "psp_compressed_token",
        COMPRESSED_TOKEN_PROGRAM_PROGRAM_ID,
        None,
    );
    program_test.add_program("psp_compressed_pda", PDA_PROGRAM_ID, None);
    program_test.add_program("spl_noop", NOOP_PROGRAM_ID, None);
    program_test.set_compute_max_units(1_400_000u64);
    program_test.start_with_context().await
}

pub struct EnvWithAccounts {
    pub context: ProgramTestContext,
    pub merkle_tree_pubkey: Pubkey,
    pub indexed_array_pubkey: Pubkey,
    pub governance_authority: Keypair,
    pub governance_authority_pda: Pubkey,
    pub group_pda: Pubkey,
    pub registered_program_pda: Pubkey,
    pub address_merkle_tree_pubkey: Pubkey,
    pub address_merkle_tree_queue_pubkey: Pubkey,
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
/// Setup test programs with accounts
/// deploys:
/// 1. light program
/// 2. account_compression program
/// 3. psp_compressed_token program
/// 4. psp_compressed_pda program
///
/// Sets up the following accounts:
/// 5. creates and initializes governance authority
/// 6. creates and initializes group authority
/// 7. registers the psp_compressed_pda program with the group authority
/// 8. initializes Merkle tree owned by
#[cfg(feature = "light_program")]
pub async fn setup_test_programs_with_accounts() -> EnvWithAccounts {
    use crate::airdrop_lamports;

    let mut context = setup_test_programs().await;
    let cpi_authority_pda = get_cpi_authority_pda();
    let authority_pda = get_governance_authority_pda();
    let payer = Keypair::from_bytes(&PAYER_KEYPAIR).unwrap();
    airdrop_lamports(&mut context, &payer.pubkey(), 100_000_000_000)
        .await
        .unwrap();

    let instruction =
        create_initialize_governance_authority_instruction(payer.pubkey(), payer.pubkey());
    create_and_send_transaction(&mut context, &[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();
    let (group_pda, seed) = get_group_account();

    let instruction =
        create_initiatialize_group_authority_instruction(payer.pubkey(), group_pda, seed);

    create_and_send_transaction(&mut context, &[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();
    let group_authority = get_account::<GroupAuthority>(&mut context, group_pda).await;
    assert_eq!(group_authority.authority, cpi_authority_pda.0);
    assert_eq!(group_authority.seed, seed);

    let gov_authority = get_account::<GroupAuthority>(&mut context, authority_pda.0).await;

    assert_eq!(gov_authority.authority, payer.pubkey());

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
            .banks_client
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(RegisteredProgram::LEN),
    );

    create_and_send_transaction(
        &mut context,
        &[transfer_instruction, instruction],
        &payer.pubkey(),
        &[&payer],
    )
    .await
    .unwrap();
    let merkle_tree_keypair = Keypair::from_bytes(&MERKLE_TREE_TEST_KEYPAIR).unwrap();

    let account_create_ix = crate::create_account_instruction(
        &payer.pubkey(),
        account_compression::state::StateMerkleTreeAccount::LEN,
        context
            .banks_client
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(account_compression::StateMerkleTreeAccount::LEN),
        &ACCOUNT_COMPRESSION_ID,
        Some(&merkle_tree_keypair),
    );
    let merkle_tree_pubkey = merkle_tree_keypair.pubkey();
    let indexed_array_keypair = Keypair::from_bytes(&INDEXED_ARRAY_TEST_KEYPAIR).unwrap();
    let indexed_array_pubkey = indexed_array_keypair.pubkey();

    let instruction =
        account_compression::instructions::append_leaves::sdk::create_initialize_merkle_tree_instruction(payer.pubkey(), merkle_tree_pubkey, Some(indexed_array_pubkey));

    let transaction = Transaction::new_signed_with_payer(
        &[account_create_ix, instruction],
        Some(&payer.pubkey()),
        &vec![&payer, &merkle_tree_keypair],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction.clone())
        .await
        .unwrap();
    create_queue_account(
        &payer,
        &mut context,
        &indexed_array_keypair,
        &merkle_tree_pubkey,
    )
    .await;

    let address_merkle_tree_keypair = Keypair::new();
    // TODO: enable, currently failing because we didn't maintain the address merkle tree account
    // create_and_initialize_address_merkle_tree(&mut context, &address_merkle_tree_keypair)
    //     .await
    //     .unwrap();
    let address_merkle_tree_queue_keypair = Keypair::new();
    create_queue_account(
        &payer,
        &mut context,
        &address_merkle_tree_queue_keypair,
        &address_merkle_tree_keypair.pubkey(),
    )
    .await;
    EnvWithAccounts {
        context,
        merkle_tree_pubkey,
        indexed_array_pubkey,
        group_pda,
        governance_authority: payer,
        governance_authority_pda: authority_pda.0,
        registered_program_pda,
        address_merkle_tree_pubkey: address_merkle_tree_keypair.pubkey(),
        address_merkle_tree_queue_pubkey: address_merkle_tree_queue_keypair.pubkey(),
    }
}
#[cfg(feature = "light_program")]
pub async fn create_queue_account(
    payer: &Keypair,
    context: &mut ProgramTestContext,
    indexed_array_keypair: &Keypair,
    merkle_tree_pubkey: &Pubkey,
) {
    let account_create_ix = crate::create_account_instruction(
        &payer.pubkey(),
        account_compression::IndexedArrayAccount::LEN,
        context
            .banks_client
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(account_compression::IndexedArrayAccount::LEN),
        &ACCOUNT_COMPRESSION_ID,
        Some(indexed_array_keypair),
    );
    let instruction = create_initialize_indexed_array_instruction(
        payer.pubkey(),
        indexed_array_keypair.pubkey(),
        0,
        Some(*merkle_tree_pubkey),
    );
    let transaction = Transaction::new_signed_with_payer(
        &[account_create_ix, instruction],
        Some(&payer.pubkey()),
        &vec![&payer, &indexed_array_keypair],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction.clone())
        .await
        .unwrap();
}
#[cfg(feature = "light_program")]
pub fn initialize_address_merkle_tree_ix(
    context: &ProgramTestContext,
    payer: Pubkey,
    pubkey: Pubkey,
) -> Instruction {
    let instruction_data = InitializeAddressMerkleTree {
        index: 1u64,
        owner: payer,
        delegate: None,
        height: 26,
        changelog_size: 1400,
        roots_size: 2800,
        canopy_depth: 0,
    };
    Instruction {
        program_id: ACCOUNT_COMPRESSION_ID,
        accounts: vec![
            AccountMeta::new(context.payer.pubkey(), true),
            AccountMeta::new(pubkey, true),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: instruction_data.data(),
    }
}
#[cfg(feature = "light_program")]
pub async fn create_and_initialize_address_merkle_tree(
    context: &mut ProgramTestContext,
    address_merkle_tree_keypair: &Keypair,
) -> Result<(), BanksClientError> {
    let account_create_ix = create_account_instruction(
        &context.payer.pubkey(),
        AddressMerkleTreeAccount::LEN,
        context
            .banks_client
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(account_compression::AddressMerkleTreeAccount::LEN),
        &account_compression::ID,
        Some(address_merkle_tree_keypair),
    );
    // Instruction: initialize address Merkle tree.
    let initialize_ix = initialize_address_merkle_tree_ix(
        context,
        context.payer.pubkey(),
        address_merkle_tree_keypair.pubkey(),
    );
    // Transaction: initialize address Merkle tree.
    let transaction = Transaction::new_signed_with_payer(
        &[account_create_ix, initialize_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &address_merkle_tree_keypair],
        context.last_blockhash,
    );
    context.banks_client.process_transaction(transaction).await
}
