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

    let (ata_pubkey, _bump) = derive_ctoken_ata(&owner_pubkey, &context.mint_pubkey);

    let create_ata_ix = if let Some(compressible) = compressible_data.as_ref() {
        let create_fn = if idempotent {
            light_compressed_token_sdk::instructions::create_compressible_associated_token_account2_idempotent
        } else {
            light_compressed_token_sdk::instructions::create_compressible_associated_token_account2
        };

        create_fn(
            light_compressed_token_sdk::instructions::CreateCompressibleAssociatedTokenAccountInputs {
                payer: payer_pubkey,
                owner: owner_pubkey,
                mint: context.mint_pubkey,
                compressible_config: context.compressible_config,
                rent_sponsor: compressible.rent_sponsor,
                pre_pay_num_epochs: compressible.num_prepaid_epochs,
                lamports_per_write: compressible.lamports_per_write,
                token_account_version: compressible.account_version,
            },
        )
        .unwrap()
    } else {
        let create_fn = if idempotent {
            light_compressed_token_sdk::instructions::create_associated_token_account2_idempotent
        } else {
            light_compressed_token_sdk::instructions::create_associated_token_account2
        };

        create_fn(payer_pubkey, owner_pubkey, context.mint_pubkey).unwrap()
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
            account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
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

        create_and_assert_ata2(&mut context, None, false, "non_compressible_ata2").await;
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
        account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
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

    assert_eq!(
        account.data.len(),
        light_ctoken_types::COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize,
        "Account should still be compressible size after idempotent recreation"
    );
}
