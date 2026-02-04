// Tests for CTokenMintToCpi invoke() and invoke_signed()

mod shared;

use borsh::{BorshDeserialize, BorshSerialize};
use light_client::rpc::Rpc;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_token::LIGHT_TOKEN_PROGRAM_ID;
use light_token_interface::state::Token;
use sdk_light_token_pinocchio_test::{InstructionType, MintToData, MINT_AUTHORITY_SEED};
use shared::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signer::Signer,
};

/// Test minting to Light Token using CTokenMintToCpi::invoke()
#[tokio::test]
async fn test_ctoken_mint_to_invoke() {
    let config = ProgramTestConfig::new_v2(
        true,
        Some(vec![("sdk_light_token_pinocchio_test", PROGRAM_ID)]),
    );
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create a decompressed mint with an ATA for the payer with 0 tokens
    let (mint_pda, _compression_address, ata_pubkeys) = setup_create_mint_with_freeze_authority(
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

    let light_token_program = LIGHT_TOKEN_PROGRAM_ID;
    let system_program = Pubkey::default();
    let instruction = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(mint_pda, false),                     // mint
            AccountMeta::new(ata, false),                          // destination
            AccountMeta::new(payer.pubkey(), true), // authority (signer, writable for top-up)
            AccountMeta::new_readonly(system_program, false), // system_program
            AccountMeta::new_readonly(light_token_program, false), // light_token_program
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
/// 1. Creates a compressed mint with PDA authority via wrapper program (auto-decompresses)
/// 2. Creates an ATA
/// 3. Mints tokens using PDA authority via invoke_signed
#[tokio::test]
async fn test_ctoken_mint_to_invoke_signed() {
    use light_client::indexer::Indexer;
    use light_token::instruction::CreateAssociatedTokenAccount;
    use sdk_light_token_pinocchio_test::{CreateCmintData, MINT_SIGNER_SEED};

    let config = ProgramTestConfig::new_v2(
        true,
        Some(vec![("sdk_light_token_pinocchio_test", PROGRAM_ID)]),
    );
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Derive the PDAs from our wrapper program
    let (mint_signer_pda, _) = Pubkey::find_program_address(&[MINT_SIGNER_SEED], &PROGRAM_ID);
    let (pda_mint_authority, _) = Pubkey::find_program_address(&[MINT_AUTHORITY_SEED], &PROGRAM_ID);

    let decimals = 9u8;
    let address_tree = rpc.get_address_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    // Derive compression address using the PDA mint_signer
    let compression_address = light_token::instruction::derive_mint_compressed_address(
        &mint_signer_pda,
        &address_tree.tree,
    );

    let (mint_pda, mint_bump) = light_token::instruction::find_mint_address(&mint_signer_pda);

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
        let default_pubkeys = light_token::utils::TokenDefaultAccounts::default();
        let compressible_config = light_token::instruction::config_pda();
        let rent_sponsor = light_token::instruction::rent_sponsor_pda();

        let create_mint_data = CreateCmintData {
            decimals,
            address_merkle_tree_root_index: rpc_result.addresses[0].root_index,
            mint_authority: pda_mint_authority.to_bytes(),
            proof: rpc_result.proof.0.unwrap(),
            compression_address,
            mint: mint_pda.to_bytes(),
            bump: mint_bump,
            freeze_authority: None,
            extensions: None,
            rent_payment: 16,
            write_top_up: 766,
        };
        // Discriminator 14 = CreateCmintWithPdaAuthority
        let wrapper_instruction_data =
            [vec![14u8], create_mint_data.try_to_vec().unwrap()].concat();

        // Account order matches process_create_mint_with_pda_authority (MintActionMetaConfig):
        // [0]: compressed_token_program
        // [1]: light_system_program
        // [2]: mint_signer (PDA)
        // [3]: authority (PDA)
        // [4]: compressible_config
        // [5]: mint
        // [6]: rent_sponsor
        // [7]: fee_payer (signer)
        // [8]: cpi_authority_pda
        // [9]: registered_program_pda
        // [10]: account_compression_authority
        // [11]: account_compression_program
        // [12]: system_program
        // [13]: output_queue
        // [14]: address_tree
        let wrapper_accounts = vec![
            AccountMeta::new_readonly(compressed_token_program_id, false), // [0]
            AccountMeta::new_readonly(default_pubkeys.light_system_program, false), // [1]
            AccountMeta::new_readonly(mint_signer_pda, false),             // [2] mint_signer PDA
            AccountMeta::new_readonly(pda_mint_authority, false),          // [3] authority PDA
            AccountMeta::new_readonly(compressible_config, false), // [4] compressible_config
            AccountMeta::new(mint_pda, false),                     // [5] mint
            AccountMeta::new(rent_sponsor, false),                 // [6] rent_sponsor
            AccountMeta::new(payer.pubkey(), true),                // [7] fee_payer (signer)
            AccountMeta::new_readonly(default_pubkeys.cpi_authority_pda, false), // [8]
            AccountMeta::new_readonly(default_pubkeys.registered_program_pda, false), // [9]
            AccountMeta::new_readonly(default_pubkeys.account_compression_authority, false), // [10]
            AccountMeta::new_readonly(default_pubkeys.account_compression_program, false), // [11]
            AccountMeta::new_readonly(default_pubkeys.system_program, false), // [12]
            AccountMeta::new(output_queue, false),                 // [13]
            AccountMeta::new(address_tree.tree, false),            // [14]
        ];

        let create_mint_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: wrapper_accounts,
            data: wrapper_instruction_data,
        };

        rpc.create_and_send_transaction(&[create_mint_ix], &payer.pubkey(), &[&payer])
            .await
            .unwrap();
    }

    // Step 2: Create ATA for payer (CreateMint now auto-decompresses)
    let ata = {
        let (ata_address, _) =
            light_token::instruction::derive_token_ata(&payer.pubkey(), &mint_pda);
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

    // Step 3: Mint tokens using PDA authority via invoke_signed
    let mut instruction_data = vec![InstructionType::CTokenMintToInvokeSigned as u8];
    let mint_data = MintToData {
        amount: mint_amount,
    };
    mint_data.serialize(&mut instruction_data).unwrap();

    let light_token_program = LIGHT_TOKEN_PROGRAM_ID;
    let system_program = Pubkey::default();
    let instruction = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(mint_pda, false),                     // mint
            AccountMeta::new(ata, false),                          // destination
            AccountMeta::new(pda_mint_authority, false), // PDA authority (program signs, writable for top-up)
            AccountMeta::new_readonly(system_program, false), // system_program
            AccountMeta::new_readonly(light_token_program, false), // light_token_program
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
