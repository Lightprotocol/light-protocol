// Tests for CreateAssociatedTokenAccount2Infos invoke() and invoke_signed()

mod shared;

use borsh::BorshSerialize;
use light_client::rpc::Rpc;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_token_sdk::token::{config_pda, derive_token_ata, rent_sponsor_pda, CTOKEN_PROGRAM_ID};
use native_ctoken_examples::{CreateAta2Data, InstructionType, ATA_SEED, ID};
use shared::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signer::Signer,
};

/// Test creating an ATA using V2 variant (owner/mint as accounts) with invoke()
#[tokio::test]
async fn test_create_ata2_invoke() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("native_ctoken_examples", ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create a compressed mint (no recipients, just the mint)
    let (mint_pda, _compression_address, _) =
        setup_create_compressed_mint(&mut rpc, &payer, payer.pubkey(), 9, vec![]).await;

    // Derive the ATA address
    let owner = payer.pubkey();
    let (ata_address, bump) = derive_token_ata(&owner, &mint_pda);

    // Verify ATA doesn't exist yet
    let ata_account_before = rpc.get_account(ata_address).await.unwrap();
    assert!(ata_account_before.is_none(), "ATA should not exist yet");

    // Get config and rent sponsor
    let compressible_config = config_pda();
    let rent_sponsor = rent_sponsor_pda();

    // Build instruction data
    let create_ata2_data = CreateAta2Data {
        bump,
        pre_pay_num_epochs: 2,
        lamports_per_write: 1000,
    };
    let instruction_data = [
        vec![InstructionType::CreateAta2Invoke as u8],
        create_ata2_data.try_to_vec().unwrap(),
    ]
    .concat();

    // Account order for CreateAta2Invoke:
    // - accounts[0]: owner (readonly)
    // - accounts[1]: mint (readonly)
    // - accounts[2]: payer (signer, writable)
    // - accounts[3]: associated_token_account (writable)
    // - accounts[4]: system_program
    // - accounts[5]: compressible_config
    // - accounts[6]: rent_sponsor (writable)
    // - accounts[7]: ctoken_program (for CPI)
    let instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new_readonly(owner, false),
            AccountMeta::new_readonly(mint_pda, false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(ata_address, false),
            AccountMeta::new_readonly(Pubkey::default(), false), // system_program
            AccountMeta::new_readonly(compressible_config, false),
            AccountMeta::new(rent_sponsor, false),
            AccountMeta::new_readonly(CTOKEN_PROGRAM_ID, false),
        ],
        data: instruction_data,
    };

    // Execute the instruction
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify ATA was created
    let ata_account_after = rpc.get_account(ata_address).await.unwrap();
    assert!(
        ata_account_after.is_some(),
        "ATA should exist after create_ata2"
    );
}

/// Test creating an ATA using V2 variant with PDA payer via invoke_signed()
#[tokio::test]
async fn test_create_ata2_invoke_signed() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("native_ctoken_examples", ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create a compressed mint (no recipients, just the mint)
    let (mint_pda, _compression_address, _) =
        setup_create_compressed_mint(&mut rpc, &payer, payer.pubkey(), 9, vec![]).await;

    // Derive the PDA that will act as payer
    let (pda_payer, _pda_bump) = Pubkey::find_program_address(&[ATA_SEED], &ID);

    // Fund the PDA payer so it can pay for the ATA creation
    let fund_ix = solana_sdk::system_instruction::transfer(&payer.pubkey(), &pda_payer, 10_000_000);
    rpc.create_and_send_transaction(&[fund_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // The owner will be the regular payer (not the PDA)
    let owner = payer.pubkey();
    let (ata_address, bump) = derive_token_ata(&owner, &mint_pda);

    // Verify ATA doesn't exist yet
    let ata_account_before = rpc.get_account(ata_address).await.unwrap();
    assert!(ata_account_before.is_none(), "ATA should not exist yet");

    // Get config and rent sponsor
    let compressible_config = config_pda();
    let rent_sponsor = rent_sponsor_pda();

    // Build instruction data
    let create_ata2_data = CreateAta2Data {
        bump,
        pre_pay_num_epochs: 2,
        lamports_per_write: 1000,
    };
    let instruction_data = [
        vec![InstructionType::CreateAta2InvokeSigned as u8],
        create_ata2_data.try_to_vec().unwrap(),
    ]
    .concat();

    // Account order for CreateAta2InvokeSigned:
    // - accounts[0]: owner (readonly)
    // - accounts[1]: mint (readonly)
    // - accounts[2]: payer (PDA, writable, not signer - program signs)
    // - accounts[3]: associated_token_account (writable)
    // - accounts[4]: system_program
    // - accounts[5]: compressible_config
    // - accounts[6]: rent_sponsor (writable)
    // - accounts[7]: ctoken_program (for CPI)
    let instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new_readonly(owner, false),
            AccountMeta::new_readonly(mint_pda, false),
            AccountMeta::new(pda_payer, false), // PDA payer, not signer
            AccountMeta::new(ata_address, false),
            AccountMeta::new_readonly(Pubkey::default(), false), // system_program
            AccountMeta::new_readonly(compressible_config, false),
            AccountMeta::new(rent_sponsor, false),
            AccountMeta::new_readonly(CTOKEN_PROGRAM_ID, false),
        ],
        data: instruction_data,
    };

    // Execute the instruction
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify ATA was created
    let ata_account_after = rpc.get_account(ata_address).await.unwrap();
    assert!(
        ata_account_after.is_some(),
        "ATA should exist after create_ata2_invoke_signed"
    );
}
