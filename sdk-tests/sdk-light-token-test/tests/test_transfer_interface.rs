// Tests for TransferInterfaceCpi - unified transfer interface that auto-detects account types

mod shared;

use borsh::BorshSerialize;
use light_client::rpc::Rpc;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_test_utils::spl::{
    create_mint_helper, create_token_2022_account, mint_spl_tokens, CREATE_MINT_HELPER_DECIMALS,
};
use light_token_sdk::{
    spl_interface::find_spl_interface_pda_with_index,
    token::{derive_token_ata, CompressibleParams, CreateAssociatedTokenAccount},
};
use light_token_types::CPI_AUTHORITY_PDA;
use native_ctoken_examples::{TransferInterfaceData, ID, TRANSFER_INTERFACE_AUTHORITY_SEED};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};

// =============================================================================
// INVOKE TESTS (regular signer authority)
// =============================================================================

/// Test TransferInterfaceCpi: SPL -> Light Token (invoke)
#[tokio::test]
async fn test_transfer_interface_spl_to_ctoken_invoke() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let sender = Keypair::new();
    light_test_utils::airdrop_lamports(&mut rpc, &sender.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    // Create SPL mint and token account
    let mint = create_mint_helper(&mut rpc, &payer).await;
    let amount = 10000u64;
    let transfer_amount = 5000u64;

    let spl_token_account_keypair = Keypair::new();
    create_token_2022_account(&mut rpc, &mint, &spl_token_account_keypair, &sender, false)
        .await
        .unwrap();
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &spl_token_account_keypair.pubkey(),
        &payer.pubkey(),
        &payer,
        amount,
        false,
    )
    .await
    .unwrap();

    // Create Light Token ATA for recipient
    let recipient = Keypair::new();
    light_test_utils::airdrop_lamports(&mut rpc, &recipient.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    let instruction = CreateAssociatedTokenAccount::new(payer.pubkey(), recipient.pubkey(), mint)
        .instruction()
        .unwrap();
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();
    let ctoken_account = derive_token_ata(&recipient.pubkey(), &mint).0;

    // Get token pool PDA
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint, 0, false);
    let compressed_token_program_id =
        Pubkey::new_from_array(light_token_interface::LIGHT_TOKEN_PROGRAM_ID);
    let cpi_authority_pda = Pubkey::new_from_array(CPI_AUTHORITY_PDA);

    // Build wrapper instruction
    let data = TransferInterfaceData {
        amount: transfer_amount,
        spl_interface_pda_bump: Some(spl_interface_pda_bump),
        decimals: CREATE_MINT_HELPER_DECIMALS,
    };
    // Discriminator 19 = TransferInterfaceInvoke
    let wrapper_instruction_data = [vec![19u8], data.try_to_vec().unwrap()].concat();

    let wrapper_accounts = vec![
        AccountMeta::new_readonly(compressed_token_program_id, false),
        AccountMeta::new(spl_token_account_keypair.pubkey(), false), // source (SPL)
        AccountMeta::new(ctoken_account, false),                     // destination (Light Token)
        AccountMeta::new_readonly(sender.pubkey(), true),            // authority (signer)
        AccountMeta::new(payer.pubkey(), true),                      // payer
        AccountMeta::new_readonly(cpi_authority_pda, false), // compressed_token_program_authority
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false), // system_program
        AccountMeta::new_readonly(mint, false),              // mint (for SPL bridge)
        AccountMeta::new(spl_interface_pda, false),          // spl_interface_pda
        AccountMeta::new_readonly(anchor_spl::token::ID, false), // spl_token_program
    ];

    let instruction = Instruction {
        program_id: ID,
        accounts: wrapper_accounts,
        data: wrapper_instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &sender])
        .await
        .unwrap();

    // Verify balances
    use spl_token_2022::pod::PodAccount;
    let spl_account_data = rpc
        .get_account(spl_token_account_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    let spl_account =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&spl_account_data.data).unwrap();
    assert_eq!(u64::from(spl_account.amount), amount - transfer_amount);

    let ctoken_account_data = rpc.get_account(ctoken_account).await.unwrap().unwrap();
    let ctoken_state =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&ctoken_account_data.data[..165]).unwrap();
    assert_eq!(u64::from(ctoken_state.amount), transfer_amount);

    println!("TransferInterface SPL->Light Token invoke test passed");
}

