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
    program_test.set_compute_max_units(1_400_000u64);
    program_test.start_with_context().await
}

#[cfg(feature = "light_program")]
pub async fn setup_test_programs_with_accounts(context: &mut ProgramTestContext) {
    let cpi_authority_pda = get_cpi_authority_pda();
    let authority_pda = get_governance_authority_pda();
    let payer = context.payer.insecure_clone();
    let instruction =
        create_initialize_governance_authority_instruction(payer.pubkey(), payer.pubkey());
    create_and_send_transaction(context, &[instruction], &payer)
        .await
        .unwrap();
    let (group_account, seed) = get_group_account();

    let instruction =
        create_initiatialize_group_authority_instruction(payer.pubkey(), group_account, seed);

    create_and_send_transaction(context, &[instruction], &payer)
        .await
        .unwrap();
    let group_authority = get_account::<GroupAuthority>(context, group_account).await;
    assert_eq!(group_authority.authority, cpi_authority_pda.0);
    assert_eq!(group_authority.seed, seed);

    let gov_authority = get_account::<GroupAuthority>(context, authority_pda.0).await;

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

    create_and_send_transaction(context, &[transfer_instruction, instruction], &payer)
        .await
        .unwrap();
}
