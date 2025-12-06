// Tests for MintToCTokenCpi (MintToCtoken instruction)

mod shared;

use borsh::{BorshDeserialize, BorshSerialize};
use light_client::{indexer::Indexer, rpc::Rpc};
use light_ctoken_sdk::{
    compressed_token::mint_action::MintActionMetaConfig, ctoken::CTOKEN_PROGRAM_ID,
};
use light_program_test::{LightProgramTest, ProgramTestConfig};
use native_ctoken_examples::{
    CreateCmintData, CreateTokenAccountData, MintToCTokenData, ID, MINT_AUTHORITY_SEED,
    MINT_SIGNER_SEED,
};
use shared::setup_create_compressed_mint;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};

/// Test minting tokens to a ctoken account using MintToCTokenCpi::invoke()
#[tokio::test]
async fn test_mint_to_ctoken() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let mint_authority = payer.pubkey();

    // Setup: Create compressed mint directly (not via wrapper program)
    let (mint_pda, compression_address, _) =
        setup_create_compressed_mint(&mut rpc, &payer, mint_authority, 9, vec![]).await;

    let ctoken_account = Keypair::new();
    let owner = payer.pubkey();
    // Create a ctoken account to mint tokens to via wrapper program
    {
        let create_token_account_data = CreateTokenAccountData {
            owner,
            pre_pay_num_epochs: 2,
            lamports_per_write: 1,
        };
        let instruction_data =
            [vec![2u8], create_token_account_data.try_to_vec().unwrap()].concat();

        use light_ctoken_sdk::ctoken::{config_pda, rent_sponsor_pda};
        let config = config_pda();
        let rent_sponsor = rent_sponsor_pda();

        let instruction = Instruction {
            program_id: ID,
            accounts: vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(ctoken_account.pubkey(), true),
                AccountMeta::new_readonly(mint_pda, false),
                AccountMeta::new_readonly(config, false),
                AccountMeta::new_readonly(Pubkey::default(), false), // system_program
                AccountMeta::new(rent_sponsor, false),
                AccountMeta::new_readonly(CTOKEN_PROGRAM_ID, false), // token_program
            ],
            data: instruction_data,
        };

        rpc.create_and_send_transaction(
            &[instruction],
            &payer.pubkey(),
            &[&payer, &ctoken_account],
        )
        .await
        .unwrap();
    }

    // Get the compressed mint account to build CompressedMintWithContext
    let compressed_mint_account = rpc
        .get_compressed_account(compression_address, None)
        .await
        .unwrap()
        .value
        .expect("Compressed mint should exist");

    // Deserialize the compressed mint data
    use light_ctoken_interface::state::CompressedMint;
    let compressed_mint =
        CompressedMint::deserialize(&mut compressed_mint_account.data.unwrap().data.as_slice())
            .unwrap();

    let amount = 1_000_000_000u64; // 1 token with 9 decimals

    // Mint ctokens with test program.
    {
        // Get validity proof for the mint operation
        let rpc_result = rpc
            .get_validity_proof(vec![compressed_mint_account.hash], vec![], None)
            .await
            .unwrap()
            .value;

        // Build CompressedMintWithContext from the compressed account
        let compressed_mint_with_context =
            light_ctoken_interface::instructions::mint_action::CompressedMintWithContext {
                address: compression_address,
                leaf_index: compressed_mint_account.leaf_index,
                prove_by_index: true,
                root_index: rpc_result.accounts[0]
                    .root_index
                    .root_index()
                    .unwrap_or_default(), // Will be updated with validity proof
                mint: compressed_mint.try_into().unwrap(),
            };
        // Build instruction data for wrapper program
        let mint_to_data = MintToCTokenData {
            compressed_mint_inputs: compressed_mint_with_context.clone(),
            amount,
            mint_authority,
            proof: rpc_result.proof,
        };
        let wrapper_instruction_data = [vec![1u8], mint_to_data.try_to_vec().unwrap()].concat();

        // Build wrapper instruction with compressed token program as first account
        let compressed_token_program_id =
            Pubkey::new_from_array(light_ctoken_interface::COMPRESSED_TOKEN_PROGRAM_ID);

        let mut wrapper_accounts = vec![AccountMeta::new_readonly(
            compressed_token_program_id,
            false,
        )];
        let account_metas = MintActionMetaConfig::new(
            payer.pubkey(),
            mint_authority,
            compressed_mint_account.tree_info.tree,
            compressed_mint_account.tree_info.queue,
            compressed_mint_account.tree_info.queue,
        )
        .with_ctoken_accounts(vec![ctoken_account.pubkey()])
        .to_account_metas();
        wrapper_accounts.extend(account_metas);

        let instruction = Instruction {
            program_id: ID,
            accounts: wrapper_accounts,
            data: wrapper_instruction_data,
        };

        rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
            .await
            .unwrap();
    }

    // Verify tokens were minted to the ctoken account
    let ctoken_account_data = rpc
        .get_account(ctoken_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    // Parse the account data to verify balance
    use light_ctoken_interface::state::CToken;
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

/// Test minting tokens with PDA mint authority using MintToCTokenCpi::invoke_signed()
///
/// This test uses the wrapper program to:
/// 1. Create a compressed mint with PDA authority (discriminator 14 - CreateCmintWithPdaAuthority)
/// 2. Mint tokens using PDA authority (discriminator 13 - MintToCtokenInvokeSigned)
#[tokio::test]
async fn test_mint_to_ctoken_invoke_signed() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();

    // Derive both PDAs from our wrapper program
    let (mint_signer_pda, _) = Pubkey::find_program_address(&[MINT_SIGNER_SEED], &ID);
    let (mint_authority_pda, _) = Pubkey::find_program_address(&[MINT_AUTHORITY_SEED], &ID);

    let decimals = 9u8;
    let address_tree = rpc.get_address_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    // Derive compression address using the PDA mint_signer
    let compression_address = light_ctoken_sdk::ctoken::derive_cmint_compressed_address(
        &mint_signer_pda,
        &address_tree.tree,
    );

    let mint_pda = light_ctoken_sdk::ctoken::find_cmint_address(&mint_signer_pda).0;

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

    let compressed_token_program_id =
        Pubkey::new_from_array(light_ctoken_interface::COMPRESSED_TOKEN_PROGRAM_ID);
    let default_pubkeys = light_ctoken_sdk::utils::CTokenDefaultAccounts::default();

    // Step 1: Create compressed mint with PDA authority using wrapper program (discriminator 14)
    {
        let create_cmint_data = CreateCmintData {
            decimals,
            address_merkle_tree_root_index: rpc_result.addresses[0].root_index,
            mint_authority: mint_authority_pda, // Will be overridden by the handler
            proof: rpc_result.proof.0.unwrap(),
            compression_address,
            mint: mint_pda,
            freeze_authority: None,
            extensions: None,
        };
        // Discriminator 14 = CreateCmintWithPdaAuthority
        let wrapper_instruction_data =
            [vec![14u8], create_cmint_data.try_to_vec().unwrap()].concat();

        // Account order for CreateCmintWithPdaAuthority:
        // [0] compressed_token_program, [1] light_system_program, [2] mint_signer (PDA),
        // [3] authority (PDA), [4] fee_payer, [5] cpi_authority_pda, [6] registered_program_pda,
        // [7] account_compression_authority, [8] account_compression_program, [9] system_program,
        // [10] output_queue, [11] address_tree
        let wrapper_accounts = vec![
            AccountMeta::new_readonly(compressed_token_program_id, false),
            AccountMeta::new_readonly(default_pubkeys.light_system_program, false),
            AccountMeta::new_readonly(mint_signer_pda, false), // PDA - program signs
            AccountMeta::new(mint_authority_pda, false),       // writable PDA - program signs
            AccountMeta::new(payer.pubkey(), true),            // fee_payer
            AccountMeta::new_readonly(default_pubkeys.cpi_authority_pda, false),
            AccountMeta::new_readonly(default_pubkeys.registered_program_pda, false),
            AccountMeta::new_readonly(default_pubkeys.account_compression_authority, false),
            AccountMeta::new_readonly(default_pubkeys.account_compression_program, false),
            AccountMeta::new_readonly(default_pubkeys.system_program, false),
            AccountMeta::new(output_queue, false),
            AccountMeta::new(address_tree.tree, false),
        ];

        let create_mint_ix = Instruction {
            program_id: ID,
            accounts: wrapper_accounts,
            data: wrapper_instruction_data,
        };

        rpc.create_and_send_transaction(&[create_mint_ix], &payer.pubkey(), &[&payer])
            .await
            .unwrap();
    }

    let ctoken_account = Keypair::new();
    let owner = payer.pubkey();

    // Create a ctoken account to mint tokens to via wrapper program
    {
        let create_token_account_data = CreateTokenAccountData {
            owner,
            pre_pay_num_epochs: 2,
            lamports_per_write: 1,
        };
        let instruction_data =
            [vec![2u8], create_token_account_data.try_to_vec().unwrap()].concat();

        use light_ctoken_sdk::ctoken::{config_pda, rent_sponsor_pda};
        let config = config_pda();
        let rent_sponsor = rent_sponsor_pda();

        let instruction = Instruction {
            program_id: ID,
            accounts: vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(ctoken_account.pubkey(), true),
                AccountMeta::new_readonly(mint_pda, false),
                AccountMeta::new_readonly(config, false),
                AccountMeta::new_readonly(Pubkey::default(), false), // system_program
                AccountMeta::new(rent_sponsor, false),
                AccountMeta::new_readonly(CTOKEN_PROGRAM_ID, false),
            ],
            data: instruction_data,
        };

        rpc.create_and_send_transaction(
            &[instruction],
            &payer.pubkey(),
            &[&payer, &ctoken_account],
        )
        .await
        .unwrap();
    }

    // Get the compressed mint account to build CompressedMintWithContext
    let compressed_mint_account = rpc
        .get_compressed_account(compression_address, None)
        .await
        .unwrap()
        .value
        .expect("Compressed mint should exist");

    // Deserialize the compressed mint data
    use light_ctoken_interface::state::CompressedMint;
    let compressed_mint =
        CompressedMint::deserialize(&mut compressed_mint_account.data.unwrap().data.as_slice())
            .unwrap();

    let amount = 1_000_000_000u64; // 1 token with 9 decimals

    // Mint ctokens with PDA authority via invoke_signed
    {
        // Get validity proof for the mint operation
        let rpc_result = rpc
            .get_validity_proof(vec![compressed_mint_account.hash], vec![], None)
            .await
            .unwrap()
            .value;

        // Build CompressedMintWithContext from the compressed account
        let compressed_mint_with_context =
            light_ctoken_interface::instructions::mint_action::CompressedMintWithContext {
                address: compression_address,
                leaf_index: compressed_mint_account.leaf_index,
                prove_by_index: true,
                root_index: rpc_result.accounts[0]
                    .root_index
                    .root_index()
                    .unwrap_or_default(),
                mint: compressed_mint.try_into().unwrap(),
            };

        // Build instruction data for wrapper program
        let mint_to_data = MintToCTokenData {
            compressed_mint_inputs: compressed_mint_with_context.clone(),
            amount,
            mint_authority: mint_authority_pda,
            proof: rpc_result.proof,
        };
        // Discriminator 13 = MintToCtokenInvokeSigned
        let wrapper_instruction_data = [vec![13u8], mint_to_data.try_to_vec().unwrap()].concat();

        // Build accounts manually since SDK marks authority as signer, but we need it as non-signer
        // for invoke_signed (the wrapper program signs via CPI)
        let compressed_token_program_id =
            Pubkey::new_from_array(light_ctoken_interface::COMPRESSED_TOKEN_PROGRAM_ID);
        let default_pubkeys = light_ctoken_sdk::utils::CTokenDefaultAccounts::default();

        let wrapper_accounts = vec![
            AccountMeta::new_readonly(compressed_token_program_id, false),
            AccountMeta::new_readonly(default_pubkeys.light_system_program, false),
            // authority NOT marked as signer - program will sign via invoke_signed
            AccountMeta::new_readonly(mint_authority_pda, false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(default_pubkeys.cpi_authority_pda, false),
            AccountMeta::new_readonly(default_pubkeys.registered_program_pda, false),
            AccountMeta::new_readonly(default_pubkeys.account_compression_authority, false),
            AccountMeta::new_readonly(default_pubkeys.account_compression_program, false),
            AccountMeta::new_readonly(default_pubkeys.system_program, false),
            AccountMeta::new(compressed_mint_account.tree_info.queue, false), // output_queue
            AccountMeta::new(compressed_mint_account.tree_info.tree, false),  // state_tree
            AccountMeta::new(compressed_mint_account.tree_info.queue, false), // input_queue
            AccountMeta::new(ctoken_account.pubkey(), false),                 // ctoken_account
        ];
        let instruction = Instruction {
            program_id: ID,
            accounts: wrapper_accounts,
            data: wrapper_instruction_data,
        };

        // Note: only payer signs, the mint_authority PDA is signed by the program via invoke_signed
        rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
            .await
            .unwrap();
    }

    // Verify tokens were minted to the ctoken account
    let ctoken_account_data = rpc
        .get_account(ctoken_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    // Parse the account data to verify balance
    use light_ctoken_interface::state::CToken;
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
