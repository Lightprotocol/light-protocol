//! Integration test for AccountLoader wrapper functionality.
//!
//! Tests that mint data can be accessed after CPI initialization using
//! type-safe deserialization patterns.

use anchor_lang::{InstructionData, ToAccountMetas};
use csdk_anchor_full_derived_test::instruction_accounts::{
    CreateMintWithAccountLoaderParams, LIGHT_MINT_TEST_SIGNER_SEED,
};
use light_client::interface::{get_create_accounts_proof, CreateAccountsProofInput};
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest},
    ProgramTestConfig, Rpc,
};
use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use light_token::instruction::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Test creating a mint and verifying the handler can access mint data after CPI.
/// This demonstrates the pattern for accessing type-safe mint data after initialization.
#[tokio::test]
async fn test_create_mint_with_account_loader_wrapper() {
    use light_token::instruction::find_mint_address as find_cmint_address;

    let program_id = csdk_anchor_full_derived_test::ID;
    let mut config = ProgramTestConfig::new_v2(
        true,
        Some(vec![("csdk_anchor_full_derived_test", program_id)]),
    );
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    // Initialize rent-free config for the test program
    let (init_config_ix, config_pda) = light_client::interface::InitializeRentFreeConfig::new(
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

    let authority = Keypair::new();

    // Derive PDA for mint signer
    let (mint_signer_pda, mint_signer_bump) = Pubkey::find_program_address(
        &[LIGHT_MINT_TEST_SIGNER_SEED, authority.pubkey().as_ref()],
        &program_id,
    );

    // Derive mint PDA
    let (cmint_pda, _) = find_cmint_address(&mint_signer_pda);

    // Get proof for the mint
    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::mint(mint_signer_pda)],
    )
    .await
    .unwrap();

    // Build the instruction
    let accounts = csdk_anchor_full_derived_test::accounts::CreateMintWithAccountLoader {
        fee_payer: payer.pubkey(),
        authority: authority.pubkey(),
        mint_signer: mint_signer_pda,
        cmint: cmint_pda,
        compression_config: config_pda,
        light_token_compressible_config: COMPRESSIBLE_CONFIG_V1,
        rent_sponsor: RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        light_token_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data =
        csdk_anchor_full_derived_test::instruction::CreateMintWithAccountLoader {
            _params: CreateMintWithAccountLoaderParams {
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

    // Execute the transaction - this will:
    // 1. Create the mint via CPI
    // 2. In the handler, access and verify the mint data
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &authority])
        .await
        .expect("CreateMintWithAccountLoader should succeed");

    // Verify mint exists on-chain with expected data
    let cmint_account = rpc
        .get_account(cmint_pda)
        .await
        .unwrap()
        .expect("Mint should exist on-chain");

    // Deserialize and verify
    use borsh::BorshDeserialize;
    use light_token_interface::state::Mint;
    let mint: Mint =
        Mint::try_from_slice(&cmint_account.data[..]).expect("Failed to deserialize Mint");

    // Verify values match what was specified in #[light_account(init)] attributes
    assert_eq!(mint.base.decimals, 6, "Mint should have 6 decimals");
    assert!(mint.base.is_initialized, "Mint should be initialized");
    assert_eq!(mint.base.supply, 0, "Initial supply should be 0");

    // Verify mint authority is the fee_payer (as specified in mint::authority = fee_payer)
    assert_eq!(
        mint.base.mint_authority,
        Some(payer.pubkey().to_bytes().into()),
        "Mint authority should be fee_payer"
    );
}
