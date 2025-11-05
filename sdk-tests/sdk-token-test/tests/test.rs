#![cfg(feature = "test-sbf")]

use anchor_lang::{AccountDeserialize, InstructionData};
use anchor_spl::token::TokenAccount;
use light_client::indexer::CompressedTokenAccount;
use light_compressed_token_sdk::{
    instructions::{
        batch_compress::{
            get_batch_compress_instruction_account_metas, BatchCompressMetaConfig, Recipient,
        },
        transfer::account_metas::{
            get_transfer_instruction_account_metas, TokenAccountsMetaConfig,
        },
    },
    token_pool::{find_token_pool_pda_with_index, get_token_pool_pda},
    TokenAccountMeta, SPL_TOKEN_PROGRAM_ID,
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

    // Now decompress some tokens from the recipient back to SPL token account
    let decompress_token_account_keypair = Keypair::new();
    let decompress_amount = 10; // Decompress a small amount
    rpc.airdrop_lamports(&transfer_recipient.pubkey(), 10_000_000_000)
        .await
        .unwrap();
    // Create a new SPL token account for decompression
    create_token_account(
        &mut rpc,
        &mint_pubkey,
        &decompress_token_account_keypair,
        &transfer_recipient,
    )
    .await
    .unwrap();

    println!(
        "Created decompress token account: {}",
        decompress_token_account_keypair.pubkey()
    );

    // Get the recipient's compressed token account after transfer
    let recipient_compressed_accounts = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&transfer_recipient.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    let recipient_compressed_account = &recipient_compressed_accounts[0];

    // Decompress tokens from recipient's compressed account to SPL token account
    decompress_compressed_tokens(
        &mut rpc,
        &transfer_recipient,
        recipient_compressed_account,
        decompress_token_account_keypair.pubkey(),
    )
    .await
    .unwrap();

    println!(
        "Decompressed {} tokens from recipient successfully",
        decompress_amount
    );

    // Verify the decompression worked
    let decompress_token_account_data = rpc
        .get_account(decompress_token_account_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();

    let decompress_token_account =
        TokenAccount::try_deserialize(&mut decompress_token_account_data.data.as_slice()).unwrap();

    // Assert the SPL token account has the decompressed amount
    assert_eq!(decompress_token_account.amount, decompress_amount);
    assert_eq!(decompress_token_account.mint, mint_pubkey);
    assert_eq!(decompress_token_account.owner, transfer_recipient.pubkey());

    println!(
        "Verified SPL token account after decompression: amount={}",
        decompress_token_account.amount
    );

    // Verify the compressed account balance was reduced
    let updated_recipient_accounts = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&transfer_recipient.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    if !updated_recipient_accounts.is_empty() {
        let updated_recipient_account = &updated_recipient_accounts[0];
        let remaining_compressed_amount = updated_recipient_account.token.amount;
        assert_eq!(
            remaining_compressed_amount,
            transfer_amount - decompress_amount
        );
        println!(
            "Verified remaining compressed balance: {}",
            remaining_compressed_amount
        );
    }

    println!("Compression, transfer, and decompress test completed successfully!");
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
    let config = TokenAccountsMetaConfig::compress_client(
        token_pool_pda,
        token_account,
        SPL_TOKEN_PROGRAM_ID.into(),
    );
    remaining_accounts.add_pre_accounts_signer_mut(payer.pubkey());
    let metas = get_transfer_instruction_account_metas(config);
    println!("metas {:?}", metas.to_vec());
    // Add the token account to pre_accounts for the compressiospl_token_programn
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
        data: sdk_token_test::instruction::CompressTokens {
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
    let config = TokenAccountsMetaConfig::new_client();
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
    let token_metas = vec![TokenAccountMeta {
        amount: compressed_account.token.amount,
        delegate_index: None,
        packed_tree_info: tree_info,
        lamports: None,
        tlv: None,
    }];

    let (accounts, _, _) = remaining_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: sdk_token_test::ID,
        accounts,
        data: sdk_token_test::instruction::TransferTokens {
            validity_proof: rpc_result.proof,
            token_metas,
            output_tree_index,
            mint: compressed_account.token.mint,
            recipient,
        }
        .data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}

async fn decompress_compressed_tokens(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    compressed_account: &CompressedTokenAccount,
    decompress_token_account: Pubkey,
) -> Result<Signature, RpcError> {
    let mut remaining_accounts = PackedAccounts::default();
    let token_pool_pda = get_token_pool_pda(&compressed_account.token.mint);
    let config = TokenAccountsMetaConfig::decompress_client(
        token_pool_pda,
        decompress_token_account,
        SPL_TOKEN_PROGRAM_ID.into(),
    );
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

    // Create input token data
    let token_data = vec![TokenAccountMeta {
        amount: compressed_account.token.amount,
        delegate_index: None,
        packed_tree_info: tree_info,
        lamports: None,
        tlv: None,
    }];

    let (remaining_accounts, _, _) = remaining_accounts.to_account_metas();
    println!(" remaining_accounts: {:?}", remaining_accounts);

    let instruction = Instruction {
        program_id: sdk_token_test::ID,
        accounts: [remaining_accounts].concat(),
        data: sdk_token_test::instruction::DecompressTokens {
            validity_proof: rpc_result.proof,
            token_data,
            output_tree_index,
            mint: compressed_account.token.mint,
        }
        .data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}

#[tokio::test]
async fn test_batch_compress() {
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
    let mint_amount = 2_000_000; // 2000 tokens with 6 decimals

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

    // Create multiple recipients for batch compression
    let recipient1 = Keypair::new().pubkey();
    let recipient2 = Keypair::new().pubkey();
    let recipient3 = Keypair::new().pubkey();

    let recipients = vec![
        Recipient {
            pubkey: recipient1,
            amount: 100_000,
        },
        Recipient {
            pubkey: recipient2,
            amount: 200_000,
        },
        Recipient {
            pubkey: recipient3,
            amount: 300_000,
        },
    ];

    let total_batch_amount: u64 = recipients.iter().map(|r| r.amount).sum();

    // Perform batch compression
    batch_compress_spl_tokens(
        &mut rpc,
        &payer,
        recipients,
        mint_pubkey,
        token_account_keypair.pubkey(),
    )
    .await
    .unwrap();

    println!(
        "Batch compressed {} tokens to {} recipients successfully",
        total_batch_amount, 3
    );

    // Verify each recipient received their compressed tokens
    for (i, recipient) in [recipient1, recipient2, recipient3].iter().enumerate() {
        let compressed_accounts = rpc
            .indexer()
            .unwrap()
            .get_compressed_token_accounts_by_owner(recipient, None, None)
            .await
            .unwrap()
            .value
            .items;

        assert!(
            !compressed_accounts.is_empty(),
            "Recipient {} should have compressed tokens",
            i + 1
        );

        let compressed_account = &compressed_accounts[0];
        assert_eq!(compressed_account.token.owner, *recipient);
        assert_eq!(compressed_account.token.mint, mint_pubkey);

        let expected_amount = match i {
            0 => 100_000,
            1 => 200_000,
            2 => 300_000,
            _ => unreachable!(),
        };
        assert_eq!(compressed_account.token.amount, expected_amount);

        println!(
            "Verified recipient {} received {} compressed tokens",
            i + 1,
            compressed_account.token.amount
        );
    }

    println!("Batch compression test completed successfully!");
}

async fn batch_compress_spl_tokens(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    recipients: Vec<Recipient>,
    mint: Pubkey,
    token_account: Pubkey,
) -> Result<Signature, RpcError> {
    let mut remaining_accounts = PackedAccounts::default();
    remaining_accounts.add_pre_accounts_signer_mut(payer.pubkey());
    let token_pool_index = 0;
    let (token_pool_pda, token_pool_bump) = find_token_pool_pda_with_index(&mint, token_pool_index);
    println!("token_pool_pda {:?}", token_pool_pda);
    // Use batch compress account metas
    let config = BatchCompressMetaConfig::new_client(
        token_pool_pda,
        token_account,
        SPL_TOKEN_PROGRAM_ID.into(),
        rpc.get_random_state_tree_info().unwrap().queue,
        false, // with_lamports
    );
    let metas = get_batch_compress_instruction_account_metas(config);
    println!("metas {:?}", metas);
    remaining_accounts.add_pre_accounts_metas(metas.as_slice());

    let (accounts, _, _) = remaining_accounts.to_account_metas();
    println!("accounts {:?}", accounts);

    let instruction = Instruction {
        program_id: sdk_token_test::ID,
        accounts,
        data: sdk_token_test::instruction::BatchCompressTokens {
            recipients,
            token_pool_index,
            token_pool_bump,
        }
        .data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}
