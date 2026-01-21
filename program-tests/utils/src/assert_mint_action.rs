use std::collections::HashMap;

use anchor_lang::prelude::borsh::BorshDeserialize;
use light_client::indexer::Indexer;
use light_compressed_account::compressed_account::CompressedAccountData;
use light_compressible::compression_info::CompressionInfo;
use light_program_test::{LightProgramTest, Rpc};
use light_token_interface::state::{extensions::AdditionalMetadata, ExtensionStruct, Mint, Token};
use solana_sdk::pubkey::Pubkey;

use crate::actions::legacy::instructions::mint_action::MintActionType;

/// Extract CompressionInfo from Light Token's Compressible extension
fn get_ctoken_compression_info(ctoken: &Token) -> Option<CompressionInfo> {
    ctoken
        .extensions
        .as_ref()?
        .iter()
        .find_map(|ext| match ext {
            ExtensionStruct::Compressible(comp) => Some(comp.info),
            _ => None,
        })
}

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
/// * Validates Light Token account balances for MintToCToken actions
pub async fn assert_mint_action(
    rpc: &mut LightProgramTest,
    compressed_mint_address: [u8; 32],
    pre_compressed_mint: Mint,
    actions: Vec<MintActionType>,
) {
    // Build expected state by applying actions to pre-state
    let mut expected_mint = pre_compressed_mint.clone();

    // Track Light Token mints for later validation (deduplicate and sum amounts)
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
            } => {
                if let Some(ref mut extensions) = expected_mint.extensions {
                    if let Some(ExtensionStruct::TokenMetadata(ref mut metadata)) =
                        extensions.get_mut(*extension_index as usize)
                    {
                        metadata.update_authority = new_authority.into();
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
            MintActionType::DecompressMint { .. } => {
                expected_mint.metadata.mint_decompressed = true;
            }
            MintActionType::CompressAndCloseMint { .. } => {
                expected_mint.metadata.mint_decompressed = false;
                // When compressed, the compression info should be default (zeroed)
                expected_mint.compression =
                    light_compressible::compression_info::CompressionInfo::default();
            }
        }
    }
    // Determine pre and post decompression states
    let post_decompressed = expected_mint.metadata.mint_decompressed;

    // Check for CompressAndCloseMint action
    let has_compress_and_close_mint = actions
        .iter()
        .any(|a| matches!(a, MintActionType::CompressAndCloseMint { .. }));

    // Fetch actual mint state from source of truth
    let actual_mint: Mint = if post_decompressed {
        // Mint PDA is source of truth when decompressed
        let mint_pda = Pubkey::from(expected_mint.metadata.mint);
        let mint_account = rpc
            .get_account(mint_pda)
            .await
            .expect("Failed to fetch Mint account")
            .expect("Mint PDA account should exist when decompressed");

        BorshDeserialize::deserialize(&mut mint_account.data.as_slice())
            .expect("Failed to deserialize Mint account")
    } else {
        // Compressed account is source of truth when not decompressed
        let actual_mint_account = rpc
            .indexer()
            .unwrap()
            .get_compressed_account(compressed_mint_address, None)
            .await
            .unwrap()
            .value
            .expect("Compressed mint account not found");

        BorshDeserialize::deserialize(&mut actual_mint_account.data.unwrap().data.as_slice())
            .expect("Failed to deserialize compressed mint")
    };

    // When decompressed, copy compression info from actual (slot/rent values are set at runtime)
    if post_decompressed {
        // Verify compression info is set (non-default) before copying
        assert_ne!(
            actual_mint.compression,
            CompressionInfo::default(),
            "Mint compression info should be set when decompressed"
        );
        expected_mint.compression = actual_mint.compression;
    }

    // Single assert_eq validates entire mint state (base, metadata, extensions, compression)
    assert_eq!(
        actual_mint, expected_mint,
        "Mint state should match expected after applying actions"
    );

    // Verify compressed account has sentinel values when decompressed
    if post_decompressed {
        let sentinel_account = rpc
            .indexer()
            .unwrap()
            .get_compressed_account(compressed_mint_address, None)
            .await
            .unwrap()
            .value
            .expect("Compressed mint account not found");
        assert_eq!(
            *sentinel_account.data.as_ref().unwrap(),
            CompressedAccountData::default(),
            "Compressed mint should have sentinel values when Mint is source of truth"
        );
    }

    // If CompressAndCloseMint, verify Mint Solana account is closed
    if has_compress_and_close_mint {
        let mint_pda = Pubkey::from(pre_compressed_mint.metadata.mint);

        let mint_account = rpc
            .get_account(mint_pda)
            .await
            .expect("Failed to fetch Mint account");

        assert!(
            mint_account.is_none(),
            "Mint PDA account should not exist after CompressAndCloseMint action"
        );
    }
    // Verify Token accounts for MintToCToken actions
    assert_token_balances(rpc, ctoken_mints).await;
}

/// Verify Token account balances after MintToCToken actions
async fn assert_token_balances(rpc: &mut LightProgramTest, ctoken_mints: HashMap<Pubkey, u64>) {
    for (account_pubkey, total_minted_amount) in ctoken_mints {
        // Get pre-transaction account state
        let pre_account = rpc
            .get_pre_transaction_account(&account_pubkey)
            .expect("Light Token account should exist before minting");

        // Parse pre-transaction Light Token state
        let mut pre_ctoken: Token =
            BorshDeserialize::deserialize(&mut &pre_account.data[..]).unwrap();

        // Apply the total minted amount (handles multiple mints to same account)
        pre_ctoken.amount = pre_ctoken
            .amount
            .checked_add(total_minted_amount)
            .expect("Token amount overflow");

        // Get actual post-transaction account
        let account_data = rpc.context.get_account(&account_pubkey).unwrap();
        let post_ctoken: Token =
            BorshDeserialize::deserialize(&mut &account_data.data[..]).unwrap();

        // Assert token amount matches expected
        assert_eq!(
            post_ctoken.amount, pre_ctoken.amount,
            "Light Token account state at {} should have {} tokens after minting, got {}",
            account_pubkey, pre_ctoken.amount, post_ctoken.amount
        );

        // Validate lamport balance changes for compressible accounts
        if let Some(compression_info) = get_ctoken_compression_info(&pre_ctoken) {
            let pre_lamports = pre_account.lamports;
            let post_lamports = account_data.lamports;

            // Calculate expected top-up using embedded compression info
            let current_slot = rpc.get_slot().await.unwrap();
            let account_size = pre_account.data.len() as u64;

            let expected_top_up = compression_info
                .calculate_top_up_lamports(account_size, current_slot, pre_lamports)
                .unwrap();

            let actual_lamport_change = post_lamports
                .checked_sub(pre_lamports)
                .expect("Post lamports should be >= pre lamports");

            assert_eq!(
                actual_lamport_change, expected_top_up,
                "Light Token account at {} should receive {} lamports top-up, got {}",
                account_pubkey, expected_top_up, actual_lamport_change
            );

            println!(
                "Lamport top-up validated: {} lamports transferred to compressible ctoken account {}",
                expected_top_up, account_pubkey
            );
        }
    }
}
