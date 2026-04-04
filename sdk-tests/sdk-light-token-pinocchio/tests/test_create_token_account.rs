// Tests for CreateTokenAccountCpi (CreateTokenAccount instructions)

mod shared;

use borsh::BorshDeserialize;
use light_client::rpc::Rpc;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_token::instruction::LIGHT_TOKEN_PROGRAM_ID;
use light_token_interface::state::{
    AccountState, ExtensionStruct, Token, ACCOUNT_TYPE_TOKEN_ACCOUNT,
};
use sdk_light_token_pinocchio_test::{CreateTokenAccountData, TOKEN_ACCOUNT_SEED};
use shared::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};

fn assert_token_account(account_state: &Token, mint_pda: Pubkey, owner: Pubkey) {
    let compressible_ext = account_state
        .extensions
        .as_ref()
        .and_then(|exts| {
            exts.iter().find_map(|e| match e {
                ExtensionStruct::Compressible(info) => Some(*info),
                _ => None,
            })
        })
        .expect("Token account should have Compressible extension");

    let expected = Token {
        mint: mint_pda.to_bytes().into(),
        owner: owner.to_bytes().into(),
        amount: 0,
        delegate: None,
        state: AccountState::Initialized,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
        extensions: Some(vec![ExtensionStruct::Compressible(compressible_ext)]),
    };

    assert_eq!(account_state, &expected);
}

/// Test creating a token account using CreateTokenAccountCpi::invoke()
#[tokio::test]
async fn test_create_token_account_invoke() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("sdk_light_token_pinocchio_test", PROGRAM_ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let mint_authority = payer.pubkey();

    let (mint_pda, _compression_address, _, _mint_seed) =
        setup_create_mint(&mut rpc, &payer, mint_authority, 9, vec![]).await;

    let ctoken_account = Keypair::new();
    let owner = payer.pubkey();

    let create_token_account_data = CreateTokenAccountData {
        owner: owner.to_bytes(),
        pre_pay_num_epochs: 2,
        lamports_per_write: 1,
    };
    // Discriminator 2 = CreateTokenAccountInvoke
    let instruction_data = [
        vec![2u8],
        borsh::to_vec(&create_token_account_data).unwrap(),
    ]
    .concat();

    use light_token::instruction::{config_pda, rent_sponsor_pda};
    let config = config_pda();
    let rent_sponsor = rent_sponsor_pda();

    let instruction = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(ctoken_account.pubkey(), true),
            AccountMeta::new_readonly(mint_pda, false),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new_readonly(Pubkey::default(), false), // system_program
            AccountMeta::new(rent_sponsor, false),
            AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID, false),
        ],
        data: instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &ctoken_account])
        .await
        .unwrap();

    let ctoken_account_data = rpc
        .get_account(ctoken_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let account_state = Token::deserialize(&mut &ctoken_account_data.data[..]).unwrap();
    assert_token_account(&account_state, mint_pda, owner);
}

/// Test creating a PDA-owned token account using CreateTokenAccountCpi::invoke_signed()
#[tokio::test]
async fn test_create_token_account_invoke_signed() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("sdk_light_token_pinocchio_test", PROGRAM_ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let mint_authority = payer.pubkey();

    let (mint_pda, _compression_address, _, _mint_seed) =
        setup_create_mint(&mut rpc, &payer, mint_authority, 9, vec![]).await;

    let token_account_seed: &[u8] = b"token_account";
    let (ctoken_account_pda, _bump) =
        Pubkey::find_program_address(&[token_account_seed], &PROGRAM_ID);

    let owner = payer.pubkey();

    let create_token_account_data = CreateTokenAccountData {
        owner: owner.to_bytes(),
        pre_pay_num_epochs: 2,
        lamports_per_write: 1,
    };
    // Discriminator 3 = CreateTokenAccountInvokeSigned
    let instruction_data = [
        vec![3u8],
        borsh::to_vec(&create_token_account_data).unwrap(),
    ]
    .concat();

    use light_token::instruction::{config_pda, rent_sponsor_pda};
    let config = config_pda();
    let rent_sponsor = rent_sponsor_pda();

    let instruction = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(ctoken_account_pda, false), // PDA, not a signer
            AccountMeta::new_readonly(mint_pda, false),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new_readonly(Pubkey::default(), false), // system_program
            AccountMeta::new(rent_sponsor, false),
            AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID, false),
        ],
        data: instruction_data,
    };

    // Only payer signs; PDA is signed by the program via invoke_signed
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    let ctoken_account_data = rpc.get_account(ctoken_account_pda).await.unwrap().unwrap();

    let account_state = Token::deserialize(&mut &ctoken_account_data.data[..]).unwrap();
    assert_token_account(&account_state, mint_pda, owner);
}

