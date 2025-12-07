// Tests for TransferSplToCtokenCpi and TransferCTokenToSplCpi

mod shared;

use borsh::BorshSerialize;
use light_client::rpc::Rpc;
use light_ctoken_sdk::{
    ctoken::{derive_ctoken_ata, CreateAssociatedTokenAccount},
    spl_interface::find_spl_interface_pda_with_index,
};
use light_ctoken_types::CPI_AUTHORITY_PDA;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_test_utils::spl::{create_mint_helper, create_token_2022_account, mint_spl_tokens};
use native_ctoken_examples::{
    TransferCTokenToSplData, TransferSplToCtokenData, ID, TRANSFER_AUTHORITY_SEED,
};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};

/// Test transferring SPL tokens to CToken using TransferSplToCtokenCpi::invoke()
#[tokio::test]
async fn test_spl_to_ctoken_invoke() {
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

    // Create SPL mint
    let mint = create_mint_helper(&mut rpc, &payer).await;
    let amount = 10000u64;
    let transfer_amount = 5000u64;

    // Create SPL token account and mint tokens
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

    // Create compressed token ATA for recipient
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

    // Get initial balances
    use spl_token_2022::pod::PodAccount;
    let spl_account_data = rpc
        .get_account(spl_token_account_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    let spl_account =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&spl_account_data.data).unwrap();
    let initial_spl_balance: u64 = spl_account.amount.into();
    assert_eq!(initial_spl_balance, amount);

    // Get token pool PDA
    let (spl_interface_pda, spl_interface_pda_bump) = find_spl_interface_pda_with_index(&mint, 0);
    let compressed_token_program_id =
        Pubkey::new_from_array(light_ctoken_interface::CTOKEN_PROGRAM_ID);
    let cpi_authority_pda = Pubkey::new_from_array(CPI_AUTHORITY_PDA);

    // Build wrapper instruction for SPL to CToken transfer
    let data = TransferSplToCtokenData {
        amount: transfer_amount,
        spl_interface_pda_bump,
    };
    // Discriminator 15 = SplToCtokenInvoke
    let wrapper_instruction_data = [vec![15u8], data.try_to_vec().unwrap()].concat();

    // Account order from handler:
    // - accounts[0]: compressed_token_program (for CPI)
    // - accounts[1]: source_spl_token_account
    // - accounts[2]: destination_ctoken_account (writable)
    // - accounts[3]: authority (signer)
    // - accounts[4]: mint
    // - accounts[5]: payer (signer)
    // - accounts[6]: spl_interface_pda
    // - accounts[7]: spl_token_program
    // - accounts[8]: compressed_token_program_authority
    let wrapper_accounts = vec![
        AccountMeta::new_readonly(compressed_token_program_id, false),
        AccountMeta::new(spl_token_account_keypair.pubkey(), false),
        AccountMeta::new(ctoken_account, false), // destination_ctoken_account (writable)
        AccountMeta::new_readonly(sender.pubkey(), true), // authority (signer)
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(payer.pubkey(), true), // payer (signer)
        AccountMeta::new(spl_interface_pda, false),
        AccountMeta::new_readonly(anchor_spl::token::ID, false),
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

    // Verify SPL token balance decreased
    let spl_account_data = rpc
        .get_account(spl_token_account_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    let spl_account =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&spl_account_data.data).unwrap();
    let final_spl_balance: u64 = spl_account.amount.into();
    assert_eq!(final_spl_balance, amount - transfer_amount);

    // Verify CToken balance increased
    let ctoken_account_data = rpc.get_account(ctoken_account).await.unwrap().unwrap();
    let ctoken_account_state =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&ctoken_account_data.data[..165]).unwrap();
    assert_eq!(
        u64::from(ctoken_account_state.amount),
        transfer_amount,
        "CToken account should have received tokens"
    );

    println!("SPL to CToken invoke test passed");
}

