use anchor_lang::{prelude::AccountMeta, InstructionData};
use light_compressed_token_sdk::instructions::{
    create_compressed_mint, create_mint_to_compressed_instruction, CTokenDefaultAccounts,
    CreateCompressedMintInputs, MintToCompressedInputs,
};
use light_ctoken_types::{
    instructions::{
        mint_action::{CompressedMintWithContext, Recipient},
        transfer2::MultiInputTokenDataWithContext,
    },
    state::{BaseMint, CompressedMintMetadata},
    COMPRESSED_MINT_SEED,
};
use light_program_test::{AddressWithTree, Indexer, LightProgramTest, ProgramTestConfig, Rpc};
use light_sdk::{
    address::v1::derive_address,
    instruction::{PackedAccounts, PackedStateTreeInfo, SystemAccountMetaConfig},
};
use light_test_utils::RpcError;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

#[tokio::test]
async fn test_4_transfer2() {
    // Initialize the test environment
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("sdk_token_test", sdk_token_test::ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();

    let (mint1_pda, mint2_pda, mint3_pda, token_account_1) =
        create_compressed_mints_and_tokens(&mut rpc, &payer).await;

    println!("✅ Test setup complete: 3 compressed mints created with compressed tokens");

    // Create compressed escrow PDA
    let initial_amount = 100; // Initial escrow amount
    let escrow_address = create_compressed_escrow_pda(&mut rpc, &payer, initial_amount)
        .await
        .unwrap();

    println!(
        "✅ Created compressed escrow PDA with address: {:?}",
        escrow_address
    );

    // Test the four_transfer2 instruction
    test_four_transfer2_instruction(
        &mut rpc,
        &payer,
        mint1_pda,
        mint2_pda,
        mint3_pda,
        escrow_address,
        initial_amount,
        token_account_1,
    )
    .await
    .unwrap();

    println!("✅ Successfully executed four_transfer2 instruction");
}

async fn create_compressed_mints_and_tokens(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
) -> (Pubkey, Pubkey, Pubkey, Pubkey) {
    let decimals = 6u8;
    let compress_amount = 1000; // Amount to mint as compressed tokens

    // Create 3 compressed mints
    let (mint1_pda, mint1_pubkey) = create_compressed_mint_helper(rpc, payer, decimals).await;
    let (mint2_pda, mint2_pubkey) = create_compressed_mint_helper(rpc, payer, decimals).await;
    let (mint3_pda, mint3_pubkey) = create_compressed_mint_helper(rpc, payer, decimals).await;

    println!("Created compressed mint 1: {}", mint1_pubkey);
    println!("Created compressed mint 2: {}", mint2_pubkey);
    println!("Created compressed mint 3: {}", mint3_pubkey);

    // Mint compressed tokens for all three mints
    mint_compressed_tokens(rpc, payer, &mint1_pda, mint1_pubkey, compress_amount).await;
    mint_compressed_tokens(rpc, payer, &mint2_pda, mint2_pubkey, compress_amount).await;
    mint_compressed_tokens(rpc, payer, &mint3_pda, mint3_pubkey, compress_amount).await;

    // Create associated token account for mint1 decompression
    let (token_account1_pubkey, _bump) =
        light_compressed_token_sdk::instructions::derive_ctoken_ata(&payer.pubkey(), &mint1_pda);
    let create_ata_instruction =
        light_compressed_token_sdk::instructions::create_associated_token_account(
            payer.pubkey(),
            payer.pubkey(),
            mint1_pda,
        )
        .unwrap();
    rpc.create_and_send_transaction(&[create_ata_instruction], &payer.pubkey(), &[payer])
        .await
        .unwrap();

    // Decompress some compressed tokens for mint1 into the associated token account
    let decompress_amount = 500u64;
    let compressed_token_accounts = rpc
        .indexer()
        .unwrap()
        .get_compressed_token_accounts_by_owner(&payer.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    let mint1_token_account = compressed_token_accounts
        .iter()
        .find(|acc| acc.token.mint == mint1_pda)
        .expect("Compressed token account for mint1 should exist");

    let decompress_instruction =
        light_token_client::instructions::transfer2::create_decompress_instruction(
            rpc,
            std::slice::from_ref(mint1_token_account),
            decompress_amount,
            token_account1_pubkey,
            payer.pubkey(),
        )
        .await
        .unwrap();

    rpc.create_and_send_transaction(&[decompress_instruction], &payer.pubkey(), &[payer])
        .await
        .unwrap();

    println!(
        "✅ Minted {} compressed tokens for all three mints and decompressed {} tokens for mint1",
        compress_amount, decompress_amount
    );

    (mint1_pda, mint2_pda, mint3_pda, token_account1_pubkey)
}

async fn create_compressed_mint_helper(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    decimals: u8,
) -> (Pubkey, Pubkey) {
    let mint_authority = payer.pubkey();
    let mint_signer = Keypair::new();
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    // Find mint PDA
    let compressed_token_program_id =
        Pubkey::new_from_array(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID);
    let (mint_pda, mint_bump) = Pubkey::find_program_address(
        &[COMPRESSED_MINT_SEED, mint_signer.pubkey().as_ref()],
        &compressed_token_program_id,
    );

    // Derive compressed mint address
    let address_seed = mint_pda.to_bytes();
    let compressed_mint_address = light_compressed_account::address::derive_address(
        &address_seed,
        &address_tree_pubkey.to_bytes(),
        &compressed_token_program_id.to_bytes(),
    );

    // Get validity proof
    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![AddressWithTree {
                address: compressed_mint_address,
                tree: address_tree_pubkey,
            }],
            None,
        )
        .await
        .unwrap()
        .value;

    // Create compressed mint
    let instruction = create_compressed_mint(CreateCompressedMintInputs {
        version: 3,
        decimals,
        mint_authority,
        freeze_authority: None,
        proof: rpc_result.proof.0.unwrap(),
        mint_bump,
        address_merkle_tree_root_index: rpc_result.addresses[0].root_index,
        mint_signer: mint_signer.pubkey(),
        payer: payer.pubkey(),
        address_tree_pubkey,
        output_queue,
        extensions: None,
    })
    .unwrap();

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer, &mint_signer])
        .await
        .unwrap();

    (mint_pda, compressed_mint_address.into())
}

