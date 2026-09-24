//! Tests for read/load parity surfaces in light-token-client.

use borsh::BorshDeserialize;
use light_client::rpc::Rpc;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_test_utils::spl::{
    create_mint_helper, create_token_account, mint_spl_tokens, CREATE_MINT_HELPER_DECIMALS,
};
use light_token::instruction::derive_token_ata;
use light_token_client::{
    actions::{CreateAta, Load, Wrap},
    read::get_ata,
};
use light_token_interface::state::Token;
use solana_sdk::{program_pack::Pack, signature::Keypair, signer::Signer};
use spl_token::state::Account as SplTokenAccount;

#[tokio::test]
async fn test_get_ata_hot_balance_view() {
    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let owner = payer.pubkey();
    let decimals = CREATE_MINT_HELPER_DECIMALS;

    // SPL mint used by Light Token wrap flow
    let mint = create_mint_helper(&mut rpc, &payer).await;
    let source_spl = Keypair::new();
    create_token_account(&mut rpc, &mint, &source_spl, &payer)
        .await
        .unwrap();

    let ata = derive_token_ata(&owner, &mint);

    CreateAta {
        mint,
        owner,
        idempotent: true,
    }
    .execute(&mut rpc, &payer)
    .await
    .unwrap();

    // Create a hot light balance by wrapping from SPL.
    let amount = 800u64;
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &source_spl.pubkey(),
        &owner,
        &payer,
        amount,
        false,
    )
    .await
    .unwrap();

    Wrap {
        source_spl_ata: source_spl.pubkey(),
        destination: ata,
        mint,
        amount,
        decimals,
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    let view = get_ata(&rpc, owner, mint).await.unwrap();
    assert_eq!(view.address, ata);
    assert_eq!(view.amount, amount);
    assert_eq!(view.hot_amount, amount);
    assert_eq!(view.compressed_amount, 0);
    assert!(view.has_hot_account);
    assert!(!view.requires_load);
    assert_eq!(view.parsed.amount, amount);
    assert!(view.parsed.is_initialized);
    assert!(!view.parsed.is_frozen);
}

#[tokio::test]
async fn test_load_wraps_spl_ata_into_light_ata() {
    use anchor_spl::associated_token::{
        get_associated_token_address, spl_associated_token_account,
    };

    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let owner = payer.pubkey();
    let mint = create_mint_helper(&mut rpc, &payer).await;
    let spl_ata = get_associated_token_address(&owner, &mint);
    let light_ata = derive_token_ata(&owner, &mint);

    let create_spl_ata_ix =
        spl_associated_token_account::instruction::create_associated_token_account(
            &payer.pubkey(),
            &owner,
            &mint,
            &anchor_spl::token::ID,
        );
    rpc.create_and_send_transaction(&[create_spl_ata_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    let mint_amount = 1_000u64;
    mint_spl_tokens(
        &mut rpc,
        &mint,
        &spl_ata,
        &owner,
        &payer,
        mint_amount,
        false,
    )
    .await
    .unwrap();

    assert!(rpc.get_account(light_ata).await.unwrap().is_none());

    let signature = Load {
        owner,
        mint,
        wrap: true,
        allow_frozen: false,
        decimals: Some(CREATE_MINT_HELPER_DECIMALS),
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .unwrap();

    assert!(signature.is_some(), "load should submit transaction");

    let light_account = rpc.get_account(light_ata).await.unwrap().unwrap();
    let light_state = Token::deserialize(&mut &light_account.data[..]).unwrap();
    assert_eq!(light_state.amount, mint_amount);

    let spl_account = rpc.get_account(spl_ata).await.unwrap().unwrap();
    let spl_state = SplTokenAccount::unpack(&spl_account.data).unwrap();
    assert_eq!(spl_state.amount, 0);
}