/// Test transferring CToken to SPL tokens using TransferCTokenToSplCpi::invoke()
#[tokio::test]
async fn test_ctoken_to_spl_invoke() {
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

    // Create SPL mint
    let mint = create_mint_helper(&mut rpc, &payer).await;
    let amount = 10000u64;
    let transfer_amount = 5000u64;

    // Create SPL token account for receiving back tokens
    let spl_token_account_keypair = Keypair::new();
    create_token_2022_account(&mut rpc, &mint, &spl_token_account_keypair, &owner, false)
        .await
        .unwrap();

    // Create ctoken ATA and fund it via SPL transfer first
    let instruction = CreateAssociatedTokenAccount::new(payer.pubkey(), owner.pubkey(), mint)
        .instruction()
        .unwrap();
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();
    let ctoken_account = derive_ctoken_ata(&owner.pubkey(), &mint).0;

    // Create a temporary SPL account to mint tokens then transfer to ctoken
    let temp_spl_account_keypair = Keypair::new();
    create_token_2022_account(&mut rpc, &mint, &temp_spl_account_keypair, &owner, false)
        .await
        .unwrap();
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &temp_spl_account_keypair.pubkey(),
        &payer.pubkey(),
        &payer,
        amount,
        false,
    )
    .await
    .unwrap();

    // Transfer from temp SPL to ctoken to fund it
    let (spl_interface_pda, spl_interface_pda_bump) = find_spl_interface_pda_with_index(&mint, 0);
    let compressed_token_program_id =
        Pubkey::new_from_array(light_ctoken_interface::CTOKEN_PROGRAM_ID);
    let cpi_authority_pda = Pubkey::new_from_array(CPI_AUTHORITY_PDA);

    {
        let data = TransferSplToCtokenData {
            amount,
            spl_interface_pda_bump,
        };
        let wrapper_instruction_data = [vec![15u8], data.try_to_vec().unwrap()].concat();
        let wrapper_accounts = vec![
            AccountMeta::new_readonly(compressed_token_program_id, false),
            AccountMeta::new(temp_spl_account_keypair.pubkey(), false),
            AccountMeta::new(ctoken_account, false), // destination_ctoken_account (writable)
            AccountMeta::new_readonly(owner.pubkey(), true),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(spl_interface_pda, false),
            AccountMeta::new_readonly(anchor_spl::token::ID, false),
            AccountMeta::new_readonly(cpi_authority_pda, false),
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

    // Verify ctoken has tokens
    use spl_token_2022::pod::PodAccount;
    let ctoken_account_data = rpc.get_account(ctoken_account).await.unwrap().unwrap();
    let ctoken_state =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&ctoken_account_data.data[..165]).unwrap();
    assert_eq!(u64::from(ctoken_state.amount), amount);

    // Now test CToken to SPL transfer
    let data = TransferCTokenToSplData {
        amount: transfer_amount,
        spl_interface_pda_bump,
    };
    // Discriminator 17 = CtokenToSplInvoke
    let wrapper_instruction_data = [vec![17u8], data.try_to_vec().unwrap()].concat();

    // Account order from handler:
    // - accounts[0]: compressed_token_program (for CPI)
    // - accounts[1]: source_ctoken_account
    // - accounts[2]: destination_spl_token_account
    // - accounts[3]: authority (signer)
    // - accounts[4]: mint
    // - accounts[5]: payer (signer)
    // - accounts[6]: spl_interface_pda
    // - accounts[7]: spl_token_program
    // - accounts[8]: compressed_token_program_authority
    let wrapper_accounts = vec![
        AccountMeta::new_readonly(compressed_token_program_id, false),
        AccountMeta::new(ctoken_account, false),
        AccountMeta::new(spl_token_account_keypair.pubkey(), false),
        AccountMeta::new_readonly(owner.pubkey(), true), // authority (signer)
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(payer.pubkey(), true), // payer (signer)
        AccountMeta::new(spl_interface_pda, false),
        AccountMeta::new_readonly(anchor_spl::token::ID, false),
        AccountMeta::new_readonly(cpi_authority_pda, false),
    ];

    let instruction = Instruction {
        program_id: ID,
        accounts: wrapper_accounts,
        data: wrapper_instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &owner])
        .await
        .unwrap();

    // Verify SPL token balance increased
    let spl_account_data = rpc
        .get_account(spl_token_account_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    let spl_account =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&spl_account_data.data).unwrap();
    let final_spl_balance: u64 = spl_account.amount.into();
    assert_eq!(final_spl_balance, transfer_amount);

    // Verify CToken balance decreased
    let ctoken_account_data = rpc.get_account(ctoken_account).await.unwrap().unwrap();
    let ctoken_state =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&ctoken_account_data.data[..165]).unwrap();
    assert_eq!(
        u64::from(ctoken_state.amount),
        amount - transfer_amount,
        "CToken account balance should have decreased"
    );

    println!("CToken to SPL invoke test passed");
}

