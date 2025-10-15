use std::collections::HashMap;

use anchor_lang::prelude::borsh::BorshDeserialize;
use light_client::indexer::Indexer;
use light_compressed_token_sdk::instructions::mint_action::MintActionType;
use light_ctoken_types::state::{
    extensions::{AdditionalMetadata, ExtensionStruct},
    CompressedMint,
};
use light_program_test::LightProgramTest;
use solana_sdk::{program_pack::Pack, pubkey::Pubkey};

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
            MintActionType::CreateSplMint { .. } => {
                expected_mint.metadata.spl_mint_initialized = true;
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
                        metadata.update_authority = (*new_authority).into();
                    }
                }
            }
            MintActionType::RemoveMetadataKey {
                extension_index,
                key,
                ..
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
        let mut expected_token_account =
            spl_token::state::Account::unpack(&pre_account.data[..165]).unwrap();

        // Apply the total minted amount (handles multiple mints to same account)
        expected_token_account.amount += total_minted_amount;

        // Get actual post-transaction account
        let account_data = rpc.context.get_account(&account_pubkey).unwrap();
        let actual_token_account =
            spl_token::state::Account::unpack(&account_data.data[..165]).unwrap();

        // Single assertion for complete account state
        assert_eq!(
            actual_token_account, expected_token_account,
            "CToken account state at {} should match expected after minting {} tokens",
            account_pubkey, total_minted_amount
        );
    }
}
