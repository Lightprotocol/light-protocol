//! Integration test for single mint macro validation.

use anchor_lang::{InstructionData, ToAccountMetas};
use light_client::interface::{
    get_create_accounts_proof, CreateAccountsProofInput, InitializeRentFreeConfig,
};
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest},
    ProgramTestConfig, Rpc,
};
use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use light_token::instruction::{
    find_mint_address, COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR as LIGHT_TOKEN_RENT_SPONSOR,
};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Derive the program's rent sponsor PDA (version 1).
fn program_rent_sponsor(program_id: &Pubkey) -> Pubkey {
    let (pda, _) =
        Pubkey::find_program_address(&[b"rent_sponsor", &1u16.to_le_bytes()], program_id);
    pda
}

/// Test creating a single mint using the macro.
/// Validates that #[light_account(init, mint, ...)] works in isolation.
#[tokio::test]
async fn test_create_single_mint() {
    use single_mint_test::{CreateMintParams, MINT_SIGNER_SEED};

    let program_id = single_mint_test::ID;
    let mut config = ProgramTestConfig::new_v2(true, Some(vec![("single_mint_test", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    // Use program's own rent sponsor for LightConfig initialization
    let (init_config_ixs, config_pda) = InitializeRentFreeConfig::new(
        &program_id,
        &payer.pubkey(),
        &program_data_pda,
        program_rent_sponsor(&program_id),
        payer.pubkey(),
        10_000_000_000,
    )
    .build();

    rpc.create_and_send_transaction(&init_config_ixs, &payer.pubkey(), &[&payer])
        .await
        .expect("Initialize config should succeed");

    let authority = Keypair::new();

    // Derive PDA for mint signer
    let (mint_signer_pda, mint_signer_bump) = Pubkey::find_program_address(
        &[MINT_SIGNER_SEED, authority.pubkey().as_ref()],
        &program_id,
    );

    // Derive mint PDA
    let (mint_pda, _) = find_mint_address(&mint_signer_pda);

    // Get proof for the mint
    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::mint(mint_signer_pda)],
    )
    .await
    .unwrap();

    let accounts = single_mint_test::accounts::CreateMint {
        fee_payer: payer.pubkey(),
        authority: authority.pubkey(),
        mint_signer: mint_signer_pda,
        mint: mint_pda,
        compression_config: config_pda,
        light_token_compressible_config: COMPRESSIBLE_CONFIG_V1,
        rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        light_token_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = single_mint_test::instruction::CreateMint {
        params: CreateMintParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            mint_signer_bump,
        },
    };

    let instruction = Instruction {
        program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &authority])
        .await
        .expect("CreateMint should succeed");

    // Verify mint exists on-chain
    let mint_account = rpc
        .get_account(mint_pda)
        .await
        .unwrap()
        .expect("Mint should exist on-chain");

    // Parse and verify mint data
    use light_token_interface::state::Mint;
    let mint: Mint = borsh::BorshDeserialize::deserialize(&mut &mint_account.data[..])
        .expect("Failed to deserialize Mint");

    // Verify decimals match what was specified in #[light_account(init)]
    assert_eq!(mint.base.decimals, 9, "Mint should have 9 decimals");

    // Verify mint authority
    assert_eq!(
        mint.base.mint_authority,
        Some(payer.pubkey().to_bytes().into()),
        "Mint authority should be fee_payer"
    );
}