/// Test transferring SPL tokens to CToken with PDA authority using invoke_signed
#[tokio::test]
async fn test_spl_to_ctoken_invoke_signed() {
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

    // Derive the PDA that will be the authority (owner) for the SPL token account
    let (authority_pda, _) = Pubkey::find_program_address(&[TRANSFER_AUTHORITY_SEED], &ID);

    // Create SPL mint
    let mint = create_mint_helper(&mut rpc, &payer).await;
    let amount = 10000u64;
    let transfer_amount = 5000u64;

    // Create SPL ATA owned by the PDA using standard SPL ATA program
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

    // Mint tokens to the PDA's ATA (we're the mint authority so we can mint directly)
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

    // Create compressed token ATA for recipient
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

    // Get SPL interface PDA
    let (spl_interface_pda, spl_interface_pda_bump) = find_spl_interface_pda_with_index(&mint, 0);
    let compressed_token_program_id =
        Pubkey::new_from_array(light_ctoken_interface::CTOKEN_PROGRAM_ID);
    let cpi_authority_pda = Pubkey::new_from_array(CPI_AUTHORITY_PDA);

    // Build wrapper instruction for SPL to CToken transfer with PDA authority
    let data = TransferSplToCtokenData {
        amount: transfer_amount,
        spl_interface_pda_bump,
    };
    // Discriminator 16 = SplToCtokenInvokeSigned
    let wrapper_instruction_data = [vec![16u8], data.try_to_vec().unwrap()].concat();

    let wrapper_accounts = vec![
        AccountMeta::new_readonly(compressed_token_program_id, false),
        AccountMeta::new(spl_ata, false),
        AccountMeta::new(ctoken_account, false), // destination_ctoken_account (writable)
        AccountMeta::new_readonly(authority_pda, false), // authority is PDA, not signer
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(payer.pubkey(), true), // payer (signer)
        AccountMeta::new(spl_interface_pda, false),
        AccountMeta::new_readonly(anchor_spl::token::ID, false),
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

    // Verify SPL token balance decreased
    use spl_token_2022::pod::PodAccount;
    let spl_account_data = rpc.get_account(spl_ata).await.unwrap().unwrap();
    let spl_account =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&spl_account_data.data).unwrap();
    let final_spl_balance: u64 = spl_account.amount.into();
    assert_eq!(final_spl_balance, amount - transfer_amount);

    // Verify CToken balance increased
    let ctoken_account_data = rpc.get_account(ctoken_account).await.unwrap().unwrap();
    let ctoken_account_state =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&ctoken_account_data.data[..165]).unwrap();
    assert_eq!(
        u64::from(ctoken_account_state.amount),
        transfer_amount,
        "CToken account should have received tokens"
    );

    println!("SPL to CToken invoke_signed test passed");
}

