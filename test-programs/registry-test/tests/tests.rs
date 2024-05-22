#![cfg(feature = "test-sbf")]

use solana_sdk::{
    signature::{Keypair, Signer},
    system_instruction,
};

use account_compression::{self, GroupAuthority, RegisteredProgram};
use light_registry::sdk::{
    create_initialize_governance_authority_instruction,
    create_initialize_group_authority_instruction, create_register_program_instruction,
    get_cpi_authority_pda, get_governance_authority_pda, get_group_account,
};
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::test_rpc::ProgramTestRpcConnection;
use light_test_utils::test_env::{setup_test_programs, PDA_PROGRAM_ID};

pub async fn setup_test_programs_with_accounts() -> ProgramTestRpcConnection {
    let context = setup_test_programs(None).await;
    let mut context = ProgramTestRpcConnection { context };
    let payer = context.get_payer().insecure_clone();
    let cpi_authority_pda = get_cpi_authority_pda();
    let authority_pda = get_governance_authority_pda();
    let instruction =
        create_initialize_governance_authority_instruction(payer.pubkey(), payer.pubkey());
    context
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();
    let (group_account, seed) = get_group_account();

    let instruction =
        create_initialize_group_authority_instruction(payer.pubkey(), group_account, seed);

    context
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();
    let group_authority = context
        .get_anchor_account::<GroupAuthority>(&group_account)
        .await;
    assert_eq!(group_authority.authority, cpi_authority_pda.0);
    assert_eq!(group_authority.seed, seed);

    let gov_authority = context
        .get_anchor_account::<GroupAuthority>(&authority_pda.0)
        .await;

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
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(RegisteredProgram::LEN),
    );

    context
        .create_and_send_transaction(
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
    let payer = context.get_payer().insecure_clone();
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
            .get_rent()
            .await
            .unwrap()
            .minimum_balance(RegisteredProgram::LEN),
    );

    context
        .create_and_send_transaction(
            &[transfer_instruction, instruction],
            &payer.pubkey(),
            &[&payer],
        )
        .await
        .unwrap();
}