/// Test TransferInterface: Light Token -> SPL (invoke)
#[tokio::test]
async fn test_transfer_interface_ctoken_to_spl_invoke() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let owner = Keypair::new();
    light_test_utils::airdrop_lamports(&mut rpc, &owner.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    let mint = create_mint_helper(&mut rpc, &payer).await;
    let amount = 10000u64;
    let transfer_amount = 5000u64;

    // Create destination SPL token account
    let spl_token_account_keypair = Keypair::new();
    create_token_2022_account(&mut rpc, &mint, &spl_token_account_keypair, &owner, false)
        .await
        .unwrap();

    // Create and fund Light Token ATA
    let instruction = CreateAssociatedTokenAccount::new(payer.pubkey(), owner.pubkey(), mint)
        .instruction()
        .unwrap();
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();
    let ctoken_account = derive_token_ata(&owner.pubkey(), &mint).0;

    // Fund Light Token via temporary SPL account
    let temp_spl_keypair = Keypair::new();
    create_token_2022_account(&mut rpc, &mint, &temp_spl_keypair, &owner, false)
        .await
        .unwrap();
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &temp_spl_keypair.pubkey(),
        &payer.pubkey(),
        &payer,
        amount,
        false,
    )
    .await
    .unwrap();

    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint, 0, false);
    let compressed_token_program_id =
        Pubkey::new_from_array(light_token_interface::LIGHT_TOKEN_PROGRAM_ID);
    let cpi_authority_pda = Pubkey::new_from_array(CPI_AUTHORITY_PDA);

    // Transfer SPL to Light Token to fund it
    {
        let data = TransferInterfaceData {
            amount,
            spl_interface_pda_bump: Some(spl_interface_pda_bump),
            decimals: CREATE_MINT_HELPER_DECIMALS,
        };
        let wrapper_instruction_data = [vec![19u8], data.try_to_vec().unwrap()].concat();
        let wrapper_accounts = vec![
            AccountMeta::new_readonly(compressed_token_program_id, false),
            AccountMeta::new(temp_spl_keypair.pubkey(), false),
            AccountMeta::new(ctoken_account, false),
            AccountMeta::new_readonly(owner.pubkey(), true),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(cpi_authority_pda, false),
            AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(spl_interface_pda, false),
            AccountMeta::new_readonly(anchor_spl::token::ID, false),
        ];
        let instruction = Instruction {
            program_id: ID,
            accounts: wrapper_accounts,
            data: wrapper_instruction_data,
        };
        rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &owner])
            .await
            .unwrap();
    }

    // Now test Light Token -> SPL transfer
    let data = TransferInterfaceData {
        amount: transfer_amount,
        spl_interface_pda_bump: Some(spl_interface_pda_bump),
        decimals: CREATE_MINT_HELPER_DECIMALS,
    };
    let wrapper_instruction_data = [vec![19u8], data.try_to_vec().unwrap()].concat();

    let wrapper_accounts = vec![
        AccountMeta::new_readonly(compressed_token_program_id, false),
        AccountMeta::new(ctoken_account, false), // source (Light Token)
        AccountMeta::new(spl_token_account_keypair.pubkey(), false), // destination (SPL)
        AccountMeta::new_readonly(owner.pubkey(), true), // authority
        AccountMeta::new(payer.pubkey(), true),  // payer
        AccountMeta::new_readonly(cpi_authority_pda, false),
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(spl_interface_pda, false),
        AccountMeta::new_readonly(anchor_spl::token::ID, false),
    ];

    let instruction = Instruction {
        program_id: ID,
        accounts: wrapper_accounts,
        data: wrapper_instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &owner])
        .await
        .unwrap();

    // Verify balances
    use spl_token_2022::pod::PodAccount;
    let spl_account_data = rpc
        .get_account(spl_token_account_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    let spl_account =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&spl_account_data.data).unwrap();
    assert_eq!(u64::from(spl_account.amount), transfer_amount);

    let ctoken_account_data = rpc.get_account(ctoken_account).await.unwrap().unwrap();
    let ctoken_state =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&ctoken_account_data.data[..165]).unwrap();
    assert_eq!(u64::from(ctoken_state.amount), amount - transfer_amount);

    println!("TransferInterface Light Token->SPL invoke test passed");
}

