use anchor_lang::InstructionData;
use compressed_token_test::ID as WRAPPER_PROGRAM_ID;
use light_client::indexer::Indexer;
use light_compressed_token_sdk::instructions::{
    derive_compressed_mint_address, find_spl_mint_address,
    mint_action::instruction::{
        create_mint_action_cpi, CreateMintCpiWriteInputs, MintActionInputs,
        MintActionInputsCpiWrite,
    },
};
use light_ctoken_types::{
    instructions::mint_action::{CompressedMintInstructionData, CompressedMintWithContext},
    state::CompressedMintMetadata,
    COMPRESSED_TOKEN_PROGRAM_ID,
};
use light_program_test::{utils::assert::assert_rpc_error, LightProgramTest, ProgramTestConfig};
use light_test_utils::Rpc;
use light_verifier::CompressedProof;
use serial_test::serial;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

struct TestSetup {
    rpc: LightProgramTest,
    mint_action_inputs: MintActionInputsCpiWrite,
    payer: Keypair,
    mint_seed: Keypair,
    mint_authority: Keypair,
    compressed_mint_address: [u8; 32],
    cpi_context_pubkey: Pubkey,
}

async fn test_setup() -> TestSetup {
    // 1. Setup test environment with wrapper program
    let rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        true,
        Some(vec![("compressed_token_test", WRAPPER_PROGRAM_ID)]),
    ))
    .await
    .expect("Failed to setup test programs");

    let payer = rpc.get_payer().insecure_clone();
    let address_tree_info = rpc.get_address_tree_v2();
    let address_tree = address_tree_info.tree;

    // Get CPI context from test accounts
    let tree_info = rpc.test_accounts.v2_state_trees[0];
    let cpi_context_pubkey = tree_info.cpi_context;

    // 2. Create mint parameters
    let mint_seed = Keypair::new();
    let mint_authority = Keypair::new();
    let freeze_authority = Pubkey::new_unique();
    let decimals = 9u8;

    // Derive addresses
    let compressed_mint_address =
        derive_compressed_mint_address(&mint_seed.pubkey(), &address_tree);
    let (spl_mint_pda, mint_bump) = find_spl_mint_address(&mint_seed.pubkey());

    // 3. Build mint action instruction using SDK
    let compressed_mint_inputs = CompressedMintWithContext {
        leaf_index: 0,
        prove_by_index: false,
        root_index: 0,
        address: compressed_mint_address,
        mint: CompressedMintInstructionData {
            supply: 0,
            decimals,
            metadata: CompressedMintMetadata {
                version: 3,
                spl_mint_initialized: false,
                mint: spl_mint_pda.into(),
            },
            mint_authority: Some(mint_authority.pubkey().into()),
            freeze_authority: Some(freeze_authority.into()),
            extensions: None,
        },
    };

    let create_mint_inputs = CreateMintCpiWriteInputs {
        compressed_mint_inputs,
        mint_seed: mint_seed.pubkey(),
        mint_bump,
        authority: mint_authority.pubkey(),
        payer: payer.pubkey(),
        cpi_context_pubkey,
        first_set_context: true,
        address_tree_index: 1,
        output_queue_index: 0,
        assigned_account_index: 0,
    };

    let mint_action_inputs = MintActionInputsCpiWrite::new_create_mint(create_mint_inputs);

    TestSetup {
        rpc,
        mint_action_inputs,
        payer,
        mint_seed,
        mint_authority,
        compressed_mint_address,
        cpi_context_pubkey,
    }
}

