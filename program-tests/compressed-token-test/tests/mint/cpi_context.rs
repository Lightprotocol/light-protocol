use anchor_lang::InstructionData;
use compressed_token_test::ID as WRAPPER_PROGRAM_ID;
use light_client::indexer::Indexer;
use light_compressed_account::instruction_data::traits::LightInstructionData;
use light_compressed_token_sdk::compressed_token::{
    create_compressed_mint::{derive_compressed_mint_address, find_spl_mint_address},
    mint_action::{
        get_mint_action_instruction_account_metas_cpi_write, MintActionMetaConfig,
        MintActionMetaConfigCpiWrite,
    },
};
use light_ctoken_types::{
    instructions::mint_action::{
        CompressedMintInstructionData, CompressedMintWithContext, CpiContext,
        MintActionCompressedInstructionData,
    },
    state::CompressedMintMetadata,
    CMINT_ADDRESS_TREE, COMPRESSED_TOKEN_PROGRAM_ID,
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
    compressed_mint_inputs: CompressedMintWithContext,
    payer: Keypair,
    mint_seed: Keypair,
    mint_authority: Keypair,
    compressed_mint_address: [u8; 32],
    cpi_context_pubkey: Pubkey,
    address_tree: Pubkey,
    address_tree_index: u8,
    output_queue: Pubkey,
    output_queue_index: u8,
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

    // Get CPI context and state tree info from test accounts
    let tree_info = rpc.test_accounts.v2_state_trees[0];
    let cpi_context_pubkey = tree_info.cpi_context;
    let output_queue = tree_info.output_queue;

    // 2. Create mint parameters
    let mint_seed = Keypair::new();
    let mint_authority = Keypair::new();
    let freeze_authority = Pubkey::new_unique();
    let decimals = 9u8;

    // Derive addresses
    let compressed_mint_address =
        derive_compressed_mint_address(&mint_seed.pubkey(), &address_tree);
    let (spl_mint_pda, _) = find_spl_mint_address(&mint_seed.pubkey());

    // 3. Build compressed mint inputs
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

    TestSetup {
        rpc,
        compressed_mint_inputs,
        payer,
        mint_seed,
        mint_authority,
        compressed_mint_address,
        cpi_context_pubkey,
        address_tree,
        address_tree_index: 1,
        output_queue,
        output_queue_index: 0,
    }
}

