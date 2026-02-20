// Tests for CreateAssociatedTokenAccountCpi (CreateAta instructions)

mod shared;

use borsh::{BorshDeserialize, BorshSerialize};
use light_client::rpc::Rpc;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_token::instruction::LIGHT_TOKEN_PROGRAM_ID;
use sdk_light_token_pinocchio_test::{CreateAtaData, ATA_SEED};
use shared::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signer::Signer,
};

/// Test creating an ATA using CreateAssociatedTokenAccountCpi::invoke()
#[tokio::test]
async fn test_create_ata_invoke() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("sdk_light_token_pinocchio_test", PROGRAM_ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let mint_authority = payer.pubkey();

    // Create compressed mint first (using helper)
    let (mint_pda, _compression_address, _, _mint_seed) =
        setup_create_mint(&mut rpc, &payer, mint_authority, 9, vec![]).await;

    // Derive the ATA address
    let owner = payer.pubkey();
    use light_token::instruction::derive_token_ata;
    let ata_address = derive_token_ata(&owner, &mint_pda);

    // Build CreateAtaData (owner and mint are passed as accounts)
    let create_ata_data = CreateAtaData {
        pre_pay_num_epochs: 2,
        lamports_per_write: 1,
    };
    // Discriminator 4 = CreateAtaInvoke
    let instruction_data = [vec![4u8], create_ata_data.try_to_vec().unwrap()].concat();

    use light_token::instruction::{config_pda, rent_sponsor_pda};
    let config = config_pda();
    let rent_sponsor = rent_sponsor_pda();

    // Account order: owner, mint, payer, ata, system_program, config, rent_sponsor, light_token_program
    let instruction = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(owner, false),
            AccountMeta::new_readonly(mint_pda, false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(ata_address, false),
            AccountMeta::new_readonly(Pubkey::default(), false), // system_program
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(rent_sponsor, false),
            AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID, false),
        ],
        data: instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify ATA was created
    let ata_account_data = rpc.get_account(ata_address).await.unwrap().unwrap();

    // Parse and verify account data
    use light_token_interface::state::{AccountState, Token};
    let account_state = Token::deserialize(&mut &ata_account_data.data[..]).unwrap();
    assert_eq!(
        account_state,
        Token {
            mint: mint_pda.to_bytes().into(),
            owner: owner.to_bytes().into(),
            amount: 0,
            delegate: None,
            state: AccountState::Initialized,
            is_native: None,
            delegated_amount: 0,
            close_authority: None,
            account_type: account_state.account_type,
            extensions: account_state.extensions.clone(),
        }
    );
}

/// Test creating an ATA with PDA payer using CreateAssociatedTokenAccountCpi::invoke_signed()
#[tokio::test]
async fn test_create_ata_invoke_signed() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("sdk_light_token_pinocchio_test", PROGRAM_ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let mint_authority = payer.pubkey();

    // Create compressed mint first (using helper)
    let (mint_pda, _compression_address, _, _mint_seed) =
        setup_create_mint(&mut rpc, &payer, mint_authority, 9, vec![]).await;

    // Derive the PDA that will act as payer/owner (using ATA_SEED)
    let (pda_owner, _pda_bump) = Pubkey::find_program_address(&[ATA_SEED], &PROGRAM_ID);

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
    use light_token::instruction::derive_token_ata;
    let ata_address = derive_token_ata(&pda_owner, &mint_pda);

    // Build CreateAtaData with PDA as owner (owner and mint are passed as accounts)
    let create_ata_data = CreateAtaData {
        pre_pay_num_epochs: 2,
        lamports_per_write: 1,
    };
    // Discriminator 5 = CreateAtaInvokeSigned
    let instruction_data = [vec![5u8], create_ata_data.try_to_vec().unwrap()].concat();

    use light_token::instruction::{config_pda, rent_sponsor_pda};
    let config = config_pda();
    let rent_sponsor = rent_sponsor_pda();

    // Account order: owner, mint, payer, ata, system_program, config, rent_sponsor, light_token_program
    let instruction = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(pda_owner, false), // owner
            AccountMeta::new_readonly(mint_pda, false),
            AccountMeta::new(pda_owner, false), // PDA payer - not a signer (program signs via invoke_signed)
            AccountMeta::new(ata_address, false),
            AccountMeta::new_readonly(Pubkey::default(), false), // system_program
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(rent_sponsor, false),
            AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID, false),
        ],
        data: instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify ATA was created
    let ata_account_data = rpc.get_account(ata_address).await.unwrap().unwrap();

    // Parse and verify account data
    use light_token_interface::state::{AccountState, Token};
    let account_state = Token::deserialize(&mut &ata_account_data.data[..]).unwrap();
    assert_eq!(
        account_state,
        Token {
            mint: mint_pda.to_bytes().into(),
            owner: pda_owner.to_bytes().into(),
            amount: 0,
            delegate: None,
            state: AccountState::Initialized,
            is_native: None,
            delegated_amount: 0,
            close_authority: None,
            account_type: account_state.account_type,
            extensions: account_state.extensions.clone(),
        }
    );
}