#[tokio::test]
#[serial]
async fn test_write_to_cpi_context_create_mint() {
    let TestSetup {
        mut rpc,
        mint_action_inputs,
        payer,
        mint_seed,
        mint_authority,
        compressed_mint_address,
        cpi_context_pubkey,
    } = test_setup().await;

    // Get the compressed token program instruction
    let ctoken_instruction =
        light_compressed_token_sdk::instructions::mint_action::instruction::mint_action_cpi_write(
            mint_action_inputs,
        )
        .expect("Failed to build mint action instruction");

    // Build wrapper program instruction
    // The wrapper just passes through all accounts and instruction data

    // Build the wrapper instruction using Anchor's InstructionData
    let wrapper_ix_data = compressed_token_test::instruction::WriteToCpiContextMintAction {
        inputs: ctoken_instruction.data.clone(),
    };

    let wrapper_instruction = Instruction {
        program_id: WRAPPER_PROGRAM_ID,
        accounts: vec![AccountMeta::new_readonly(
            Pubkey::new_from_array(COMPRESSED_TOKEN_PROGRAM_ID),
            false,
        )]
        .into_iter()
        .chain(ctoken_instruction.accounts.clone())
        .collect(),
        data: wrapper_ix_data.data(),
    };

    // Execute wrapper instruction
    rpc.create_and_send_transaction(
        &[wrapper_instruction],
        &payer.pubkey(),
        &[&payer, &mint_seed, &mint_authority],
    )
    .await
    .expect("Failed to execute wrapper instruction");

    // Verify CPI context account has data written
    let cpi_context_account_data = rpc
        .get_account(cpi_context_pubkey)
        .await
        .expect("Failed to get CPI context account")
        .expect("CPI context account should exist");

    // Verify the account has data (not empty)
    assert!(
        !cpi_context_account_data.data.is_empty(),
        "CPI context account should have data"
    );

    // Verify the account is owned by light system program
    assert_eq!(
        cpi_context_account_data.owner,
        light_system_program::ID,
        "CPI context account should be owned by light system program"
    );

    // Verify no on-chain compressed mint was created (write mode doesn't execute)
    let indexer_result = rpc
        .indexer()
        .unwrap()
        .get_compressed_account(compressed_mint_address, None)
        .await
        .unwrap()
        .value;

    assert!(
        indexer_result.is_none(),
        "Compressed mint should NOT exist (write mode doesn't execute)"
    );
}

#[tokio::test]
#[serial]
async fn test_write_to_cpi_context_invalid_address_tree() {
    let TestSetup {
        mut rpc,
        mut mint_action_inputs,
        payer,
        mint_seed,
        mint_authority,
        compressed_mint_address: _,
        cpi_context_pubkey: _,
    } = test_setup().await;

    // Swap the address tree pubkey to a random one (this should fail validation)
    let invalid_address_tree = Pubkey::new_unique();
    mint_action_inputs.cpi_context.address_tree_pubkey = invalid_address_tree.to_bytes();

    // Get the compressed token program instruction
    let ctoken_instruction =
        light_compressed_token_sdk::instructions::mint_action::instruction::mint_action_cpi_write(
            mint_action_inputs,
        )
        .expect("Failed to build mint action instruction");

    // Build wrapper program instruction
    let wrapper_ix_data = compressed_token_test::instruction::WriteToCpiContextMintAction {
        inputs: ctoken_instruction.data.clone(),
    };

    let wrapper_instruction = Instruction {
        program_id: WRAPPER_PROGRAM_ID,
        accounts: vec![AccountMeta::new_readonly(
            Pubkey::new_from_array(COMPRESSED_TOKEN_PROGRAM_ID),
            false,
        )]
        .into_iter()
        .chain(ctoken_instruction.accounts.clone())
        .collect(),
        data: wrapper_ix_data.data(),
    };

    // Execute wrapper instruction - should fail
    let result = rpc
        .create_and_send_transaction(
            &[wrapper_instruction],
            &payer.pubkey(),
            &[&payer, &mint_seed, &mint_authority],
        )
        .await;

    // Assert that the transaction failed with MintActionInvalidCpiContextAddressTreePubkey error
    // Error code 105 = MintActionInvalidCpiContextAddressTreePubkey
    assert_rpc_error(result, 0, 105).unwrap();
}

