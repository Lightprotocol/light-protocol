#[cfg(feature = "light_program")]
use account_compression::{GroupAuthority, RegisteredProgram};
#[cfg(feature = "light_program")]
use light::sdk::{
    create_initialize_governance_authority_instruction,
    create_initiatialize_group_authority_instruction, create_register_program_instruction,
    get_cpi_authority_pda, get_governance_authority_pda, get_group_account,
};
use light_macros::pubkey;
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_sdk::pubkey::Pubkey;
#[cfg(feature = "light_program")]
use solana_sdk::{signature::Signer, system_instruction};

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
}
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
    use account_compression::indexed_array_sdk::create_initialize_indexed_array_instruction;
    use solana_sdk::transaction::Transaction;

    let mut context = setup_test_programs().await;
    let cpi_authority_pda = get_cpi_authority_pda();
    let authority_pda = get_governance_authority_pda();
    let payer = context.payer.insecure_clone();
    let instruction =
        create_initialize_governance_authority_instruction(payer.pubkey(), payer.pubkey());
    create_and_send_transaction(&mut context, &[instruction], &payer)
        .await
        .unwrap();
    let (group_account, seed) = get_group_account();

    let instruction =
        create_initiatialize_group_authority_instruction(payer.pubkey(), group_account, seed);

    create_and_send_transaction(&mut context, &[instruction], &payer)
        .await
        .unwrap();
    let group_authority = get_account::<GroupAuthority>(&mut context, group_account).await;
    assert_eq!(group_authority.authority, cpi_authority_pda.0);
    assert_eq!(group_authority.seed, seed);

    let gov_authority = get_account::<GroupAuthority>(&mut context, authority_pda.0).await;

    assert_eq!(gov_authority.authority, payer.pubkey());

    let (instruction, _) = create_register_program_instruction(
        payer.pubkey(),
        authority_pda,
        group_account,
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

    create_and_send_transaction(&mut context, &[transfer_instruction, instruction], &payer)
        .await
        .unwrap();
    let (merkle_tree_keypair, account_create_ix) = crate::create_account_instruction(
        &context.payer.pubkey(),
        account_compression::state::StateMerkleTreeAccount::LEN,
        context
            .banks_client
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(account_compression::StateMerkleTreeAccount::LEN),
        &ACCOUNT_COMPRESSION_ID,
    );
    let merkle_tree_pubkey = merkle_tree_keypair.pubkey();

    let instruction =
        account_compression::instructions::insert_two_leaves_transaction::sdk::create_initialize_merkle_tree_instruction(context.payer.pubkey(), merkle_tree_pubkey);

    let transaction = Transaction::new_signed_with_payer(
        &[account_create_ix, instruction],
        Some(&context.payer.pubkey()),
        &vec![&context.payer, &merkle_tree_keypair],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction.clone())
        .await
        .unwrap();

    let (indexed_array_keypair, account_create_ix) = crate::create_account_instruction(
        &context.payer.pubkey(),
        account_compression::IndexedArrayAccount::LEN,
        context
            .banks_client
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(account_compression::IndexedArrayAccount::LEN),
        &ACCOUNT_COMPRESSION_ID,
    );
    let indexed_array_pubkey = indexed_array_keypair.pubkey();
    let instruction = create_initialize_indexed_array_instruction(
        context.payer.pubkey(),
        indexed_array_keypair.pubkey(),
        0,
    );
    let transaction = Transaction::new_signed_with_payer(
        &[account_create_ix, instruction],
        Some(&context.payer.pubkey()),
        &vec![&context.payer, &indexed_array_keypair],
        context.last_blockhash,
    );
    context
        .banks_client
        .process_transaction(transaction.clone())
        .await
        .unwrap();
    EnvWithAccounts {
        context,
        merkle_tree_pubkey,
        indexed_array_pubkey,
    }
}
