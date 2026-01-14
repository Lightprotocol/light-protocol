// Tests for CreateTokenAccountCpi (CreateTokenAccount instructions)

mod shared;

use borsh::{BorshDeserialize, BorshSerialize};
use light_client::rpc::Rpc;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_token_sdk::token::LIGHT_TOKEN_PROGRAM_ID;
use native_ctoken_examples::{CreateTokenAccountData, ID};
use shared::setup_create_compressed_mint;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};

/// Test creating a token account using CreateTokenAccountCpi::invoke()
#[tokio::test]
async fn test_create_token_account_invoke() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let mint_authority = payer.pubkey();

    // Create compressed mint first (using helper)
    let (mint_pda, _compression_address, _, _mint_seed) =
        setup_create_compressed_mint(&mut rpc, &payer, mint_authority, 9, vec![]).await;

    // Create ctoken account via wrapper program
    let ctoken_account = Keypair::new();
    let owner = payer.pubkey();

    let create_token_account_data = CreateTokenAccountData {
        owner,
        pre_pay_num_epochs: 2,
        lamports_per_write: 1,
    };
    let instruction_data = [vec![2u8], create_token_account_data.try_to_vec().unwrap()].concat();

    use light_token_sdk::token::{config_pda, rent_sponsor_pda};
    let config = config_pda();
    let rent_sponsor = rent_sponsor_pda();

    let instruction = Instruction {
        program_id: ID,
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

    // Verify ctoken account was created
    let ctoken_account_data = rpc
        .get_account(ctoken_account.pubkey())
        .await
        .unwrap()
        .unwrap();

    // Parse and verify account data
    use light_token_interface::state::Token;
    let account_state = Token::deserialize(&mut &ctoken_account_data.data[..]).unwrap();
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

/// Test creating a PDA-owned token account using CreateTokenAccountCpi::invoke_signed()
#[tokio::test]
async fn test_create_token_account_invoke_signed() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(
        false,
        Some(vec![("native_ctoken_examples", ID)]),
    ))
    .await
    .unwrap();

    let payer = rpc.get_payer().insecure_clone();
    let mint_authority = payer.pubkey();

    // Create compressed mint first (using helper)
    let (mint_pda, _compression_address, _, _mint_seed) =
        setup_create_compressed_mint(&mut rpc, &payer, mint_authority, 9, vec![]).await;

    // Derive the PDA for the token account (same seeds as in the program)
    let token_account_seed: &[u8] = b"token_account";
    let (ctoken_account_pda, _bump) = Pubkey::find_program_address(&[token_account_seed], &ID);

    let owner = payer.pubkey();

    let create_token_account_data = CreateTokenAccountData {
        owner,
        pre_pay_num_epochs: 2,
        lamports_per_write: 1,
    };
    // Discriminator 3 = CreateTokenAccountInvokeSigned
    let instruction_data = [vec![3u8], create_token_account_data.try_to_vec().unwrap()].concat();

    use light_token_sdk::token::{config_pda, rent_sponsor_pda};
    let config = config_pda();
    let rent_sponsor = rent_sponsor_pda();

    let instruction = Instruction {
        program_id: ID,
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

    // Note: only payer signs, the PDA account is signed by the program via invoke_signed
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify ctoken account was created
    let ctoken_account_data = rpc.get_account(ctoken_account_pda).await.unwrap().unwrap();

    // Parse and verify account data
    use light_token_interface::state::Token;
    let account_state = Token::deserialize(&mut &ctoken_account_data.data[..]).unwrap();
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
