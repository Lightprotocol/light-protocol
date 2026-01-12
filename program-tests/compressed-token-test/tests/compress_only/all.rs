//! Tests for compress and close with all Token-2022 extensions.
//!
//! This module tests the full compress -> decompress cycle with all extensions enabled.

use borsh::BorshDeserialize;
use light_program_test::program_test::TestRpc;
use light_token_interface::state::{
    AccountState, ExtensionStruct, Token, ACCOUNT_TYPE_TOKEN_ACCOUNT,
};
use serial_test::serial;
use solana_sdk::{signature::Keypair, signer::Signer};

use super::shared::{setup_extensions_test, Rpc, ALL_EXTENSIONS};

/// Test that forester can compress and close a CToken account with Token-2022 extensions
/// after prepaid epochs expire, and then decompress it back to a CToken account.
#[tokio::test]
#[serial]
async fn test_compress_and_close_ctoken_with_extensions() {
    #[allow(unused_imports)]
    use light_client::indexer::CompressedTokenAccount;
    use light_client::indexer::Indexer;
    use light_test_utils::mint_2022::{create_token_22_account, mint_spl_tokens_22};
    use light_token_client::instructions::transfer2::{
        create_generic_transfer2_instruction, DecompressInput, Transfer2InstructionType,
    };
    use light_token_interface::{
        instructions::extensions::{
            CompressedOnlyExtensionInstructionData, ExtensionInstructionData,
        },
        state::TokenDataVersion,
    };
    use light_token_sdk::{
        ctoken::{CompressibleParams, CreateTokenAccount, TransferSplToToken},
        spl_interface::find_spl_interface_pda_with_index,
    };

    let mut context = setup_extensions_test(ALL_EXTENSIONS).await.unwrap();
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;

    // 1. Create SPL Token-2022 account and mint tokens
    let spl_account =
        create_token_22_account(&mut context.rpc, &payer, &mint_pubkey, &payer.pubkey()).await;
    let mint_amount = 1_000_000_000u64;
    mint_spl_tokens_22(
        &mut context.rpc,
        &payer,
        &mint_pubkey,
        &spl_account,
        mint_amount,
    )
    .await;

    // 2. Create CToken account with 0 prepaid epochs (immediately compressible)
    let owner = Keypair::new();
    let account_keypair = Keypair::new();
    let ctoken_account = account_keypair.pubkey();

    let create_ix =
        CreateTokenAccount::new(payer.pubkey(), ctoken_account, mint_pubkey, owner.pubkey())
            .with_compressible(CompressibleParams {
                compressible_config: context
                    .rpc
                    .test_accounts
                    .funding_pool_config
                    .compressible_config_pda,
                rent_sponsor: context
                    .rpc
                    .test_accounts
                    .funding_pool_config
                    .rent_sponsor_pda,
                pre_pay_num_epochs: 0, // Immediately compressible after 1 epoch
                lamports_per_write: Some(100),
                compress_to_account_pubkey: None,
                token_account_version: TokenDataVersion::ShaFlat,
                compression_only: true,
            })
            .instruction()
            .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[create_ix], &payer.pubkey(), &[&payer, &account_keypair])
        .await
        .unwrap();

    // 3. Transfer tokens to CToken using hot path (required for mints with restricted extensions)
    let (spl_interface_pda, spl_interface_pda_bump) =
        find_spl_interface_pda_with_index(&mint_pubkey, 0, true);
    let transfer_ix = TransferSplToToken {
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
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    // Verify tokens are in the CToken account
    let account_before = context
        .rpc
        .get_account(ctoken_account)
        .await
        .unwrap()
        .unwrap();
    assert!(
        account_before.lamports > 0,
        "Account should exist before compression"
    );

    // 4. Advance 2 epochs to trigger forester compression
    // Account created with 0 prepaid epochs needs time to become compressible
    context.rpc.warp_epoch_forward(30).await.unwrap();

    // 5. Assert the account has been compressed (closed) and compressed token account exists
    let account_after = context.rpc.get_account(ctoken_account).await.unwrap();
    assert!(
        account_after.is_none() || account_after.unwrap().lamports == 0,
        "CToken account should be closed"
    );

    let compressed_accounts = context
        .rpc
        .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(
        compressed_accounts.len(),
        1,
        "Should have exactly 1 compressed token account"
    );

    // Build expected TokenData with CompressedOnly extension
    // The CToken had marker extensions (PausableAccount, PermanentDelegateAccount),
    // so the compressed token should have CompressedOnly TLV extension
    use light_token_interface::state::{
        CompressedOnlyExtension, CompressedTokenAccountState, TokenData,
    };

    let expected_token_data = TokenData {
        mint: mint_pubkey.into(),
        owner: owner.pubkey().into(),
        amount: mint_amount,
        delegate: None,
        state: CompressedTokenAccountState::Initialized as u8,
        tlv: Some(vec![ExtensionStruct::CompressedOnly(
            CompressedOnlyExtension {
                delegated_amount: 0,
                withheld_transfer_fee: 0,
                is_ata: 0, // Non-ATA regular account
            },
        )]),
    };

    assert_eq!(
        compressed_accounts[0].token,
        expected_token_data.into(),
        "Compressed token account should match expected TokenData"
    );

    // 6. Create a new CToken account for decompress destination
    let decompress_dest_keypair = Keypair::new();
    let decompress_dest_account = decompress_dest_keypair.pubkey();

    let create_dest_ix = CreateTokenAccount::new(
        payer.pubkey(),
        decompress_dest_account,
        mint_pubkey,
        owner.pubkey(),
    )
    .with_compressible(CompressibleParams {
        compressible_config: context
            .rpc
            .test_accounts
            .funding_pool_config
            .compressible_config_pda,
        rent_sponsor: context
            .rpc
            .test_accounts
            .funding_pool_config
            .rent_sponsor_pda,
        pre_pay_num_epochs: 2, // More epochs so account won't be compressed again
        lamports_per_write: Some(100),
        compress_to_account_pubkey: None,
        token_account_version: TokenDataVersion::ShaFlat,
        compression_only: true,
    })
    .instruction()
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(
            &[create_dest_ix],
            &payer.pubkey(),
            &[&payer, &decompress_dest_keypair],
        )
        .await
        .unwrap();

    println!(
        "Created decompress destination CToken account: {}",
        decompress_dest_account
    );

    // 7. Decompress the compressed account back to the new CToken account
    // Need to include in_tlv for the CompressedOnly extension
    let in_tlv = vec![vec![ExtensionInstructionData::CompressedOnly(
        CompressedOnlyExtensionInstructionData {
            delegated_amount: 0,
            withheld_transfer_fee: 0,
            is_frozen: false,
            compression_index: 0,
            is_ata: false, // Non-ATA regular account
            bump: 0,
            owner_index: 0,
        },
    )]];

    let decompress_ix = create_generic_transfer2_instruction(
        &mut context.rpc,
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
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[decompress_ix], &payer.pubkey(), &[&payer, &owner])
        .await
        .unwrap();

    // 8. Verify the CToken account has the tokens and proper extension state

    let dest_account_data = context
        .rpc
        .get_account(decompress_dest_account)
        .await
        .unwrap()
        .unwrap();

    let dest_ctoken = Token::deserialize(&mut &dest_account_data.data[..])
        .expect("Failed to deserialize destination CToken account");

    // Build expected CToken account
    // Compression fields are now in the Compressible extension
    let expected_dest_token = Token {
        mint: mint_pubkey.to_bytes().into(),
        owner: owner.pubkey().to_bytes().into(),
        amount: mint_amount,
        delegate: None,
        state: AccountState::Initialized,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
        // Extensions include Compressible + marker extensions from mint
        extensions: dest_ctoken.extensions.clone(),
    };

    assert_eq!(
        dest_ctoken, expected_dest_token,
        "Decompressed CToken account should match expected with all extensions"
    );

    // Verify no more compressed accounts for this owner
    let remaining_compressed = context
        .rpc
        .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(
        remaining_compressed.len(),
        0,
        "Should have no more compressed token accounts after full decompress"
    );

    println!(
        "Successfully completed compress-and-close -> decompress cycle with extension state transfer"
    );
}