/// Test TransferInterface: Light Token -> Light Token (invoke)
#[tokio::test]
async fn test_transfer_interface_ctoken_to_ctoken_invoke() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let sender = Keypair::new();
    let recipient = Keypair::new();
    light_test_utils::airdrop_lamports(&mut rpc, &sender.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    light_test_utils::airdrop_lamports(&mut rpc, &recipient.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    let mint = create_mint_helper(&mut rpc, &payer).await;
    let amount = 10000u64;
    let transfer_amount = 5000u64;

    // Create sender Light Token ATA
    let instruction = CreateAssociatedTokenAccount::new(payer.pubkey(), sender.pubkey(), mint)
        .instruction()
        .unwrap();
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();
    let sender_ctoken = derive_token_ata(&sender.pubkey(), &mint).0;

    // Create recipient Light Token ATA
    let instruction = CreateAssociatedTokenAccount::new(payer.pubkey(), recipient.pubkey(), mint)
        .instruction()
        .unwrap();
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();
    let recipient_ctoken = derive_token_ata(&recipient.pubkey(), &mint).0;

    // Fund sender Light Token via SPL
    let temp_spl_keypair = Keypair::new();
    create_token_2022_account(&mut rpc, &mint, &temp_spl_keypair, &sender, false)
        .await
        .unwrap();
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &temp_spl_keypair.pubkey(),
        &payer.pubkey(),
        &payer,
        amount,
        false,
    )
    .await
    .unwrap();

    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint, 0, false);
    let compressed_token_program_id =
        Pubkey::new_from_array(light_token_interface::LIGHT_TOKEN_PROGRAM_ID);
    let cpi_authority_pda = Pubkey::new_from_array(CPI_AUTHORITY_PDA);

    // Fund sender Light Token
    {
        let data = TransferInterfaceData {
            amount,
            spl_interface_pda_bump: Some(spl_interface_pda_bump),
            decimals: CREATE_MINT_HELPER_DECIMALS,
        };
        let wrapper_instruction_data = [vec![19u8], data.try_to_vec().unwrap()].concat();
        let wrapper_accounts = vec![
            AccountMeta::new_readonly(compressed_token_program_id, false),
            AccountMeta::new(temp_spl_keypair.pubkey(), false),
            AccountMeta::new(sender_ctoken, false),
            AccountMeta::new_readonly(sender.pubkey(), true),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(cpi_authority_pda, false),
            AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(spl_interface_pda, false),
            AccountMeta::new_readonly(anchor_spl::token::ID, false),
        ];
        let instruction = Instruction {
            program_id: ID,
            accounts: wrapper_accounts,
            data: wrapper_instruction_data,
        };
        rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &sender])
            .await
            .unwrap();
    }

    // Now test Light Token -> Light Token transfer (no SPL bridge needed)
    let data = TransferInterfaceData {
        amount: transfer_amount,
        spl_interface_pda_bump: None, // Not needed for Light Token->Light Token
        decimals: CREATE_MINT_HELPER_DECIMALS,
    };
    let wrapper_instruction_data = [vec![19u8], data.try_to_vec().unwrap()].concat();

    // For Light Token->Light Token, we need 7 accounts (no SPL bridge, but system_program is required)
    let wrapper_accounts = vec![
        AccountMeta::new_readonly(compressed_token_program_id, false),
        AccountMeta::new(sender_ctoken, false), // source (Light Token)
        AccountMeta::new(recipient_ctoken, false), // destination (Light Token)
        AccountMeta::new_readonly(sender.pubkey(), true), // authority
        AccountMeta::new(payer.pubkey(), true), // payer
        AccountMeta::new_readonly(cpi_authority_pda, false),
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false), // system_program
    ];

    let instruction = Instruction {
        program_id: ID,
        accounts: wrapper_accounts,
        data: wrapper_instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &sender])
        .await
        .unwrap();

    // Verify balances
    use spl_token_2022::pod::PodAccount;
    let sender_ctoken_data = rpc.get_account(sender_ctoken).await.unwrap().unwrap();
    let sender_state =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&sender_ctoken_data.data[..165]).unwrap();
    assert_eq!(u64::from(sender_state.amount), amount - transfer_amount);

    let recipient_ctoken_data = rpc.get_account(recipient_ctoken).await.unwrap().unwrap();
    let recipient_state =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&recipient_ctoken_data.data[..165])
            .unwrap();
    assert_eq!(u64::from(recipient_state.amount), transfer_amount);

    println!("TransferInterface Light Token->Light Token invoke test passed");
}

// =============================================================================
// INVOKE_SIGNED TESTS (PDA authority)
// =============================================================================

/// Test TransferInterface: SPL -> Light Token with PDA authority (invoke_signed)
#[tokio::test]
async fn test_transfer_interface_spl_to_ctoken_invoke_signed() {
    use anchor_spl::associated_token::{
        get_associated_token_address, spl_associated_token_account,
    };

    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();

    // Derive PDA authority
    let (authority_pda, _) =
        Pubkey::find_program_address(&[TRANSFER_INTERFACE_AUTHORITY_SEED], &ID);

    let mint = create_mint_helper(&mut rpc, &payer).await;
    let amount = 10000u64;
    let transfer_amount = 5000u64;

    // Create SPL ATA owned by PDA
    let spl_ata = get_associated_token_address(&authority_pda, &mint);
    let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
        &payer.pubkey(),
        &authority_pda,
        &mint,
        &anchor_spl::token::ID,
    );
    rpc.create_and_send_transaction(&[create_ata_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Mint tokens to PDA's ATA
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &spl_ata,
        &payer.pubkey(),
        &payer,
        amount,
        false,
    )
    .await
    .unwrap();

    // Create destination Light Token ATA
    let recipient = Keypair::new();
    light_test_utils::airdrop_lamports(&mut rpc, &recipient.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    let instruction = CreateAssociatedTokenAccount::new(payer.pubkey(), recipient.pubkey(), mint)
        .instruction()
        .unwrap();
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();
    let ctoken_account = derive_token_ata(&recipient.pubkey(), &mint).0;

    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint, 0, false);
    let compressed_token_program_id =
        Pubkey::new_from_array(light_token_interface::LIGHT_TOKEN_PROGRAM_ID);
    let cpi_authority_pda = Pubkey::new_from_array(CPI_AUTHORITY_PDA);

    let data = TransferInterfaceData {
        amount: transfer_amount,
        spl_interface_pda_bump: Some(spl_interface_pda_bump),
        decimals: CREATE_MINT_HELPER_DECIMALS,
    };
    // Discriminator 20 = TransferInterfaceInvokeSigned
    let wrapper_instruction_data = [vec![20u8], data.try_to_vec().unwrap()].concat();

    let wrapper_accounts = vec![
        AccountMeta::new_readonly(compressed_token_program_id, false),
        AccountMeta::new(spl_ata, false), // source (SPL owned by PDA)
        AccountMeta::new(ctoken_account, false), // destination (Light Token)
        AccountMeta::new_readonly(authority_pda, false), // authority (PDA, not signer)
        AccountMeta::new(payer.pubkey(), true), // payer
        AccountMeta::new_readonly(cpi_authority_pda, false), // compressed_token_program_authority
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false), // system_program
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(spl_interface_pda, false),
        AccountMeta::new_readonly(anchor_spl::token::ID, false),
    ];

    let instruction = Instruction {
        program_id: ID,
        accounts: wrapper_accounts,
        data: wrapper_instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify balances
    use spl_token_2022::pod::PodAccount;
    let spl_account_data = rpc.get_account(spl_ata).await.unwrap().unwrap();
    let spl_account =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&spl_account_data.data).unwrap();
    assert_eq!(u64::from(spl_account.amount), amount - transfer_amount);

    let ctoken_account_data = rpc.get_account(ctoken_account).await.unwrap().unwrap();
    let ctoken_state =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&ctoken_account_data.data[..165]).unwrap();
    assert_eq!(u64::from(ctoken_state.amount), transfer_amount);

    println!("TransferInterface SPL->Light Token invoke_signed test passed");
}

