use anchor_lang::InstructionData;
use light_compressed_token_sdk::{
    instructions::batch_compress::{
        get_batch_compress_instruction_account_metas, BatchCompressMetaConfig, Recipient,
    },
    token_pool::find_token_pool_pda_with_index,
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
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
};

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
    let owner = Keypair::new();
    let deposit_amount = 1000u64;

    // Create recipients for batch compression
    let recipient1 = Keypair::new().pubkey();
    let recipient2 = Keypair::new().pubkey();

    let recipients = vec![
        Recipient {
            pubkey: recipient1,
            amount: 100_000,
        },
        Recipient {
            pubkey: recipient2,
            amount: 200_000,
        },
    ];

    // Execute batch compress (this will create mint, token account, and compress)
    batch_compress_spl_tokens(&mut rpc, &payer, recipients)
        .await
        .unwrap();

    println!("Batch compressed tokens successfully");

    // Derive the address that will be created for deposit
    let address_tree_info = rpc.get_address_tree_v1();
    let (address, _) = derive_address(
        &[b"deposit", owner.pubkey().to_bytes().as_ref()],
        &address_tree_info.tree,
        &sdk_token_test::ID,
    );

    // Create deposit instruction
    create_deposit_compressed_account(&mut rpc, &payer, owner.pubkey(), deposit_amount)
        .await
        .unwrap();

    println!("Created compressed account deposit successfully");

    // Verify the compressed account was created at the expected address
    let compressed_account = rpc
        .get_compressed_account(address, None)
        .await
        .unwrap()
        .value;

    println!("Created compressed account: {:?}", compressed_account);

    println!("Deposit compressed account test completed successfully!");
}

async fn create_deposit_compressed_account(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    owner: Pubkey,
    amount: u64,
) -> Result<Signature, RpcError> {
    let tree_info = rpc.get_random_state_tree_info().unwrap();

    let mut remaining_accounts = PackedAccounts::default();

    let output_tree_index = tree_info
        .pack_output_tree_index(&mut remaining_accounts)
        .unwrap();
    let config = SystemAccountMetaConfig::new_with_cpi_context(
        sdk_token_test::ID,
        tree_info.cpi_context.unwrap(),
    );
    remaining_accounts.add_system_accounts(config);

    let address_tree_info = rpc.get_address_tree_v1();

    let (address, _) = derive_address(
        &[b"deposit", owner.to_bytes().as_ref()],
        &address_tree_info.tree,
        &sdk_token_test::ID,
    );

    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![AddressWithTree {
                address,
                tree: address_tree_info.tree,
            }],
            None,
        )
        .await?
        .value;
    let packed_accounts = rpc_result.pack_tree_infos(&mut remaining_accounts);

    let (remaining_accounts, _, _) = remaining_accounts.to_account_metas();

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
            output_tree_index,
            deposit_amount: amount,
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
) -> Result<Signature, RpcError> {
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
        .await
}
