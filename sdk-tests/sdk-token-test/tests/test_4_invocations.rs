use anchor_lang::{prelude::AccountMeta, AccountDeserialize, InstructionData};
use light_compressed_token_sdk::{
    instructions::{
        transfer::account_metas::{
            get_transfer_instruction_account_metas, TokenAccountsMetaConfig,
        },
        CTokenDefaultAccounts,
    },
    token_pool::get_token_pool_pda,
    SPL_TOKEN_PROGRAM_ID,
};
use light_program_test::{AddressWithTree, Indexer, LightProgramTest, ProgramTestConfig, Rpc};
use light_sdk::{
    address::v1::derive_address,
    instruction::{PackedAccounts, SystemAccountMetaConfig},
};
use light_test_utils::{
    spl::{create_mint_helper, create_token_account, mint_spl_tokens},
    RpcError,
};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
};

#[ignore = "fix cpi context usage"]
#[tokio::test]
async fn test_4_invocations() {
    // Initialize the test environment
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("sdk_token_test", sdk_token_test::ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();

    let (mint1, mint2, mint3, token_account_1, token_account_2, token_account_3) =
        create_mints_and_tokens(&mut rpc, &payer).await;

    println!("✅ Test setup complete: 3 mints created and minted to 3 token accounts");

    // Compress tokens
    let compress_amount = 1000; // Compress 1000 tokens

    compress_tokens_bundled(
        &mut rpc,
        &payer,
        vec![
            (token_account_2, compress_amount, Some(mint2)),
            (token_account_3, compress_amount, Some(mint3)),
        ],
    )
    .await
    .unwrap();

    println!(
        "✅ Completed compression of {} tokens from mint 2 and mint 3",
        compress_amount
    );

    // Create compressed escrow PDA
    let initial_amount = 100; // Initial escrow amount
    let escrow_address = create_compressed_escrow_pda(&mut rpc, &payer, initial_amount)
        .await
        .unwrap();

    println!(
        "✅ Created compressed escrow PDA with address: {:?}",
        escrow_address
    );

    // Test the four_invokes instruction
    test_four_invokes_instruction(
        &mut rpc,
        &payer,
        mint1,
        mint2,
        mint3,
        escrow_address,
        initial_amount,
        token_account_1,
    )
    .await
    .unwrap();

    println!("✅ Successfully executed four_invokes instruction");
}

async fn create_mints_and_tokens(
    rpc: &mut impl Rpc,
    payer: &Keypair,
) -> (
    solana_sdk::pubkey::Pubkey, // mint1
    solana_sdk::pubkey::Pubkey, // mint2
    solana_sdk::pubkey::Pubkey, // mint3
    solana_sdk::pubkey::Pubkey, // token1
    solana_sdk::pubkey::Pubkey, // token2
    solana_sdk::pubkey::Pubkey, // token3
) {
    // Create 3 SPL mints
    let mint1_pubkey = create_mint_helper(rpc, payer).await;
    let mint2_pubkey = create_mint_helper(rpc, payer).await;
    let mint3_pubkey = create_mint_helper(rpc, payer).await;

    println!("Created mint 1: {}", mint1_pubkey);
    println!("Created mint 2: {}", mint2_pubkey);
    println!("Created mint 3: {}", mint3_pubkey);

    // Create 3 SPL token accounts (one for each mint)
    let token_account1_keypair = Keypair::new();
    let token_account2_keypair = Keypair::new();
    let token_account3_keypair = Keypair::new();

    // Create token account for mint 1
    create_token_account(rpc, &mint1_pubkey, &token_account1_keypair, payer)
        .await
        .unwrap();

    // Create token account for mint 2
    create_token_account(rpc, &mint2_pubkey, &token_account2_keypair, payer)
        .await
        .unwrap();

    // Create token account for mint 3
    create_token_account(rpc, &mint3_pubkey, &token_account3_keypair, payer)
        .await
        .unwrap();

    println!(
        "Created token account 1: {}",
        token_account1_keypair.pubkey()
    );
    println!(
        "Created token account 2: {}",
        token_account2_keypair.pubkey()
    );
    println!(
        "Created token account 3: {}",
        token_account3_keypair.pubkey()
    );

    // Mint tokens to each account
    let mint_amount = 1_000_000; // 1000 tokens with 6 decimals

    // Mint to token account 1
    mint_spl_tokens(
        rpc,
        &mint1_pubkey,
        &token_account1_keypair.pubkey(),
        &payer.pubkey(), // owner
        payer,           // mint authority
        mint_amount,
        false, // not token22
    )
    .await
    .unwrap();

    // Mint to token account 2
    mint_spl_tokens(
        rpc,
        &mint2_pubkey,
        &token_account2_keypair.pubkey(),
        &payer.pubkey(), // owner
        payer,           // mint authority
        mint_amount,
        false, // not token22
    )
    .await
    .unwrap();

    // Mint to token account 3
    mint_spl_tokens(
        rpc,
        &mint3_pubkey,
        &token_account3_keypair.pubkey(),
        &payer.pubkey(), // owner
        payer,           // mint authority
        mint_amount,
        false, // not token22
    )
    .await
    .unwrap();

    println!("Minted {} tokens to each account", mint_amount);

    // Verify all token accounts have the correct balances
    verify_token_account_balance(
        rpc,
        &token_account1_keypair.pubkey(),
        &mint1_pubkey,
        &payer.pubkey(),
        mint_amount,
    )
    .await;
    verify_token_account_balance(
        rpc,
        &token_account2_keypair.pubkey(),
        &mint2_pubkey,
        &payer.pubkey(),
        mint_amount,
    )
    .await;
    verify_token_account_balance(
        rpc,
        &token_account3_keypair.pubkey(),
        &mint3_pubkey,
        &payer.pubkey(),
        mint_amount,
    )
    .await;

    (
        mint1_pubkey,
        mint2_pubkey,
        mint3_pubkey,
        token_account1_keypair.pubkey(),
        token_account2_keypair.pubkey(),
        token_account3_keypair.pubkey(),
    )
}

async fn verify_token_account_balance(
    rpc: &mut impl Rpc,
    token_account_pubkey: &solana_sdk::pubkey::Pubkey,
    expected_mint: &solana_sdk::pubkey::Pubkey,
    expected_owner: &solana_sdk::pubkey::Pubkey,
    expected_amount: u64,
) {
    use anchor_lang::AccountDeserialize;
    use anchor_spl::token::TokenAccount;

    let token_account_data = rpc
        .get_account(*token_account_pubkey)
        .await
        .unwrap()
        .unwrap();

    let token_account =
        TokenAccount::try_deserialize(&mut token_account_data.data.as_slice()).unwrap();

    assert_eq!(token_account.amount, expected_amount);
    assert_eq!(token_account.mint, *expected_mint);
    assert_eq!(token_account.owner, *expected_owner);

    println!(
        "✅ Verified token account {} has correct balance and properties",
        token_account_pubkey
    );
}

// Copy the working compress function from test.rs
async fn compress_spl_tokens(
    rpc: &mut impl Rpc,
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
    remaining_accounts.add_pre_accounts_metas(metas.as_slice());

    let output_tree_index = rpc
        .get_random_state_tree_info()
        .unwrap()
        .pack_output_tree_index(&mut remaining_accounts)
        .unwrap();

    let (remaining_accounts, _, _) = remaining_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: sdk_token_test::ID,
        accounts: remaining_accounts,
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

async fn compress_tokens(
    rpc: &mut impl Rpc,
    payer: &Keypair,
    sender_token_account: Pubkey,
    amount: u64,
    mint: Option<Pubkey>,
) -> Result<Signature, RpcError> {
    // Get mint from token account if not provided
    let mint = match mint {
        Some(mint) => mint,
        None => {
            let token_account_data = rpc
                .get_account(sender_token_account)
                .await?
                .ok_or_else(|| RpcError::CustomError("Token account not found".to_string()))?;

            let token_account = anchor_spl::token::TokenAccount::try_deserialize(
                &mut token_account_data.data.as_slice(),
            )
            .map_err(|e| {
                RpcError::CustomError(format!("Failed to deserialize token account: {}", e))
            })?;

            token_account.mint
        }
    };

    // Use the working compress function
    compress_spl_tokens(
        rpc,
        payer,
        payer.pubkey(), // recipient
        mint,
        amount,
        sender_token_account,
    )
    .await
}

async fn compress_tokens_bundled(
    rpc: &mut impl Rpc,
    payer: &Keypair,
    compressions: Vec<(Pubkey, u64, Option<Pubkey>)>, // (token_account, amount, optional_mint)
) -> Result<Vec<Signature>, RpcError> {
    let mut signatures = Vec::new();

    for (token_account, amount, mint) in compressions {
        let sig = compress_tokens(rpc, payer, token_account, amount, mint).await?;
        signatures.push(sig);
        println!(
            "✅ Compressed {} tokens from token account {}",
            amount, token_account
        );
    }

    Ok(signatures)
}

async fn create_compressed_escrow_pda(
    rpc: &mut (impl Rpc + Indexer),
    payer: &Keypair,
    initial_amount: u64,
) -> Result<[u8; 32], RpcError> {
    let tree_info = rpc.get_random_state_tree_info().unwrap();
    let mut remaining_accounts = PackedAccounts::default();
    remaining_accounts.add_pre_accounts_signer_mut(payer.pubkey());

    // Add system accounts configuration
    let config = SystemAccountMetaConfig::new(sdk_token_test::ID);
    remaining_accounts.add_system_accounts(config).unwrap();

    // Get address tree info and derive the PDA address
    let address_tree_info = rpc.get_address_tree_v1();
    let (address, address_seed) = derive_address(
        &[b"escrow", payer.pubkey().to_bytes().as_ref()],
        &address_tree_info.tree,
        &sdk_token_test::ID,
    );

    let output_tree_index = tree_info
        .pack_output_tree_index(&mut remaining_accounts)
        .unwrap();

    // Get validity proof with address
    let rpc_result = rpc
        .get_validity_proof(
            vec![], // No compressed accounts to prove
            vec![AddressWithTree {
                address,
                tree: address_tree_info.tree,
            }],
            None,
        )
        .await?
        .value;

    let packed_tree_info = rpc_result.pack_tree_infos(&mut remaining_accounts);
    let new_address_params = packed_tree_info.address_trees[0]
        .into_new_address_params_assigned_packed(address_seed, Some(0));

    let (accounts, _, _) = remaining_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: sdk_token_test::ID,
        accounts,
        data: sdk_token_test::instruction::CreateEscrowPda {
            proof: rpc_result.proof,
            output_tree_index,
            amount: initial_amount,
            address,
            new_address_params,
        }
        .data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await?;

    Ok(address)
}

#[allow(clippy::too_many_arguments)]
async fn test_four_invokes_instruction(
    rpc: &mut (impl Rpc + Indexer),
    payer: &Keypair,
    mint1: Pubkey,
    mint2: Pubkey,
    mint3: Pubkey,
    escrow_address: [u8; 32],
    initial_escrow_amount: u64,
    compression_token_account: Pubkey,
) -> Result<(), RpcError> {
    let default_pubkeys = CTokenDefaultAccounts::default();
    let mut remaining_accounts = PackedAccounts::default();
    let token_pool_pda1 = get_token_pool_pda(&mint1);
    // Remaining accounts 0
    remaining_accounts.add_pre_accounts_meta(AccountMeta::new(compression_token_account, false));
    // Remaining accounts 1
    remaining_accounts.add_pre_accounts_meta(AccountMeta::new(token_pool_pda1, false));
    // Remaining accounts 2
    remaining_accounts.add_pre_accounts_meta(AccountMeta::new(SPL_TOKEN_PROGRAM_ID.into(), false));
    // Remaining accounts 3
    remaining_accounts.add_pre_accounts_meta(AccountMeta::new(
        default_pubkeys.compressed_token_program,
        false,
    ));
    // Remaining accounts 4
    remaining_accounts
        .add_pre_accounts_meta(AccountMeta::new(default_pubkeys.cpi_authority_pda, false));

    // Add system accounts configuration with CPI context
    let tree_info = rpc.get_random_state_tree_info().unwrap();

    // Check if CPI context is available, otherwise this instruction can't work
    if tree_info.cpi_context.is_none() {
        panic!("CPI context account is required for four_invokes instruction but not available in tree_info");
    }

    let config = SystemAccountMetaConfig::new_with_cpi_context(
        sdk_token_test::ID,
        tree_info.cpi_context.unwrap(),
    );
    remaining_accounts.add_system_accounts(config).unwrap();

    // Get validity proof - need to prove the escrow PDA and compressed token accounts
    let escrow_account = rpc
        .get_compressed_account(escrow_address, None)
        .await?
        .value
        .ok_or_else(|| RpcError::CustomError("Escrow account not found".to_string()))?;

    // Get compressed token accounts for mint2 and mint3
    let compressed_token_accounts = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&payer.pubkey(), None, None)
        .await?
        .value
        .items;

    let mint2_token_account = compressed_token_accounts
        .iter()
        .find(|acc| acc.token.mint == mint2)
        .expect("Compressed token account for mint2 should exist");

    let mint3_token_account = compressed_token_accounts
        .iter()
        .find(|acc| acc.token.mint == mint3)
        .expect("Compressed token account for mint3 should exist");

    let rpc_result = rpc
        .get_validity_proof(
            vec![
                escrow_account.hash,
                mint2_token_account.account.hash,
                mint3_token_account.account.hash,
            ],
            vec![],
            None,
        )
        .await?
        .value;
    // We need to pack the tree after the cpi context.
    remaining_accounts.insert_or_get(rpc_result.accounts[0].tree_info.tree);

    let packed_tree_info = rpc_result.pack_tree_infos(&mut remaining_accounts);
    let output_tree_index = packed_tree_info
        .state_trees
        .as_ref()
        .unwrap()
        .output_tree_index;

    // Create token metas from compressed accounts - each uses its respective tree info index
    // Index 0: escrow PDA, Index 1: mint2 token account, Index 2: mint3 token account
    let mint2_tree_info = packed_tree_info
        .state_trees
        .as_ref()
        .unwrap()
        .packed_tree_infos[1];

    let mint3_tree_info = packed_tree_info
        .state_trees
        .as_ref()
        .unwrap()
        .packed_tree_infos[2];

    // Create FourInvokesParams
    let four_invokes_params = sdk_token_test::FourInvokesParams {
        compress_1: sdk_token_test::CompressParams {
            mint: mint1,
            amount: 500,
            recipient: payer.pubkey(),
            recipient_bump: 0,
            token_account: compression_token_account,
        },
        transfer_2: sdk_token_test::TransferParams {
            mint: mint2,
            transfer_amount: 300,
            token_metas: vec![light_compressed_token_sdk::TokenAccountMeta {
                amount: mint2_token_account.token.amount,
                delegate_index: None,
                packed_tree_info: mint2_tree_info,
                lamports: None,
                tlv: None,
            }],
            recipient: payer.pubkey(),
            recipient_bump: 0,
        },
        transfer_3: sdk_token_test::TransferParams {
            mint: mint3,
            transfer_amount: 200,
            token_metas: vec![light_compressed_token_sdk::TokenAccountMeta {
                amount: mint3_token_account.token.amount,
                delegate_index: None,
                packed_tree_info: mint3_tree_info,
                lamports: None,
                tlv: None,
            }],
            recipient: payer.pubkey(),
            recipient_bump: 0,
        },
    };

    // Create PdaParams - escrow PDA uses tree info index 0
    let escrow_tree_info = packed_tree_info
        .state_trees
        .as_ref()
        .unwrap()
        .packed_tree_infos[0];

    let pda_params = sdk_token_test::PdaParams {
        account_meta: light_sdk::instruction::account_meta::CompressedAccountMeta {
            address: escrow_address,
            tree_info: escrow_tree_info,
            output_state_tree_index: output_tree_index,
        },
        existing_amount: initial_escrow_amount,
    };

    let (accounts, system_accounts_start_offset, _) = remaining_accounts.to_account_metas();

    // We need to concat here to separate remaining accounts from the payer account.
    let accounts = [vec![AccountMeta::new(payer.pubkey(), true)], accounts].concat();
    let instruction = Instruction {
        program_id: sdk_token_test::ID,
        accounts,
        data: sdk_token_test::instruction::FourInvokes {
            output_tree_index,
            proof: rpc_result.proof,
            system_accounts_start_offset: system_accounts_start_offset as u8,
            four_invokes_params,
            pda_params,
        }
        .data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await?;

    Ok(())
}
