// #![cfg(feature = "test-sbf")]

use anchor_lang::{AccountDeserialize, InstructionData};
use anchor_spl::token::TokenAccount;
use light_compressed_token_sdk::{
    instruction::{get_transfer_instruction_account_metas, TokenAccountsMetaConfig},
    token_pool::get_token_pool_pda,
};
use light_program_test::{LightProgramTest, ProgramTestConfig, Rpc};
use light_sdk::instruction::PackedAccounts;
use light_test_utils::{
    spl::{create_mint_helper, create_token_account, mint_spl_tokens},
    RpcError,
};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
};

#[tokio::test]
async fn test_compress_spl_tokens() {
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

    // Verify the token account has the correct balance before compression
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

    println!("Verified token account balance before compression");

    // Now compress the SPL tokens
    let compress_amount = 500_000; // Compress half of the tokens
    let recipient = payer.pubkey(); // Compress to the same owner

    compress_spl_tokens(
        &mut rpc,
        &payer,
        recipient,
        mint_pubkey,
        compress_amount,
        token_account_keypair.pubkey(),
    )
    .await
    .unwrap();

    println!("Compressed {} tokens successfully", compress_amount);

    // TODO: Add verification of compressed token accounts
    // This would require checking the compressed account state tree
    // and verifying the token balance was reduced in the SPL account

    println!("Compression test completed successfully!");
}

async fn compress_spl_tokens(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    recipient: Pubkey,
    mint: Pubkey,
    amount: u64,
    token_account: Pubkey,
) -> Result<Signature, RpcError> {
    let mut remaining_accounts = PackedAccounts::default();
    let token_pool_pda = get_token_pool_pda(&mint);
    let config = TokenAccountsMetaConfig::compress(
        payer.pubkey(),
        payer.pubkey(),
        token_pool_pda,
        token_account,
        false,
    );
    remaining_accounts.add_pre_accounts_signer_mut(payer.pubkey());
    // Add the token account to pre_accounts for the compression
    remaining_accounts
        .add_pre_accounts_metas(get_transfer_instruction_account_metas(config).as_slice());

    let output_tree_index = rpc
        .get_random_state_tree_info()
        .unwrap()
        .pack_output_tree_index(&mut remaining_accounts)
        .unwrap();

    let (remaining_accounts, _, _) = remaining_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: sdk_token_test::ID,
        accounts: [
            vec![AccountMeta::new(payer.pubkey(), true)],
            remaining_accounts,
        ]
        .concat(),
        data: sdk_token_test::instruction::Compress {
            output_tree_index,
            recipient,
            mint,
            amount,
        }
        .data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}
