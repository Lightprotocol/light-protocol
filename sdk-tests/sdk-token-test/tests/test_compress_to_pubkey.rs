use anchor_lang::InstructionData;
use light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID;
use light_program_test::{
    program_test::TestRpc, Indexer, LightProgramTest, ProgramTestConfig, Rpc,
};
use light_sdk::instruction::PackedAccounts;
use light_test_utils::spl::create_mint_helper;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Signer,
};

#[tokio::test]
async fn test_compress_to_pubkey() {
    // Initialize the test environment
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("sdk_token_test", sdk_token_test::ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();

    // Create a mint
    let mint_pubkey = create_mint_helper(&mut rpc, &payer).await;

    // Get compressible config from test accounts
    let compressible_config = rpc
        .test_accounts
        .funding_pool_config
        .compressible_config_pda;
    let rent_sponsor = rpc.test_accounts.funding_pool_config.rent_sponsor_pda;

    // Calculate the PDA that tokens will compress to
    let seeds = &[b"compress_target", mint_pubkey.as_ref()];
    let (token_account_pubkey, _bump) = Pubkey::find_program_address(seeds, &sdk_token_test::ID);

    println!("token_account_pubkey: {}", token_account_pubkey);

    // Build the instruction to create the ctoken account with compress_to_pubkey
    let mut remaining_accounts = PackedAccounts::default();

    // Add required accounts for creating the compressible token account
    remaining_accounts.add_pre_accounts_meta(AccountMeta::new(payer.pubkey(), true)); // Payer
    remaining_accounts.add_pre_accounts_meta(AccountMeta::new(token_account_pubkey, false)); // Token account to create
    remaining_accounts.add_pre_accounts_meta(AccountMeta::new_readonly(mint_pubkey, false)); // Mint
    remaining_accounts.add_pre_accounts_meta(AccountMeta::new_readonly(compressible_config, false)); // Compressible config
    remaining_accounts.add_pre_accounts_meta(AccountMeta::new_readonly(
        solana_sdk::system_program::id(),
        false,
    )); // System program
    remaining_accounts.add_pre_accounts_meta(AccountMeta::new(rent_sponsor, false)); // Rent recipient
    remaining_accounts.add_pre_accounts_meta(AccountMeta::new_readonly(
        COMPRESSED_TOKEN_PROGRAM_ID.into(),
        false,
    ));
    let (account_metas, _, _) = remaining_accounts.to_account_metas();

    let instruction_data = sdk_token_test::instruction::CreateCtokenWithCompressToPubkey {
        mint: mint_pubkey,
        token_account_pubkey,
        compressible_config,
        rent_sponsor,
    };

    let instruction = Instruction {
        program_id: sdk_token_test::ID,
        accounts: account_metas,
        data: instruction_data.data(),
    };

    // Execute the transaction
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify the token account was created
    let token_account_data = rpc
        .get_account(token_account_pubkey)
        .await
        .unwrap()
        .expect("Token account should exist");

    println!(
        "Token account created successfully at: {}",
        token_account_pubkey
    );
    println!("Account data length: {}", token_account_data.data.len());

    // Compresses the account.
    rpc.warp_epoch_forward(2).await.unwrap();
    // Assert that the ctoken account is closed and the compressed account exists.
    {
        let closed_token_account_data = rpc.get_account(token_account_pubkey).await.unwrap();
        if let Some(token_account) = closed_token_account_data {
            assert_eq!(
                token_account.lamports, 0,
                "Token account not closed and compressed"
            );
        }
        let compressed_token_account = rpc
            .get_compressed_token_accounts_by_owner(&token_account_pubkey, None, None)
            .await
            .unwrap()
            .value
            .items[0]
            .clone();
        println!("compressed_token_account {:?}", compressed_token_account);
        assert_eq!(compressed_token_account.token.owner, token_account_pubkey);
        assert_eq!(compressed_token_account.token.amount, 0);
    }
}
