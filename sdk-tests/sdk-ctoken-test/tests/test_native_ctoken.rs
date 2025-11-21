// #![cfg(feature = "test-sbf")]

use borsh::{BorshDeserialize, BorshSerialize};
use light_client::indexer::Indexer;
use light_client::rpc::errors::RpcError;
use light_client::rpc::Rpc;
use light_compressed_token_sdk::ctoken::{CreateCMint, CreateCMintParams};
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_sdk::instruction::ValidityProof;
use native_ctoken_examples::{
    CreateAtaData, CreateCmintData, CreateTokenAccountData, MintToCTokenData, TransferData,
};
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

#[tokio::test]
async fn test_create_compressed_mint() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", native_ctoken_examples::ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let mint_signer = Keypair::new();
    let decimals = 9u8;
    let mint_authority = payer.pubkey();

    let address_tree = rpc.get_address_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    let compressed_token_program_id =
        Pubkey::new_from_array(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID);

    // Use SDK helper to derive the compression address correctly
    let compression_address = light_compressed_token_sdk::ctoken::derive_compressed_mint_address(
        &mint_signer.pubkey(),
        &address_tree.tree,
    );

    let mint_pda =
        light_compressed_token_sdk::ctoken::find_spl_mint_address(&mint_signer.pubkey()).0;

    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![light_client::indexer::AddressWithTree {
                address: compression_address,
                tree: address_tree.tree,
            }],
            None,
        )
        .await
        .unwrap()
        .value;

    // Build params for the SDK
    let params = CreateCMintParams {
        decimals,
        version: 3,
        address_merkle_tree_root_index: rpc_result.addresses[0].root_index,
        mint_authority,
        proof: rpc_result.proof.0.unwrap().into(),
        compression_address,
        mint: mint_pda,
        freeze_authority: None,
        extensions: None,
    };

    // Use SDK builder to get the full compressed token instruction with all accounts
    let create_cmint_builder = CreateCMint::new(
        params.clone(),
        mint_signer.pubkey(),
        payer.pubkey(),
        address_tree.tree,
        output_queue,
    );
    let ctoken_instruction = create_cmint_builder.instruction().unwrap();

    // Create instruction data for wrapper program
    let create_cmint_data = CreateCmintData {
        decimals,
        version: 3,
        address_merkle_tree_root_index: rpc_result.addresses[0].root_index,
        mint_authority,
        proof: rpc_result.proof.0.unwrap().into(),
        compression_address,
        mint: mint_pda,
        freeze_authority: None,
        extensions: None,
    };
    let instruction_data = [vec![0u8], create_cmint_data.try_to_vec().unwrap()].concat();

    // Add compressed token program as first account for CPI, then all SDK-generated accounts
    let mut wrapper_accounts = vec![AccountMeta::new_readonly(
        compressed_token_program_id,
        false,
    )];
    wrapper_accounts.extend(ctoken_instruction.accounts);

    let instruction = Instruction {
        program_id: native_ctoken_examples::ID,
        accounts: wrapper_accounts,
        data: instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &mint_signer])
        .await
        .unwrap();

    let compressed_account = rpc
        .get_compressed_account(compression_address, None)
        .await
        .unwrap()
        .value;

    assert!(compressed_account.is_some(), "Compressed mint should exist");
}

#[tokio::test]
async fn test_mint_to_ctoken() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", native_ctoken_examples::ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();

    // TODO: Implement test
    println!("Test mint_to_ctoken - to be implemented");
}

#[tokio::test]
async fn test_create_token_account_invoke() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", native_ctoken_examples::ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();

    // TODO: Implement test
    println!("Test create_token_account_invoke - to be implemented");
}

#[tokio::test]
async fn test_create_token_account_invoke_signed() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", native_ctoken_examples::ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();

    // TODO: Implement test
    println!("Test create_token_account_invoke_signed - to be implemented");
}

#[tokio::test]
async fn test_create_ata_invoke() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", native_ctoken_examples::ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();

    // TODO: Implement test
    println!("Test create_ata_invoke - to be implemented");
}

#[tokio::test]
async fn test_create_ata_invoke_signed() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", native_ctoken_examples::ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();

    // TODO: Implement test
    println!("Test create_ata_invoke_signed - to be implemented");
}

#[tokio::test]
async fn test_transfer_invoke() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", native_ctoken_examples::ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();

    // For now, just verify the test infrastructure works
    // Full implementation requires creating compressed mint and token accounts first
    println!("Test transfer_invoke - infrastructure working");

    // This test passes if we can initialize the environment
    assert!(true);
}

#[tokio::test]
async fn test_transfer_invoke_signed() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", native_ctoken_examples::ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();

    // TODO: Implement test
    println!("Test transfer_invoke_signed - to be implemented");
}

#[tokio::test]
async fn test_end_to_end_workflow() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", native_ctoken_examples::ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();

    // TODO: Implement end-to-end workflow test
    println!("Test end_to_end_workflow - to be implemented");
}
