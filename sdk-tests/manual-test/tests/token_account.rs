//! Test token vault pattern - PDA token account with rent-free CPI.

mod shared;

use anchor_lang::{InstructionData, ToAccountMetas};
use borsh::BorshDeserialize;
use light_program_test::Rpc;
use light_token::instruction::{config_pda, rent_sponsor_pda, LIGHT_TOKEN_PROGRAM_ID};
use light_token_interface::state::{AccountState, Token};
use manual_test::{CreateTokenVaultParams, TOKEN_VAULT_SEED};
use solana_sdk::{
    instruction::Instruction,
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

    // Derive token vault PDA
    let (token_vault, vault_bump) =
        Pubkey::find_program_address(&[TOKEN_VAULT_SEED, mint.as_ref()], &manual_test::ID);

    let params = CreateTokenVaultParams { vault_bump };

    let accounts = manual_test::accounts::CreateTokenVaultAccounts {
        payer: payer.pubkey(),
        mint,
        vault_owner: vault_owner.pubkey(),
        token_vault,
        compressible_config: config_pda(),
        rent_sponsor: rent_sponsor_pda(),
        light_token_program: LIGHT_TOKEN_PROGRAM_ID,
        system_program: solana_sdk::system_program::ID,
    };

    let ix = Instruction {
        program_id: manual_test::ID,
        accounts: accounts.to_account_metas(None),
        data: manual_test::instruction::CreateTokenVault { params }.data(),
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
