//! Integration test for single PDA macro validation.

use anchor_lang::{InstructionData, ToAccountMetas};
use light_client::interface::{
    get_create_accounts_proof, CreateAccountsProofInput, InitializeRentFreeConfig,
};
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest},
    ProgramTestConfig, Rpc,
};
use light_sdk::utils::derive_rent_sponsor_pda;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Test creating a single compressible PDA using the macro.
/// Validates that #[light_account(init)] works in isolation for PDAs.
#[tokio::test]
async fn test_create_single_pda() {
    use single_pda_test::CreatePdaParams;

    let program_id = single_pda_test::ID;
    let mut config = ProgramTestConfig::new_v2(true, Some(vec![("single_pda_test", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    // Derive rent sponsor PDA for this program
    let (rent_sponsor, _) = derive_rent_sponsor_pda(&program_id);

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

    let owner = Keypair::new().pubkey();

    // Derive PDA for record
    let (record_pda, _) =
        Pubkey::find_program_address(&[b"minimal_record", owner.as_ref()], &program_id);

    // Get proof for the PDA
    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::pda(record_pda)],
    )
    .await
    .unwrap();

    let accounts = single_pda_test::accounts::CreatePda {
        fee_payer: payer.pubkey(),
        compression_config: config_pda,
        record: record_pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = single_pda_test::instruction::CreatePda {
        params: CreatePdaParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
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

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .expect("CreatePda should succeed");

    // Verify PDA exists on-chain
    let record_account = rpc
        .get_account(record_pda)
        .await
        .unwrap()
        .expect("Record PDA should exist on-chain");

    // Parse and verify record data
    use single_pda_test::MinimalRecord;
    let record: MinimalRecord =
        borsh::BorshDeserialize::deserialize(&mut &record_account.data[8..])
            .expect("Failed to deserialize MinimalRecord");

    // Verify owner field
    assert_eq!(record.owner, owner, "Record owner should match");

    // Verify compression_info state is decompressed (indicates compressible registration)
    assert!(
        !record.compression_info.is_compressed(),
        "Record should be in decompressed state"
    );
}