#[tokio::test]
#[serial]
async fn test_write_to_cpi_context_create_mint() {
    let TestSetup {
        mut rpc,
        compressed_mint_inputs,
        payer,
        mint_seed,
        mint_authority,
        compressed_mint_address,
        cpi_context_pubkey,
        address_tree,
        address_tree_index,
        output_queue: _,
        output_queue_index,
    } = test_setup().await;

    // Build instruction data using new builder API
    let instruction_data = MintActionCompressedInstructionData::new_mint(
        compressed_mint_inputs.address,
        compressed_mint_inputs.root_index,
        CompressedProof::default(),
        compressed_mint_inputs.mint.clone(),
    )
    .with_cpi_context(CpiContext {
        set_context: false,
        first_set_context: true,
        in_tree_index: address_tree_index,
        in_queue_index: 0,
        out_queue_index: output_queue_index,
        token_out_queue_index: 0,
        assigned_account_index: 0,
        read_only_address_trees: [0; 4],
        address_tree_pubkey: address_tree.to_bytes(),
    });

    // Build account metas using helper
    let config = MintActionMetaConfigCpiWrite {
        fee_payer: payer.pubkey(),
        mint_signer: Some(mint_seed.pubkey()),
        authority: mint_authority.pubkey(),
        cpi_context: cpi_context_pubkey,
    };

    let account_metas = get_mint_action_instruction_account_metas_cpi_write(config);

    // Serialize instruction data
    let data = instruction_data
        .data()
        .expect("Failed to serialize instruction data");

    // Build compressed token instruction
    let ctoken_instruction = Instruction {
        program_id: Pubkey::new_from_array(COMPRESSED_TOKEN_PROGRAM_ID),
        accounts: account_metas,
        data: data.clone(),
    };

    // Build wrapper instruction using Anchor's InstructionData
    let wrapper_ix_data =
        compressed_token_test::instruction::WriteToCpiContextMintAction { inputs: data };

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
        compressed_mint_inputs,
        payer,
        mint_seed,
        mint_authority,
        compressed_mint_address: _,
        cpi_context_pubkey,
        address_tree: _,
        address_tree_index,
        output_queue: _,
        output_queue_index,
    } = test_setup().await;

    // Swap the address tree pubkey to a random one (this should fail validation)
    let invalid_address_tree = Pubkey::new_unique();

    // Build instruction data with invalid address tree
    let instruction_data = MintActionCompressedInstructionData::new_mint(
        compressed_mint_inputs.address,
        compressed_mint_inputs.root_index,
        CompressedProof::default(),
        compressed_mint_inputs.mint.clone(),
    )
    .with_cpi_context(CpiContext {
        set_context: false,
        first_set_context: true,
        in_tree_index: address_tree_index,
        in_queue_index: 0,
        out_queue_index: output_queue_index,
        token_out_queue_index: 0,
        assigned_account_index: 0,
        read_only_address_trees: [0; 4],
        address_tree_pubkey: invalid_address_tree.to_bytes(),
    });

    // Build account metas using helper
    let config = MintActionMetaConfigCpiWrite {
        fee_payer: payer.pubkey(),
        mint_signer: Some(mint_seed.pubkey()),
        authority: mint_authority.pubkey(),
        cpi_context: cpi_context_pubkey,
    };

    let account_metas = get_mint_action_instruction_account_metas_cpi_write(config);

    // Serialize instruction data
    let data = instruction_data
        .data()
        .expect("Failed to serialize instruction data");

    // Build compressed token instruction
    let ctoken_instruction = Instruction {
        program_id: Pubkey::new_from_array(COMPRESSED_TOKEN_PROGRAM_ID),
        accounts: account_metas,
        data: data.clone(),
    };

    // Build wrapper instruction
    let wrapper_ix_data =
        compressed_token_test::instruction::WriteToCpiContextMintAction { inputs: data };

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
        compressed_mint_inputs,
        payer,
        mint_seed,
        mint_authority,
        compressed_mint_address: _,
        cpi_context_pubkey,
        address_tree,
        address_tree_index,
        output_queue: _,
        output_queue_index,
    } = test_setup().await;

    // Swap the compressed address to a random one (this should fail validation)
    // Keep the correct address_tree_pubkey but provide wrong address
    let invalid_compressed_address = [42u8; 32];

    // Build instruction data with invalid compressed address
    let instruction_data = MintActionCompressedInstructionData::new_mint(
        invalid_compressed_address,
        compressed_mint_inputs.root_index,
        CompressedProof::default(),
        compressed_mint_inputs.mint.clone(),
    )
    .with_cpi_context(CpiContext {
        set_context: false,
        first_set_context: true,
        in_tree_index: address_tree_index,
        in_queue_index: 0,
        out_queue_index: output_queue_index,
        token_out_queue_index: 0,
        assigned_account_index: 0,
        read_only_address_trees: [0; 4],
        address_tree_pubkey: address_tree.to_bytes(),
    });

    // Build account metas using helper
    let config = MintActionMetaConfigCpiWrite {
        fee_payer: payer.pubkey(),
        mint_signer: Some(mint_seed.pubkey()),
        authority: mint_authority.pubkey(),
        cpi_context: cpi_context_pubkey,
    };

    let account_metas = get_mint_action_instruction_account_metas_cpi_write(config);

    // Serialize instruction data
    let data = instruction_data
        .data()
        .expect("Failed to serialize instruction data");

    // Build compressed token instruction
    let ctoken_instruction = Instruction {
        program_id: Pubkey::new_from_array(COMPRESSED_TOKEN_PROGRAM_ID),
        accounts: account_metas,
        data: data.clone(),
    };

    // Build wrapper instruction
    let wrapper_ix_data =
        compressed_token_test::instruction::WriteToCpiContextMintAction { inputs: data };

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
        compressed_mint_inputs,
        payer,
        mint_seed,
        mint_authority,
        compressed_mint_address: _,
        cpi_context_pubkey,
        address_tree: _,
        address_tree_index: _,
        output_queue,
        output_queue_index: _,
    } = test_setup().await;

    // Build execute mode CPI context with invalid tree index
    let execute_cpi_context = CpiContext {
        set_context: false,
        first_set_context: false, // Execute mode
        in_tree_index: 5,         // Invalid! Should be 1
        in_queue_index: 0,
        out_queue_index: 0,
        token_out_queue_index: 0,
        assigned_account_index: 0,
        read_only_address_trees: [0; 4],
        address_tree_pubkey: CMINT_ADDRESS_TREE,
    };

    // Build instruction data for execute mode - must mark as create_mint
    let instruction_data = MintActionCompressedInstructionData::new_mint(
        compressed_mint_inputs.address,
        compressed_mint_inputs.root_index,
        CompressedProof::default(),
        compressed_mint_inputs.mint.clone(),
    )
    .with_cpi_context(execute_cpi_context);

    // Build account metas using regular MintActionMetaConfig for execute mode
    let mut config = MintActionMetaConfig::new_create_mint(
        payer.pubkey(),
        mint_authority.pubkey(),
        mint_seed.pubkey(),
        Pubkey::new_from_array(CMINT_ADDRESS_TREE),
        output_queue,
    );

    // Set CPI context for execute mode
    config.cpi_context = Some(cpi_context_pubkey);

    let account_metas = config.to_account_metas();

    // Serialize instruction data
    let data = instruction_data
        .data()
        .expect("Failed to serialize instruction data");

    // Build compressed token instruction
    let execute_instruction = Instruction {
        program_id: Pubkey::new_from_array(COMPRESSED_TOKEN_PROGRAM_ID),
        accounts: account_metas,
        data: data.clone(),
    };

    // Build wrapper instruction
    let execute_wrapper_ix_data =
        compressed_token_test::instruction::ExecuteCpiContextMintAction { inputs: data };

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
