use anchor_lang::InstructionData;
use compressed_token_test::ID as WRAPPER_PROGRAM_ID;
use light_compressed_account::instruction_data::traits::LightInstructionData;
use light_compressed_token_sdk::compressed_token::{
    create_compressed_mint::{derive_mint_compressed_address, find_mint_address},
    mint_action::{
        get_mint_action_instruction_account_metas_cpi_write, MintActionMetaConfig,
        MintActionMetaConfigCpiWrite,
    },
};
use light_compressible::config::CompressibleConfig;
use light_program_test::{utils::assert::assert_rpc_error, LightProgramTest, ProgramTestConfig};
use light_test_utils::{assert_mint_creation_fee, Rpc};
use light_token_interface::{
    instructions::mint_action::{
        CpiContext, DecompressMintAction, MintActionCompressedInstructionData, MintInstructionData,
        MintToAction, MintWithContext,
    },
    state::MintMetadata,
    LIGHT_TOKEN_PROGRAM_ID, MINT_ADDRESS_TREE,
};
use light_verifier::CompressedProof;
use serial_test::serial;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

struct TestSetup {
    rpc: LightProgramTest,
    compressed_mint_inputs: MintWithContext,
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
        derive_mint_compressed_address(&mint_seed.pubkey(), &address_tree);
    let (spl_mint_pda, _) = find_mint_address(&mint_seed.pubkey());

    // 3. Build compressed mint inputs
    let (_, bump) = find_mint_address(&mint_seed.pubkey());
    let compressed_mint_inputs = MintWithContext {
        leaf_index: 0,
        prove_by_index: false,
        root_index: 0,
        address: compressed_mint_address,
        mint: Some(MintInstructionData {
            supply: 0,
            decimals,
            metadata: MintMetadata {
                version: 3,
                mint_decompressed: false,
                mint: spl_mint_pda.into(),
                mint_signer: mint_seed.pubkey().to_bytes(),
                bump,
            },
            mint_authority: Some(mint_authority.pubkey().into()),
            freeze_authority: Some(freeze_authority.into()),
            extensions: None,
        }),
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
        compressed_mint_address: _,
        cpi_context_pubkey,
        address_tree,
        address_tree_index,
        output_queue: _,
        output_queue_index,
    } = test_setup().await;

    let rent_sponsor = rpc.test_accounts.funding_pool_config.rent_sponsor_pda;
    let rent_sponsor_before = rpc
        .get_account(rent_sponsor)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // Build instruction data using new builder API
    let instruction_data = MintActionCompressedInstructionData::new_mint(
        compressed_mint_inputs.root_index,
        CompressedProof::default(),
        compressed_mint_inputs.mint.clone().unwrap(),
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
        rent_sponsor: Some(rent_sponsor),
        cpi_context: cpi_context_pubkey,
    };

    let account_metas = get_mint_action_instruction_account_metas_cpi_write(config);

    // Serialize instruction data
    let data = instruction_data
        .data()
        .expect("Failed to serialize instruction data");

    // Build compressed token instruction
    let ctoken_instruction = Instruction {
        program_id: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        accounts: account_metas,
        data: data.clone(),
    };

    // Build wrapper instruction using Anchor's InstructionData
    let wrapper_ix_data =
        compressed_token_test::instruction::WriteToCpiContextMintAction { inputs: data };

    let wrapper_instruction = Instruction {
        program_id: WRAPPER_PROGRAM_ID,
        accounts: vec![AccountMeta::new_readonly(
            Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
            false,
        )]
        .into_iter()
        .chain(ctoken_instruction.accounts.clone())
        .collect(),
        data: wrapper_ix_data.data(),
    };

    // create_mint + write_to_cpi_context is allowed. The mint creation fee is charged
    // in write mode against the hardcoded RENT_SPONSOR_V1 constant.
    rpc.create_and_send_transaction(
        &[wrapper_instruction],
        &payer.pubkey(),
        &[&payer, &mint_seed, &mint_authority],
    )
    .await
    .expect("create_mint + write_to_cpi_context should succeed");

    let rent_sponsor_after = rpc
        .get_account(rent_sponsor)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    assert_mint_creation_fee(rent_sponsor_before, rent_sponsor_after);
}