async fn mint_compressed_tokens(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    mint_pda: &Pubkey,
    mint_pubkey: Pubkey,
    amount: u64,
) {
    let tree_info = rpc.get_random_state_tree_info().unwrap();
    let output_queue = tree_info.queue;

    // Get the compressed mint account to use in the inputs
    let compressed_mint_account = rpc
        .indexer()
        .unwrap()
        .get_compressed_account(mint_pubkey.to_bytes(), None)
        .await
        .unwrap()
        .value
        .ok_or("Compressed mint account not found")
        .unwrap();

    // Create expected compressed mint for the input
    let expected_compressed_mint = light_ctoken_types::state::CompressedMint {
        base: BaseMint {
            mint_authority: Some(payer.pubkey().into()),
            supply: 0,
            decimals: 6,
            is_initialized: true,
            freeze_authority: None,
        },
        metadata: CompressedMintMetadata {
            version: 3,
            mint: mint_pda.into(),
            spl_mint_initialized: false,
        },
        extensions: None,
    };

    let mint_to_instruction = create_mint_to_compressed_instruction(
        MintToCompressedInputs {
            cpi_context_pubkey: None,
            compressed_mint_inputs: CompressedMintWithContext {
                prove_by_index: true,
                leaf_index: compressed_mint_account.leaf_index,
                root_index: 0,
                address: compressed_mint_account.address.unwrap(),
                mint: expected_compressed_mint.try_into().unwrap(),
            },
            proof: None,
            recipients: vec![Recipient {
                recipient: payer.pubkey().into(),
                amount,
            }],
            mint_authority: payer.pubkey(),
            payer: payer.pubkey(),
            state_merkle_tree: compressed_mint_account.tree_info.tree,
            input_queue: compressed_mint_account.tree_info.queue,
            output_queue_cmint: compressed_mint_account.tree_info.queue,
            output_queue_tokens: output_queue,
            decompressed_mint_config: None,
            token_account_version: 2,
            token_pool: None,
        },
        None,
    )
    .unwrap();

    rpc.create_and_send_transaction(&[mint_to_instruction], &payer.pubkey(), &[payer])
        .await
        .unwrap();
}

