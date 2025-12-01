//! Tests for load_ata and get_ata_interface

mod shared;

use light_client::rpc::Rpc;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_token_client::actions::transfer2::{get_ata_interface, load_ata, load_ata_instructions};
use shared::*;
use solana_sdk::signer::Signer;

#[tokio::test]
async fn test_get_ata_interface_with_compressed() {
    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let owner = payer.pubkey();

    let (mint, _compression_address, _ata_pubkeys) =
        setup_create_compressed_mint(&mut rpc, &payer, owner, 9, vec![(1000, owner)]).await;

    let ata_interface = get_ata_interface(&mut rpc, owner, mint).await.unwrap();

    assert_eq!(ata_interface.total_amount, 1000);
    assert!(ata_interface.is_cold);
    assert!(ata_interface.has_cold());
    assert_eq!(ata_interface.cold_balance(), 1000);
}

#[tokio::test]
async fn test_load_ata_instructions_cold() {
    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let owner = payer.pubkey();

    let (mint, _compression_address, ata_pubkeys) =
        setup_create_compressed_mint(&mut rpc, &payer, owner, 9, vec![(1000, owner)]).await;

    let ctoken_ata = ata_pubkeys[0];

    let instructions = load_ata_instructions(&mut rpc, payer.pubkey(), ctoken_ata, owner, mint)
        .await
        .unwrap();

    assert!(!instructions.is_empty());
}

#[tokio::test]
async fn test_load_ata_cold_to_hot() {
    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let owner = payer.pubkey();

    let (mint, _compression_address, ata_pubkeys) =
        setup_create_compressed_mint(&mut rpc, &payer, owner, 9, vec![(1000, owner)]).await;

    let ctoken_ata = ata_pubkeys[0];

    let before = get_ata_interface(&mut rpc, owner, mint).await.unwrap();
    assert!(before.is_cold);

    let sig = load_ata(&mut rpc, &payer, ctoken_ata, &payer, mint)
        .await
        .unwrap();
    assert!(sig.is_some());

    let after = get_ata_interface(&mut rpc, owner, mint).await.unwrap();
    assert_eq!(after.cold_balance(), 0);
}

#[tokio::test]
async fn test_load_ata_nothing_to_load() {
    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let owner = payer.pubkey();

    let (mint, _compression_address, ata_pubkeys) =
        setup_create_compressed_mint(&mut rpc, &payer, owner, 9, vec![(1000, owner)]).await;

    let ctoken_ata = ata_pubkeys[0];

    load_ata(&mut rpc, &payer, ctoken_ata, &payer, mint)
        .await
        .unwrap();

    let result = load_ata(&mut rpc, &payer, ctoken_ata, &payer, mint)
        .await
        .unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_ata_interface_helpers() {
    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();
    let owner = payer.pubkey();

    let (mint, _, _) =
        setup_create_compressed_mint(&mut rpc, &payer, owner, 9, vec![(500, owner)]).await;

    let ata = get_ata_interface(&mut rpc, owner, mint).await.unwrap();

    assert_eq!(ata.owner, owner);
    assert_eq!(ata.mint, mint);
    assert!(ata.has_cold());
    assert!(!ata.has_spl());
    assert!(!ata.has_t22());
    assert_eq!(ata.cold_balance(), 500);
    assert_eq!(ata.hot_balance(), 0);
}
