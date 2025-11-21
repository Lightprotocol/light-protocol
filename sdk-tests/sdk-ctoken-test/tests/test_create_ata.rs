// Tests for CreateAssociatedTokenAccountInfos (CreateAta instructions)

mod shared;

use borsh::{BorshDeserialize, BorshSerialize};
use light_client::rpc::Rpc;
use light_compressed_token_sdk::ctoken::CTOKEN_PROGRAM_ID;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use native_ctoken_examples::{CreateAtaData, ATA_SEED, ID};
use shared::setup_create_compressed_mint;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signer::Signer,
};

/// Test creating an ATA using CreateAssociatedTokenAccountInfos::invoke()
#[tokio::test]
async fn test_create_ata_invoke() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let mint_authority = payer.pubkey();

    // Create compressed mint first (using helper)
    let (mint_pda, _compression_address, _) =
        setup_create_compressed_mint(&mut rpc, &payer, mint_authority, 9, vec![]).await;

    // Derive the ATA address
    let owner = payer.pubkey();
    use light_compressed_token_sdk::ctoken::derive_ctoken_ata;
    let (ata_address, bump) = derive_ctoken_ata(&owner, &mint_pda);

    // Build CreateAtaData
    let create_ata_data = CreateAtaData {
        owner,
        mint: mint_pda,
        bump,
        pre_pay_num_epochs: 2,
        lamports_per_write: 1,
    };
    // Discriminator 4 = CreateAtaInvoke
    let instruction_data = [vec![4u8], create_ata_data.try_to_vec().unwrap()].concat();

    use light_compressed_token_sdk::ctoken::{config_pda, rent_sponsor_pda};
    let config = config_pda();
    let rent_sponsor = rent_sponsor_pda();

    let instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(ata_address, false),
            AccountMeta::new_readonly(Pubkey::default(), false), // system_program
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(rent_sponsor, false),
            AccountMeta::new_readonly(CTOKEN_PROGRAM_ID.into(), false),
        ],
        data: instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify ATA was created
    let ata_account_data = rpc.get_account(ata_address).await.unwrap().unwrap();

    // Parse and verify account data
    use light_ctoken_types::state::CToken;
    let account_state = CToken::deserialize(&mut &ata_account_data.data[..]).unwrap();
    assert_eq!(
        account_state.mint.to_bytes(),
        mint_pda.to_bytes(),
        "Mint should match"
    );
    assert_eq!(
        account_state.owner.to_bytes(),
        owner.to_bytes(),
        "Owner should match"
    );
    assert_eq!(account_state.amount, 0, "Initial amount should be 0");
}

/// Test creating an ATA with PDA payer using CreateAssociatedTokenAccountInfos::invoke_signed()
#[tokio::test]
async fn test_create_ata_invoke_signed() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let mint_authority = payer.pubkey();

    // Create compressed mint first (using helper)
    let (mint_pda, _compression_address, _) =
        setup_create_compressed_mint(&mut rpc, &payer, mint_authority, 9, vec![]).await;

    // Derive the PDA that will act as payer/owner (using ATA_SEED)
    let (pda_owner, _pda_bump) = Pubkey::find_program_address(&[ATA_SEED], &ID);

    // Fund the PDA so it can pay for the ATA creation
    let fund_ix = solana_sdk::system_instruction::transfer(
        &payer.pubkey(),
        &pda_owner,
        1_000_000_000, // 1 SOL
    );
    rpc.create_and_send_transaction(&[fund_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Derive the ATA address for the PDA owner
    use light_compressed_token_sdk::ctoken::derive_ctoken_ata;
    let (ata_address, bump) = derive_ctoken_ata(&pda_owner, &mint_pda);

    // Build CreateAtaData with PDA as owner
    let create_ata_data = CreateAtaData {
        owner: pda_owner,
        mint: mint_pda,
        bump,
        pre_pay_num_epochs: 2,
        lamports_per_write: 1,
    };
    // Discriminator 5 = CreateAtaInvokeSigned
    let instruction_data = [vec![5u8], create_ata_data.try_to_vec().unwrap()].concat();

    use light_compressed_token_sdk::ctoken::{config_pda, rent_sponsor_pda};
    let config = config_pda();
    let rent_sponsor = rent_sponsor_pda();

    let instruction = Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(pda_owner, false), // PDA payer - not a signer (program signs via invoke_signed)
            AccountMeta::new(ata_address, false),
            AccountMeta::new_readonly(Pubkey::default(), false), // system_program
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(rent_sponsor, false),
            AccountMeta::new_readonly(CTOKEN_PROGRAM_ID.into(), false),
        ],
        data: instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify ATA was created
    let ata_account_data = rpc.get_account(ata_address).await.unwrap().unwrap();

    // Parse and verify account data
    use light_ctoken_types::state::CToken;
    let account_state = CToken::deserialize(&mut &ata_account_data.data[..]).unwrap();
    assert_eq!(
        account_state.mint.to_bytes(),
        mint_pda.to_bytes(),
        "Mint should match"
    );
    assert_eq!(
        account_state.owner.to_bytes(),
        pda_owner.to_bytes(),
        "Owner should match PDA"
    );
    assert_eq!(account_state.amount, 0, "Initial amount should be 0");
}