async fn create_compressed_escrow_pda(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    initial_amount: u64,
) -> Result<[u8; 32], RpcError> {
    let tree_info = rpc.get_random_state_tree_info().unwrap();
    let mut remaining_accounts = PackedAccounts::default();
    remaining_accounts.add_pre_accounts_signer_mut(payer.pubkey());

    // Add system accounts configuration
    let config = SystemAccountMetaConfig::new(sdk_token_test::ID);
    remaining_accounts.add_system_accounts_v2(config).unwrap();

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
async fn test_four_transfer2_instruction(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    mint1: Pubkey,
    mint2: Pubkey,
    mint3: Pubkey,
    escrow_address: [u8; 32],
    initial_escrow_amount: u64,
    token_account_1: Pubkey,
) -> Result<(), RpcError> {
    let default_pubkeys = CTokenDefaultAccounts::default();
    let mut remaining_accounts = PackedAccounts::default();
    // We don't need SPL token accounts for this test since we're using compressed tokens
    // Just add the compressed token program and CPI authority PDA
    // Remaining accounts 0
    remaining_accounts.add_pre_accounts_meta(AccountMeta::new(
        default_pubkeys.compressed_token_program,
        false,
    ));
    // Remaining accounts 1
    remaining_accounts
        .add_pre_accounts_meta(AccountMeta::new(default_pubkeys.cpi_authority_pda, false));
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

    let cpi_context = mint2_token_account
        .account
        .tree_info
        .cpi_context
        .expect("CPI context should exist");

    let config = SystemAccountMetaConfig::new_with_cpi_context(sdk_token_test::ID, cpi_context);
    remaining_accounts.add_system_accounts_v2(config).unwrap();
    println!("next index {}", remaining_accounts.packed_pubkeys().len());

    // Get validity proof - need to prove the escrow PDA and compressed token accounts
    let escrow_account = rpc
        .get_compressed_account(escrow_address, None)
        .await?
        .value
        .ok_or_else(|| RpcError::CustomError("Escrow account not found".to_string()))?;

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

    // Create FourTransfer2Params
    let four_transfer2_params = sdk_token_test::process_four_transfer2::FourTransfer2Params {
        compress_1: sdk_token_test::process_four_transfer2::CompressParams {
            mint: remaining_accounts.insert_or_get(mint1),
            amount: 500,
            recipient: remaining_accounts.insert_or_get(payer.pubkey()),
            solana_token_account: remaining_accounts.insert_or_get(token_account_1),
            authority: remaining_accounts.insert_or_get(payer.pubkey()), // Payer is the authority for compression
        },
        transfer_2: sdk_token_test::process_four_transfer2::TransferParams {
            transfer_amount: 300,
            token_metas: vec![pack_input_token_account(
                mint2_token_account,
                &mint2_tree_info,
                &mut remaining_accounts,
                &mut Vec::new(),
            )],
            recipient: remaining_accounts.insert_or_get(payer.pubkey()),
        },
        transfer_3: sdk_token_test::process_four_transfer2::TransferParams {
            transfer_amount: 200,
            token_metas: vec![pack_input_token_account(
                mint3_token_account,
                &mint3_tree_info,
                &mut remaining_accounts,
                &mut Vec::new(),
            )],
            recipient: remaining_accounts.insert_or_get(payer.pubkey()),
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

    let (accounts, system_accounts_start_offset, tree_accounts_start_offset) =
        remaining_accounts.to_account_metas();
    let packed_accounts_start_offset = tree_accounts_start_offset;
    println!("accounts {:?}", accounts);
    println!(
        "system_accounts_start_offset {}",
        system_accounts_start_offset
    );
    println!(
        "packed_accounts_start_offset {}",
        packed_accounts_start_offset
    );
    println!(
        "accounts packed_accounts_start_offset {:?}",
        accounts[packed_accounts_start_offset..].to_vec()
    );

    // We need to concat here to separate remaining accounts from the payer account.
    let accounts = [vec![AccountMeta::new(payer.pubkey(), true)], accounts].concat();
    let instruction = Instruction {
        program_id: sdk_token_test::ID,
        accounts,
        data: sdk_token_test::instruction::FourTransfer2 {
            output_tree_index,
            proof: rpc_result.proof,
            system_accounts_start_offset: system_accounts_start_offset as u8,
            packed_accounts_start_offset: tree_accounts_start_offset as u8,
            four_transfer2_params,
            pda_params,
        }
        .data(),
    };
    // Print test setup values
    println!("=== TEST SETUP VALUES ===");
    println!("  mint1_pda: {}", mint1);
    println!("  mint2_pda: {}", mint2);
    println!("  mint3_pda: {}", mint3);
    println!("  token_account_1: {}", token_account_1);
    println!("  escrow_address: {:?}", escrow_address);
    println!("  initial_escrow_amount: {}", initial_escrow_amount);
    println!("  payer: {}", payer.pubkey());

    // Print all instruction accounts with names
    println!("=== INSTRUCTION ACCOUNTS ===");
    for (i, account) in instruction.accounts.iter().enumerate() {
        let name = match i {
            0 => "payer",
            1 => "compressed_token_program",
            2 => "cpi_authority_pda",
            3 => "system_program",
            4 => "light_system_program",
            5 => "account_compression_authority",
            6 => "noop_program",
            7 => "registered_program_pda",
            8 => "account_compression_program",
            9 => "self_program",
            10 => "sol_pool_pda",
            i if i >= 11 && i < 11 + system_accounts_start_offset => &format!("tree_{}", i - 11),
            _ => "remaining_account",
        };
        println!("  {}: {} - {}", i, name, account.pubkey);
    }
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await?;

    Ok(())
}

fn pack_input_token_account(
    account: &light_client::indexer::CompressedTokenAccount,
    tree_info: &PackedStateTreeInfo,
    packed_accounts: &mut PackedAccounts,
    in_lamports: &mut Vec<u64>,
) -> MultiInputTokenDataWithContext {
    let delegate_index = if let Some(delegate) = account.token.delegate {
        packed_accounts.insert_or_get_read_only(delegate) // TODO: cover delegated transfer
    } else {
        0
    };
    println!("account {:?}", account);
    if account.account.lamports != 0 {
        in_lamports.push(account.account.lamports);
    }
    MultiInputTokenDataWithContext {
        amount: account.token.amount,
        merkle_context: light_compressed_account::compressed_account::PackedMerkleContext {
            merkle_tree_pubkey_index: tree_info.merkle_tree_pubkey_index,
            queue_pubkey_index: tree_info.queue_pubkey_index,
            leaf_index: tree_info.leaf_index,
            prove_by_index: tree_info.prove_by_index,
        },
        root_index: tree_info.root_index,
        mint: packed_accounts.insert_or_get_read_only(account.token.mint),
        owner: packed_accounts.insert_or_get_config(account.token.owner, true, false),
        has_delegate: account.token.delegate.is_some(),
        delegate: delegate_index,
        version: 2,
    }
}
