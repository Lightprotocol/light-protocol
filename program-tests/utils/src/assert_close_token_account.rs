use light_client::rpc::Rpc;
use light_compressible::rent::AccountRentState;
use light_ctoken_types::state::{ctoken::CToken, ZExtensionStruct};
use light_program_test::LightProgramTest;
use light_zero_copy::traits::ZeroCopyAt;
use solana_sdk::{pubkey::Pubkey, signer::Signer};

pub async fn assert_close_token_account(
    rpc: &mut LightProgramTest,
    token_account_pubkey: Pubkey,
    authority_pubkey: Pubkey,
    destination: Pubkey,
) {
    // Get pre-transaction state from cached context
    let pre_account = rpc
        .get_pre_transaction_account(&token_account_pubkey)
        .expect("Token account should exist in pre-transaction context");

    let account_data_before_close = pre_account.data.as_slice();
    let account_lamports_before_close = pre_account.lamports;

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

    // Parse to find destination (rent_sponsor) from compressible extension
    let (compressed_token, _) = CToken::zero_copy_at(account_data_before_close)
        .expect("Failed to deserialize compressible token account");

    // Get initial authority balance from pre-transaction context
    let initial_authority_lamports = rpc
        .get_pre_transaction_account(&authority_pubkey)
        .map(|acc| acc.lamports)
        .unwrap_or(0);
    // Verify authority received correct amount (account may not exist if never funded)
    let final_authority_lamports = rpc
        .get_account(authority_pubkey)
        .await
        .expect("Failed to get authority account")
        .map(|acc| acc.lamports)
        .unwrap_or(0);
    // Validate compressible account closure (we already have the parsed data)
    // Extract the compressible extension (already parsed above)
    if let Some(extension) = compressed_token.extensions.as_ref() {
        assert_compressible_extension(
            rpc,
            extension,
            authority_pubkey,
            account_data_before_close,
            account_lamports_before_close,
            initial_authority_lamports,
            destination,
        )
        .await;
    } else {
        // For non-compressible accounts, all lamports go to the destination
        // Get initial destination balance from pre-transaction context
        let initial_destination_lamports = rpc
            .get_pre_transaction_account(&destination)
            .map(|acc| acc.lamports)
            .unwrap_or(0);

        // Get final destination balance
        let final_destination_lamports = rpc
            .get_account(destination)
            .await
            .expect("Failed to get destination account")
            .expect("Destination account should exist")
            .lamports;

        assert_eq!(
            final_destination_lamports,
            initial_destination_lamports + account_lamports_before_close,
            "Destination should receive all {} lamports from closed account",
            account_lamports_before_close
        );

        // For non-compressible accounts, authority balance check depends on if they're also the destination
        if authority_pubkey == destination {
            // Authority is the destination, they receive the lamports
            assert_eq!(
                final_authority_lamports,
                initial_authority_lamports + account_lamports_before_close,
                "Authority (as destination) should receive all {} lamports for non-compressible account closure",
                account_lamports_before_close
            );
        } else {
            // Authority is not the destination, shouldn't receive anything
            assert_eq!(
                final_authority_lamports, initial_authority_lamports,
                "Authority (not destination) should not receive any lamports for non-compressible account closure"
            );
        }
    };
}

