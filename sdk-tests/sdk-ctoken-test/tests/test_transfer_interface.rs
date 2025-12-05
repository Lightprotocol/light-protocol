// Tests for TransferInterface - unified transfer interface that auto-detects account types

mod shared;

use borsh::BorshSerialize;
use light_client::rpc::Rpc;
use light_compressed_token_sdk::{
    ctoken::{derive_ctoken_ata, CreateAssociatedTokenAccount},
    token_pool::find_token_pool_pda_with_index,
};
use light_compressed_token_types::CPI_AUTHORITY_PDA;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_test_utils::spl::{create_mint_helper, create_token_2022_account, mint_spl_tokens};
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

/// Test TransferInterface: SPL -> CToken (invoke)
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

    // Create CToken ATA for recipient
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
    let ctoken_account = derive_ctoken_ata(&recipient.pubkey(), &mint).0;

    // Get token pool PDA
    let (token_pool_pda, token_pool_pda_bump) = find_token_pool_pda_with_index(&mint, 0);
    let compressed_token_program_id =
        Pubkey::new_from_array(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID);
    let cpi_authority_pda = Pubkey::new_from_array(CPI_AUTHORITY_PDA);

    // Build wrapper instruction
    let data = TransferInterfaceData {
        amount: transfer_amount,
        token_pool_pda_bump: Some(token_pool_pda_bump),
    };
    // Discriminator 19 = TransferInterfaceInvoke
    let wrapper_instruction_data = [vec![19u8], data.try_to_vec().unwrap()].concat();

    let wrapper_accounts = vec![
        AccountMeta::new_readonly(compressed_token_program_id, false),
        AccountMeta::new(spl_token_account_keypair.pubkey(), false), // source (SPL)
        AccountMeta::new(ctoken_account, false),                     // destination (CToken)
        AccountMeta::new_readonly(sender.pubkey(), true),            // authority (signer)
        AccountMeta::new(payer.pubkey(), true),                      // payer
        AccountMeta::new_readonly(cpi_authority_pda, false), // compressed_token_program_authority
        AccountMeta::new_readonly(mint, false),              // mint (for SPL bridge)
        AccountMeta::new(token_pool_pda, false),             // token_pool_pda
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

    println!("TransferInterface SPL->CToken invoke test passed");
}

