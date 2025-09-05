use light_client::rpc::Rpc;
use light_ctoken_types::state::{
    extensions::compressible::calculate_close_lamports, solana_ctoken::CompressedToken,
};
use light_zero_copy::traits::ZeroCopyAt;
use solana_sdk::pubkey::Pubkey;

/// Assert that a token account was closed correctly.
/// Verifies that the account has 0 lamports, cleared data, and lamports were transferred correctly.
/// If account_data_before_close is provided, validates compressible account closure.
pub async fn assert_close_token_account<R: Rpc>(
    rpc: &mut R,
    token_account_pubkey: Pubkey,
    account_data_before_close: Option<&[u8]>,
    account_lamports_before_close: u64,
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

        // Verify the destination matches the rent recipient from the extension
        let expected_destination = Pubkey::from(*compressible_extension.rent_recipient.unwrap());
        assert_eq!(
            destination_pubkey, expected_destination,
            "Destination should match rent recipient from compressible extension"
        );

        // Verify compressible extension fields are valid
        let current_slot = rpc.get_slot().await.expect("Failed to get current slot");
        assert!(
            u64::from(*compressible_extension.last_claimed_slot) <= current_slot,
            "Last claimed slot ({}) should not be greater than current slot ({})",
            u64::from(*compressible_extension.last_claimed_slot),
            current_slot
        );

        // Verify version is initialized
        assert!(
            compressible_extension.version == 1,
            "Version should be 1 (initialized), got {}",
            compressible_extension.version
        );

        // Calculate expected lamport distribution using the same function as the program
        let account_size = account_data.len() as u64;
        let (lamports_to_destination, _lamports_to_authority) = calculate_close_lamports(
            account_size,
            current_slot,
            account_lamports_before_close,
            *compressible_extension.last_claimed_slot,
            *compressible_extension.lamports_at_last_claimed_slot,
        );

        // Verify lamports were transferred to destination (rent recipient)
        let final_destination_lamports = rpc
            .get_account(destination_pubkey)
            .await
            .expect("Failed to get destination account")
            .expect("Destination account should exist")
            .lamports;

        assert_eq!(
            final_destination_lamports,
            initial_destination_lamports + lamports_to_destination,
            "Destination should receive calculated rent lamports"
        );
    } else {
        // Basic account closure - verify lamports were transferred to destination
        let final_destination_lamports = rpc
            .get_account(destination_pubkey)
            .await
            .expect("Failed to get destination account")
            .expect("Destination account should exist")
            .lamports;

        assert_eq!(
            final_destination_lamports,
            initial_destination_lamports + account_lamports_before_close,
            "Destination should receive all lamports from closed account"
        );
    }
}
