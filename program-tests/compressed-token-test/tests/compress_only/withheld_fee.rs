//! Tests for withheld_transfer_fee preservation through compress/decompress cycle.
//!
//! This module tests:
//! - Withheld transfer fee preservation (spec #27)

use borsh::BorshDeserialize;
use light_client::indexer::Indexer;
use light_program_test::{program_test::TestRpc, LightProgramTest, ProgramTestConfig};
use light_test_utils::{
    mint_2022::{create_mint_22_with_extension_types, create_token_22_account, mint_spl_tokens_22},
    Rpc, RpcError,
};
use light_token_client::instructions::transfer2::{
    create_generic_transfer2_instruction, DecompressInput, Transfer2InstructionType,
};
use light_token_interface::{
    instructions::extensions::{CompressedOnlyExtensionInstructionData, ExtensionInstructionData},
    state::{
        CompressedOnlyExtension, CompressedTokenAccountState, ExtensionStruct, Token, TokenData,
        TokenDataVersion,
    },
};
use light_token_sdk::{
    spl_interface::find_spl_interface_pda_with_index,
    token::{CompressibleParams, CreateTokenAccount, TransferFromSpl},
};
use serial_test::serial;
use solana_sdk::{signature::Keypair, signer::Signer};
use spl_token_2022::extension::ExtensionType;

use super::shared::set_ctoken_withheld_fee;

