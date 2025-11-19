use std::collections::HashMap;

use anchor_lang::prelude::borsh::BorshDeserialize;
use light_client::indexer::Indexer;
use light_compressed_token_sdk::instructions::mint_action::MintActionType;
use light_ctoken_types::state::{
    extensions::{AdditionalMetadata, ExtensionStruct},
    CToken, CompressedMint,
};
use light_program_test::{LightProgramTest, Rpc};
use solana_sdk::pubkey::Pubkey;

/// Assert that mint actions produce the expected state changes
///
/// # Arguments
/// * `rpc` - RPC client to fetch actual state
/// * `compressed_mint_address` - Address of the compressed mint
/// * `pre_compressed_mint` - Mint state before the actions
/// * `actions` - Actions that were executed
///
/// # Assertions
/// * Single assert_eq! comparing actual vs expected mint state
/// * Validates CToken account balances for MintToCToken actions
pub async fn assert_mint_action(
    rpc: &mut LightProgramTest,
    compressed_mint_address: [u8; 32],
    pre_compressed_mint: CompressedMint,
    actions: Vec<MintActionType>,
) {
    // Build expected state by applying actions to pre-state
    let mut expected_mint = pre_compressed_mint.clone();

    // Track CToken mints for later validation (deduplicate and sum amounts)
    let mut ctoken_mints: HashMap<Pubkey, u64> = HashMap::new();

    for action in actions.iter() {
        match action {
            MintActionType::MintTo { recipients, .. } => {
                let total_amount: u64 = recipients.iter().map(|r| r.amount).sum();
                expected_mint.base.supply += total_amount;
            }
            MintActionType::MintToCToken { account, amount } => {
                expected_mint.base.supply += *amount;
                // Track this mint for later balance verification (accumulate amounts)
                *ctoken_mints.entry(*account).or_insert(0) += *amount;
            }
            MintActionType::UpdateMintAuthority { new_authority } => {
                expected_mint.base.mint_authority = new_authority.map(Into::into);
            }
            MintActionType::UpdateFreezeAuthority { new_authority } => {
                expected_mint.base.freeze_authority = new_authority.map(Into::into);
            }
            MintActionType::UpdateMetadataField {
                extension_index,
                field_type,
                key,
                value,
            } => {
                if let Some(ref mut extensions) = expected_mint.extensions {
                    if let Some(ExtensionStruct::TokenMetadata(ref mut metadata)) =
                        extensions.get_mut(*extension_index as usize)
                    {
                        match field_type {
                            0 => metadata.name = value.clone(),
                            1 => metadata.symbol = value.clone(),
                            2 => metadata.uri = value.clone(),
                            3 => {
                                // Update existing or add new additional metadata
                                if let Some(entry) = metadata
                                    .additional_metadata
                                    .iter_mut()
                                    .find(|m| m.key == *key)
                                {
                                    entry.value = value.clone();
                                } else {
                                    metadata.additional_metadata.push(AdditionalMetadata {
                                        key: key.clone(),
                                        value: value.clone(),
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            MintActionType::UpdateMetadataAuthority {
                extension_index,
                new_authority,
                idempotent: _,
            } => {
                if let Some(ref mut extensions) = expected_mint.extensions {
                    if let Some(ExtensionStruct::TokenMetadata(ref mut metadata)) =
                        extensions.get_mut(*extension_index as usize)
                    {
                        metadata.update_authority = new_authority
                            .map(|a| a.to_bytes().into())
                            .unwrap_or([0u8; 32].into());
                    }
                }
            }
            MintActionType::RemoveMetadataKey {
                extension_index,
                key,
                idempotent: _,
            } => {
                if let Some(ref mut extensions) = expected_mint.extensions {
                    if let Some(ExtensionStruct::TokenMetadata(ref mut metadata)) =
                        extensions.get_mut(*extension_index as usize)
                    {
                        metadata.additional_metadata.retain(|m| m.key != *key);
                    }
                }
            }
        }
    }

    // Get actual post-transaction state
    let actual_mint_account = rpc
        .indexer()
        .unwrap()
        .get_compressed_account(compressed_mint_address, None)
        .await
        .unwrap()
        .value
        .expect("Compressed mint account not found");

    let actual_mint: CompressedMint =
        BorshDeserialize::deserialize(&mut actual_mint_account.data.unwrap().data.as_slice())
            .unwrap();

    // Single assertion
    assert_eq!(
        actual_mint, expected_mint,
        "Compressed mint state after mint_action should match expected"
    );

    // Verify CToken accounts for MintToCToken actions
    for (account_pubkey, total_minted_amount) in ctoken_mints {
        // Get pre-transaction account state
        let pre_account = rpc
            .get_pre_transaction_account(&account_pubkey)
            .expect("CToken account should exist before minting");

        // Parse pre-transaction CToken state
        let mut pre_ctoken: CToken =
            BorshDeserialize::deserialize(&mut &pre_account.data[..]).unwrap();

        // Apply the total minted amount (handles multiple mints to same account)
        pre_ctoken.amount = pre_ctoken
            .amount
            .checked_add(total_minted_amount)
            .expect("Token amount overflow");

        // Get actual post-transaction account
        let account_data = rpc.context.get_account(&account_pubkey).unwrap();
        let post_ctoken: CToken =
            BorshDeserialize::deserialize(&mut &account_data.data[..]).unwrap();

        // Assert token amount matches expected
        assert_eq!(
            post_ctoken.amount, pre_ctoken.amount,
            "CToken account state at {} should have {} tokens after minting, got {}",
            account_pubkey, pre_ctoken.amount, post_ctoken.amount
        );

        // Validate lamport balance changes for compressible accounts
        let pre_lamports = pre_account.lamports;
        let post_lamports = account_data.lamports;

        // Check if account has compressible extension (reuse pre_ctoken parsed earlier)
        if let Some(extensions) = pre_ctoken.extensions.as_ref() {
            // Look for compressible extension
            let compressible_ext = extensions.iter().find_map(|ext| {
                if let ExtensionStruct::Compressible(comp) = ext {
                    Some(comp)
                } else {
                    None
                }
            });

            if let Some(compressible) = compressible_ext {
                // Account has compressible extension - calculate expected top-up
                let current_slot = rpc.get_slot().await.unwrap();
                let account_size = pre_account.data.len() as u64;

                let expected_top_up = compressible
                    .calculate_top_up_lamports(
                        account_size,
                        current_slot,
                        pre_lamports,
                        compressible.lamports_per_write,
                        light_ctoken_types::COMPRESSIBLE_TOKEN_RENT_EXEMPTION,
                    )
                    .unwrap();

                let actual_lamport_change = post_lamports
                    .checked_sub(pre_lamports)
                    .expect("Post lamports should be >= pre lamports");

                assert_eq!(
                    actual_lamport_change, expected_top_up,
                    "CToken account at {} should receive {} lamports top-up for compressible extension, got {}",
                    account_pubkey, expected_top_up, actual_lamport_change
                );

                println!(
                        "✓ Lamport top-up validated: {} lamports transferred to compressible ctoken account {}",
                        expected_top_up, account_pubkey
                    );
            } else {
                // Has extensions but no compressible extension - lamports should not change
                assert_eq!(
                    pre_lamports, post_lamports,
                    "Non-compressible CToken account at {} should not receive lamport top-up",
                    account_pubkey
                );
            }
        } else {
            // No extensions - lamports should not change
            assert_eq!(
                pre_lamports, post_lamports,
                "CToken account without extensions at {} should not receive lamport top-up",
                account_pubkey
            );
        }
    }
}