/// Test TransferInterface: Light Token -> SPL with PDA authority (invoke_signed)
#[tokio::test]
async fn test_transfer_interface_ctoken_to_spl_invoke_signed() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();

    // Derive PDA authority
    let (authority_pda, _) =
        Pubkey::find_program_address(&[TRANSFER_INTERFACE_AUTHORITY_SEED], &ID);

    let mint = create_mint_helper(&mut rpc, &payer).await;
    let amount = 10000u64;
    let transfer_amount = 5000u64;

    // Create destination SPL token account
    let destination_owner = Keypair::new();
    light_test_utils::airdrop_lamports(&mut rpc, &destination_owner.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let spl_token_account_keypair = Keypair::new();
    create_token_2022_account(
        &mut rpc,
        &mint,
        &spl_token_account_keypair,
        &destination_owner,
        false,
    )
    .await
    .unwrap();

    // Create Light Token ATA owned by PDA
    let (ctoken_account, bump) = derive_token_ata(&authority_pda, &mint);
    let instruction = CreateAssociatedTokenAccount {
        idempotent: false,
        bump,
        payer: payer.pubkey(),
        owner: authority_pda,
        mint,
        associated_token_account: ctoken_account,
        compressible: CompressibleParams::default_ata(),
    }
    .instruction()
    .unwrap();
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Fund PDA's Light Token via temporary SPL account
    let temp_spl_keypair = Keypair::new();
    let temp_owner = Keypair::new();
    light_test_utils::airdrop_lamports(&mut rpc, &temp_owner.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    create_token_2022_account(&mut rpc, &mint, &temp_spl_keypair, &temp_owner, false)
        .await
        .unwrap();
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &temp_spl_keypair.pubkey(),
        &payer.pubkey(),
        &payer,
        amount,
        false,
    )
    .await
    .unwrap();

    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint, 0, false);
    let compressed_token_program_id =
        Pubkey::new_from_array(light_token_interface::LIGHT_TOKEN_PROGRAM_ID);
    let cpi_authority_pda = Pubkey::new_from_array(CPI_AUTHORITY_PDA);

    // Fund PDA's Light Token
    {
        let data = TransferInterfaceData {
            amount,
            spl_interface_pda_bump: Some(spl_interface_pda_bump),
            decimals: CREATE_MINT_HELPER_DECIMALS,
        };
        let wrapper_instruction_data = [vec![19u8], data.try_to_vec().unwrap()].concat();
        let wrapper_accounts = vec![
            AccountMeta::new_readonly(compressed_token_program_id, false),
            AccountMeta::new(temp_spl_keypair.pubkey(), false),
            AccountMeta::new(ctoken_account, false),
            AccountMeta::new_readonly(temp_owner.pubkey(), true),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(cpi_authority_pda, false),
            AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(spl_interface_pda, false),
            AccountMeta::new_readonly(anchor_spl::token::ID, false),
        ];
        let instruction = Instruction {
            program_id: ID,
            accounts: wrapper_accounts,
            data: wrapper_instruction_data,
        };
        rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &temp_owner])
            .await
            .unwrap();
    }

    // Now test Light Token -> SPL with PDA authority
    let data = TransferInterfaceData {
        amount: transfer_amount,
        spl_interface_pda_bump: Some(spl_interface_pda_bump),
        decimals: CREATE_MINT_HELPER_DECIMALS,
    };
    // Discriminator 20 = TransferInterfaceInvokeSigned
    let wrapper_instruction_data = [vec![20u8], data.try_to_vec().unwrap()].concat();

    let wrapper_accounts = vec![
        AccountMeta::new_readonly(compressed_token_program_id, false),
        AccountMeta::new(ctoken_account, false), // source (Light Token owned by PDA)
        AccountMeta::new(spl_token_account_keypair.pubkey(), false), // destination (SPL)
        AccountMeta::new_readonly(authority_pda, false), // authority (PDA)
        AccountMeta::new(payer.pubkey(), true),  // payer
        AccountMeta::new_readonly(cpi_authority_pda, false),
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(spl_interface_pda, false),
        AccountMeta::new_readonly(anchor_spl::token::ID, false),
    ];

    let instruction = Instruction {
        program_id: ID,
        accounts: wrapper_accounts,
        data: wrapper_instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify balances
    use spl_token_2022::pod::PodAccount;
    let spl_account_data = rpc
        .get_account(spl_token_account_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    let spl_account =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&spl_account_data.data).unwrap();
    assert_eq!(u64::from(spl_account.amount), transfer_amount);

    let ctoken_account_data = rpc.get_account(ctoken_account).await.unwrap().unwrap();
    let ctoken_state =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&ctoken_account_data.data[..165]).unwrap();
    assert_eq!(u64::from(ctoken_state.amount), amount - transfer_amount);

    println!("TransferInterface Light Token->SPL invoke_signed test passed");
}