#[tokio::test]
#[serial]
async fn test_write_to_cpi_context_create_mint_invalid_rent_sponsor() {
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

    // Use a random pubkey as rent_sponsor (not the valid RENT_SPONSOR_V1)
    let invalid_rent_sponsor = Pubkey::new_unique();

    // Build instruction data
    let instruction_data = MintActionCompressedInstructionData::new_mint(
        compressed_mint_inputs.root_index,
        CompressedProof::default(),
        compressed_mint_inputs.mint.clone().unwrap(),
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

    // Build account metas with invalid rent_sponsor
    let config = MintActionMetaConfigCpiWrite {
        fee_payer: payer.pubkey(),
        mint_signer: Some(mint_seed.pubkey()),
        authority: mint_authority.pubkey(),
        rent_sponsor: Some(invalid_rent_sponsor),
        cpi_context: cpi_context_pubkey,
    };

    let account_metas = get_mint_action_instruction_account_metas_cpi_write(config);

    // Serialize instruction data
    let data = instruction_data
        .data()
        .expect("Failed to serialize instruction data");

    // Build compressed token instruction
    let ctoken_instruction = Instruction {
        program_id: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        accounts: account_metas,
        data: data.clone(),
    };

    // Build wrapper instruction
    let wrapper_ix_data =
        compressed_token_test::instruction::WriteToCpiContextMintAction { inputs: data };

    let wrapper_instruction = Instruction {
        program_id: WRAPPER_PROGRAM_ID,
        accounts: vec![AccountMeta::new_readonly(
            Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
            false,
        )]
        .into_iter()
        .chain(ctoken_instruction.accounts.clone())
        .collect(),
        data: wrapper_ix_data.data(),
    };

    // Should fail with InvalidRentSponsor because rent_sponsor doesn't match RENT_SPONSOR_V1
    // Error code 6100 = InvalidRentSponsor
    let result = rpc
        .create_and_send_transaction(
            &[wrapper_instruction],
            &payer.pubkey(),
            &[&payer, &mint_seed, &mint_authority],
        )
        .await;

    assert_rpc_error(result, 0, 6099).unwrap();
}

#[tokio::test]
#[serial]
async fn test_write_to_cpi_context_create_mint_missing_rent_sponsor() {
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

    // Build instruction data with create_mint
    let instruction_data = MintActionCompressedInstructionData::new_mint(
        compressed_mint_inputs.root_index,
        CompressedProof::default(),
        compressed_mint_inputs.mint.clone().unwrap(),
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

    // Build account metas WITHOUT rent_sponsor (None).
    // The program expects rent_sponsor when create_mint is true in write mode,
    // so not providing it will cause the account iterator to misparse accounts.
    let config = MintActionMetaConfigCpiWrite {
        fee_payer: payer.pubkey(),
        mint_signer: Some(mint_seed.pubkey()),
        authority: mint_authority.pubkey(),
        rent_sponsor: None,
        cpi_context: cpi_context_pubkey,
    };

    let account_metas = get_mint_action_instruction_account_metas_cpi_write(config);

    // Serialize instruction data
    let data = instruction_data
        .data()
        .expect("Failed to serialize instruction data");

    // Build compressed token instruction
    let ctoken_instruction = Instruction {
        program_id: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        accounts: account_metas,
        data: data.clone(),
    };

    // Build wrapper instruction
    let wrapper_ix_data =
        compressed_token_test::instruction::WriteToCpiContextMintAction { inputs: data };

    let wrapper_instruction = Instruction {
        program_id: WRAPPER_PROGRAM_ID,
        accounts: vec![AccountMeta::new_readonly(
            Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
            false,
        )]
        .into_iter()
        .chain(ctoken_instruction.accounts.clone())
        .collect(),
        data: wrapper_ix_data.data(),
    };

    // Should fail - when rent_sponsor is missing, the account iterator shifts:
    // fee_payer is parsed as rent_sponsor, then CpiContextLightSystemAccounts
    // runs out of accounts. Error 20009 is from the account iterator.
    let result = rpc
        .create_and_send_transaction(
            &[wrapper_instruction],
            &payer.pubkey(),
            &[&payer, &mint_seed, &mint_authority],
        )
        .await;

    // Error from the account iterator when it runs out of accounts due to
    // fee_payer being parsed as rent_sponsor (account shift), leaving the
    // iterator one account short. Defined in light-account-checks error codes.
    const ACCOUNT_ITERATOR_EXHAUSTED: u32 = 20009;
    assert_rpc_error(result, 0, ACCOUNT_ITERATOR_EXHAUSTED).unwrap();
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
        address_tree_pubkey: MINT_ADDRESS_TREE,
    };

    // Build instruction data for execute mode - must mark as create_mint
    let instruction_data = MintActionCompressedInstructionData::new_mint(
        compressed_mint_inputs.root_index,
        CompressedProof::default(),
        compressed_mint_inputs.mint.clone().unwrap(),
    )
    .with_cpi_context(execute_cpi_context);

    // Build account metas using regular MintActionMetaConfig for execute mode
    let rent_sponsor = rpc.test_accounts.funding_pool_config.rent_sponsor_pda;
    let compressible_config = CompressibleConfig::light_token_v1_config_pda();
    let mut config = MintActionMetaConfig::new_create_mint(
        payer.pubkey(),
        mint_authority.pubkey(),
        mint_seed.pubkey(),
        Pubkey::new_from_array(MINT_ADDRESS_TREE),
        output_queue,
        compressible_config,
        rent_sponsor,
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
        program_id: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        accounts: account_metas,
        data: data.clone(),
    };

    // Build wrapper instruction
    let execute_wrapper_ix_data =
        compressed_token_test::instruction::ExecuteCpiContextMintAction { inputs: data };

    let execute_wrapper_instruction = Instruction {
        program_id: WRAPPER_PROGRAM_ID,
        accounts: vec![AccountMeta::new_readonly(
            Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
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
    // Error code 6104 = MintActionInvalidCpiContextForCreateMint
    assert_rpc_error(result, 0, 6104).unwrap();
}

#[tokio::test]
#[serial]
async fn test_write_to_cpi_context_decompressed_mint_fails() {
    let TestSetup {
        mut rpc,
        compressed_mint_inputs: _,
        payer,
        mint_seed: _,
        mint_authority,
        compressed_mint_address,
        cpi_context_pubkey,
        address_tree,
        address_tree_index,
        output_queue: _,
        output_queue_index,
    } = test_setup().await;

    // Build instruction data with mint = None (simulates decompressed mint)
    // This triggers mint_decompressed = true in AccountsConfig
    let mint_with_context = MintWithContext {
        leaf_index: 0,
        prove_by_index: false,
        root_index: 0,
        address: compressed_mint_address,
        mint: None,
    };

    let instruction_data = MintActionCompressedInstructionData::new(mint_with_context, None)
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

    // Build account metas for CPI write mode
    let config = MintActionMetaConfigCpiWrite {
        fee_payer: payer.pubkey(),
        mint_signer: None,
        authority: mint_authority.pubkey(),
        rent_sponsor: None,
        cpi_context: cpi_context_pubkey,
    };

    let account_metas = get_mint_action_instruction_account_metas_cpi_write(config);

    // Serialize instruction data
    let data = instruction_data
        .data()
        .expect("Failed to serialize instruction data");

    // Build compressed token instruction
    let ctoken_instruction = Instruction {
        program_id: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        accounts: account_metas,
        data: data.clone(),
    };

    // Build wrapper instruction
    let wrapper_ix_data =
        compressed_token_test::instruction::WriteToCpiContextMintAction { inputs: data };

    let wrapper_instruction = Instruction {
        program_id: WRAPPER_PROGRAM_ID,
        accounts: vec![AccountMeta::new_readonly(
            Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
            false,
        )]
        .into_iter()
        .chain(ctoken_instruction.accounts.clone())
        .collect(),
        data: wrapper_ix_data.data(),
    };

    // Execute wrapper instruction - should fail with CpiContextSetNotUsable
    let result = rpc
        .create_and_send_transaction(
            &[wrapper_instruction],
            &payer.pubkey(),
            &[&payer, &mint_authority],
        )
        .await;

    // Assert error code 6035 = CpiContextSetNotUsable
    // "Decompress mint not allowed when writing to cpi context"
    assert_rpc_error(result, 0, 6035).unwrap();
}

#[tokio::test]
#[serial]
async fn test_write_to_cpi_context_mint_to_ctoken_fails() {
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

    // Build instruction data for create mint with MintToCToken action
    // MintToCToken is not allowed when writing to CPI context
    let instruction_data = MintActionCompressedInstructionData::new_mint(
        compressed_mint_inputs.root_index,
        CompressedProof::default(),
        compressed_mint_inputs.mint.clone().unwrap(),
    )
    .with_mint_to(MintToAction {
        account_index: 0,
        amount: 1000,
    })
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

    // Build account metas for CPI write mode
    let config = MintActionMetaConfigCpiWrite {
        fee_payer: payer.pubkey(),
        mint_signer: Some(mint_seed.pubkey()),
        authority: mint_authority.pubkey(),
        rent_sponsor: None,
        cpi_context: cpi_context_pubkey,
    };

    let account_metas = get_mint_action_instruction_account_metas_cpi_write(config);

    // Serialize instruction data
    let data = instruction_data
        .data()
        .expect("Failed to serialize instruction data");

    // Build compressed token instruction
    let ctoken_instruction = Instruction {
        program_id: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        accounts: account_metas,
        data: data.clone(),
    };

    // Build wrapper instruction
    let wrapper_ix_data =
        compressed_token_test::instruction::WriteToCpiContextMintAction { inputs: data };

    let wrapper_instruction = Instruction {
        program_id: WRAPPER_PROGRAM_ID,
        accounts: vec![AccountMeta::new_readonly(
            Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
            false,
        )]
        .into_iter()
        .chain(ctoken_instruction.accounts.clone())
        .collect(),
        data: wrapper_ix_data.data(),
    };

    // Execute wrapper instruction - should fail with CpiContextSetNotUsable
    let result = rpc
        .create_and_send_transaction(
            &[wrapper_instruction],
            &payer.pubkey(),
            &[&payer, &mint_seed, &mint_authority],
        )
        .await;

    // Assert error code 6035 = CpiContextSetNotUsable
    // "Mint to ctokens not allowed when writing to cpi context"
    assert_rpc_error(result, 0, 6035).unwrap();
}

#[tokio::test]
#[serial]
async fn test_write_to_cpi_context_decompress_mint_action_fails() {
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

    // Build instruction data for create mint with DecompressMint action
    // DecompressMint is not allowed when writing to CPI context
    let instruction_data = MintActionCompressedInstructionData::new_mint(
        compressed_mint_inputs.root_index,
        CompressedProof::default(),
        compressed_mint_inputs.mint.clone().unwrap(),
    )
    .with_decompress_mint(DecompressMintAction {
        rent_payment: 2,
        write_top_up: 1000,
    })
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

    // Build account metas for CPI write mode
    let config = MintActionMetaConfigCpiWrite {
        fee_payer: payer.pubkey(),
        mint_signer: Some(mint_seed.pubkey()),
        authority: mint_authority.pubkey(),
        rent_sponsor: None,
        cpi_context: cpi_context_pubkey,
    };

    let account_metas = get_mint_action_instruction_account_metas_cpi_write(config);

    // Serialize instruction data
    let data = instruction_data
        .data()
        .expect("Failed to serialize instruction data");

    // Build compressed token instruction
    let ctoken_instruction = Instruction {
        program_id: Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
        accounts: account_metas,
        data: data.clone(),
    };

    // Build wrapper instruction
    let wrapper_ix_data =
        compressed_token_test::instruction::WriteToCpiContextMintAction { inputs: data };

    let wrapper_instruction = Instruction {
        program_id: WRAPPER_PROGRAM_ID,
        accounts: vec![AccountMeta::new_readonly(
            Pubkey::new_from_array(LIGHT_TOKEN_PROGRAM_ID),
            false,
        )]
        .into_iter()
        .chain(ctoken_instruction.accounts.clone())
        .collect(),
        data: wrapper_ix_data.data(),
    };

    // Execute wrapper instruction - should fail with CpiContextSetNotUsable
    let result = rpc
        .create_and_send_transaction(
            &[wrapper_instruction],
            &payer.pubkey(),
            &[&payer, &mint_seed, &mint_authority],
        )
        .await;

    // Assert error code 6035 = CpiContextSetNotUsable
    // "Decompress mint not allowed when writing to cpi context"
    assert_rpc_error(result, 0, 6035).unwrap();
}
