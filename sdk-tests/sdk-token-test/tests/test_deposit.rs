use anchor_lang::InstructionData;
use light_client::indexer::{CompressedAccount, CompressedTokenAccount, IndexerRpcConfig};
use light_compressed_token_sdk::{
    instructions::{
        batch_compress::{
            get_batch_compress_instruction_account_metas, BatchCompressMetaConfig, Recipient,
        },
        CTokenDefaultAccounts,
    },
    token_pool::find_token_pool_pda_with_index,
    TokenAccountMeta, SPL_TOKEN_PROGRAM_ID,
};
use light_program_test::{AddressWithTree, Indexer, LightProgramTest, ProgramTestConfig, Rpc};
use light_sdk::{
    address::v1::derive_address,
    instruction::{account_meta::CompressedAccountMeta, PackedAccounts, SystemAccountMetaConfig},
};
use light_test_utils::{
    spl::{create_mint_helper, create_token_account, mint_spl_tokens},
    RpcError,
};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
};

#[ignore = "fix cpi context usage"]
#[tokio::test]
async fn test_deposit_compressed_account() {
    // Initialize the test environment
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("sdk_token_test", sdk_token_test::ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let deposit_amount = 1000u64;

    let recipients = vec![Recipient {
        pubkey: payer.pubkey(),
        amount: 100_000_000,
    }];

    // Execute batch compress (this will create mint, token account, and compress)
    batch_compress_spl_tokens(&mut rpc, &payer, recipients.clone())
        .await
        .unwrap();

    println!("Batch compressed tokens successfully");

    // Fetch the compressed token accounts created by batch compress
    let recipient1 = recipients[0].pubkey;
    let compressed_accounts = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&recipient1, None, None)
        .await
        .unwrap()
        .value
        .items;

    assert!(
        !compressed_accounts.is_empty(),
        "Should have compressed token accounts"
    );
    let ctoken_account = &compressed_accounts[0];

    println!(
        "Found compressed token account: amount={}, owner={}",
        ctoken_account.token.amount, ctoken_account.token.owner
    );

    // Derive the address that will be created for deposit
    let address_tree_info = rpc.get_address_tree_v1();
    let (deposit_address, _) = derive_address(
        &[b"escrow", payer.pubkey().to_bytes().as_ref()],
        &address_tree_info.tree,
        &sdk_token_test::ID,
    );

    // Derive recipient PDA from the deposit address
    let (recipient_pda, recipient_bump) =
        Pubkey::find_program_address(&[b"escrow", deposit_address.as_ref()], &sdk_token_test::ID);
    println!("seeds: {:?}", b"escrow");
    println!("seeds: {:?}", deposit_address);
    println!("recipient_bump: {:?}", recipient_bump);
    // Create deposit instruction with the compressed token account
    create_deposit_compressed_account(
        &mut rpc,
        &payer,
        ctoken_account,
        recipient_bump,
        deposit_amount,
    )
    .await
    .unwrap();

    println!("Created compressed account deposit successfully");

    // Verify the compressed account was created at the expected address
    let compressed_account = rpc
        .get_compressed_account(deposit_address, None)
        .await
        .unwrap()
        .value
        .ok_or("Compressed account not found")
        .unwrap();

    println!("Created compressed account: {:?}", compressed_account);

    println!("Deposit compressed account test completed successfully!");

    let slot = rpc.get_slot().await.unwrap();

    let deposit_account = rpc
        .get_compressed_token_accounts_by_owner(
            &payer.pubkey(),
            None,
            Some(IndexerRpcConfig {
                slot,
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .value
        .items[0]
        .clone();
    let escrow_token_account = rpc
        .get_compressed_token_accounts_by_owner(&recipient_pda, None, None)
        .await
        .unwrap()
        .value
        .items[0]
        .clone();

    update_deposit_compressed_account(
        &mut rpc,
        &payer,
        &deposit_account,
        &escrow_token_account,
        compressed_account,
        recipient_bump,
        deposit_amount,
    )
    .await
    .unwrap();
}

async fn create_deposit_compressed_account(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    ctoken_account: &CompressedTokenAccount,
    recipient_bump: u8,
    amount: u64,
) -> Result<Signature, RpcError> {
    let tree_info = rpc.get_random_state_tree_info().unwrap();
    println!("tree_info {:?}", tree_info);

    let mut remaining_accounts = PackedAccounts::default();
    // new_with_anchor_none is only recommended for pinocchio else additional account infos cost approx 1k CU
    // used here for consistentcy with into_account_infos_checked
    // let config = TokenAccountsMetaConfig::new_client();
    // let metas = get_transfer_instruction_account_metas(config);
    // remaining_accounts.add_pre_accounts_metas(metas);
    // Alternative even though we pass fewer account infos this is minimally more efficient.
    let default_pubkeys = CTokenDefaultAccounts::default();
    remaining_accounts.add_pre_accounts_meta(AccountMeta::new(
        default_pubkeys.compressed_token_program,
        false,
    ));
    remaining_accounts
        .add_pre_accounts_meta(AccountMeta::new(default_pubkeys.cpi_authority_pda, false));

    let config = SystemAccountMetaConfig::new_with_cpi_context(
        sdk_token_test::ID,
        tree_info.cpi_context.unwrap(),
    );
    println!("cpi_context {:?}", config);
    remaining_accounts.add_system_accounts(config).unwrap();
    let address_tree_info = rpc.get_address_tree_v1();

    let (address, _) = derive_address(
        &[b"escrow", payer.pubkey().to_bytes().as_ref()],
        &address_tree_info.tree,
        &sdk_token_test::ID,
    );

    // Get mint from the compressed token account
    let mint = ctoken_account.token.mint;
    println!(
        "ctoken_account.account.hash {:?}",
        ctoken_account.account.hash
    );
    println!("ctoken_account.account {:?}", ctoken_account.account);
    // Get validity proof for the compressed token account and new address
    let rpc_result = rpc
        .get_validity_proof(
            vec![ctoken_account.account.hash],
            vec![AddressWithTree {
                address,
                tree: address_tree_info.tree,
            }],
            None,
        )
        .await?
        .value;
    let packed_accounts = rpc_result.pack_tree_infos(&mut remaining_accounts);
    println!("packed_accounts {:?}", packed_accounts.state_trees);

    // Create token meta from compressed account
    let tree_info = packed_accounts
        .state_trees
        .as_ref()
        .unwrap()
        .packed_tree_infos[0];

    let token_metas = vec![TokenAccountMeta {
        amount: ctoken_account.token.amount,
        delegate_index: None,
        packed_tree_info: tree_info,
        lamports: None,
        tlv: None,
    }];

    let (remaining_accounts, system_accounts_start_offset, _packed_accounts_start_offset) =
        remaining_accounts.to_account_metas();
    let system_accounts_start_offset = system_accounts_start_offset as u8;
    println!("remaining_accounts {:?}", remaining_accounts);
    let instruction = Instruction {
        program_id: sdk_token_test::ID,
        accounts: [
            vec![AccountMeta::new(payer.pubkey(), true)],
            remaining_accounts,
        ]
        .concat(),
        data: sdk_token_test::instruction::Deposit {
            proof: rpc_result.proof,
            address_tree_info: packed_accounts.address_trees[0],
            output_tree_index: packed_accounts.state_trees.unwrap().output_tree_index,
            deposit_amount: amount,
            token_metas,
            mint,
            recipient_bump,
            system_accounts_start_offset,
        }
        .data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}

async fn update_deposit_compressed_account(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    deposit_ctoken_account: &CompressedTokenAccount,
    escrow_ctoken_account: &CompressedTokenAccount,
    escrow_pda: CompressedAccount,
    recipient_bump: u8,
    amount: u64,
) -> Result<Signature, RpcError> {
    println!("deposit_ctoken_account {:?}", deposit_ctoken_account);
    println!("escrow_ctoken_account {:?}", escrow_ctoken_account);
    println!("escrow_pda {:?}", escrow_pda);
    let rpc_result = rpc
        .get_validity_proof(
            vec![
                escrow_pda.hash,
                deposit_ctoken_account.account.hash,
                escrow_ctoken_account.account.hash,
            ],
            vec![],
            None,
        )
        .await?
        .value;
    let mut remaining_accounts = PackedAccounts::default();

    let default_pubkeys = CTokenDefaultAccounts::default();
    remaining_accounts.add_pre_accounts_meta(AccountMeta::new(
        default_pubkeys.compressed_token_program,
        false,
    ));
    remaining_accounts
        .add_pre_accounts_meta(AccountMeta::new(default_pubkeys.cpi_authority_pda, false));

    let config = SystemAccountMetaConfig::new_with_cpi_context(
        sdk_token_test::ID,
        rpc_result.accounts[0].tree_info.cpi_context.unwrap(),
    );
    println!("pre accounts {:?}", remaining_accounts.pre_accounts);

    println!("cpi_context {:?}", config);
    remaining_accounts.add_system_accounts(config).unwrap();
    println!(
        "rpc_result.accounts[0].tree_info.tree {:?}",
        rpc_result.accounts[0].tree_info.tree.to_bytes()
    );
    println!(
        "rpc_result.accounts[0].tree_info.queue {:?}",
        rpc_result.accounts[0].tree_info.queue.to_bytes()
    );
    // We need to pack the tree after the cpi context.
    let index = remaining_accounts.insert_or_get(rpc_result.accounts[0].tree_info.tree);
    println!("index {}", index);
    // Get mint from the compressed token account
    let mint = deposit_ctoken_account.token.mint;
    println!(
        "ctoken_account.account.hash {:?}",
        deposit_ctoken_account.account.hash
    );
    println!(
        "deposit_ctoken_account.account {:?}",
        deposit_ctoken_account.account
    );
    // Get validity proof for the compressed token account and new address
    println!("rpc_result {:?}", rpc_result);

    let packed_accounts = rpc_result.pack_tree_infos(&mut remaining_accounts);
    println!("packed_accounts {:?}", packed_accounts.state_trees);
    // TODO: investigate why packed_tree_infos seem to be out of order
    // Create token meta from compressed account
    let tree_info = packed_accounts
        .state_trees
        .as_ref()
        .unwrap()
        .packed_tree_infos[1];
    let depositing_token_metas = vec![TokenAccountMeta {
        amount: deposit_ctoken_account.token.amount,
        delegate_index: None,
        packed_tree_info: tree_info,
        lamports: None,
        tlv: None,
    }];
    println!("depositing_token_metas {:?}", depositing_token_metas);
    let tree_info = packed_accounts
        .state_trees
        .as_ref()
        .unwrap()
        .packed_tree_infos[2];
    let escrowed_token_meta = TokenAccountMeta {
        amount: escrow_ctoken_account.token.amount,
        delegate_index: None,
        packed_tree_info: tree_info,
        lamports: None,
        tlv: None,
    };
    println!("escrowed_token_meta {:?}", escrowed_token_meta);

    let (remaining_accounts, system_accounts_start_offset, _packed_accounts_start_offset) =
        remaining_accounts.to_account_metas();
    let system_accounts_start_offset = system_accounts_start_offset as u8;
    println!("remaining_accounts {:?}", remaining_accounts);

    let tree_info = packed_accounts
        .state_trees
        .as_ref()
        .unwrap()
        .packed_tree_infos[0];
    let account_meta = CompressedAccountMeta {
        tree_info,
        address: escrow_pda.address.unwrap(),
        output_state_tree_index: packed_accounts
            .state_trees
            .as_ref()
            .unwrap()
            .output_tree_index,
    };

    let instruction = Instruction {
        program_id: sdk_token_test::ID,
        accounts: [
            vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(escrow_ctoken_account.token.owner, false),
            ],
            remaining_accounts,
        ]
        .concat(),
        data: sdk_token_test::instruction::UpdateDeposit {
            proof: rpc_result.proof,
            output_tree_index: packed_accounts
                .state_trees
                .as_ref()
                .unwrap()
                .packed_tree_infos[0]
                .merkle_tree_pubkey_index,
            output_tree_queue_index: packed_accounts.state_trees.unwrap().packed_tree_infos[0]
                .queue_pubkey_index,
            system_accounts_start_offset,
            token_params: sdk_token_test::TokenParams {
                deposit_amount: amount,
                depositing_token_metas,
                mint,
                escrowed_token_meta,
                recipient_bump,
            },
            pda_params: sdk_token_test::PdaParams {
                account_meta,
                existing_amount: amount,
            },
        }
        .data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}

async fn batch_compress_spl_tokens(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    recipients: Vec<Recipient>,
) -> Result<Pubkey, RpcError> {
    // Create mint and token account
    let mint = create_mint_helper(rpc, payer).await;
    println!("Created mint: {}", mint);

    let token_account_keypair = Keypair::new();
    create_token_account(rpc, &mint, &token_account_keypair, payer)
        .await
        .unwrap();

    println!("Created token account: {}", token_account_keypair.pubkey());

    // Calculate total amount needed and mint tokens
    let total_amount: u64 = recipients.iter().map(|r| r.amount).sum();
    let mint_amount = total_amount + 100_000; // Add some buffer

    mint_spl_tokens(
        rpc,
        &mint,
        &token_account_keypair.pubkey(),
        &payer.pubkey(),
        payer,
        mint_amount,
        false,
    )
    .await
    .unwrap();

    println!("Minted {} tokens to account", mint_amount);

    let token_account = token_account_keypair.pubkey();
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
        .await?;

    Ok(mint)
}
