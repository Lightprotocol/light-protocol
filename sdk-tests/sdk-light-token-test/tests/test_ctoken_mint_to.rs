// Tests for CTokenMintToCpi invoke() and invoke_signed()

mod shared;

use borsh::{BorshDeserialize, BorshSerialize};
use light_client::rpc::Rpc;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use light_token_interface::state::Token;
use native_ctoken_examples::{InstructionType, MintToData, ID, MINT_AUTHORITY_SEED};
use shared::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signer::Signer,
};

/// Test minting to Light Token using CTokenMintToCpi::invoke()
#[tokio::test]
async fn test_ctoken_mint_to_invoke() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("native_ctoken_examples", ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create a decompressed mint with an ATA for the payer with 0 tokens
    let (mint_pda, _compression_address, ata_pubkeys) =
        setup_create_compressed_mint_with_freeze_authority(
            &mut rpc,
            &payer,
            payer.pubkey(), // mint authority is payer
            None,
            9,
            vec![(0, payer.pubkey())], // Start with 0 tokens
        )
        .await;

    let ata = ata_pubkeys[0];
    let mint_amount = 500u64;

    // Get initial state
    let ata_account_before = rpc.get_account(ata).await.unwrap().unwrap();
    let ctoken_before = Token::deserialize(&mut &ata_account_before.data[..]).unwrap();

    // Build mint instruction via wrapper program
    let mut instruction_data = vec![InstructionType::CTokenMintToInvoke as u8];
    let mint_data = MintToData {
        amount: mint_amount,
    };
    mint_data.serialize(&mut instruction_data).unwrap();

    let ctoken_program = Pubkey::from(LIGHT_TOKEN_PROGRAM_ID);
    let system_program = Pubkey::default();
    let instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(mint_pda, false),                // cmint
            AccountMeta::new(ata, false),                     // destination
            AccountMeta::new(payer.pubkey(), true), // authority (signer, writable for top-up)
            AccountMeta::new_readonly(system_program, false), // system_program
            AccountMeta::new_readonly(ctoken_program, false), // ctoken_program
        ],
        data: instruction_data,
    };

    // Execute the mint instruction
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify with single assert_eq
    let ata_account_after = rpc.get_account(ata).await.unwrap().unwrap();
    let ctoken_after = Token::deserialize(&mut &ata_account_after.data[..]).unwrap();

    let mut expected_ctoken = ctoken_before;
    expected_ctoken.amount = 500; // 0 + 500

    assert_eq!(
        ctoken_after, expected_ctoken,
        "Light Token should match expected state after mint"
    );
}

