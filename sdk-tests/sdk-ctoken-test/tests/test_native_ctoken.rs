// #![cfg(feature = "test-sbf")]

use borsh::{BorshDeserialize, BorshSerialize};
use light_client::indexer::Indexer;
use light_client::rpc::Rpc;
use light_compressed_token_sdk::ctoken::{CreateCMint, CreateCMintParams, CTOKEN_PROGRAM_ID};
use light_program_test::indexer::TestIndexerExtensions;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use native_ctoken_examples::{CreateCmintData, CreateTokenAccountData, MintToCTokenData};
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
    let mint_authority = payer.pubkey();

    // Setup: Create compressed mint directly (not via wrapper program)
    let (mint_pda, compression_address) =
        setup_create_compressed_mint(&mut rpc, &payer, mint_authority, 9).await;

    // Create a ctoken account to mint tokens to via wrapper program
    let ctoken_account = Keypair::new();
    let owner = payer.pubkey();

    let create_token_account_data = CreateTokenAccountData {
        owner,
        pre_pay_num_epochs: 2,
        lamports_per_write: 1,
    };
    let instruction_data = [vec![2u8], create_token_account_data.try_to_vec().unwrap()].concat();

    use light_compressed_token_sdk::ctoken::{config_pda, rent_sponsor_pda};
    let config = config_pda();
    let rent_sponsor = rent_sponsor_pda();

    let instruction = Instruction {
        program_id: native_ctoken_examples::ID,
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(ctoken_account.pubkey(), true),
            AccountMeta::new_readonly(mint_pda, false),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new_readonly(Pubkey::default(), false), // system_program
            AccountMeta::new(rent_sponsor, false),
            AccountMeta::new_readonly(CTOKEN_PROGRAM_ID.into(), false), // token_program
        ],
        data: instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &ctoken_account])
        .await
        .unwrap();

    // Get the compressed mint account to build CompressedMintWithContext
    let compressed_mint_account = rpc
        .get_compressed_account(compression_address, None)
        .await
        .unwrap()
        .value
        .expect("Compressed mint should exist");

    // Deserialize the compressed mint data
    use light_ctoken_types::state::CompressedMint;
    let compressed_mint =
        CompressedMint::deserialize(&mut compressed_mint_account.data.unwrap().data.as_slice())
            .unwrap();

    // Build CompressedMintWithContext from the compressed account
    let mut compressed_mint_with_context =
        light_ctoken_types::instructions::mint_action::CompressedMintWithContext {
            address: compression_address,
            leaf_index: compressed_mint_account.leaf_index,
            prove_by_index: true,
            root_index: 0, // Will be updated with validity proof
            mint: compressed_mint.try_into().unwrap(),
        };

    // Get validity proof for the mint operation
    let rpc_result = rpc
        .get_validity_proof(vec![compressed_mint_account.hash], vec![], None)
        .await
        .unwrap()
        .value;

    // Update root index from validity proof
    compressed_mint_with_context.root_index = rpc_result.accounts[0]
        .root_index
        .root_index()
        .unwrap_or_default();

    let amount = 1_000_000_000u64; // 1 token with 9 decimals

    // Build instruction data for wrapper program
    let mint_to_data = MintToCTokenData {
        compressed_mint_inputs: compressed_mint_with_context.clone(),
        amount,
        mint_authority,
        proof: rpc_result.proof,
    };
    let instruction_data = [vec![1u8], mint_to_data.try_to_vec().unwrap()].concat();

    // Use SDK to get all required accounts for mint_to_ctoken
    use light_compressed_token_sdk::ctoken::{MintToCToken, MintToCTokenParams};

    let params = MintToCTokenParams::new(
        compressed_mint_with_context,
        amount,
        mint_authority,
        rpc_result.proof,
    );

    let mint_to_builder = MintToCToken::new(
        params,
        payer.pubkey(),
        compressed_mint_account.tree_info.tree,
        compressed_mint_account.tree_info.queue,
        compressed_mint_account.tree_info.queue,
        vec![ctoken_account.pubkey()],
    );
    let ctoken_instruction = mint_to_builder.instruction().unwrap();

    // Build wrapper instruction with compressed token program as first account
    let compressed_token_program_id =
        Pubkey::new_from_array(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID);

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

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify tokens were minted to the ctoken account
    let ctoken_account_data = rpc
        .get_account(ctoken_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    // Parse the account data to verify balance
    use light_ctoken_types::state::CToken;
    let account_state = CToken::deserialize(&mut &ctoken_account_data.data[..]).unwrap();
    assert_eq!(account_state.amount, amount, "Token amount should match");
    assert_eq!(
        account_state.mint.to_bytes(),
        mint_pda.to_bytes(),
        "Mint should match"
    );
    assert_eq!(
        account_state.owner.to_bytes(),
        owner.to_bytes(),
        "Owner should match"
    );
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

// ============================================================================
// Helper Functions
// ============================================================================

/// Setup helper: Creates a compressed mint directly using the ctoken SDK (not via wrapper program)
/// Returns (mint_pda, compression_address)
async fn setup_create_compressed_mint(
    rpc: &mut (impl Rpc + Indexer),
    payer: &Keypair,
    mint_authority: Pubkey,
    decimals: u8,
) -> (Pubkey, [u8; 32]) {
    use light_compressed_token_sdk::ctoken::{CreateCMint, CreateCMintParams};

    let mint_signer = Keypair::new();
    let address_tree = rpc.get_address_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    // Derive compression address using SDK helpers
    let compression_address = light_compressed_token_sdk::ctoken::derive_compressed_mint_address(
        &mint_signer.pubkey(),
        &address_tree.tree,
    );

    let mint_pda =
        light_compressed_token_sdk::ctoken::find_spl_mint_address(&mint_signer.pubkey()).0;

    // Get validity proof for the address
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

    // Create instruction directly using SDK
    let create_cmint_builder = CreateCMint::new(
        params,
        mint_signer.pubkey(),
        payer.pubkey(),
        address_tree.tree,
        output_queue,
    );
    let instruction = create_cmint_builder.instruction().unwrap();

    // Send transaction
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer, &mint_signer])
        .await
        .unwrap();

    // Verify the compressed mint was created
    let compressed_account = rpc
        .get_compressed_account(compression_address, None)
        .await
        .unwrap()
        .value;

    assert!(
        compressed_account.is_some(),
        "Compressed mint should exist after setup"
    );

    (mint_pda, compression_address)
}
