// #![cfg(feature = "test-sbf")]

use anchor_lang::AccountDeserialize;
use anchor_spl::token::TokenAccount;
use light_program_test::{LightProgramTest, ProgramTestConfig, Rpc};
use light_test_utils::spl::{create_mint_helper, create_token_account, mint_spl_tokens};
use solana_sdk::{signature::Keypair, signer::Signer};

//#[serial]
#[tokio::test]
async fn test_create_token_account_and_mint() {
    // Initialize the test environment
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new(
        false,
        Some(vec![("sdk_token_test", sdk_token_test::ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();

    // Create a mint
    let mint_pubkey = create_mint_helper(&mut rpc, &payer).await;
    println!("Created mint: {}", mint_pubkey);

    // Create a token account
    let token_account_keypair = Keypair::new();

    create_token_account(&mut rpc, &mint_pubkey, &token_account_keypair, &payer)
        .await
        .unwrap();

    println!("Created token account: {}", token_account_keypair.pubkey());

    // Mint some tokens to the account
    let mint_amount = 1_000_000; // 1000 tokens with 6 decimals

    mint_spl_tokens(
        &mut rpc,
        &mint_pubkey,
        &token_account_keypair.pubkey(),
        &payer.pubkey(), // owner
        &payer,          // mint authority
        mint_amount,
        false, // not token22
    )
    .await
    .unwrap();

    println!("Minted {} tokens to account", mint_amount);

    // Verify the token account has the correct balance
    let token_account_data = rpc
        .get_account(token_account_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();

    let token_account =
        TokenAccount::try_deserialize(&mut token_account_data.data.as_slice()).unwrap();

    assert_eq!(token_account.amount, mint_amount);
    assert_eq!(token_account.mint, mint_pubkey);
    assert_eq!(token_account.owner, payer.pubkey());

    println!("Test completed successfully!");
}