/// Test TransferInterface: Light Token -> Light Token with PDA authority (invoke_signed)
#[tokio::test]
async fn test_transfer_interface_ctoken_to_ctoken_invoke_signed() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();

    // Derive PDA authority
    let (authority_pda, _) =
        Pubkey::find_program_address(&[TRANSFER_INTERFACE_AUTHORITY_SEED], &ID);

    let mint = create_mint_helper(&mut rpc, &payer).await;
    let amount = 10000u64;
    let transfer_amount = 5000u64;

    // Create source Light Token ATA owned by PDA
    let (source_ctoken, bump) = derive_token_ata(&authority_pda, &mint);
    let instruction = CreateAssociatedTokenAccount {
        idempotent: false,
        bump,
        payer: payer.pubkey(),
        owner: authority_pda,
        mint,
        associated_token_account: source_ctoken,
        compressible: CompressibleParams::default_ata(),
    }
    .instruction()
    .unwrap();
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Create destination Light Token ATA
    let recipient = Keypair::new();
    light_test_utils::airdrop_lamports(&mut rpc, &recipient.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let instruction = CreateAssociatedTokenAccount::new(payer.pubkey(), recipient.pubkey(), mint)
        .instruction()
        .unwrap();
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();
    let dest_ctoken = derive_token_ata(&recipient.pubkey(), &mint).0;

    // Fund source Light Token via temporary SPL account
    let temp_spl_keypair = Keypair::new();
    let temp_owner = Keypair::new();
    light_test_utils::airdrop_lamports(&mut rpc, &temp_owner.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    create_token_2022_account(&mut rpc, &mint, &temp_spl_keypair, &temp_owner, false)
        .await
        .unwrap();
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &temp_spl_keypair.pubkey(),
        &payer.pubkey(),
        &payer,
        amount,
        false,
    )
    .await
    .unwrap();

    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint, 0, false);
    let compressed_token_program_id =
        Pubkey::new_from_array(light_token_interface::LIGHT_TOKEN_PROGRAM_ID);
    let cpi_authority_pda = Pubkey::new_from_array(CPI_AUTHORITY_PDA);

    // Fund source Light Token
    {
        let data = TransferInterfaceData {
            amount,
            spl_interface_pda_bump: Some(spl_interface_pda_bump),
            decimals: CREATE_MINT_HELPER_DECIMALS,
        };
        let wrapper_instruction_data = [vec![19u8], data.try_to_vec().unwrap()].concat();
        let wrapper_accounts = vec![
            AccountMeta::new_readonly(compressed_token_program_id, false),
            AccountMeta::new(temp_spl_keypair.pubkey(), false),
            AccountMeta::new(source_ctoken, false),
            AccountMeta::new_readonly(temp_owner.pubkey(), true),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(cpi_authority_pda, false),
            AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(spl_interface_pda, false),
            AccountMeta::new_readonly(anchor_spl::token::ID, false),
        ];
        let instruction = Instruction {
            program_id: ID,
            accounts: wrapper_accounts,
            data: wrapper_instruction_data,
        };
        rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &temp_owner])
            .await
            .unwrap();
    }

    // Now test Light Token -> Light Token with PDA authority
    let data = TransferInterfaceData {
        amount: transfer_amount,
        spl_interface_pda_bump: None, // Not needed for Light Token->Light Token
        decimals: CREATE_MINT_HELPER_DECIMALS,
    };
    // Discriminator 20 = TransferInterfaceInvokeSigned
    let wrapper_instruction_data = [vec![20u8], data.try_to_vec().unwrap()].concat();

    // For Light Token->Light Token, we only need 6 accounts (no SPL bridge)
    let wrapper_accounts = vec![
        AccountMeta::new_readonly(compressed_token_program_id, false),
        AccountMeta::new(source_ctoken, false), // source (Light Token owned by PDA)
        AccountMeta::new(dest_ctoken, false),   // destination (Light Token)
        AccountMeta::new_readonly(authority_pda, false), // authority (PDA)
        AccountMeta::new(payer.pubkey(), true), // payer
        AccountMeta::new_readonly(cpi_authority_pda, false),
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    let instruction = Instruction {
        program_id: ID,
        accounts: wrapper_accounts,
        data: wrapper_instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify balances
    use spl_token_2022::pod::PodAccount;
    let source_ctoken_data = rpc.get_account(source_ctoken).await.unwrap().unwrap();
    let source_state =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&source_ctoken_data.data[..165]).unwrap();
    assert_eq!(u64::from(source_state.amount), amount - transfer_amount);

    let dest_ctoken_data = rpc.get_account(dest_ctoken).await.unwrap().unwrap();
    let dest_state =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&dest_ctoken_data.data[..165]).unwrap();
    assert_eq!(u64::from(dest_state.amount), transfer_amount);

    println!("TransferInterface Light Token->Light Token invoke_signed test passed");
}

