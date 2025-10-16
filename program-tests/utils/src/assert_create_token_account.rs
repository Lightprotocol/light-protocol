use anchor_spl::token_2022::spl_token_2022;
use light_client::rpc::Rpc;
use light_compressed_token_sdk::instructions::create_associated_token_account::derive_ctoken_ata;
use light_compressible::rent::RentConfig;
use light_ctoken_types::{
    state::{ctoken::CToken, extensions::CompressionInfo, AccountState},
    BASE_TOKEN_ACCOUNT_SIZE, COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
};
use light_program_test::LightProgramTest;
use light_zero_copy::traits::ZeroCopyAt;
use solana_sdk::{program_pack::Pack, pubkey::Pubkey};

#[derive(Debug, Clone)]
pub struct CompressibleData {
    pub compression_authority: Pubkey,
    pub rent_sponsor: Pubkey,
    pub num_prepaid_epochs: u8,
    pub lamports_per_write: Option<u32>,
    pub compress_to_pubkey: bool,
    pub account_version: light_ctoken_types::state::TokenDataVersion,
    pub payer: Pubkey,
}

/// Assert that a token account was created correctly.
/// If compressible_data is provided, validates compressible token account with extensions.
/// If compressible_data is None, validates basic SPL token account.
/// If is_ata is true, expects 1 signer (payer only), otherwise expects 2 signers (token_account_keypair + payer).
/// Automatically detects idempotent mode by checking if account existed before transaction.
pub async fn assert_create_token_account_internal(
    rpc: &mut LightProgramTest,
    token_account_pubkey: Pubkey,
    mint_pubkey: Pubkey,
    owner_pubkey: Pubkey,
    compressible_data: Option<CompressibleData>,
    is_ata: bool,
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

            let rent_with_compression = RentConfig::default().get_rent_with_compression_cost(
                COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
                compressible_info.num_prepaid_epochs as u64,
            );
            let expected_lamports = rent_exemption + rent_with_compression;

            assert_eq!(
                account_info.lamports, expected_lamports,
                "Account should have rent-exempt balance ({}) plus prepaid rent with compression cost ({}) = {} lamports, but has {}",
                rent_exemption, rent_with_compression, expected_lamports, account_info.lamports
            );

            // Use zero-copy deserialization for compressible account
            let (actual_token_account, _) = CToken::zero_copy_at(&account_info.data)
                .expect("Failed to deserialize compressible token account with zero-copy");

            // Get current slot for validation (program sets this to current slot)
            let current_slot = rpc.get_slot().await.expect("Failed to get current slot");

            // Create expected compressible token account
            let expected_token_account = CToken {
                mint: mint_pubkey.into(),
                owner: owner_pubkey.into(),
                amount: 0,
                delegate: None,
                state: AccountState::Initialized, // Initialized
                is_native: None,
                delegated_amount: 0,
                close_authority: None,
                extensions: Some(vec![
                    light_ctoken_types::state::extensions::ExtensionStruct::Compressible(
                        CompressionInfo {
                            config_account_version: 1,
                            last_claimed_slot: current_slot,
                            rent_config: RentConfig::default(),
                            lamports_per_write: compressible_info.lamports_per_write.unwrap_or(0),
                            compression_authority: compressible_info
                                .compression_authority
                                .to_bytes(),
                            rent_sponsor: compressible_info.rent_sponsor.to_bytes(),
                            compress_to_pubkey: compressible_info.compress_to_pubkey as u8,
                            account_version: compressible_info.account_version as u8,
                        },
                    ),
                ]),
            };

            assert_eq!(actual_token_account, expected_token_account);

            // Check if account existed before transaction (for idempotent mode)
            let account_existed_before = rpc
                .get_pre_transaction_account(&token_account_pubkey)
                .is_some();

            // Assert payer and rent sponsor balance changes
            let payer_balance_before = rpc
                .get_pre_transaction_account(&compressible_info.payer)
                .expect("Payer should exist in pre-transaction context")
                .lamports;

            let payer_balance_after = rpc
                .get_account(compressible_info.payer)
                .await
                .expect("Failed to get payer account")
                .expect("Payer should exist")
                .lamports;

            let rent_sponsor_balance_before = rpc
                .get_pre_transaction_account(&compressible_info.rent_sponsor)
                .expect("Rent sponsor should exist in pre-transaction context")
                .lamports;

            let rent_sponsor_balance_after = rpc
                .get_account(compressible_info.rent_sponsor)
                .await
                .expect("Failed to get rent sponsor account")
                .expect("Rent sponsor should exist")
                .lamports;

            // Transaction fee: 5000 lamports per signature
            // For ATA: 1 signer (payer only) = 5,000 lamports
            // For regular token account: 2 signers (token_account_keypair + payer) = 10,000 lamports
            let tx_fee = if is_ata { 5_000 } else { 10_000 };

            // If account existed before (idempotent mode), only tx fee is charged
            if account_existed_before {
                // In idempotent mode, account already existed, so only tx fee is paid
                assert_eq!(
                    payer_balance_before - payer_balance_after,
                    tx_fee,
                    "In idempotent mode (account already existed), payer should only pay tx fee ({} lamports), but paid {}",
                    tx_fee,
                    payer_balance_before - payer_balance_after
                );
                return;
            }

            // Check if payer is the rent sponsor (custom fee payer case)
            if compressible_info.payer == compressible_info.rent_sponsor {
                // Case 2: Custom fee payer - payer pays everything (rent_exemption + rent_with_compression + tx_fee)
                assert_eq!(
                    payer_balance_before - payer_balance_after,
                    rent_exemption + rent_with_compression + tx_fee,
                    "Custom fee payer should have paid {} lamports (rent exemption) + {} lamports (rent with compression cost) + {} lamports (tx fee) = {} total, but paid {}",
                    rent_exemption,
                    rent_with_compression,
                    tx_fee,
                    rent_exemption + rent_with_compression + tx_fee,
                    payer_balance_before - payer_balance_after
                );
            } else {
                // Case 1: With rent sponsor - split payment
                // Payer pays: rent_with_compression + tx_fee
                assert_eq!(
                    payer_balance_before - payer_balance_after,
                    rent_with_compression + tx_fee,
                    "Payer should have paid {} lamports (rent with compression cost) + {} lamports (tx fee) = {} total, but paid {}",
                    rent_with_compression,
                    tx_fee,
                    rent_with_compression + tx_fee,
                    payer_balance_before - payer_balance_after
                );

                // Rent sponsor pays: rent_exemption only
                assert_eq!(
                    rent_sponsor_balance_before - rent_sponsor_balance_after,
                    rent_exemption,
                    "Rent sponsor should have paid {} lamports (rent exemption only), but paid {}",
                    rent_exemption,
                    rent_sponsor_balance_before - rent_sponsor_balance_after
                );
            }
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

/// Assert that a regular token account was created correctly.
/// Public wrapper for non-ATA token accounts (expects 2 signers).
pub async fn assert_create_token_account(
    rpc: &mut LightProgramTest,
    token_account_pubkey: Pubkey,
    mint_pubkey: Pubkey,
    owner_pubkey: Pubkey,
    compressible_data: Option<CompressibleData>,
) {
    assert_create_token_account_internal(
        rpc,
        token_account_pubkey,
        mint_pubkey,
        owner_pubkey,
        compressible_data,
        false, // Not an ATA
    )
    .await;
}

/// Assert that an associated token account was created correctly.
/// Automatically derives the ATA address from owner and mint.
/// If compressible_data is provided, validates compressible ATA with extensions.
/// If compressible_data is None, validates basic SPL ATA.
pub async fn assert_create_associated_token_account(
    rpc: &mut LightProgramTest,
    owner_pubkey: Pubkey,
    mint_pubkey: Pubkey,
    compressible_data: Option<CompressibleData>,
) {
    // Derive the associated token account address
    let (ata_pubkey, _bump) = derive_ctoken_ata(&owner_pubkey, &mint_pubkey);

    // Verify the account exists at the derived address
    let account = rpc
        .get_account(ata_pubkey)
        .await
        .expect("Failed to get ATA account");

    assert!(
        account.is_some(),
        "ATA should exist at derived address {} for owner {} and mint {}",
        ata_pubkey,
        owner_pubkey,
        mint_pubkey
    );

    // Use the internal assertion function with is_ata=true (expects 1 signer)
    assert_create_token_account_internal(
        rpc,
        ata_pubkey,
        mint_pubkey,
        owner_pubkey,
        compressible_data,
        true, // Is an ATA
    )
    .await;
}
