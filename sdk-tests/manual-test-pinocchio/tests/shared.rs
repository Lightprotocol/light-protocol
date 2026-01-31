//! Shared test helpers for manual-test-pinocchio integration tests.

use light_account_pinocchio::derive_rent_sponsor_pda;
use light_client::interface::{
    get_create_accounts_proof, CreateAccountsProofInput, InitializeRentFreeConfig,
};
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest},
    ProgramTestConfig, Rpc,
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Setup test environment with Light Protocol and compression config.
/// Returns (rpc, payer, config_pda).
pub async fn setup_test_env() -> (LightProgramTest, Keypair, Pubkey) {
    let program_id = Pubkey::new_from_array(manual_test_pinocchio::ID);
    let mut config =
        ProgramTestConfig::new_v2(true, Some(vec![("manual_test_pinocchio", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    // Derive rent sponsor PDA for this program (pinocchio version takes &[u8; 32])
    let (rent_sponsor_bytes, _) = derive_rent_sponsor_pda(&program_id.to_bytes());
    let rent_sponsor = Pubkey::new_from_array(rent_sponsor_bytes);

    let (init_config_ix, config_pda) = InitializeRentFreeConfig::new(
        &program_id,
        &payer.pubkey(),
        &program_data_pda,
        rent_sponsor,
        payer.pubkey(),
    )
    .build();

    rpc.create_and_send_transaction(&[init_config_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("Initialize config should succeed");

    (rpc, payer, config_pda)
}

/// Create a test mint using the two_mints instruction and return the mint pubkey.
#[allow(dead_code)]
pub async fn create_test_mint(rpc: &mut LightProgramTest, payer: &Keypair) -> Pubkey {
    use manual_test_pinocchio::two_mints::accounts::{
        CreateDerivedMintsParams, MINT_SIGNER_0_SEED, MINT_SIGNER_1_SEED,
    };
    use solana_sdk::instruction::{AccountMeta, Instruction};

    let program_id = Pubkey::new_from_array(manual_test_pinocchio::ID);
    let authority = Keypair::new();

    // Derive mint signer PDAs
    let (mint_signer_0, mint_signer_0_bump) = Pubkey::find_program_address(
        &[MINT_SIGNER_0_SEED, authority.pubkey().as_ref()],
        &program_id,
    );
    let (mint_signer_1, mint_signer_1_bump) = Pubkey::find_program_address(
        &[MINT_SIGNER_1_SEED, authority.pubkey().as_ref()],
        &program_id,
    );

    // Derive mint PDAs
    let (mint_0, _) = light_token::instruction::find_mint_address(&mint_signer_0);
    let (mint_1, _) = light_token::instruction::find_mint_address(&mint_signer_1);

    // Get proof for the mints
    let proof_result = get_create_accounts_proof(
        rpc,
        &program_id,
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

    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(authority.pubkey(), true),
        AccountMeta::new_readonly(mint_signer_0, false),
        AccountMeta::new_readonly(mint_signer_1, false),
        AccountMeta::new(mint_0, false),
        AccountMeta::new(mint_1, false),
        AccountMeta::new_readonly(light_token::instruction::config_pda(), false),
        AccountMeta::new(light_token::instruction::rent_sponsor_pda(), false),
        AccountMeta::new_readonly(light_token::instruction::LIGHT_TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(
            Pubkey::new_from_array(light_token_types::CPI_AUTHORITY_PDA),
            false,
        ),
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    let ix = Instruction {
        program_id,
        accounts: [accounts, proof_result.remaining_accounts].concat(),
        data: [
            manual_test_pinocchio::discriminators::CREATE_DERIVED_MINTS.as_slice(),
            &borsh::to_vec(&params).unwrap(),
        ]
        .concat(),
    };

    rpc.create_and_send_transaction(&[ix], &payer.pubkey(), &[payer, &authority])
        .await
        .expect("Create mint should succeed");

    mint_0 // Return first mint
}
