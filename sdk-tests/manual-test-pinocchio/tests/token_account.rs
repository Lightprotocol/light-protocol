//! Test token vault pattern - PDA token account with rent-free CPI.

mod shared;

use borsh::BorshDeserialize;
use light_program_test::Rpc;
use light_token::instruction::{config_pda, rent_sponsor_pda, LIGHT_TOKEN_PROGRAM_ID};
use light_token_interface::state::{AccountState, Token};
use manual_test_pinocchio::{CreateTokenVaultParams, TOKEN_VAULT_SEED};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

/// Test creating a PDA token vault using CreateTokenAccountCpi.
#[tokio::test]
async fn test_create_token_vault() {
    let (mut rpc, payer, _) = shared::setup_test_env().await;

    // Create a mint to use for the token vault
    let mint = shared::create_test_mint(&mut rpc, &payer).await;

    // Vault owner - can be any pubkey (e.g., a PDA authority)
    let vault_owner = Keypair::new();

    let program_id = Pubkey::new_from_array(manual_test_pinocchio::ID);

    // Derive token vault PDA
    let (token_vault, vault_bump) =
        Pubkey::find_program_address(&[TOKEN_VAULT_SEED, mint.as_ref()], &program_id);

    let params = CreateTokenVaultParams { vault_bump };

    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new_readonly(vault_owner.pubkey(), false),
        AccountMeta::new(token_vault, false),
        AccountMeta::new_readonly(config_pda(), false),
        AccountMeta::new(rent_sponsor_pda(), false),
        AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    let ix = Instruction {
        program_id,
        accounts,
        data: [
            manual_test_pinocchio::discriminators::CREATE_TOKEN_VAULT.as_slice(),
            &borsh::to_vec(&params).unwrap(),
        ]
        .concat(),
    };

    rpc.create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
        .await
        .expect("CreateTokenVault should succeed");

    // Verify token account exists and has correct state
    let vault_account = rpc
        .get_account(token_vault)
        .await
        .unwrap()
        .expect("Token vault should exist");

    let token =
        Token::deserialize(&mut &vault_account.data[..]).expect("Should deserialize as Token");

    assert_eq!(token.mint.to_bytes(), mint.to_bytes());
    assert_eq!(token.owner.to_bytes(), vault_owner.pubkey().to_bytes());
    assert_eq!(token.amount, 0);
    assert_eq!(token.state, AccountState::Initialized);
}