/// 1. if authority is owner
///     - if has rent recipient rent and rent exemption should go to rent recipient
///         - remaining funds go to the owner
///     - else all funds go to the owner
/// 2. else authority is rent authority ()
///     - all funds (rent exemption + remaining) should go to rent recipient
async fn assert_compressible_extension(
    rpc: &mut LightProgramTest,
    extension: &[ZExtensionStruct<'_>],
    authority_pubkey: Pubkey,
    account_data_before_close: &[u8],
    account_lamports_before_close: u64,
    initial_authority_lamports: u64,
    destination_pubkey: Pubkey,
) {
    let compressible_extension = extension
        .iter()
        .find_map(|ext| match ext {
            light_ctoken_types::state::extensions::ZExtensionStruct::Compressible(comp) => {
                Some(comp)
            }
            _ => None,
        })
        .expect("If a token account has extensions it must be a compressible extension");

    // Get initial destination balance from pre-transaction context
    let initial_destination_lamports = rpc
        .get_pre_transaction_account(&destination_pubkey)
        .map(|acc| acc.lamports)
        .unwrap_or(0);

    // Verify lamports were transferred to destination (rent recipient)
    let final_destination_lamports = rpc
        .get_account(destination_pubkey)
        .await
        .expect("Failed to get destination account")
        .expect("Destination account should exist")
        .lamports;

    // Verify authority received correct amount (account may not exist if never funded)
    let final_authority_lamports = rpc
        .get_account(authority_pubkey)
        .await
        .expect("Failed to get authority account")
        .map(|acc| acc.lamports)
        .unwrap_or(0);

    // Transaction fee: 5000 lamports per signature * 2 signers = 10,000 lamports
    let tx_fee = 10_000;

    // Get the transaction payer (who pays the tx fee)
    let payer_pubkey = rpc.get_payer().pubkey();

    // Verify compressible extension fields are valid
    let current_slot = rpc.get_slot().await.expect("Failed to get current slot");
    assert!(
        u64::from(compressible_extension.last_claimed_slot) <= current_slot,
        "Last claimed slot ({}) should not be greater than current slot ({})",
        u64::from(compressible_extension.last_claimed_slot),
        current_slot
    );

    // Verify config_account_version is initialized
    assert!(
        compressible_extension.config_account_version == 1,
        "Config account version should be 1 (initialized), got {}",
        compressible_extension.config_account_version
    );

    // Calculate expected lamport distribution using the same function as the program
    let account_size = account_data_before_close.len() as u64;
    let base_lamports = rpc
        .get_minimum_balance_for_rent_exemption(account_size as usize)
        .await
        .unwrap();

    // Create AccountRentState and use the method to calculate distribution
    let state = AccountRentState {
        num_bytes: account_size,
        current_slot,
        current_lamports: account_lamports_before_close,
        last_claimed_slot: u64::from(compressible_extension.last_claimed_slot),
    };

    let distribution =
        state.calculate_close_distribution(&compressible_extension.rent_config, base_lamports);
    let (mut lamports_to_rent_sponsor, mut lamports_to_destination) =
        (distribution.to_rent_sponsor, distribution.to_user);

    let compression_cost: u64 = compressible_extension.rent_config.compression_cost.into();

    // Get the rent recipient from the extension
    let rent_sponsor = Pubkey::from(compressible_extension.rent_sponsor);

    // Check if rent authority is the signer
    // Check if compression_authority is set (non-zero)
    let is_compression_authority_signer =
        if compressible_extension.compression_authority != [0u8; 32] {
            authority_pubkey == Pubkey::from(compressible_extension.compression_authority)
        } else {
            false
        };

    // Adjust distribution based on who signed (matching processor logic)
    if is_compression_authority_signer {
        // When rent authority closes:
        // - Extract compression incentive from rent_sponsor portion
        // - User funds also go to rent_sponsor
        // - Compression incentive goes to destination (forester)
        lamports_to_rent_sponsor = lamports_to_rent_sponsor
            .checked_sub(compression_cost)
            .expect("Rent recipient should have enough for compression incentive");
        lamports_to_rent_sponsor += lamports_to_destination;
        lamports_to_destination = compression_cost;
    }

    // Now verify the actual transfers
    if is_compression_authority_signer {
        // When rent authority closes, destination gets compression incentive
        assert_eq!(
            final_destination_lamports,
            initial_destination_lamports + lamports_to_destination,
            "Destination should receive compression incentive ({} lamports) when rent authority closes",
            compression_cost
        );

        // Get the rent recipient's initial and final balances
        let initial_rent_sponsor_lamports = rpc
            .get_pre_transaction_account(&rent_sponsor)
            .map(|acc| acc.lamports)
            .unwrap_or(0);

        let final_rent_sponsor_lamports = rpc
            .get_account(rent_sponsor)
            .await
            .expect("Failed to get rent recipient account")
            .expect("Rent recipient account should exist")
            .lamports;

        // When rent authority closes, check if rent_sponsor is also the payer
        if rent_sponsor == payer_pubkey {
            assert_eq!(
                final_rent_sponsor_lamports,
                initial_rent_sponsor_lamports + lamports_to_rent_sponsor - tx_fee,
                "Rent recipient should receive {} lamports - {} lamports (tx fee) = {} lamports when they are also the transaction payer (rent authority closes)",
                lamports_to_rent_sponsor,
                tx_fee,
                lamports_to_rent_sponsor - tx_fee
            );
        } else {
            assert_eq!(
                final_rent_sponsor_lamports,
                initial_rent_sponsor_lamports + lamports_to_rent_sponsor,
                "Rent recipient should receive {} lamports (rent authority closes)",
                lamports_to_rent_sponsor
            );
        }
    } else {
        // When owner closes, normal distribution
        assert_eq!(
            final_destination_lamports,
            initial_destination_lamports + lamports_to_destination,
            "Destination should receive user funds ({} lamports) when owner closes",
            lamports_to_destination
        );

        // Rent recipient still gets their portion
        let initial_rent_sponsor_lamports = rpc
            .get_pre_transaction_account(&rent_sponsor)
            .map(|acc| acc.lamports)
            .unwrap_or(0);

        let final_rent_sponsor_lamports = rpc
            .get_account(rent_sponsor)
            .await
            .expect("Failed to get rent recipient account")
            .expect("Rent recipient account should exist")
            .lamports;

        // When rent_sponsor == payer (tx fee payer), they pay tx_fee, so adjust expectation
        if rent_sponsor == payer_pubkey {
            assert_eq!(
                final_rent_sponsor_lamports,
                initial_rent_sponsor_lamports + lamports_to_rent_sponsor - tx_fee,
                "Rent recipient should receive {} lamports - {} lamports (tx fee) = {} lamports when they are also the transaction payer",
                lamports_to_rent_sponsor,
                tx_fee,
                lamports_to_rent_sponsor - tx_fee
            );
        } else {
            assert_eq!(
                final_rent_sponsor_lamports,
                initial_rent_sponsor_lamports + lamports_to_rent_sponsor,
                "Rent recipient should receive {} lamports",
                lamports_to_rent_sponsor
            );
        }
    }

    // Authority balance check:
    // - If authority == destination, they receive lamports_to_destination
    // - Otherwise, authority should receive nothing
    if authority_pubkey == destination_pubkey {
        // Authority is also the destination, so they receive the destination lamports
        let expected_authority_lamports = if authority_pubkey == payer_pubkey {
            // If authority is also the payer, subtract tx fee
            initial_authority_lamports + lamports_to_destination - tx_fee
        } else {
            initial_authority_lamports + lamports_to_destination
        };

        assert_eq!(
            final_authority_lamports, expected_authority_lamports,
            "Authority (as destination) should receive {} lamports (rent authority signer: {})",
            lamports_to_destination, is_compression_authority_signer
        );
    } else {
        // Authority is not the destination, shouldn't receive anything
        let expected_authority_lamports = if authority_pubkey == payer_pubkey {
            // If authority is the payer, subtract tx fee
            initial_authority_lamports - tx_fee
        } else {
            initial_authority_lamports
        };

        assert_eq!(
            final_authority_lamports, expected_authority_lamports,
            "Authority (not destination) should not receive any lamports (rent authority signer: {})",
            is_compression_authority_signer
        );
    }
}
