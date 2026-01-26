//! Shared test helpers for manual-test integration tests.

use anchor_lang::InstructionData;
use light_client::interface::{
    get_create_accounts_proof, CreateAccountsProofInput, InitializeRentFreeConfig,
};
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest},
    ProgramTestConfig, Rpc,
};
use light_token::instruction::RENT_SPONSOR;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Setup test environment with Light Protocol and compression config.
/// Returns (rpc, payer, config_pda).
pub async fn setup_test_env() -> (LightProgramTest, Keypair, Pubkey) {
    let program_id = manual_test::ID;
    let mut config = ProgramTestConfig::new_v2(true, Some(vec![("manual_test", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    let (init_config_ix, config_pda) = InitializeRentFreeConfig::new(
        &program_id,
        &payer.pubkey(),
        &program_data_pda,
        RENT_SPONSOR,
        payer.pubkey(),
    )
    .build();

    rpc.create_and_send_transaction(&[init_config_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("Initialize config should succeed");

    (rpc, payer, config_pda)
}

/// Create a test mint using the two_mints instruction and return the mint pubkey.
pub async fn create_test_mint(rpc: &mut LightProgramTest, payer: &Keypair) -> Pubkey {
    use anchor_lang::ToAccountMetas;
    use manual_test::{CreateDerivedMintsParams, MINT_SIGNER_0_SEED, MINT_SIGNER_1_SEED};

    let authority = Keypair::new();

    // Derive mint signer PDAs
    let (mint_signer_0, mint_signer_0_bump) = Pubkey::find_program_address(
        &[MINT_SIGNER_0_SEED, authority.pubkey().as_ref()],
        &manual_test::ID,
    );
    let (mint_signer_1, mint_signer_1_bump) = Pubkey::find_program_address(
        &[MINT_SIGNER_1_SEED, authority.pubkey().as_ref()],
        &manual_test::ID,
    );

    // Derive mint PDAs
    let (mint_0, _) = light_token::instruction::find_mint_address(&mint_signer_0);
    let (mint_1, _) = light_token::instruction::find_mint_address(&mint_signer_1);

    // Get proof for the mints
    let proof_result = get_create_accounts_proof(
        rpc,
        &manual_test::ID,
        vec![
            CreateAccountsProofInput::mint(mint_signer_0),
            CreateAccountsProofInput::mint(mint_signer_1),
        ],
    )
    .await
    .unwrap();

    let params = CreateDerivedMintsParams {
        create_accounts_proof: proof_result.create_accounts_proof,
        mint_signer_0_bump,
        mint_signer_1_bump,
    };

    let accounts = manual_test::accounts::CreateDerivedMintsAccounts {
        payer: payer.pubkey(),
        authority: authority.pubkey(),
        mint_signer_0,
        mint_signer_1,
        mint_0,
        mint_1,
        compressible_config: light_token::instruction::config_pda(),
        rent_sponsor: light_token::instruction::rent_sponsor_pda(),
        light_token_program: light_token::instruction::LIGHT_TOKEN_PROGRAM_ID.into(),
        cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let ix = solana_sdk::instruction::Instruction {
        program_id: manual_test::ID,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: manual_test::instruction::CreateDerivedMints { params }.data(),
    };

    rpc.create_and_send_transaction(&[ix], &payer.pubkey(), &[payer, &authority])
        .await
        .expect("Create mint should succeed");

    mint_0 // Return first mint
}