#[tokio::test]
#[serial]
async fn test_write_to_cpi_context_invalid_compressed_address() {
    let TestSetup {
        mut rpc,
        mut mint_action_inputs,
        payer,
        mint_seed,
        mint_authority,
        compressed_mint_address: _,
        cpi_context_pubkey: _,
    } = test_setup().await;

    // Swap the compressed address to a random one (this should fail validation)
    // Keep the correct address_tree_pubkey (CMINT_ADDRESS_TREE) but provide wrong address
    let invalid_compressed_address = [42u8; 32];
    mint_action_inputs.compressed_mint_inputs.address = invalid_compressed_address;

    // Get the compressed token program instruction
    let ctoken_instruction =
        light_compressed_token_sdk::instructions::mint_action::instruction::mint_action_cpi_write(
            mint_action_inputs,
        )
        .expect("Failed to build mint action instruction");

    // Build wrapper program instruction
    let wrapper_ix_data = compressed_token_test::instruction::WriteToCpiContextMintAction {
        inputs: ctoken_instruction.data.clone(),
    };

    let wrapper_instruction = Instruction {
        program_id: WRAPPER_PROGRAM_ID,
        accounts: vec![AccountMeta::new_readonly(
            Pubkey::new_from_array(COMPRESSED_TOKEN_PROGRAM_ID),
            false,
        )]
        .into_iter()
        .chain(ctoken_instruction.accounts.clone())
        .collect(),
        data: wrapper_ix_data.data(),
    };

    // Execute wrapper instruction - should fail
    let result = rpc
        .create_and_send_transaction(
            &[wrapper_instruction],
            &payer.pubkey(),
            &[&payer, &mint_seed, &mint_authority],
        )
        .await;

    // Assert that the transaction failed with MintActionInvalidCompressedMintAddress error
    // Error code 103 = MintActionInvalidCompressedMintAddress
    assert_rpc_error(result, 0, 103).unwrap();
}

#[tokio::test]
#[serial]
async fn test_execute_cpi_context_invalid_tree_index() {
    let TestSetup {
        mut rpc,
        mint_action_inputs,
        payer,
        mint_seed,
        mint_authority,
        compressed_mint_address: _,
        cpi_context_pubkey,
    } = test_setup().await;

    // Now try to execute with invalid in_tree_index (should fail)
    // Build execute mode CPI context with invalid tree index
    let execute_cpi_context = light_ctoken_types::instructions::mint_action::CpiContext {
        set_context: false,
        first_set_context: false, // Execute mode
        in_tree_index: 5,         // Invalid! Should be 1
        in_queue_index: 0,
        out_queue_index: 0,
        token_out_queue_index: 0,
        assigned_account_index: 0,
        read_only_address_trees: [0; 4],
        address_tree_pubkey: light_ctoken_types::CMINT_ADDRESS_TREE,
    };

    // Get tree info for execute mode
    let tree_info = rpc.test_accounts.v2_state_trees[0];

    // Build MintActionInputs for execute mode
    let execute_inputs = MintActionInputs {
        compressed_mint_inputs: mint_action_inputs.compressed_mint_inputs.clone(),
        mint_seed: mint_seed.pubkey(),
        mint_bump: mint_action_inputs.mint_bump,
        create_mint: true,
        authority: mint_action_inputs.authority,
        payer: mint_action_inputs.payer,
        proof: Some(CompressedProof::default()),
        actions: vec![],
        address_tree_pubkey: Pubkey::new_from_array(light_ctoken_types::CMINT_ADDRESS_TREE),
        input_queue: None,
        output_queue: tree_info.output_queue,
        tokens_out_queue: None,
        token_pool: None,
    };

    let execute_instruction = create_mint_action_cpi(
        execute_inputs,
        Some(execute_cpi_context),
        Some(cpi_context_pubkey),
    )
    .expect("Failed to build execute instruction");

    let execute_wrapper_ix_data = compressed_token_test::instruction::ExecuteCpiContextMintAction {
        inputs: execute_instruction.data.clone(),
    };

    let execute_wrapper_instruction = Instruction {
        program_id: WRAPPER_PROGRAM_ID,
        accounts: vec![AccountMeta::new_readonly(
            Pubkey::new_from_array(COMPRESSED_TOKEN_PROGRAM_ID),
            false,
        )]
        .into_iter()
        .chain(execute_instruction.accounts.clone())
        .collect(),
        data: execute_wrapper_ix_data.data(),
    };

    // Execute wrapper instruction - should fail
    let result = rpc
        .create_and_send_transaction(
            &[execute_wrapper_instruction],
            &payer.pubkey(),
            &[&payer, &mint_seed, &mint_authority],
        )
        .await;

    // Assert that the transaction failed with MintActionInvalidCpiContextForCreateMint error
    // Error code 104 = MintActionInvalidCpiContextForCreateMint
    assert_rpc_error(result, 0, 104).unwrap();
}