/// Test TransferInterface: CToken -> SPL (invoke)
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

    // Create and fund CToken ATA
    let instruction = CreateAssociatedTokenAccount::new(payer.pubkey(), owner.pubkey(), mint)
        .instruction()
        .unwrap();
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();
    let ctoken_account = derive_ctoken_ata(&owner.pubkey(), &mint).0;

    // Fund CToken via temporary SPL account
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

    let (token_pool_pda, token_pool_pda_bump) = find_token_pool_pda_with_index(&mint, 0);
    let compressed_token_program_id =
        Pubkey::new_from_array(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID);
    let cpi_authority_pda = Pubkey::new_from_array(CPI_AUTHORITY_PDA);

    // Transfer SPL to CToken to fund it
    {
        let data = TransferInterfaceData {
            amount,
            token_pool_pda_bump: Some(token_pool_pda_bump),
        };
        let wrapper_instruction_data = [vec![19u8], data.try_to_vec().unwrap()].concat();
        let wrapper_accounts = vec![
            AccountMeta::new_readonly(compressed_token_program_id, false),
            AccountMeta::new(temp_spl_keypair.pubkey(), false),
            AccountMeta::new(ctoken_account, false),
            AccountMeta::new_readonly(owner.pubkey(), true),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(cpi_authority_pda, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(token_pool_pda, false),
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

    // Now test CToken -> SPL transfer
    let data = TransferInterfaceData {
        amount: transfer_amount,
        token_pool_pda_bump: Some(token_pool_pda_bump),
    };
    let wrapper_instruction_data = [vec![19u8], data.try_to_vec().unwrap()].concat();

    let wrapper_accounts = vec![
        AccountMeta::new_readonly(compressed_token_program_id, false),
        AccountMeta::new(ctoken_account, false), // source (CToken)
        AccountMeta::new(spl_token_account_keypair.pubkey(), false), // destination (SPL)
        AccountMeta::new_readonly(owner.pubkey(), true), // authority
        AccountMeta::new(payer.pubkey(), true),  // payer
        AccountMeta::new_readonly(cpi_authority_pda, false),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(token_pool_pda, false),
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

    println!("TransferInterface CToken->SPL invoke test passed");
}

/// Test TransferInterface: CToken -> CToken (invoke)
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

    // Create sender CToken ATA
    let instruction = CreateAssociatedTokenAccount::new(payer.pubkey(), sender.pubkey(), mint)
        .instruction()
        .unwrap();
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();
    let sender_ctoken = derive_ctoken_ata(&sender.pubkey(), &mint).0;

    // Create recipient CToken ATA
    let instruction = CreateAssociatedTokenAccount::new(payer.pubkey(), recipient.pubkey(), mint)
        .instruction()
        .unwrap();
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();
    let recipient_ctoken = derive_ctoken_ata(&recipient.pubkey(), &mint).0;

    // Fund sender CToken via SPL
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

    let (token_pool_pda, token_pool_pda_bump) = find_token_pool_pda_with_index(&mint, 0);
    let compressed_token_program_id =
        Pubkey::new_from_array(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID);
    let cpi_authority_pda = Pubkey::new_from_array(CPI_AUTHORITY_PDA);

    // Fund sender CToken
    {
        let data = TransferInterfaceData {
            amount,
            token_pool_pda_bump: Some(token_pool_pda_bump),
        };
        let wrapper_instruction_data = [vec![19u8], data.try_to_vec().unwrap()].concat();
        let wrapper_accounts = vec![
            AccountMeta::new_readonly(compressed_token_program_id, false),
            AccountMeta::new(temp_spl_keypair.pubkey(), false),
            AccountMeta::new(sender_ctoken, false),
            AccountMeta::new_readonly(sender.pubkey(), true),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(cpi_authority_pda, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(token_pool_pda, false),
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

    // Now test CToken -> CToken transfer (no SPL bridge needed)
    let data = TransferInterfaceData {
        amount: transfer_amount,
        token_pool_pda_bump: None, // Not needed for CToken->CToken
    };
    let wrapper_instruction_data = [vec![19u8], data.try_to_vec().unwrap()].concat();

    // For CToken->CToken, we only need 6 accounts (no SPL bridge)
    let wrapper_accounts = vec![
        AccountMeta::new_readonly(compressed_token_program_id, false),
        AccountMeta::new(sender_ctoken, false), // source (CToken)
        AccountMeta::new(recipient_ctoken, false), // destination (CToken)
        AccountMeta::new_readonly(sender.pubkey(), true), // authority
        AccountMeta::new(payer.pubkey(), true), // payer
        AccountMeta::new_readonly(cpi_authority_pda, false),
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

    println!("TransferInterface CToken->CToken invoke test passed");
}

// =============================================================================
// INVOKE_SIGNED TESTS (PDA authority)
// =============================================================================

/// Test TransferInterface: SPL -> CToken with PDA authority (invoke_signed)
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

    // Create destination CToken ATA
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
    let ctoken_account = derive_ctoken_ata(&recipient.pubkey(), &mint).0;

    let (token_pool_pda, token_pool_pda_bump) = find_token_pool_pda_with_index(&mint, 0);
    let compressed_token_program_id =
        Pubkey::new_from_array(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID);
    let cpi_authority_pda = Pubkey::new_from_array(CPI_AUTHORITY_PDA);

    let data = TransferInterfaceData {
        amount: transfer_amount,
        token_pool_pda_bump: Some(token_pool_pda_bump),
    };
    // Discriminator 20 = TransferInterfaceInvokeSigned
    let wrapper_instruction_data = [vec![20u8], data.try_to_vec().unwrap()].concat();

    let wrapper_accounts = vec![
        AccountMeta::new_readonly(compressed_token_program_id, false),
        AccountMeta::new(spl_ata, false), // source (SPL owned by PDA)
        AccountMeta::new(ctoken_account, false), // destination (CToken)
        AccountMeta::new_readonly(authority_pda, false), // authority (PDA, not signer)
        AccountMeta::new(payer.pubkey(), true), // payer
        AccountMeta::new_readonly(cpi_authority_pda, false), // compressed_token_program_authority
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(token_pool_pda, false),
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

    println!("TransferInterface SPL->CToken invoke_signed test passed");
}

/// Test TransferInterface: CToken -> SPL with PDA authority (invoke_signed)
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

    // Create CToken ATA owned by PDA
    let (ctoken_account, bump) = derive_ctoken_ata(&authority_pda, &mint);
    let instruction = CreateAssociatedTokenAccount {
        idempotent: false,
        bump,
        payer: payer.pubkey(),
        owner: authority_pda,
        mint,
        associated_token_account: ctoken_account,
        compressible: None,
    }
    .instruction()
    .unwrap();
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Fund PDA's CToken via temporary SPL account
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

    let (token_pool_pda, token_pool_pda_bump) = find_token_pool_pda_with_index(&mint, 0);
    let compressed_token_program_id =
        Pubkey::new_from_array(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID);
    let cpi_authority_pda = Pubkey::new_from_array(CPI_AUTHORITY_PDA);

    // Fund PDA's CToken
    {
        let data = TransferInterfaceData {
            amount,
            token_pool_pda_bump: Some(token_pool_pda_bump),
        };
        let wrapper_instruction_data = [vec![19u8], data.try_to_vec().unwrap()].concat();
        let wrapper_accounts = vec![
            AccountMeta::new_readonly(compressed_token_program_id, false),
            AccountMeta::new(temp_spl_keypair.pubkey(), false),
            AccountMeta::new(ctoken_account, false),
            AccountMeta::new_readonly(temp_owner.pubkey(), true),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(cpi_authority_pda, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(token_pool_pda, false),
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

    // Now test CToken -> SPL with PDA authority
    let data = TransferInterfaceData {
        amount: transfer_amount,
        token_pool_pda_bump: Some(token_pool_pda_bump),
    };
    // Discriminator 20 = TransferInterfaceInvokeSigned
    let wrapper_instruction_data = [vec![20u8], data.try_to_vec().unwrap()].concat();

    let wrapper_accounts = vec![
        AccountMeta::new_readonly(compressed_token_program_id, false),
        AccountMeta::new(ctoken_account, false), // source (CToken owned by PDA)
        AccountMeta::new(spl_token_account_keypair.pubkey(), false), // destination (SPL)
        AccountMeta::new_readonly(authority_pda, false), // authority (PDA)
        AccountMeta::new(payer.pubkey(), true),  // payer
        AccountMeta::new_readonly(cpi_authority_pda, false),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(token_pool_pda, false),
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

    println!("TransferInterface CToken->SPL invoke_signed test passed");
}

/// Test TransferInterface: CToken -> CToken with PDA authority (invoke_signed)
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

    // Create source CToken ATA owned by PDA
    let (source_ctoken, bump) = derive_ctoken_ata(&authority_pda, &mint);
    let instruction = CreateAssociatedTokenAccount {
        idempotent: false,
        bump,
        payer: payer.pubkey(),
        owner: authority_pda,
        mint,
        associated_token_account: source_ctoken,
        compressible: None,
    }
    .instruction()
    .unwrap();
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Create destination CToken ATA
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
    let dest_ctoken = derive_ctoken_ata(&recipient.pubkey(), &mint).0;

    // Fund source CToken via temporary SPL account
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

    let (token_pool_pda, token_pool_pda_bump) = find_token_pool_pda_with_index(&mint, 0);
    let compressed_token_program_id =
        Pubkey::new_from_array(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID);
    let cpi_authority_pda = Pubkey::new_from_array(CPI_AUTHORITY_PDA);

    // Fund source CToken
    {
        let data = TransferInterfaceData {
            amount,
            token_pool_pda_bump: Some(token_pool_pda_bump),
        };
        let wrapper_instruction_data = [vec![19u8], data.try_to_vec().unwrap()].concat();
        let wrapper_accounts = vec![
            AccountMeta::new_readonly(compressed_token_program_id, false),
            AccountMeta::new(temp_spl_keypair.pubkey(), false),
            AccountMeta::new(source_ctoken, false),
            AccountMeta::new_readonly(temp_owner.pubkey(), true),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(cpi_authority_pda, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(token_pool_pda, false),
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

    // Now test CToken -> CToken with PDA authority
    let data = TransferInterfaceData {
        amount: transfer_amount,
        token_pool_pda_bump: None, // Not needed for CToken->CToken
    };
    // Discriminator 20 = TransferInterfaceInvokeSigned
    let wrapper_instruction_data = [vec![20u8], data.try_to_vec().unwrap()].concat();

    // For CToken->CToken, we only need 6 accounts (no SPL bridge)
    let wrapper_accounts = vec![
        AccountMeta::new_readonly(compressed_token_program_id, false),
        AccountMeta::new(source_ctoken, false), // source (CToken owned by PDA)
        AccountMeta::new(dest_ctoken, false),   // destination (CToken)
        AccountMeta::new_readonly(authority_pda, false), // authority (PDA)
        AccountMeta::new(payer.pubkey(), true), // payer
        AccountMeta::new_readonly(cpi_authority_pda, false),
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

    println!("TransferInterface CToken->CToken invoke_signed test passed");
}