/// Test transferring CToken to SPL with PDA authority using invoke_signed
#[tokio::test]
async fn test_ctoken_to_spl_invoke_signed() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();

    // Derive the PDA that will be the authority
    let (authority_pda, _) = Pubkey::find_program_address(&[TRANSFER_AUTHORITY_SEED], &ID);

    // Create SPL mint
    let mint = create_mint_helper(&mut rpc, &payer).await;
    let amount = 10000u64;
    let transfer_amount = 5000u64;

    // Create SPL token account for receiving tokens
    let spl_token_account_keypair = Keypair::new();
    let destination_owner = Keypair::new();
    light_test_utils::airdrop_lamports(&mut rpc, &destination_owner.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    create_token_2022_account(
        &mut rpc,
        &mint,
        &spl_token_account_keypair,
        &destination_owner,
        false,
    )
    .await
    .unwrap();

    // Create ctoken ATA owned by the PDA
    // We need to use a non-compressible ATA so it can be owned by a PDA
    let (ctoken_account, bump) = derive_ctoken_ata(&authority_pda, &mint);
    let instruction = CreateAssociatedTokenAccount {
        idempotent: false,
        bump,
        payer: payer.pubkey(),
        owner: authority_pda,
        mint,
        associated_token_account: ctoken_account,
        compressible: None, // Non-compressible so PDA can own it
    }
    .instruction()
    .unwrap();
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Fund the ctoken account via SPL transfer from a temporary account
    let temp_spl_account_keypair = Keypair::new();
    let temp_owner = Keypair::new();
    light_test_utils::airdrop_lamports(&mut rpc, &temp_owner.pubkey(), 1_000_000_000)
        .await
        .unwrap();
    create_token_2022_account(
        &mut rpc,
        &mint,
        &temp_spl_account_keypair,
        &temp_owner,
        false,
    )
    .await
    .unwrap();
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &temp_spl_account_keypair.pubkey(),
        &payer.pubkey(),
        &payer,
        amount,
        false,
    )
    .await
    .unwrap();

    // Transfer from temp SPL to ctoken to fund it
    let (spl_interface_pda, spl_interface_pda_bump) = find_spl_interface_pda_with_index(&mint, 0);
    let compressed_token_program_id =
        Pubkey::new_from_array(light_ctoken_interface::CTOKEN_PROGRAM_ID);
    let cpi_authority_pda = Pubkey::new_from_array(CPI_AUTHORITY_PDA);

    {
        let data = TransferSplToCtokenData {
            amount,
            spl_interface_pda_bump,
        };
        let wrapper_instruction_data = [vec![15u8], data.try_to_vec().unwrap()].concat();
        let wrapper_accounts = vec![
            AccountMeta::new_readonly(compressed_token_program_id, false),
            AccountMeta::new(temp_spl_account_keypair.pubkey(), false),
            AccountMeta::new(ctoken_account, false), // destination_ctoken_account (writable)
            AccountMeta::new_readonly(temp_owner.pubkey(), true),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(spl_interface_pda, false),
            AccountMeta::new_readonly(anchor_spl::token::ID, false),
            AccountMeta::new_readonly(cpi_authority_pda, false),
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

    // Verify ctoken has tokens
    use spl_token_2022::pod::PodAccount;
    let ctoken_account_data = rpc.get_account(ctoken_account).await.unwrap().unwrap();
    let ctoken_state =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&ctoken_account_data.data[..165]).unwrap();
    assert_eq!(u64::from(ctoken_state.amount), amount);

    // Now test CToken to SPL transfer with PDA authority
    let data = TransferCTokenToSplData {
        amount: transfer_amount,
        spl_interface_pda_bump,
    };
    // Discriminator 18 = CtokenToSplInvokeSigned
    let wrapper_instruction_data = [vec![18u8], data.try_to_vec().unwrap()].concat();

    let wrapper_accounts = vec![
        AccountMeta::new_readonly(compressed_token_program_id, false),
        AccountMeta::new(ctoken_account, false),
        AccountMeta::new(spl_token_account_keypair.pubkey(), false),
        AccountMeta::new_readonly(authority_pda, false), // authority is PDA, not signer
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(payer.pubkey(), true), // payer (signer)
        AccountMeta::new(spl_interface_pda, false),
        AccountMeta::new_readonly(anchor_spl::token::ID, false),
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

    // Verify SPL token balance increased
    let spl_account_data = rpc
        .get_account(spl_token_account_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();
    let spl_account =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&spl_account_data.data).unwrap();
    let final_spl_balance: u64 = spl_account.amount.into();
    assert_eq!(final_spl_balance, transfer_amount);

    // Verify CToken balance decreased
    let ctoken_account_data = rpc.get_account(ctoken_account).await.unwrap().unwrap();
    let ctoken_state =
        spl_pod::bytemuck::pod_from_bytes::<PodAccount>(&ctoken_account_data.data[..165]).unwrap();
    assert_eq!(
        u64::from(ctoken_state.amount),
        amount - transfer_amount,
        "CToken account balance should have decreased"
    );

    println!("CToken to SPL invoke_signed test passed");
}
