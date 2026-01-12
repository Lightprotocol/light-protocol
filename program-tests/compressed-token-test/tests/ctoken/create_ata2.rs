use super::shared::*;

async fn create_and_assert_ata2(
    context: &mut AccountTestContext,
    compressible_data: Option<CompressibleData>,
    idempotent: bool,
    name: &str,
) -> Pubkey {
    println!("ATA2 creation initiated for: {}", name);

    let payer_pubkey = context.payer.pubkey();
    let owner_pubkey = context.owner_keypair.pubkey();

    let (ata_pubkey, bump) = derive_token_ata(&owner_pubkey, &context.mint_pubkey);

    let create_ata_ix = if let Some(compressible) = compressible_data.as_ref() {
        let compressible_params = CompressibleParams {
            compressible_config: context.compressible_config,
            rent_sponsor: compressible.rent_sponsor,
            pre_pay_num_epochs: compressible.num_prepaid_epochs,
            lamports_per_write: compressible.lamports_per_write,
            compress_to_account_pubkey: None,
            token_account_version: compressible.account_version,
            compression_only: true,
        };

        let mut builder =
            CreateAssociatedTokenAccount::new(payer_pubkey, owner_pubkey, context.mint_pubkey)
                .with_compressible(compressible_params);

        if idempotent {
            builder = builder.idempotent();
        }

        builder.instruction().unwrap()
    } else {
        // Create non-compressible account
        let mut builder = CreateAssociatedTokenAccount {
            idempotent: false,
            bump,
            payer: payer_pubkey,
            owner: owner_pubkey,
            mint: context.mint_pubkey,
            associated_token_account: ata_pubkey,
            compressible: CompressibleParams::default(),
        };

        if idempotent {
            builder = builder.idempotent();
        }

        builder.instruction().unwrap()
    };

    context
        .rpc
        .create_and_send_transaction(&[create_ata_ix], &payer_pubkey, &[&context.payer])
        .await
        .unwrap();

    assert_create_associated_token_account(
        &mut context.rpc,
        owner_pubkey,
        context.mint_pubkey,
        compressible_data,
        None,
    )
    .await;

    ata_pubkey
}

#[tokio::test]
async fn test_create_ata2_basic() {
    let mut context = setup_account_test().await.unwrap();
    let payer_pubkey = context.payer.pubkey();

    {
        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor: context.rent_sponsor,
            num_prepaid_epochs: 2,
            lamports_per_write: Some(100),
            account_version: light_token_interface::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: payer_pubkey,
        };

        create_and_assert_ata2(
            &mut context,
            Some(compressible_data),
            false,
            "compressible_ata2",
        )
        .await;
    }

    {
        context.mint_pubkey = solana_sdk::pubkey::Pubkey::new_unique();
        // All accounts now have compression infrastructure, so pass CompressibleData
        // with 0 prepaid epochs (immediately compressible)
        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor: context.rent_sponsor,
            num_prepaid_epochs: 0,
            lamports_per_write: None,
            account_version: light_token_interface::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: payer_pubkey,
        };
        create_and_assert_ata2(
            &mut context,
            Some(compressible_data),
            false,
            "ata2_zero_epochs",
        )
        .await;
    }
}

#[tokio::test]
async fn test_create_ata2_idempotent() {
    let mut context = setup_account_test().await.unwrap();
    let payer_pubkey = context.payer.pubkey();

    let compressible_data = CompressibleData {
        compression_authority: context.compression_authority,
        rent_sponsor: context.rent_sponsor,
        num_prepaid_epochs: 2,
        lamports_per_write: Some(100),
        account_version: light_token_interface::state::TokenDataVersion::ShaFlat,
        compress_to_pubkey: false,
        payer: payer_pubkey,
    };

    let ata_pubkey = create_and_assert_ata2(
        &mut context,
        Some(compressible_data.clone()),
        true,
        "idempotent_first_creation",
    )
    .await;

    let ata_pubkey_second = create_and_assert_ata2(
        &mut context,
        Some(compressible_data),
        true,
        "idempotent_second_creation",
    )
    .await;

    assert_eq!(
        ata_pubkey, ata_pubkey_second,
        "Both idempotent creations should return the same ATA address"
    );

    let account = context.rpc.get_account(ata_pubkey).await.unwrap().unwrap();

    // Calculate expected size for account with Compressible extension
    use light_token_interface::state::{
        calculate_token_account_size, CompressibleExtensionConfig, CompressionInfoConfig,
        ExtensionStructConfig,
    };
    let expected_size = calculate_token_account_size(Some(&[ExtensionStructConfig::Compressible(
        CompressibleExtensionConfig {
            info: CompressionInfoConfig { rent_config: () },
        },
    )]))
    .unwrap();

    assert_eq!(
        account.data.len(),
        expected_size,
        "Account should still be compressible size after idempotent recreation"
    );
}
