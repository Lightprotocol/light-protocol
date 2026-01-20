// Tests for CreateMintCpi (CreateCmint instruction)

mod shared;

use borsh::BorshSerialize;
use light_client::{indexer::Indexer, rpc::Rpc};
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_token::{
    compressed_token::mint_action::MintActionMetaConfig,
    token::{config_pda, rent_sponsor_pda},
};
use light_token_interface::{
    instructions::extensions::{
        token_metadata::TokenMetadataInstructionData, ExtensionInstructionData,
    },
    state::AdditionalMetadata,
};
use native_ctoken_examples::{CreateCmintData, ID, MINT_SIGNER_SEED};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};

/// Test creating a compressed mint using CreateMintCpi::invoke()
#[tokio::test]
async fn test_create_compressed_mint() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", ID)]),
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
        Pubkey::new_from_array(light_token_interface::LIGHT_TOKEN_PROGRAM_ID);

    // Use SDK helper to derive the compression address correctly
    let compression_address = light_token::token::derive_mint_compressed_address(
        &mint_signer.pubkey(),
        &address_tree.tree,
    );

    let (mint_pda, mint_bump) = light_token::token::find_mint_address(&mint_signer.pubkey());

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

    // Create instruction data for wrapper program with TokenMetadata extension
    let create_mint_data = CreateCmintData {
        decimals,
        address_merkle_tree_root_index: rpc_result.addresses[0].root_index,
        mint_authority,
        proof: rpc_result.proof.0.unwrap(),
        compression_address,
        mint: mint_pda,
        bump: mint_bump,
        freeze_authority: None,
        extensions: Some(vec![ExtensionInstructionData::TokenMetadata(
            TokenMetadataInstructionData {
                update_authority: Some(payer.pubkey().to_bytes().into()),
                name: b"Test Token".to_vec(),
                symbol: b"TEST".to_vec(),
                uri: b"https://example.com/metadata.json".to_vec(),
                additional_metadata: Some(vec![
                    AdditionalMetadata {
                        key: b"test1".to_vec(),
                        value: b"value1".to_vec(),
                    },
                    AdditionalMetadata {
                        key: b"test2".to_vec(),
                        value: b"value2".to_vec(),
                    },
                ]),
            },
        )]),
        rent_payment: 16,
        write_top_up: 766,
    };
    let instruction_data = [vec![0u8], create_mint_data.try_to_vec().unwrap()].concat();

    // Add compressed token program as first account for CPI, then all SDK-generated accounts
    let mut wrapper_accounts = vec![AccountMeta::new_readonly(
        compressed_token_program_id,
        false,
    )];
    let account_metas = MintActionMetaConfig::new_create_mint(
        payer.pubkey(),
        mint_authority,
        mint_signer.pubkey(),
        address_tree.tree,
        output_queue,
    )
    .with_compressible_mint(mint_pda, config_pda(), rent_sponsor_pda())
    .to_account_metas();
    wrapper_accounts.extend(account_metas);

    let instruction = Instruction {
        program_id: ID,
        accounts: wrapper_accounts,
        data: instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &mint_signer])
        .await
        .unwrap();

    // Verify the Mint Solana account was created (CreateMint now decompresses automatically)
    let mint_account = rpc.get_account(mint_pda).await.unwrap();
    assert!(mint_account.is_some(), "Mint Solana account should exist");
}

/// Test creating a compressed mint with PDA mint signer using CreateMintCpi::invoke_signed()
#[tokio::test]
async fn test_create_compressed_mint_invoke_signed() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let decimals = 9u8;
    let mint_authority = payer.pubkey();

    // Derive the PDA mint signer from our program
    let (mint_signer_pda, _bump) = Pubkey::find_program_address(&[MINT_SIGNER_SEED], &ID);

    let address_tree = rpc.get_address_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    let compressed_token_program_id =
        Pubkey::new_from_array(light_token_interface::LIGHT_TOKEN_PROGRAM_ID);

    // Use SDK helper to derive the compression address correctly
    let compression_address =
        light_token::token::derive_mint_compressed_address(&mint_signer_pda, &address_tree.tree);

    let (mint_pda, mint_bump) = light_token::token::find_mint_address(&mint_signer_pda);

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

    // Create instruction data for wrapper program
    let create_mint_data = CreateCmintData {
        decimals,
        address_merkle_tree_root_index: rpc_result.addresses[0].root_index,
        mint_authority,
        proof: rpc_result.proof.0.unwrap(),
        compression_address,
        mint: mint_pda,
        bump: mint_bump,
        freeze_authority: None,
        extensions: None,
        rent_payment: 16,
        write_top_up: 766,
    };
    // Discriminator 12 = CreateCmintInvokeSigned
    let instruction_data = [vec![12u8], create_mint_data.try_to_vec().unwrap()].concat();

    // Build accounts manually since SDK marks mint_signer as signer, but we need it as non-signer
    // for invoke_signed (the wrapper program signs via CPI)
    // Account order matches MintActionMetaConfig::to_account_metas() with mint_signer as non-signer
    let system_accounts = light_token::token::SystemAccounts::default();
    let wrapper_accounts = vec![
        AccountMeta::new_readonly(compressed_token_program_id, false), // [0] compressed_token_program
        AccountMeta::new_readonly(system_accounts.light_system_program, false), // [1] light_system_program
        // mint_signer NOT marked as signer - program will sign via invoke_signed
        AccountMeta::new_readonly(mint_signer_pda, false), // [2] mint_signer (PDA)
        AccountMeta::new_readonly(payer.pubkey(), true),   // [3] authority (signer)
        AccountMeta::new_readonly(config_pda(), false),    // [4] compressible_config
        AccountMeta::new(mint_pda, false),                 // [5] mint
        AccountMeta::new(rent_sponsor_pda(), false),       // [6] rent_sponsor
        AccountMeta::new(payer.pubkey(), true),            // [7] fee_payer (signer)
        AccountMeta::new_readonly(system_accounts.cpi_authority_pda, false), // [8]
        AccountMeta::new_readonly(system_accounts.registered_program_pda, false), // [9]
        AccountMeta::new_readonly(system_accounts.account_compression_authority, false), // [10]
        AccountMeta::new_readonly(system_accounts.account_compression_program, false), // [11]
        AccountMeta::new_readonly(system_accounts.system_program, false), // [12]
        AccountMeta::new(output_queue, false),             // [13]
        AccountMeta::new(address_tree.tree, false),        // [14]
    ];

    let instruction = Instruction {
        program_id: ID,
        accounts: wrapper_accounts,
        data: instruction_data,
    };

    // Note: only payer signs, the mint_signer PDA is signed by the program via invoke_signed
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify the Mint Solana account was created (CreateMint now decompresses automatically)
    let mint_account = rpc.get_account(mint_pda).await.unwrap();
    assert!(mint_account.is_some(), "Mint Solana account should exist");
}
