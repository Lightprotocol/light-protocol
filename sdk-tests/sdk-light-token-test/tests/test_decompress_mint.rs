// Tests for DecompressMint SDK instruction
// Flow: Create compressed-only mint -> DecompressMint

mod shared;

use borsh::BorshDeserialize;
use light_client::{indexer::Indexer, rpc::Rpc};
use light_compressible::compression_info::CompressionInfo;
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_test_utils::actions::legacy::instructions::mint_action::{
    create_mint_action_instruction, MintActionParams, MintActionType,
};
use light_token::instruction::derive_mint_compressed_address;
use light_token_interface::state::Mint;
use solana_sdk::signer::Signer;

/// Test decompressing a compressed-only mint
///
/// Flow:
/// 1. Create a compressed-only mint (using setup_create_compressed_only_mint)
/// 2. Decompress it using DecompressMint
#[tokio::test]
async fn test_decompress_compressed_only_mint() {
    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let mint_authority = payer.pubkey();
    let decimals = 9u8;

    // Step 1: Create a compressed-only mint (no decompression)
    let (mint_pda, compression_address, mint_seed) =
        shared::setup_create_compressed_only_mint(&mut rpc, &payer, mint_authority, decimals).await;

    // Verify mint does NOT exist on-chain (compressed-only)
    let mint_account = rpc.get_account(mint_pda).await.unwrap();
    assert!(
        mint_account.is_none(),
        "Mint should NOT exist after creating compressed-only mint"
    );

    // Verify compressed mint exists
    let compressed_account = rpc
        .get_compressed_account(compression_address, None)
        .await
        .unwrap()
        .value;
    assert!(
        compressed_account.is_some(),
        "Compressed mint should exist after creation"
    );

    let address_tree = rpc.get_address_tree_v2();
    let compressed_mint_address =
        derive_mint_compressed_address(&mint_seed.pubkey(), &address_tree.tree);

    // Step 2: Decompress the mint
    let decompress_ix = create_mint_action_instruction(
        &mut rpc,
        MintActionParams {
            compressed_mint_address,
            mint_seed: mint_seed.pubkey(),
            authority: mint_authority,
            payer: payer.pubkey(),
            actions: vec![MintActionType::DecompressMint {
                rent_payment: 16,
                write_top_up: 766,
            }],
            new_mint: None,
        },
    )
    .await
    .unwrap();

    rpc.create_and_send_transaction(&[decompress_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify mint account now exists
    let mint_account_after_decompress = rpc.get_account(mint_pda).await.unwrap();
    assert!(
        mint_account_after_decompress.is_some(),
        "Mint should exist after DecompressMint"
    );

    // Verify mint state
    let mint_data = mint_account_after_decompress.unwrap();
    let mint = Mint::deserialize(&mut &mint_data.data[..]).unwrap();

    // Verify basic mint properties
    assert_eq!(mint.base.decimals, decimals, "Decimals should match");
    assert_eq!(
        mint.base.mint_authority,
        Some(mint_authority.to_bytes().into()),
        "Mint authority should match"
    );

    // Verify mint_decompressed flag is set
    assert!(
        mint.metadata.mint_decompressed,
        "Mint should be marked as decompressed"
    );

    // Verify compression info is set (non-default)
    assert_ne!(
        mint.compression,
        CompressionInfo::default(),
        "Mint compression info should be set when decompressed"
    );
}

/// Test that CreateMint automatically decompresses the mint
#[tokio::test]
async fn test_create_mint_auto_decompresses() {
    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let mint_authority = payer.pubkey();
    let decimals = 9u8;

    // Create a compressed mint (now auto-decompresses)
    let (mint_pda, compression_address, _, _mint_seed) =
        shared::setup_create_mint(&mut rpc, &payer, mint_authority, decimals, vec![]).await;

    // Verify Mint account exists on-chain (auto-decompressed)
    let mint_account = rpc.get_account(mint_pda).await.unwrap();
    assert!(
        mint_account.is_some(),
        "Mint should exist after CreateMint (auto-decompress)"
    );

    // Verify Mint state
    let mint_data = mint_account.unwrap();
    let mint = Mint::deserialize(&mut &mint_data.data[..]).unwrap();

    // Verify basic mint properties
    assert_eq!(mint.base.decimals, decimals, "Decimals should match");
    assert_eq!(
        mint.base.mint_authority,
        Some(mint_authority.to_bytes().into()),
        "Mint authority should match"
    );
    assert_eq!(mint.base.supply, 0, "Initial supply should be 0");

    // Verify mint_decompressed flag is set
    assert!(
        mint.metadata.mint_decompressed,
        "Mint should be marked as decompressed"
    );

    // Verify compression info is set (non-default)
    assert_ne!(
        mint.compression,
        CompressionInfo::default(),
        "Mint compression info should be set when decompressed"
    );

    // Verify compressed mint still exists (both forms coexist)
    let compressed_account = rpc
        .get_compressed_account(compression_address, None)
        .await
        .unwrap()
        .value;
    assert!(
        compressed_account.is_some(),
        "Compressed mint should still exist after decompression"
    );
}

/// Test CreateMint with freeze authority auto-decompresses
#[tokio::test]
async fn test_create_mint_with_freeze_authority_auto_decompresses() {
    let config = ProgramTestConfig::new_v2(true, None);
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let mint_authority = payer.pubkey();
    let freeze_authority = Some(payer.pubkey());
    let decimals = 6u8;

    // Create a compressed mint with freeze_authority (now auto-decompresses)
    let (mint_pda, _compression_address, _atas) = shared::setup_create_mint_with_freeze_authority(
        &mut rpc,
        &payer,
        mint_authority,
        freeze_authority,
        decimals,
        vec![],
    )
    .await;

    // Verify Mint account exists on-chain (auto-decompressed)
    let mint_account = rpc.get_account(mint_pda).await.unwrap();
    assert!(
        mint_account.is_some(),
        "Mint should exist after CreateMint (auto-decompress)"
    );

    // Verify Mint state
    let mint_data = mint_account.unwrap();
    let mint = Mint::deserialize(&mut &mint_data.data[..]).unwrap();

    // Verify freeze authority is set
    assert_eq!(
        mint.base.freeze_authority,
        freeze_authority.map(|p| p.to_bytes().into()),
        "Freeze authority should match"
    );

    // Verify mint_decompressed flag is set
    assert!(
        mint.metadata.mint_decompressed,
        "Mint should be marked as decompressed"
    );
}
