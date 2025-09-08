use light_client::rpc::Rpc;
use light_ctoken_types::state::{
    extensions::compressible::calculate_close_lamports, solana_ctoken::CompressedToken,
    ZExtensionStruct,
};
use light_program_test::LightProgramTest;
use light_zero_copy::traits::ZeroCopyAt;
use solana_sdk::pubkey::Pubkey;

pub async fn assert_close_token_account(
    rpc: &mut LightProgramTest,
    token_account_pubkey: Pubkey,
    // authority with compressible, destination without compressible ext
    authority_pubkey: Pubkey,
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

    // Parse to find destination (rent_recipient) from compressible extension
    let (compressed_token, _) = CompressedToken::zero_copy_at(account_data_before_close)
        .expect("Failed to deserialize compressible token account");

    // Get initial authority balance from pre-transaction context
    let initial_authority_lamports = rpc
        .get_pre_transaction_account(&authority_pubkey)
        .map(|acc| acc.lamports)
        .unwrap_or(0);
    // Verify authority received correct amount
    let final_authority_lamports = rpc
        .get_account(authority_pubkey)
        .await
        .expect("Failed to get authority account")
        .expect("Authority account should exist")
        .lamports;
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
        )
        .await;
    } else {
        // For non-compressible accounts, all lamports go to the destination
        assert_eq!(
            final_authority_lamports,
            initial_authority_lamports + account_lamports_before_close,
            "Authority should receive all {} lamports from closed account",
            account_lamports_before_close
        );
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
    let destination_pubkey = compressible_extension
        .rent_recipient
        .map(|r| Pubkey::from(*r))
        .unwrap_or(authority_pubkey);

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

    // Verify authority received correct amount
    let final_authority_lamports = rpc
        .get_account(authority_pubkey)
        .await
        .expect("Failed to get authority account")
        .expect("Authority account should exist")
        .lamports;
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
    let account_size = account_data_before_close.len() as u64;
    let (mut lamports_to_destination, mut lamports_to_authority) = calculate_close_lamports(
        account_size,
        current_slot,
        account_lamports_before_close,
        *compressible_extension.last_claimed_slot,
        *compressible_extension.base_lamports_balance,
    );

    // Check if rent authority is the signer
    let is_rent_authority_signer = if let Some(rent_auth) = compressible_extension.rent_authority {
        authority_pubkey == Pubkey::from(*rent_auth)
    } else {
        false
    };

    // Adjust distribution based on who signed
    if is_rent_authority_signer {
        // Rent authority gets everything
        lamports_to_destination += lamports_to_authority;
        lamports_to_authority = 0;
    }

    assert_eq!(
        final_destination_lamports,
        initial_destination_lamports + lamports_to_destination,
        "Destination should receive calculated rent lamports"
    );

    assert_eq!(
        final_authority_lamports,
        initial_authority_lamports + lamports_to_authority,
        "Authority should receive {} lamports (rent authority signer: {})",
        lamports_to_authority,
        is_rent_authority_signer
    );
}