// =============================================================================
// SPL-TO-SPL TESTS
// =============================================================================

/// Test TransferInterface: SPL -> SPL (invoke)
#[tokio::test]
async fn test_transfer_interface_spl_to_spl_invoke() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let sender = Keypair::new();
    light_test_utils::airdrop_lamports(&mut rpc, &sender.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    // Create SPL mint and token accounts
    let mint = create_mint_helper(&mut rpc, &payer).await;
    let amount = 10000u64;
    let transfer_amount = 5000u64;

    // Create source SPL token account
    let source_spl_keypair = Keypair::new();
    create_token_2022_account(&mut rpc, &mint, &source_spl_keypair, &sender, false)
        .await
        .unwrap();
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &source_spl_keypair.pubkey(),
        &payer.pubkey(),
        &payer,
        amount,
        false,
    )
    .await
    .unwrap();

    // Create destination SPL token account
    let recipient = Keypair::new();
    light_test_utils::airdrop_lamports(&mut rpc, &recipient.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let dest_spl_keypair = Keypair::new();
    create_token_2022_account(&mut rpc, &mint, &dest_spl_keypair, &recipient, false)
        .await
        .unwrap();

    // Get SPL interface PDA (not actually used for SPL->SPL, but needed by wrapper)
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint, 0, false);
    let compressed_token_program_id =
        Pubkey::new_from_array(light_token_interface::LIGHT_TOKEN_PROGRAM_ID);
    let cpi_authority_pda = Pubkey::new_from_array(CPI_AUTHORITY_PDA);

    // Build wrapper instruction for SPL->SPL transfer
    let data = TransferInterfaceData {
        amount: transfer_amount,
        spl_interface_pda_bump: Some(spl_interface_pda_bump),
        decimals: CREATE_MINT_HELPER_DECIMALS,
    };
    // Discriminator 19 = TransferInterfaceInvoke
    let wrapper_instruction_data = [vec![19u8], data.try_to_vec().unwrap()].concat();

    let wrapper_accounts = vec![
        AccountMeta::new_readonly(compressed_token_program_id, false),
        AccountMeta::new(source_spl_keypair.pubkey(), false), // source (SPL)
        AccountMeta::new(dest_spl_keypair.pubkey(), false),   // destination (SPL)
        AccountMeta::new_readonly(sender.pubkey(), true),     // authority (signer)
        AccountMeta::new(payer.pubkey(), true),               // payer
        AccountMeta::new_readonly(cpi_authority_pda, false),  // compressed_token_program_authority
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false), // system_program
        AccountMeta::new_readonly(mint, false),               // mint (for SPL transfer_checked)
        AccountMeta::new(spl_interface_pda, false), // spl_interface_pda (passed but not used)
        AccountMeta::new_readonly(anchor_spl::token::ID, false), // spl_token_program
    ];

    let instruction = Instruction {
        program_id: ID,
        accounts: wrapper_accounts,
        data: wrapper_instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &sender])
        .await
        .unwrap();

    // Verify balances
    use spl_token_2022::pod::PodAccount;
    let source_account_data = rpc
        .get_account(source_spl_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    let source_account =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&source_account_data.data).unwrap();
    assert_eq!(u64::from(source_account.amount), amount - transfer_amount);

    let dest_account_data = rpc
        .get_account(dest_spl_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    let dest_account =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&dest_account_data.data).unwrap();
    assert_eq!(u64::from(dest_account.amount), transfer_amount);

    println!("TransferInterface SPL->SPL invoke test passed");
}

