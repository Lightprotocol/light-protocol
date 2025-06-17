// #![cfg(feature = "test-sbf")]

use anchor_lang::{AccountDeserialize, InstructionData};
use anchor_spl::token::TokenAccount;
use light_compressed_token_sdk::{
    instruction::{get_transfer_instruction_account_metas, TokenAccountsMetaConfig},
    token_pool::get_token_pool_pda,
    InputTokenDataWithContext, PackedMerkleContext,
};
use light_program_test::{Indexer, LightProgramTest, ProgramTestConfig, Rpc};
use light_sdk::instruction::PackedAccounts;
use light_test_utils::{
    spl::{create_mint_helper, create_token_account, mint_spl_tokens},
    RpcError,
};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
};

use light_client::indexer::CompressedTokenAccount;

#[tokio::test]
async fn test() {
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
    let compression_recipient = payer.pubkey(); // Compress to the same owner

    // Declare transfer parameters early
    let transfer_recipient = Keypair::new();
    let transfer_amount = 10;

    compress_spl_tokens(
        &mut rpc,
        &payer,
        compression_recipient,
        mint_pubkey,
        compress_amount,
        token_account_keypair.pubkey(),
    )
    .await
    .unwrap();

    println!("Compressed {} tokens successfully", compress_amount);

    // Get the compressed token account from indexer
    let compressed_accounts = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&payer.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    let compressed_account = &compressed_accounts[0];

    // Assert the compressed token account properties
    assert_eq!(compressed_account.token.owner, payer.pubkey());
    assert_eq!(compressed_account.token.mint, mint_pubkey);

    // Verify the token amount (should match the compressed amount)
    let amount = compressed_account.token.amount;
    assert_eq!(amount, compress_amount);

    println!(
        "Verified compressed token account: owner={}, mint={}, amount={}",
        payer.pubkey(),
        mint_pubkey,
        amount
    );
    println!("compressed_account {:?}", compressed_account);
    // Now transfer some compressed tokens to a recipient
    transfer_compressed_tokens(
        &mut rpc,
        &payer,
        transfer_recipient.pubkey(),
        compressed_account,
    )
    .await
    .unwrap();

    println!(
        "Transferred {} compressed tokens to recipient successfully",
        transfer_amount
    );

    // Verify the transfer by checking both sender and recipient accounts
    let updated_accounts = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&payer.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    let recipient_accounts = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&transfer_recipient.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    // Sender should have (compress_amount - transfer_amount) remaining
    if !updated_accounts.is_empty() {
        let sender_account = &updated_accounts[0];
        let sender_amount = sender_account.token.amount;
        assert_eq!(sender_amount, compress_amount - transfer_amount);
        println!("Verified sender remaining balance: {}", sender_amount);
    }

    // Recipient should have transfer_amount
    assert!(
        !recipient_accounts.is_empty(),
        "Recipient should have compressed token account"
    );
    let recipient_account = &recipient_accounts[0];
    assert_eq!(recipient_account.token.owner, transfer_recipient.pubkey());
    let recipient_amount = recipient_account.token.amount;
    assert_eq!(recipient_amount, transfer_amount);
    println!("Verified recipient balance: {}", recipient_amount);

    println!("Compression and transfer test completed successfully!");
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
    let metas = get_transfer_instruction_account_metas(config);
    println!("metas {:?}", metas.to_vec());
    // Add the token account to pre_accounts for the compression
    remaining_accounts.add_pre_accounts_metas(metas.as_slice());

    let output_tree_index = rpc
        .get_random_state_tree_info()
        .unwrap()
        .pack_output_tree_index(&mut remaining_accounts)
        .unwrap();

    let (remaining_accounts, _, _) = remaining_accounts.to_account_metas();
    println!("remaining_accounts {:?}", remaining_accounts.to_vec());

    let instruction = Instruction {
        program_id: sdk_token_test::ID,
        accounts: [remaining_accounts].concat(),
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

async fn transfer_compressed_tokens(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    recipient: Pubkey,
    compressed_account: &CompressedTokenAccount,
) -> Result<Signature, RpcError> {
    let mut remaining_accounts = PackedAccounts::default();
    let config = TokenAccountsMetaConfig::new(payer.pubkey(), payer.pubkey());
    remaining_accounts.add_pre_accounts_signer_mut(payer.pubkey());
    let metas = get_transfer_instruction_account_metas(config);
    remaining_accounts.add_pre_accounts_metas(metas.as_slice());

    // Get validity proof from RPC
    let rpc_result = rpc
        .get_validity_proof(vec![compressed_account.account.hash], vec![], None)
        .await?
        .value;

    let packed_tree_info = rpc_result.pack_tree_infos(&mut remaining_accounts);
    let output_tree_index = packed_tree_info
        .state_trees
        .as_ref()
        .unwrap()
        .output_tree_index;

    // Use the tree info from the validity proof result
    let tree_info = packed_tree_info
        .state_trees
        .as_ref()
        .unwrap()
        .packed_tree_infos[0];
    println!("Transfer tree_info: {:?}", tree_info);

    // Create input token data
    let token_data = vec![InputTokenDataWithContext {
        amount: compressed_account.token.amount,
        delegate_index: None,
        merkle_context: PackedMerkleContext {
            merkle_tree_pubkey_index: tree_info.merkle_tree_pubkey_index,
            nullifier_queue_pubkey_index: tree_info.queue_pubkey_index,
            leaf_index: tree_info.leaf_index,
            proof_by_index: tree_info.prove_by_index,
        },
        root_index: tree_info.root_index,
        lamports: None,
        tlv: None,
    }];

    let (remaining_accounts, _, _) = remaining_accounts.to_account_metas();
    println!("remaining_accounts {:?}", remaining_accounts);
    let instruction = Instruction {
        program_id: sdk_token_test::ID,
        accounts: [remaining_accounts].concat(),
        data: sdk_token_test::instruction::Transfer {
            validity_proof: rpc_result.proof,
            token_data,
            output_tree_index,
            mint: compressed_account.token.mint,
            recipient,
        }
        .data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}
