use anchor_lang::InstructionData;
use light_program_test::{AddressWithTree, Indexer, LightProgramTest, ProgramTestConfig, Rpc};
use light_token_sdk::token::{derive_mint_compressed_address, find_mint_address, SystemAccounts};
use sdk_token_test::{CreateMintParamsData, CreateTwoMintsData};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    signature::{Keypair, Signer},
};

/// Test creating two compressed mints using CPI context in a single transaction.
/// First CPI writes first mint to CPI context, second CPI executes both with single proof.
/// Both mints remain as compressed accounts (no Solana account created).
#[tokio::test]
async fn test_create_two_compressed_mints_cpi_context() {
    // 1. Setup test environment
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("sdk_token_test", sdk_token_test::ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();

    // 2. Generate two mint signers
    let mint_signer_1 = Keypair::new();
    let mint_signer_2 = Keypair::new();

    // 3. Get tree info
    let address_tree_info = rpc.get_address_tree_v2();
    let state_tree_info = rpc.get_random_state_tree_info().unwrap();

    // 4. Derive addresses for both mints
    let compression_address_1 =
        derive_mint_compressed_address(&mint_signer_1.pubkey(), &address_tree_info.tree);
    let compression_address_2 =
        derive_mint_compressed_address(&mint_signer_2.pubkey(), &address_tree_info.tree);
    let (mint_pda_1, bump_1) = find_mint_address(&mint_signer_1.pubkey());
    let (mint_pda_2, bump_2) = find_mint_address(&mint_signer_2.pubkey());

    // 5. Get SINGLE validity proof for BOTH addresses
    let proof_result = rpc
        .get_validity_proof(
            vec![],
            vec![
                AddressWithTree {
                    address: compression_address_1,
                    tree: address_tree_info.tree,
                },
                AddressWithTree {
                    address: compression_address_2,
                    tree: address_tree_info.tree,
                },
            ],
            None,
        )
        .await
        .unwrap()
        .value;

    // 6. Build CreateMintParamsData for both mints
    let params_1 = CreateMintParamsData {
        decimals: 9,
        address_merkle_tree_root_index: proof_result.addresses[0].root_index,
        mint_authority: payer.pubkey(),
        compression_address: compression_address_1,
        mint: mint_pda_1,
        bump: bump_1,
        freeze_authority: None,
    };

    let params_2 = CreateMintParamsData {
        decimals: 6,
        address_merkle_tree_root_index: proof_result.addresses[1].root_index,
        mint_authority: payer.pubkey(),
        compression_address: compression_address_2,
        mint: mint_pda_2,
        bump: bump_2,
        freeze_authority: None,
    };

    // 7. Build instruction data
    let data = CreateTwoMintsData {
        params_1,
        params_2,
        proof: proof_result.proof.0.unwrap(),
    };

    // 8. Build account metas
    let system_accounts = SystemAccounts::default();
    let cpi_context_pubkey = state_tree_info
        .cpi_context
        .expect("CPI context account required");
    let compressed_token_program_id =
        solana_sdk::pubkey::Pubkey::new_from_array(light_token_interface::LIGHT_TOKEN_PROGRAM_ID);

    // Account layout (remaining_accounts):
    // - accounts[0]: light_system_program
    // - accounts[1]: mint_signer_1 (SIGNER)
    // - accounts[2]: mint_signer_2 (SIGNER)
    // - accounts[3]: cpi_authority_pda
    // - accounts[4]: registered_program_pda
    // - accounts[5]: account_compression_authority
    // - accounts[6]: account_compression_program
    // - accounts[7]: system_program
    // - accounts[8]: cpi_context_account (writable)
    // - accounts[9]: output_queue (writable)
    // - accounts[10]: address_tree (writable)
    // - accounts[11]: compressed_token_program (for CPI)
    let accounts = vec![
        // Anchor accounts (signer is the payer)
        AccountMeta::new(payer.pubkey(), true),
        // remaining_accounts
        AccountMeta::new_readonly(system_accounts.light_system_program, false), // [0]
        AccountMeta::new_readonly(mint_signer_1.pubkey(), true),                 // [1] SIGNER
        AccountMeta::new_readonly(mint_signer_2.pubkey(), true),                 // [2] SIGNER
        AccountMeta::new_readonly(system_accounts.cpi_authority_pda, false),     // [3]
        AccountMeta::new_readonly(system_accounts.registered_program_pda, false), // [4]
        AccountMeta::new_readonly(system_accounts.account_compression_authority, false), // [5]
        AccountMeta::new_readonly(system_accounts.account_compression_program, false), // [6]
        AccountMeta::new_readonly(system_accounts.system_program, false),        // [7]
        AccountMeta::new(cpi_context_pubkey, false),                             // [8]
        AccountMeta::new(state_tree_info.queue, false),                          // [9]
        AccountMeta::new(address_tree_info.tree, false),                         // [10]
        AccountMeta::new_readonly(compressed_token_program_id, false),           // [11]
    ];

    let instruction = Instruction {
        program_id: sdk_token_test::ID,
        accounts,
        data: sdk_token_test::instruction::CreateTwoMints { data }.data(),
    };

    // 9. Send transaction
    rpc.create_and_send_transaction(
        &[instruction],
        &payer.pubkey(),
        &[&payer, &mint_signer_1, &mint_signer_2],
    )
    .await
    .unwrap();

    // 10. Verify both compressed mints were created by querying the indexer
    // Since these are compressed accounts (no Solana account), we verify via indexer
    let compressed_accounts = rpc
        .indexer()
        .unwrap()
        .get_compressed_accounts_by_owner(&payer.pubkey(), None, None)
        .await
        .unwrap();

    // Check that we have at least 2 compressed accounts (the mints)
    assert!(
        compressed_accounts.value.items.len() >= 2,
        "Should have at least 2 compressed accounts"
    );

    println!("Successfully created two compressed mints in single transaction!");
    println!("  Mint 1 address: {:?}", compression_address_1);
    println!("  Mint 2 address: {:?}", compression_address_2);
}