/// Test TransferInterface: SPL -> SPL with PDA authority (invoke_signed)
#[tokio::test]
async fn test_transfer_interface_spl_to_spl_invoke_signed() {
    use anchor_spl::associated_token::{
        get_associated_token_address, spl_associated_token_account,
    };

    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();

    // Derive PDA authority
    let (authority_pda, _) =
        Pubkey::find_program_address(&[TRANSFER_INTERFACE_AUTHORITY_SEED], &ID);

    let mint = create_mint_helper(&mut rpc, &payer).await;
    let amount = 10000u64;
    let transfer_amount = 5000u64;

    // Create SPL ATA owned by PDA
    let source_spl_ata = get_associated_token_address(&authority_pda, &mint);
    let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
        &payer.pubkey(),
        &authority_pda,
        &mint,
        &anchor_spl::token::ID,
    );
    rpc.create_and_send_transaction(&[create_ata_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Mint tokens to PDA's ATA
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &source_spl_ata,
        &payer.pubkey(),
        &payer,
        amount,
        false,
    )
    .await
    .unwrap();

    // Create destination SPL token account
    let recipient = Keypair::new();
    light_test_utils::airdrop_lamports(&mut rpc, &recipient.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let dest_spl_keypair = Keypair::new();
    create_token_2022_account(&mut rpc, &mint, &dest_spl_keypair, &recipient, false)
        .await
        .unwrap();

    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint, 0, false);
    let compressed_token_program_id =
        Pubkey::new_from_array(light_token_interface::LIGHT_TOKEN_PROGRAM_ID);
    let cpi_authority_pda = Pubkey::new_from_array(CPI_AUTHORITY_PDA);

    let data = TransferInterfaceData {
        amount: transfer_amount,
        spl_interface_pda_bump: Some(spl_interface_pda_bump),
        decimals: CREATE_MINT_HELPER_DECIMALS,
    };
    // Discriminator 20 = TransferInterfaceInvokeSigned
    let wrapper_instruction_data = [vec![20u8], data.try_to_vec().unwrap()].concat();

    let wrapper_accounts = vec![
        AccountMeta::new_readonly(compressed_token_program_id, false),
        AccountMeta::new(source_spl_ata, false), // source (SPL owned by PDA)
        AccountMeta::new(dest_spl_keypair.pubkey(), false), // destination (SPL)
        AccountMeta::new_readonly(authority_pda, false), // authority (PDA, not signer)
        AccountMeta::new(payer.pubkey(), true),  // payer
        AccountMeta::new_readonly(cpi_authority_pda, false), // compressed_token_program_authority
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false), // system_program
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(spl_interface_pda, false),
        AccountMeta::new_readonly(anchor_spl::token::ID, false),
    ];

    let instruction = Instruction {
        program_id: ID,
        accounts: wrapper_accounts,
        data: wrapper_instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify balances
    use spl_token_2022::pod::PodAccount;
    let source_account_data = rpc.get_account(source_spl_ata).await.unwrap().unwrap();
    let source_account =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&source_account_data.data).unwrap();
    assert_eq!(u64::from(source_account.amount), amount - transfer_amount);

    let dest_account_data = rpc
        .get_account(dest_spl_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    let dest_account =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&dest_account_data.data).unwrap();
    assert_eq!(u64::from(dest_account.amount), transfer_amount);

    println!("TransferInterface SPL->SPL invoke_signed test passed");
}

// =============================================================================
// TOKEN-2022 TO TOKEN-2022 TESTS
// =============================================================================