/// Test that withheld_transfer_fee is preserved through compress -> decompress cycle.
///
/// Covers spec requirement #27: Full round-trip withheld_transfer_fee preserved
#[tokio::test]
#[serial]
async fn test_roundtrip_withheld_transfer_fee_preserved() -> Result<(), RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None)).await?;
    let payer = rpc.get_payer().insecure_clone();

    // 1. Create mint with TransferFeeConfig extension
    let extensions = &[ExtensionType::TransferFeeConfig];
    let (mint_keypair, _extension_config) =
        create_mint_22_with_extension_types(&mut rpc, &payer, 9, extensions).await;
    let mint_pubkey = mint_keypair.pubkey();

    // 2. Create SPL Token-2022 account and mint tokens
    let spl_account =
        create_token_22_account(&mut rpc, &payer, &mint_pubkey, &payer.pubkey()).await;
    let mint_amount = 1_000_000_000u64;
    mint_spl_tokens_22(&mut rpc, &payer, &mint_pubkey, &spl_account, mint_amount).await;

    // 3. Create Light Token account with compression_only
    let owner = Keypair::new();
    let account_keypair = Keypair::new();
    let ctoken_account = account_keypair.pubkey();

    let create_ix =
        CreateTokenAccount::new(payer.pubkey(), ctoken_account, mint_pubkey, owner.pubkey())
            .with_compressible(CompressibleParams {
                compressible_config: rpc
                    .test_accounts
                    .funding_pool_config
                    .compressible_config_pda,
                rent_sponsor: rpc.test_accounts.funding_pool_config.rent_sponsor_pda,
                pre_pay_num_epochs: 0,
                lamports_per_write: Some(100),
                compress_to_account_pubkey: None,
                token_account_version: TokenDataVersion::ShaFlat,
                compression_only: true,
            })
            .instruction()
            .map_err(|e| RpcError::CustomError(format!("Failed to create instruction: {:?}", e)))?;

    rpc.create_and_send_transaction(&[create_ix], &payer.pubkey(), &[&payer, &account_keypair])
        .await?;

    // 4. Transfer tokens to Light Token
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0, true); // true = restricted
    let transfer_ix = TransferFromSpl {
        amount: mint_amount,
        spl_interface_pda_bump,
        decimals: 9,
        source_spl_token_account: spl_account,
        destination_ctoken_account: ctoken_account,
        authority: payer.pubkey(),
        mint: mint_pubkey,
        payer: payer.pubkey(),
        spl_interface_pda,
        spl_token_program: spl_token_2022::ID,
    }
    .instruction()
    .map_err(|e| {
        RpcError::CustomError(format!("Failed to create transfer instruction: {:?}", e))
    })?;

    rpc.create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &[&payer])
        .await?;

    // 5. Set withheld_amount to a non-zero value BEFORE compression
    let withheld_amount = 12345u64;
    set_ctoken_withheld_fee(&mut rpc, ctoken_account, withheld_amount).await?;

    // Verify the withheld_amount was set correctly
    let account_before = rpc.get_account(ctoken_account).await?.unwrap();
    let ctoken_before = Token::deserialize(&mut &account_before.data[..]).map_err(|e| {
        RpcError::CustomError(format!("Failed to deserialize Light Token: {:?}", e))
    })?;

    let withheld_before = ctoken_before
        .extensions
        .as_ref()
        .and_then(|exts| {
            exts.iter().find_map(|e| match e {
                ExtensionStruct::TransferFeeAccount(fee) => Some(fee.withheld_amount),
                _ => None,
            })
        })
        .unwrap_or(0);

    assert_eq!(
        withheld_before, withheld_amount,
        "Withheld amount should be set before compression"
    );

    // 6. Warp to trigger forester compression
    rpc.warp_epoch_forward(30).await?;

    // 7. Verify account was compressed
    let account_after = rpc.get_account(ctoken_account).await?;
    assert!(
        account_after.is_none() || account_after.unwrap().lamports == 0,
        "Light Token account should be closed after compression"
    );

    // 8. Get compressed account and verify withheld_transfer_fee in CompressedOnly extension
    let compressed_accounts = rpc
        .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
        .await?
        .value
        .items;

    assert_eq!(
        compressed_accounts.len(),
        1,
        "Should have exactly 1 compressed token account"
    );

    // Build expected TokenData with withheld_transfer_fee
    let expected_token_data = TokenData {
        mint: mint_pubkey.into(),
        owner: owner.pubkey().into(),
        amount: mint_amount,
        delegate: None,
        state: CompressedTokenAccountState::Initialized as u8,
        tlv: Some(vec![ExtensionStruct::CompressedOnly(
            CompressedOnlyExtension {
                delegated_amount: 0,
                withheld_transfer_fee: withheld_amount,
                is_ata: 0, // Non-ATA regular account
            },
        )]),
    };

    assert_eq!(
        compressed_accounts[0].token,
        expected_token_data.into(),
        "Compressed token should have withheld_transfer_fee preserved"
    );

    // 9. Create destination Light Token for decompress
    let decompress_dest_keypair = Keypair::new();
    let decompress_dest_account = decompress_dest_keypair.pubkey();

    let create_dest_ix = CreateTokenAccount::new(
        payer.pubkey(),
        decompress_dest_account,
        mint_pubkey,
        owner.pubkey(),
    )
    .with_compressible(CompressibleParams {
        compressible_config: rpc
            .test_accounts
            .funding_pool_config
            .compressible_config_pda,
        rent_sponsor: rpc.test_accounts.funding_pool_config.rent_sponsor_pda,
        pre_pay_num_epochs: 2,
        lamports_per_write: Some(100),
        compress_to_account_pubkey: None,
        token_account_version: TokenDataVersion::ShaFlat,
        compression_only: true,
    })
    .instruction()
    .map_err(|e| RpcError::CustomError(format!("Failed to create dest instruction: {:?}", e)))?;

    rpc.create_and_send_transaction(
        &[create_dest_ix],
        &payer.pubkey(),
        &[&payer, &decompress_dest_keypair],
    )
    .await?;

    // 10. Decompress with withheld_transfer_fee in in_tlv
    let in_tlv = vec![vec![ExtensionInstructionData::CompressedOnly(
        CompressedOnlyExtensionInstructionData {
            delegated_amount: 0,
            withheld_transfer_fee: withheld_amount,
            is_frozen: false,
            compression_index: 0,
            is_ata: false, // Non-ATA regular account
            bump: 0,
            owner_index: 0,
        },
    )]];

    let decompress_ix = create_generic_transfer2_instruction(
        &mut rpc,
        vec![Transfer2InstructionType::Decompress(DecompressInput {
            compressed_token_account: vec![compressed_accounts[0].clone()],
            decompress_amount: mint_amount,
            solana_token_account: decompress_dest_account,
            amount: mint_amount,
            pool_index: None,
            decimals: 9,
            in_tlv: Some(in_tlv),
        })],
        payer.pubkey(),
        true,
    )
    .await
    .map_err(|e| {
        RpcError::CustomError(format!("Failed to create decompress instruction: {:?}", e))
    })?;

    rpc.create_and_send_transaction(&[decompress_ix], &payer.pubkey(), &[&payer, &owner])
        .await?;

    // 11. Verify decompressed Light Token has withheld_amount restored
    let dest_account_data = rpc
        .get_account(decompress_dest_account)
        .await?
        .ok_or_else(|| RpcError::CustomError("Dest account not found".to_string()))?;

    let dest_ctoken = Token::deserialize(&mut &dest_account_data.data[..]).map_err(|e| {
        RpcError::CustomError(format!("Failed to deserialize Light Token: {:?}", e))
    })?;

    let withheld_after = dest_ctoken
        .extensions
        .as_ref()
        .and_then(|exts| {
            exts.iter().find_map(|e| match e {
                ExtensionStruct::TransferFeeAccount(fee) => Some(fee.withheld_amount),
                _ => None,
            })
        })
        .ok_or_else(|| {
            RpcError::CustomError("TransferFeeAccount extension not found".to_string())
        })?;

    assert_eq!(
        withheld_after, withheld_amount,
        "Withheld amount should be restored after decompress"
    );

    println!(
        "Successfully verified withheld_transfer_fee {} preserved through compress/decompress",
        withheld_amount
    );

    Ok(())
}