/// Test minting to Light Token with PDA authority using CTokenMintToCpi::invoke_signed()
///
/// This test:
/// 1. Creates a compressed mint with PDA authority via wrapper program (discriminator 14)
/// 2. Decompresses the mint (permissionless)
/// 3. Creates an ATA
/// 4. Mints tokens using PDA authority via invoke_signed
#[tokio::test]
async fn test_ctoken_mint_to_invoke_signed() {
    use light_client::indexer::Indexer;
    use light_token_interface::{
        instructions::mint_action::CompressedMintWithContext, state::CompressedMint,
    };
    use light_token_sdk::token::CreateAssociatedTokenAccount;
    use native_ctoken_examples::{
        CreateCmintData, DecompressCmintData, InstructionType as WrapperInstructionType,
        MINT_SIGNER_SEED,
    };

    let config = ProgramTestConfig::new_v2(true, Some(vec![("native_ctoken_examples", ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Derive the PDAs from our wrapper program
    let (mint_signer_pda, _) = Pubkey::find_program_address(&[MINT_SIGNER_SEED], &ID);
    let (pda_mint_authority, _) = Pubkey::find_program_address(&[MINT_AUTHORITY_SEED], &ID);

    let decimals = 9u8;
    let address_tree = rpc.get_address_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    // Derive compression address using the PDA mint_signer
    let compression_address = light_token_sdk::token::derive_mint_compressed_address(
        &mint_signer_pda,
        &address_tree.tree,
    );

    let (mint_pda, mint_bump) = light_token_sdk::token::find_mint_address(&mint_signer_pda);

    // Step 1: Create compressed mint with PDA authority using wrapper program (discriminator 14)
    {
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
            Pubkey::new_from_array(light_token_interface::LIGHT_TOKEN_PROGRAM_ID);
        let default_pubkeys = light_token_sdk::utils::TokenDefaultAccounts::default();

        let create_cmint_data = CreateCmintData {
            decimals,
            address_merkle_tree_root_index: rpc_result.addresses[0].root_index,
            mint_authority: pda_mint_authority,
            proof: rpc_result.proof.0.unwrap(),
            compression_address,
            mint: mint_pda,
            bump: mint_bump,
            freeze_authority: None,
            extensions: None,
        };
        // Discriminator 14 = CreateCmintWithPdaAuthority
        let wrapper_instruction_data =
            [vec![14u8], create_cmint_data.try_to_vec().unwrap()].concat();

        let wrapper_accounts = vec![
            AccountMeta::new_readonly(compressed_token_program_id, false),
            AccountMeta::new_readonly(default_pubkeys.light_system_program, false),
            AccountMeta::new_readonly(mint_signer_pda, false),
            AccountMeta::new(pda_mint_authority, false),
            AccountMeta::new(payer.pubkey(), true),
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

    // Step 2: Decompress the mint via wrapper program (PDA authority requires CPI)
    {
        let compressed_mint_account = rpc
            .get_compressed_account(compression_address, None)
            .await
            .unwrap()
            .value
            .expect("Compressed mint should exist");

        let compressed_mint = CompressedMint::deserialize(
            &mut compressed_mint_account
                .data
                .as_ref()
                .unwrap()
                .data
                .as_slice(),
        )
        .unwrap();

        let rpc_result = rpc
            .get_validity_proof(vec![compressed_mint_account.hash], vec![], None)
            .await
            .unwrap()
            .value;

        let compressed_mint_with_context = CompressedMintWithContext {
            address: compression_address,
            leaf_index: compressed_mint_account.leaf_index,
            prove_by_index: true,
            root_index: rpc_result.accounts[0]
                .root_index
                .root_index()
                .unwrap_or_default(),
            mint: Some(compressed_mint.try_into().unwrap()),
        };

        let default_pubkeys = light_token_sdk::utils::TokenDefaultAccounts::default();
        let compressible_config = light_token_sdk::token::config_pda();
        let rent_sponsor = light_token_sdk::token::rent_sponsor_pda();

        let decompress_data = DecompressCmintData {
            compressed_mint_with_context,
            proof: rpc_result.proof,
            rent_payment: 16,
            write_top_up: 766,
        };

        // Discriminator 33 = DecompressCmintInvokeSigned
        let wrapper_instruction_data = [
            vec![WrapperInstructionType::DecompressCmintInvokeSigned as u8],
            decompress_data.try_to_vec().unwrap(),
        ]
        .concat();

        // Account order matches process_decompress_cmint_invoke_signed:
        // 0: mint_seed (readonly)
        // 1: authority (PDA, readonly - program signs)
        // 2: payer (signer, writable)
        // 3: cmint (writable)
        // 4: compressible_config (readonly)
        // 5: rent_sponsor (writable)
        // 6: state_tree (writable)
        // 7: input_queue (writable)
        // 8: output_queue (writable)
        // 9: light_system_program (readonly)
        // 10: cpi_authority_pda (readonly)
        // 11: registered_program_pda (readonly)
        // 12: account_compression_authority (readonly)
        // 13: account_compression_program (readonly)
        // 14: system_program (readonly)
        // 15: ctoken_program (readonly) - required for CPI
        let light_token_program_id =
            Pubkey::new_from_array(light_token_interface::LIGHT_TOKEN_PROGRAM_ID);
        let wrapper_accounts = vec![
            AccountMeta::new_readonly(mint_signer_pda, false),
            AccountMeta::new_readonly(pda_mint_authority, false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(mint_pda, false),
            AccountMeta::new_readonly(compressible_config, false),
            AccountMeta::new(rent_sponsor, false),
            AccountMeta::new(compressed_mint_account.tree_info.tree, false),
            AccountMeta::new(compressed_mint_account.tree_info.queue, false),
            AccountMeta::new(output_queue, false),
            AccountMeta::new_readonly(default_pubkeys.light_system_program, false),
            AccountMeta::new_readonly(default_pubkeys.cpi_authority_pda, false),
            AccountMeta::new_readonly(default_pubkeys.registered_program_pda, false),
            AccountMeta::new_readonly(default_pubkeys.account_compression_authority, false),
            AccountMeta::new_readonly(default_pubkeys.account_compression_program, false),
            AccountMeta::new_readonly(default_pubkeys.system_program, false),
            AccountMeta::new_readonly(light_token_program_id, false),
        ];

        let decompress_ix = Instruction {
            program_id: ID,
            accounts: wrapper_accounts,
            data: wrapper_instruction_data,
        };

        rpc.create_and_send_transaction(&[decompress_ix], &payer.pubkey(), &[&payer])
            .await
            .unwrap();
    }

    // Step 3: Create ATA for payer
    let ata = {
        let (ata_address, _) = light_token_sdk::token::derive_token_ata(&payer.pubkey(), &mint_pda);
        let create_ata =
            CreateAssociatedTokenAccount::new(payer.pubkey(), payer.pubkey(), mint_pda);
        let ata_instruction = create_ata.instruction().unwrap();

        rpc.create_and_send_transaction(&[ata_instruction], &payer.pubkey(), &[&payer])
            .await
            .unwrap();

        ata_address
    };

    let mint_amount = 1000u64;

    // Get initial state
    let ata_account_before = rpc.get_account(ata).await.unwrap().unwrap();
    let ctoken_before = Token::deserialize(&mut &ata_account_before.data[..]).unwrap();

    // Step 4: Mint tokens using PDA authority via invoke_signed
    let mut instruction_data = vec![InstructionType::CTokenMintToInvokeSigned as u8];
    let mint_data = MintToData {
        amount: mint_amount,
    };
    mint_data.serialize(&mut instruction_data).unwrap();

    let ctoken_program = Pubkey::from(LIGHT_TOKEN_PROGRAM_ID);
    let system_program = Pubkey::default();
    let instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(mint_pda, false),                // cmint
            AccountMeta::new(ata, false),                     // destination
            AccountMeta::new(pda_mint_authority, false), // PDA authority (program signs, writable for top-up)
            AccountMeta::new_readonly(system_program, false), // system_program
            AccountMeta::new_readonly(ctoken_program, false), // ctoken_program
        ],
        data: instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify with single assert_eq
    let ata_account_after = rpc.get_account(ata).await.unwrap().unwrap();
    let ctoken_after = Token::deserialize(&mut &ata_account_after.data[..]).unwrap();

    let mut expected_ctoken = ctoken_before;
    expected_ctoken.amount = 1000; // 0 + 1000

    assert_eq!(
        ctoken_after, expected_ctoken,
        "Light Token should match expected state after mint"
    );
}
