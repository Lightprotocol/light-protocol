use anchor_spl::token_2022::spl_token_2022;
use light_client::rpc::Rpc;
use light_compressed_token_sdk::instructions::create_associated_token_account::derive_ctoken_ata;
use light_compressible::rent::{
    get_rent_with_compression_cost, RentConfig, COMPRESSION_COST, COMPRESSION_INCENTIVE, MIN_RENT,
    RENT_PER_BYTE,
};
use light_ctoken_types::{
    state::{extensions::CompressibleExtension, solana_ctoken::CompressedToken},
    BASE_TOKEN_ACCOUNT_SIZE, COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
};
use light_zero_copy::traits::ZeroCopyAt;
use solana_sdk::{program_pack::Pack, pubkey::Pubkey};

#[derive(Debug, Clone)]
pub struct CompressibleData {
    pub rent_authority: Pubkey,
    pub rent_recipient: Pubkey,
    pub num_prepaid_epochs: u64,
    pub write_top_up_lamports: Option<u32>,
}

/// Assert that a token account was created correctly.
/// If compressible_data is provided, validates compressible token account with extensions.
/// If compressible_data is None, validates basic SPL token account.
pub async fn assert_create_token_account<R: Rpc>(
    rpc: &mut R,
    token_account_pubkey: Pubkey,
    mint_pubkey: Pubkey,
    owner_pubkey: Pubkey,
    compressible_data: Option<CompressibleData>,
) {
    // Get the token account data
    let account_info = rpc
        .get_account(token_account_pubkey)
        .await
        .expect("Failed to get token account")
        .expect("Token account should exist");

    // Verify basic account properties
    assert_eq!(account_info.owner, light_compressed_token::ID);
    assert!(account_info.lamports > 0);
    assert!(!account_info.executable);

    match compressible_data {
        Some(compressible_info) => {
            // Validate compressible token account
            assert_eq!(
                account_info.data.len(),
                COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize
            );

            // Calculate expected lamports balance
            let rent_exemption = rpc
                .get_minimum_balance_for_rent_exemption(COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize)
                .await
                .expect("Failed to get rent exemption");

            let rent_with_compression = get_rent_with_compression_cost(
                MIN_RENT as u64,
                RENT_PER_BYTE as u64,
                COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
                compressible_info.num_prepaid_epochs,
                (COMPRESSION_COST + COMPRESSION_INCENTIVE) as u64,
            );
            let expected_lamports = rent_exemption + rent_with_compression;

            assert_eq!(
                account_info.lamports, expected_lamports,
                "Account should have rent-exempt balance ({}) plus prepaid rent with compression cost ({}) = {} lamports, but has {}",
                rent_exemption, rent_with_compression, expected_lamports, account_info.lamports
            );

            // Use zero-copy deserialization for compressible account
            let (actual_token_account, _) = CompressedToken::zero_copy_at(&account_info.data)
                .expect("Failed to deserialize compressible token account with zero-copy");

            // Get current slot for validation (program sets this to current slot)
            let current_slot = rpc.get_slot().await.expect("Failed to get current slot");

            // Create expected compressible token account
            let expected_token_account = CompressedToken {
                mint: mint_pubkey.into(),
                owner: owner_pubkey.into(),
                amount: 0,
                delegate: None,
                state: 1, // Initialized
                is_native: None,
                delegated_amount: 0,
                close_authority: None,
                extensions: Some(vec![
                    light_ctoken_types::state::extensions::ExtensionStruct::Compressible(
                        CompressibleExtension {
                            version: 1,
                            last_claimed_slot: current_slot,
                            rent_config: RentConfig::default(),
                            write_top_up_lamports: compressible_info
                                .write_top_up_lamports
                                .unwrap_or(0),
                            rent_authority: compressible_info.rent_authority.to_bytes(),
                            rent_recipient: compressible_info.rent_recipient.to_bytes(),
                        },
                    ),
                ]),
            };

            assert_eq!(actual_token_account, expected_token_account);
        }
        None => {
            // Validate basic SPL token account
            assert_eq!(account_info.data.len(), 165); // SPL token account size

            // Use SPL token Pack trait for basic account
            let actual_spl_token_account =
                spl_token_2022::state::Account::unpack(&account_info.data)
                    .expect("Failed to unpack basic token account data");

            // Create expected SPL token account
            let expected_spl_token_account = spl_token_2022::state::Account {
                mint: mint_pubkey,
                owner: owner_pubkey,
                amount: 0,
                delegate: actual_spl_token_account.delegate, // Copy the actual COption value
                state: spl_token_2022::state::AccountState::Initialized,
                is_native: actual_spl_token_account.is_native, // Copy the actual COption value
                delegated_amount: 0,
                close_authority: actual_spl_token_account.close_authority, // Copy the actual COption value
            };

            assert_eq!(actual_spl_token_account, expected_spl_token_account);
            assert_eq!(account_info.data.len(), BASE_TOKEN_ACCOUNT_SIZE as usize);

            // Calculate expected lamports balance
            let rent_exemption = rpc
                .get_minimum_balance_for_rent_exemption(BASE_TOKEN_ACCOUNT_SIZE as usize)
                .await
                .expect("Failed to get rent exemption");
            assert_eq!(
                account_info.lamports, rent_exemption,
                "Account should have rent-exempt balance ({}) lamports, but has {}",
                rent_exemption, account_info.lamports
            );
        }
    }
}

/// Assert that an associated token account was created correctly.
/// Automatically derives the ATA address from owner and mint.
/// If compressible_data is provided, validates compressible ATA with extensions.
/// If compressible_data is None, validates basic SPL ATA.
pub async fn assert_create_associated_token_account<R: Rpc>(
    rpc: &mut R,
    owner_pubkey: Pubkey,
    mint_pubkey: Pubkey,
    compressible_data: Option<CompressibleData>,
) {
    // Derive the associated token account address
    let (ata_pubkey, _bump) = derive_ctoken_ata(&owner_pubkey, &mint_pubkey);

    // Use the main assertion function
    assert_create_token_account(
        rpc,
        ata_pubkey,
        mint_pubkey,
        owner_pubkey,
        compressible_data,
    )
    .await;
}