/// Test TransferInterface: T22 -> T22 (invoke)
#[tokio::test]
async fn test_transfer_interface_t22_to_t22_invoke() {
    use light_test_utils::spl::create_mint_22_helper;

    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let sender = Keypair::new();
    light_test_utils::airdrop_lamports(&mut rpc, &sender.pubkey(), 1_000_000_000)
        .await
        .unwrap();

    // Create T22 mint and token accounts
    let mint = create_mint_22_helper(&mut rpc, &payer).await;
    let amount = 10000u64;
    let transfer_amount = 5000u64;

    // Create source T22 token account
    let source_t22_keypair = Keypair::new();
    create_token_2022_account(&mut rpc, &mint, &source_t22_keypair, &sender, true)
        .await
        .unwrap();
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &source_t22_keypair.pubkey(),
        &payer.pubkey(),
        &payer,
        amount,
        true,
    )
    .await
    .unwrap();

    // Create destination T22 token account
    let recipient = Keypair::new();
    light_test_utils::airdrop_lamports(&mut rpc, &recipient.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let dest_t22_keypair = Keypair::new();
    create_token_2022_account(&mut rpc, &mint, &dest_t22_keypair, &recipient, true)
        .await
        .unwrap();

    // Get SPL interface PDA for T22
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint, 0, true);
    let compressed_token_program_id =
        Pubkey::new_from_array(light_token_interface::LIGHT_TOKEN_PROGRAM_ID);
    let cpi_authority_pda = Pubkey::new_from_array(CPI_AUTHORITY_PDA);

    // Build wrapper instruction for T22->T22 transfer
    let data = TransferInterfaceData {
        amount: transfer_amount,
        spl_interface_pda_bump: Some(spl_interface_pda_bump),
        decimals: CREATE_MINT_HELPER_DECIMALS,
    };
    // Discriminator 19 = TransferInterfaceInvoke
    let wrapper_instruction_data = [vec![19u8], data.try_to_vec().unwrap()].concat();

    let wrapper_accounts = vec![
        AccountMeta::new_readonly(compressed_token_program_id, false),
        AccountMeta::new(source_t22_keypair.pubkey(), false), // source (T22)
        AccountMeta::new(dest_t22_keypair.pubkey(), false),   // destination (T22)
        AccountMeta::new_readonly(sender.pubkey(), true),     // authority (signer)
        AccountMeta::new(payer.pubkey(), true),               // payer
        AccountMeta::new_readonly(cpi_authority_pda, false),  // compressed_token_program_authority
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false), // system_program
        AccountMeta::new_readonly(mint, false),               // mint (for T22 transfer_checked)
        AccountMeta::new(spl_interface_pda, false), // spl_interface_pda (passed but not used)
        AccountMeta::new_readonly(anchor_spl::token_2022::ID, false), // T22 token program
    ];

    let instruction = Instruction {
        program_id: ID,
        accounts: wrapper_accounts,
        data: wrapper_instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &sender])
        .await
        .unwrap();

    // Verify balances using T22 state unpacking (handles extensions)
    use spl_token_2022::{extension::StateWithExtensions, state::Account as T22Account};

    let source_account_data = rpc
        .get_account(source_t22_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    let source_state =
        StateWithExtensions::<T22Account>::unpack(&source_account_data.data).unwrap();
    assert_eq!(source_state.base.amount, amount - transfer_amount);

    let dest_account_data = rpc
        .get_account(dest_t22_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    let dest_state = StateWithExtensions::<T22Account>::unpack(&dest_account_data.data).unwrap();
    assert_eq!(dest_state.base.amount, transfer_amount);

    println!("TransferInterface T22->T22 invoke test passed");
}

/// Test TransferInterface: T22 -> T22 with PDA authority (invoke_signed)
#[tokio::test]
async fn test_transfer_interface_t22_to_t22_invoke_signed() {
    use anchor_spl::associated_token::{
        get_associated_token_address_with_program_id, spl_associated_token_account,
    };
    use light_test_utils::spl::create_mint_22_helper;

    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();

    // Derive PDA authority
    let (authority_pda, _) =
        Pubkey::find_program_address(&[TRANSFER_INTERFACE_AUTHORITY_SEED], &ID);

    let mint = create_mint_22_helper(&mut rpc, &payer).await;
    let amount = 10000u64;
    let transfer_amount = 5000u64;

    // Create T22 ATA owned by PDA
    let source_t22_ata = get_associated_token_address_with_program_id(
        &authority_pda,
        &mint,
        &anchor_spl::token_2022::ID,
    );
    let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
        &payer.pubkey(),
        &authority_pda,
        &mint,
        &anchor_spl::token_2022::ID,
    );
    rpc.create_and_send_transaction(&[create_ata_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Mint tokens to PDA's ATA
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &source_t22_ata,
        &payer.pubkey(),
        &payer,
        amount,
        true,
    )
    .await
    .unwrap();

    // Create destination T22 token account
    let recipient = Keypair::new();
    light_test_utils::airdrop_lamports(&mut rpc, &recipient.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    let dest_t22_keypair = Keypair::new();
    create_token_2022_account(&mut rpc, &mint, &dest_t22_keypair, &recipient, true)
        .await
        .unwrap();

    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint, 0, true);
    let compressed_token_program_id =
        Pubkey::new_from_array(light_token_interface::LIGHT_TOKEN_PROGRAM_ID);
    let cpi_authority_pda = Pubkey::new_from_array(CPI_AUTHORITY_PDA);

    let data = TransferInterfaceData {
        amount: transfer_amount,
        spl_interface_pda_bump: Some(spl_interface_pda_bump),
        decimals: CREATE_MINT_HELPER_DECIMALS,
    };
    // Discriminator 20 = TransferInterfaceInvokeSigned
    let wrapper_instruction_data = [vec![20u8], data.try_to_vec().unwrap()].concat();

    let wrapper_accounts = vec![
        AccountMeta::new_readonly(compressed_token_program_id, false),
        AccountMeta::new(source_t22_ata, false), // source (T22 owned by PDA)
        AccountMeta::new(dest_t22_keypair.pubkey(), false), // destination (T22)
        AccountMeta::new_readonly(authority_pda, false), // authority (PDA, not signer)
        AccountMeta::new(payer.pubkey(), true),  // payer
        AccountMeta::new_readonly(cpi_authority_pda, false), // compressed_token_program_authority
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false), // system_program
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(spl_interface_pda, false),
        AccountMeta::new_readonly(anchor_spl::token_2022::ID, false), // T22 token program
    ];

    let instruction = Instruction {
        program_id: ID,
        accounts: wrapper_accounts,
        data: wrapper_instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify balances using T22 state unpacking (handles extensions)
    use spl_token_2022::{extension::StateWithExtensions, state::Account as T22Account};

    let source_account_data = rpc.get_account(source_t22_ata).await.unwrap().unwrap();
    let source_state =
        StateWithExtensions::<T22Account>::unpack(&source_account_data.data).unwrap();
    assert_eq!(source_state.base.amount, amount - transfer_amount);

    let dest_account_data = rpc
        .get_account(dest_t22_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    let dest_state = StateWithExtensions::<T22Account>::unpack(&dest_account_data.data).unwrap();
    assert_eq!(dest_state.base.amount, transfer_amount);

    println!("TransferInterface T22->T22 invoke_signed test passed");
}