/// Test creating a token account using CreateTokenAccountCpi::invoke_with()
#[tokio::test]
async fn test_create_token_account_invoke_with() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("sdk_light_token_pinocchio_test", PROGRAM_ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let mint_authority = payer.pubkey();

    let (mint_pda, _compression_address, _, _mint_seed) =
        setup_create_mint(&mut rpc, &payer, mint_authority, 9, vec![]).await;

    let ctoken_account = Keypair::new();
    let owner = payer.pubkey();

    let create_token_account_data = CreateTokenAccountData {
        owner: owner.to_bytes(),
        pre_pay_num_epochs: 2,
        lamports_per_write: 1,
    };
    // Discriminator 41 = CreateTokenAccountInvokeWith
    let instruction_data = [
        vec![41u8],
        borsh::to_vec(&create_token_account_data).unwrap(),
    ]
    .concat();

    use light_token::instruction::{config_pda, rent_sponsor_pda};
    let config = config_pda();
    let rent_sponsor = rent_sponsor_pda();

    let instruction = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(ctoken_account.pubkey(), true),
            AccountMeta::new_readonly(mint_pda, false),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new_readonly(Pubkey::default(), false), // system_program
            AccountMeta::new(rent_sponsor, false),
            AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID, false),
        ],
        data: instruction_data,
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &ctoken_account])
        .await
        .unwrap();

    let ctoken_account_data = rpc
        .get_account(ctoken_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    let account_state = Token::deserialize(&mut &ctoken_account_data.data[..]).unwrap();
    assert_token_account(&account_state, mint_pda, owner);
}

/// Test creating a PDA-owned token account using CreateTokenAccountCpi::invoke_signed_with()
#[tokio::test]
async fn test_create_token_account_invoke_signed_with() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("sdk_light_token_pinocchio_test", PROGRAM_ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let mint_authority = payer.pubkey();

    let (mint_pda, _compression_address, _, _mint_seed) =
        setup_create_mint(&mut rpc, &payer, mint_authority, 9, vec![]).await;

    let (ctoken_account_pda, _bump) =
        Pubkey::find_program_address(&[TOKEN_ACCOUNT_SEED], &PROGRAM_ID);

    let owner = payer.pubkey();

    let create_token_account_data = CreateTokenAccountData {
        owner: owner.to_bytes(),
        pre_pay_num_epochs: 2,
        lamports_per_write: 1,
    };
    // Discriminator 42 = CreateTokenAccountInvokeSignedWith
    let instruction_data = [
        vec![42u8],
        borsh::to_vec(&create_token_account_data).unwrap(),
    ]
    .concat();

    use light_token::instruction::{config_pda, rent_sponsor_pda};
    let config = config_pda();
    let rent_sponsor = rent_sponsor_pda();

    let instruction = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(ctoken_account_pda, false), // PDA, not a signer
            AccountMeta::new_readonly(mint_pda, false),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new_readonly(Pubkey::default(), false), // system_program
            AccountMeta::new(rent_sponsor, false),
            AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID, false),
        ],
        data: instruction_data,
    };

    // Only payer signs; PDA is signed by the program via invoke_signed
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    let ctoken_account_data = rpc.get_account(ctoken_account_pda).await.unwrap().unwrap();

    let account_state = Token::deserialize(&mut &ctoken_account_data.data[..]).unwrap();
    assert_token_account(&account_state, mint_pda, owner);
}
