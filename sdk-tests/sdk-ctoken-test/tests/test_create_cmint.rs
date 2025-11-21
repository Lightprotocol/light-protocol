// Tests for CreateCMintAccountInfos (CreateCmint instruction)

mod shared;

use borsh::BorshSerialize;
use light_client::{indexer::Indexer, rpc::Rpc};
use light_compressed_token_sdk::compressed_token::mint_action::MintActionMetaConfig;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use native_ctoken_examples::{CreateCmintData, ID, MINT_SIGNER_SEED};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};

/// Test creating a compressed mint using CreateCMintAccountInfos::invoke()
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

    // Create instruction data for wrapper program
    let create_cmint_data = CreateCmintData {
        decimals,
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
    let account_metas = MintActionMetaConfig::new_create_mint(
        payer.pubkey(),
        mint_authority,
        mint_signer.pubkey(),
        address_tree.tree,
        output_queue,
    )
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

    let compressed_account = rpc
        .get_compressed_account(compression_address, None)
        .await
        .unwrap()
        .value;

    assert!(compressed_account.is_some(), "Compressed mint should exist");
}

/// Test creating a compressed mint with PDA mint signer using CreateCMintAccountInfos::invoke_signed()
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
        Pubkey::new_from_array(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID);

    // Use SDK helper to derive the compression address correctly
    let compression_address = light_compressed_token_sdk::ctoken::derive_compressed_mint_address(
        &mint_signer_pda,
        &address_tree.tree,
    );

    let mint_pda = light_compressed_token_sdk::ctoken::find_spl_mint_address(&mint_signer_pda).0;

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
    let create_cmint_data = CreateCmintData {
        decimals,
        address_merkle_tree_root_index: rpc_result.addresses[0].root_index,
        mint_authority,
        proof: rpc_result.proof.0.unwrap().into(),
        compression_address,
        mint: mint_pda,
        freeze_authority: None,
        extensions: None,
    };
    // Discriminator 12 = CreateCmintInvokeSigned
    let instruction_data = [vec![12u8], create_cmint_data.try_to_vec().unwrap()].concat();

    // Build accounts manually since SDK marks mint_signer as signer, but we need it as non-signer
    // for invoke_signed (the wrapper program signs via CPI)
    let default_pubkeys = light_compressed_token_sdk::utils::CTokenDefaultAccounts::default();
    let wrapper_accounts = vec![
        AccountMeta::new_readonly(compressed_token_program_id, false),
        AccountMeta::new_readonly(default_pubkeys.light_system_program, false),
        // mint_signer NOT marked as signer - program will sign via invoke_signed
        AccountMeta::new_readonly(mint_signer_pda, false),
        AccountMeta::new_readonly(mint_authority, true),
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(default_pubkeys.cpi_authority_pda, false),
        AccountMeta::new_readonly(default_pubkeys.registered_program_pda, false),
        AccountMeta::new_readonly(default_pubkeys.account_compression_authority, false),
        AccountMeta::new_readonly(default_pubkeys.account_compression_program, false),
        AccountMeta::new_readonly(default_pubkeys.system_program, false),
        AccountMeta::new(output_queue, false),
        AccountMeta::new(address_tree.tree, false),
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

    let compressed_account = rpc
        .get_compressed_account(compression_address, None)
        .await
        .unwrap()
        .value;

    assert!(compressed_account.is_some(), "Compressed mint should exist");
}
