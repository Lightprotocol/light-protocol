use anchor_spl::token_2022::spl_token_2022::{self, solana_program::program_pack::Pack};
use light_client::rpc::Rpc;
use light_ctoken_interface::state::CToken;
use light_program_test::LightProgramTest;
use light_zero_copy::traits::ZeroCopyAt;
use solana_sdk::pubkey::Pubkey;

/// Assert compressible extension properties for an account, using cached pre-transaction state
pub async fn assert_compressible_for_account(
    rpc: &mut LightProgramTest,
    name: &str,
    account_pubkey: Pubkey,
) {
    // Get pre-transaction state from cache
    let pre_account = rpc
        .get_pre_transaction_account(&account_pubkey)
        .expect("Account should exist in pre-transaction context");

    let data_before = pre_account.data.as_slice();
    let lamports_before = pre_account.lamports;

    // Get post-transaction state
    let post_account = rpc
        .get_account(account_pubkey)
        .await
        .expect("Failed to get account after transaction")
        .expect("Account should exist after transaction");

    let data_after = post_account.data.as_slice();
    let lamports_after = post_account.lamports;

    // Parse tokens
    let token_before = if data_before.len() > 165 {
        CToken::zero_copy_at(data_before).ok()
    } else {
        None
    };

    let token_after = if data_after.len() > 165 {
        CToken::zero_copy_at(data_after).ok()
    } else {
        None
    };

    if let (Some((token_before, _)), Some((token_after, _))) = (&token_before, &token_after) {
        // Get compression info from Compressible extension
        let compressible_before = token_before.get_compressible_extension();
        let compressible_after = token_after.get_compressible_extension();

        if let (Some(comp_before), Some(comp_after)) = (compressible_before, compressible_after) {
            let compression_before = &comp_before.info;
            let compression_after = &comp_after.info;

            assert_eq!(
                u64::from(compression_after.last_claimed_slot),
                u64::from(compression_before.last_claimed_slot),
                "{} last_claimed_slot should be different from current slot before transfer",
                name
            );

            assert_eq!(
                compression_before.compression_authority, compression_after.compression_authority,
                "{} compression_authority should not change",
                name
            );
            assert_eq!(
                compression_before.rent_sponsor, compression_after.rent_sponsor,
                "{} rent_sponsor should not change",
                name
            );
            assert_eq!(
                compression_before.config_account_version, compression_after.config_account_version,
                "{} config_account_version should not change",
                name
            );
            let current_slot = rpc.get_slot().await.unwrap();
            let top_up = compression_before
                .calculate_top_up_lamports(data_before.len() as u64, current_slot, lamports_before)
                .unwrap();
            // Check if top-up was applied
            if top_up != 0 {
                assert_eq!(
                    lamports_before + top_up,
                    lamports_after,
                    "{} account should be topped up by {} lamports",
                    name,
                    top_up
                );
            } else {
                assert_eq!(
                    lamports_before, lamports_after,
                    "{} account should not be topped up",
                    name
                );
            }
        }
    }
}

/// Assert that a decompressed token transfer was successful by checking complete account state including extensions.
/// Automatically retrieves pre-transaction state from the cached context.
///
/// # Arguments
/// * `rpc` - RPC client to fetch account data (must be LightProgramTest)
/// * `sender_account` - Source token account pubkey
/// * `recipient_account` - Destination token account pubkey
/// * `transfer_amount` - Amount that was transferred
///
/// # Assertions
/// * Sender balance decreased by transfer amount
/// * Recipient balance increased by transfer amount
/// * All other fields remain unchanged (mint, owner, delegate, etc.)
/// * Extensions are preserved (including compressible extensions)
/// * If compressible extensions exist, last_written_slot should be updated to current slot
pub async fn assert_ctoken_transfer(
    rpc: &mut LightProgramTest,
    sender_account: Pubkey,
    recipient_account: Pubkey,
    transfer_amount: u64,
) {
    // Get pre-transaction state from cache for both accounts
    let sender_before = rpc
        .get_pre_transaction_account(&sender_account)
        .expect("Sender account should exist in pre-transaction context");
    let recipient_before = rpc
        .get_pre_transaction_account(&recipient_account)
        .expect("Recipient account should exist in pre-transaction context");

    let sender_data_before = sender_before.data.as_slice();
    let recipient_data_before = recipient_before.data.as_slice();

    // Fetch updated account data
    let sender_account_data = rpc.get_account(sender_account).await.unwrap().unwrap();
    let recipient_account_data = rpc.get_account(recipient_account).await.unwrap().unwrap();

    // Check compressible extensions for both sender and recipient
    assert_compressible_for_account(rpc, "Sender", sender_account).await;
    assert_compressible_for_account(rpc, "Recipient", recipient_account).await;

    {
        // Parse as SPL token accounts first
        let mut sender_token_before =
            spl_token_2022::state::Account::unpack(&sender_data_before[..165]).unwrap();
        sender_token_before.amount -= transfer_amount;
        let mut recipient_token_before =
            spl_token_2022::state::Account::unpack(&recipient_data_before[..165]).unwrap();
        recipient_token_before.amount += transfer_amount;

        // Parse as SPL token accounts first
        let sender_account_after =
            spl_token_2022::state::Account::unpack(&sender_account_data.data[..165]).unwrap();
        let recipient_account_after =
            spl_token_2022::state::Account::unpack(&recipient_account_data.data[..165]).unwrap();

        assert_eq!(
            recipient_account_after, recipient_token_before,
            "transfer_amount {}",
            transfer_amount
        );
        assert_eq!(
            sender_account_after, sender_token_before,
            "transfer_amount {}",
            transfer_amount
        );
    }
}
