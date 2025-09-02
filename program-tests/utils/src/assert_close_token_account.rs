use light_client::rpc::Rpc;
use light_ctoken_types::state::solana_ctoken::CompressedToken;
use light_zero_copy::traits::ZeroCopyAt;
use solana_sdk::pubkey::Pubkey;

/// Assert that a token account was closed correctly.
/// Verifies that the account has 0 lamports, cleared data, and lamports were transferred correctly.
/// If account_data_before_close is provided, validates compressible account closure.
pub async fn assert_close_token_account<R: Rpc>(
    rpc: &mut R,
    token_account_pubkey: Pubkey,
    account_data_before_close: Option<&[u8]>,
    destination_pubkey: Pubkey,
    initial_destination_lamports: u64,
) {
    // Verify the account was closed (data should be cleared, lamports should be 0)
    let closed_account = rpc
        .get_account(token_account_pubkey)
        .await
        .expect("Failed to get closed token account");

    if let Some(account) = closed_account {
        // Account still exists, but should have 0 lamports and cleared data
        assert_eq!(account.lamports, 0, "Closed account should have 0 lamports");
        assert!(
            account.data.iter().all(|&b| b == 0),
            "Closed account data should be cleared"
        );
    }

    // If account data is provided, validate compressible account closure
    if let Some(account_data) = account_data_before_close {
        // Try to deserialize as compressible token account
        let (compressed_token, _) = CompressedToken::zero_copy_at(account_data)
            .expect("Failed to deserialize compressible token account");

        // Extract the compressible extension
        let compressible_extension = compressed_token
            .extensions
            .as_ref()
            .expect("Compressible account should have extensions")
            .iter()
            .find_map(|ext| match ext {
                light_ctoken_types::state::extensions::ZExtensionStruct::Compressible(comp) => {
                    Some(comp)
                }
                _ => None,
            })
            .expect("Should have compressible extension");

        // Calculate rent exemption based on account data length
        let rent_exemption = rpc
            .get_minimum_balance_for_rent_exemption(account_data.len())
            .await
            .expect("Failed to get rent exemption");

        // Verify the destination matches the rent recipient from the extension
        let expected_destination = Pubkey::from(compressible_extension.rent_recipient.to_bytes());
        assert_eq!(
            destination_pubkey, expected_destination,
            "Destination should match rent recipient from compressible extension"
        );

        // Verify compressible extension fields are valid
        let current_slot = rpc.get_slot().await.expect("Failed to get current slot");
        assert!(
            compressible_extension.last_written_slot <= current_slot,
            "Last written slot ({}) should not be greater than current slot ({})",
            compressible_extension.last_written_slot,
            current_slot
        );

        // Verify slots_until_compression is a valid value (should be >= 0)
        // Note: This is a u64 so it's always >= 0, but we can check it's reasonable
        assert!(
            compressible_extension.slots_until_compression < 1_000_000, // Reasonable upper bound
            "Slots until compression ({}) should be a reasonable value",
            compressible_extension.slots_until_compression
        );

        // Verify lamports were transferred to destination
        let final_destination_lamports = rpc
            .get_account(destination_pubkey)
            .await
            .expect("Failed to get destination account")
            .expect("Destination account should exist")
            .lamports;

        assert_eq!(
            final_destination_lamports,
            initial_destination_lamports + rent_exemption,
            "Destination should receive rent exemption lamports from closed account"
        );
    } else {
        // Basic account closure - verify lamports were transferred to destination
        let final_destination_lamports = rpc
            .get_account(destination_pubkey)
            .await
            .expect("Failed to get destination account")
            .expect("Destination account should exist")
            .lamports;

        // Calculate rent exemption based on basic account size
        let rent_exemption = rpc
            .get_minimum_balance_for_rent_exemption(165) // Basic SPL token account size
            .await
            .expect("Failed to get rent exemption");

        assert_eq!(
            final_destination_lamports,
            initial_destination_lamports + rent_exemption,
            "Destination should receive rent exemption lamports from closed account"
        );
    }
}
