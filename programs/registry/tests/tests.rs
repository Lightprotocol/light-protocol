#![cfg(feature = "test-sbf")]

use account_compression::{self, GroupAuthority, RegisteredProgram};
use light_registry::sdk::{
    create_initialize_governance_authority_instruction,
    create_initiatialize_group_authority_instruction, create_register_program_instruction,
    get_cpi_authority_pda, get_governance_authority_pda, get_group_account,
};
use light_test_utils::{
    create_and_send_transaction, get_account,
    test_env::{setup_test_programs, PDA_PROGRAM_ID},
};
use solana_program_test::ProgramTestContext;
use solana_sdk::{
    signature::{Keypair, Signer},
    system_instruction,
};
pub async fn setup_test_programs_with_accounts() -> ProgramTestContext {
    let mut context = setup_test_programs(None).await;
    let cpi_authority_pda = get_cpi_authority_pda();
    let authority_pda = get_governance_authority_pda();
    let payer = context.payer.insecure_clone();
    let instruction =
        create_initialize_governance_authority_instruction(payer.pubkey(), payer.pubkey());
    create_and_send_transaction(&mut context, &[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();
    let (group_account, seed) = get_group_account();

    let instruction =
        create_initiatialize_group_authority_instruction(payer.pubkey(), group_account, seed);

    create_and_send_transaction(&mut context, &[instruction], &payer.pubkey(), &[&payer])
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

    create_and_send_transaction(
        &mut context,
        &[transfer_instruction, instruction],
        &payer.pubkey(),
        &[&payer],
    )
    .await
    .unwrap();

    context
}

#[tokio::test]
async fn test_e2e() {
    let mut context = setup_test_programs_with_accounts().await;
    let payer = context.payer.insecure_clone();
    let cpi_authority_pda = get_cpi_authority_pda();
    let authority_pda = get_governance_authority_pda();
    let (group_account, _) = get_group_account();

    let random_program_id = Keypair::new().pubkey();

    // register a random program to test env setup function
    let (instruction, _) = create_register_program_instruction(
        payer.pubkey(),
        authority_pda,
        group_account,
        random_program_id,
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
}
