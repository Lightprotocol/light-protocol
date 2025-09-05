use anchor_spl::token_2022::spl_token_2022::{self, solana_program::program_pack::Pack};
use light_client::rpc::Rpc;
use light_ctoken_types::state::CompressedToken;
use light_zero_copy::traits::ZeroCopyAt;
use solana_sdk::pubkey::Pubkey;

/// Assert compressible extension properties for one token account pair (before/after)
pub fn assert_compressible_for_account(
    name: &str,
    data_before: &[u8],
    lamports_before: u64,
    lamports_after: u64,
    data_after: &[u8],
    current_slot: u64,
) {
    println!("{} current_slot", current_slot);
    // Parse tokens
    let token_before = if data_before.len() > 165 {
        CompressedToken::zero_copy_at(data_before).ok()
    } else {
        None
    };
    println!("{:?} token_before", token_before);

    let token_after = if data_after.len() > 165 {
        CompressedToken::zero_copy_at(data_after).ok()
    } else {
        None
    };

    if let (Some((token_before, _)), Some((token_after, _))) = (&token_before, &token_after) {
        if let Some(extensions_before) = &token_before.extensions {
            if let Some(compressible_before) = extensions_before.iter().find_map(|ext| {
                if let light_ctoken_types::state::ZExtensionStruct::Compressible(comp) = ext {
                    Some(comp)
                } else {
                    None
                }
            }) {
                let compressible_after = token_after
                    .extensions
                    .as_ref()
                    .and_then(|extensions| {
                        extensions.iter().find_map(|ext| {
                            if let light_ctoken_types::state::ZExtensionStruct::Compressible(comp) =
                                ext
                            {
                                Some(comp)
                            } else {
                                None
                            }
                        })
                    })
                    .unwrap_or_else(|| {
                        panic!("{} should have compressible extension after transfer", name)
                    });

                assert_eq!(
                    u64::from(*compressible_after.last_claimed_slot),
                    u64::from(*compressible_before.last_claimed_slot),
                    "{} last_claimed_slot should be different from current slot before transfer",
                    name
                );

                assert_eq!(
                    compressible_before.rent_authority, compressible_after.rent_authority,
                    "{} rent_authority should not change",
                    name
                );
                assert_eq!(
                    compressible_before.rent_recipient, compressible_after.rent_recipient,
                    "{} rent_recipient should not change",
                    name
                );
                assert_eq!(
                    compressible_before.version, compressible_after.version,
                    "{} version should not change",
                    name
                );
                if let Some(write_top_up_lamports) = compressible_before.write_top_up_lamports {
                    assert_eq!(
                        lamports_before + write_top_up_lamports.get() as u64,
                        lamports_after
                    );
                }
                println!("{:?} compressible_before", compressible_before);
                println!("{:?} compressible_after", compressible_after);
            }
        }
    }
}

/// Assert that a decompressed token transfer was successful by checking complete account state including extensions.
///
/// # Arguments
/// * `rpc` - RPC client to fetch account data
/// * `sender_account` - Source token account pubkey
/// * `recipient_account` - Destination token account pubkey
/// * `transfer_amount` - Amount that was transferred
/// * `sender_before` - Complete sender account state before transfer
/// * `recipient_before` - Complete recipient account state before transfer
/// * `sender_data_before` - Complete sender account data before transfer (for extension comparison)
/// * `recipient_data_before` - Complete recipient account data before transfer (for extension comparison)
///
/// # Assertions
/// * Sender balance decreased by transfer amount
/// * Recipient balance increased by transfer amount
/// * All other fields remain unchanged (mint, owner, delegate, etc.)
/// * Extensions are preserved (including compressible extensions)
/// * If compressible extensions exist, last_written_slot should be updated to current slot
#[allow(clippy::too_many_arguments)]
pub async fn assert_decompressed_token_transfer<R: Rpc>(
    rpc: &mut R,
    sender_account: Pubkey,
    recipient_account: Pubkey,
    transfer_amount: u64,
    sender_data_before: &[u8],
    recipient_data_before: &[u8],
    sender_lamports_before: u64,
    recipient_lamports_before: u64,
) {
    // Fetch updated account data
    let sender_account_data = rpc.get_account(sender_account).await.unwrap().unwrap();
    let recipient_account_data = rpc.get_account(recipient_account).await.unwrap().unwrap();
    let sender_account_data_after = sender_account_data.data.as_slice();
    let recipient_account_data_after = recipient_account_data.data.as_slice();

    let current_slot = rpc.get_slot().await.unwrap();

    // Check compressible extensions for both sender and recipient
    assert_compressible_for_account(
        "Sender",
        sender_data_before,
        sender_lamports_before,
        sender_account_data.lamports,
        sender_account_data_after,
        current_slot,
    );
    assert_compressible_for_account(
        "Recipient",
        recipient_data_before,
        recipient_lamports_before,
        recipient_account_data.lamports,
        recipient_account_data_after,
        current_slot,
    );

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
        assert_eq!(sender_account_after, sender_token_before);
        assert_eq!(recipient_account_after, recipient_token_before);
    }
}
