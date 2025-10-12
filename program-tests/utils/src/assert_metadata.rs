use anchor_lang::prelude::borsh::BorshDeserialize;
use light_client::{
    indexer::{CompressedAccount, Indexer},
    rpc::{Rpc, RpcError},
};
use light_ctoken_types::state::{
    extensions::{AdditionalMetadata, ExtensionStruct, TokenMetadata},
    CompressedMint,
};
use light_hasher::{sha256::Sha256BE, Hasher, HasherError};
use solana_sdk::{pubkey::Pubkey, signature::Signature};

/// Expected metadata state for comprehensive testing
#[derive(Debug, PartialEq, Clone)]
pub struct ExpectedMetadataState {
    pub update_authority: Option<Pubkey>,
    pub name: Vec<u8>,
    pub symbol: Vec<u8>,
    pub uri: Vec<u8>,
    pub additional_metadata: Vec<AdditionalMetadata>,
}

/// Assert complete metadata state matches expected values
///
/// # Arguments
/// * `rpc` - RPC client to fetch account data
/// * `compressed_mint_address` - Address of the compressed mint account
/// * `expected` - Expected metadata state to compare against
///
/// # Returns
/// * The actual TokenMetadata from the account for further analysis
///
/// # Assertions
/// * Mint account exists and is properly formatted
/// * Extensions exist and contain TokenMetadata
/// * Complete TokenMetadata struct matches expected state
/// * All fields match: update_authority, metadata, additional_metadata, version
pub async fn assert_metadata_state<R: Rpc + Indexer>(
    rpc: &mut R,
    compressed_mint_address: [u8; 32],
    expected: &ExpectedMetadataState,
) -> TokenMetadata {
    // Fetch current account data
    let compressed_mint_account = rpc
        .indexer()
        .unwrap()
        .get_compressed_account(compressed_mint_address, None)
        .await
        .expect("Failed to get compressed mint account")
        .value
        .ok_or(RpcError::AccountDoesNotExist(format!(
            "{:?}",
            compressed_mint_address
        )))
        .expect("Compressed mint account not found");
    assert_sha_account_hash(&compressed_mint_account).unwrap();

    // Deserialize the CompressedMint
    let mint_data: CompressedMint =
        BorshDeserialize::deserialize(&mut compressed_mint_account.data.unwrap().data.as_slice())
            .expect("Failed to deserialize CompressedMint");

    // Verify mint has extensions
    assert!(
        mint_data.extensions.is_some(),
        "Expected mint to have extensions but found none"
    );

    let extensions = mint_data.extensions.unwrap();
    assert!(
        !extensions.is_empty(),
        "Extensions array should not be empty"
    );

    // Get TokenMetadata extension (should be first extension)
    let actual_metadata = match &extensions[0] {
        ExtensionStruct::TokenMetadata(metadata) => metadata,
        _ => panic!("Expected first extension to be TokenMetadata"),
    };

    // Create expected TokenMetadata for complete struct comparison
    let expected_metadata = TokenMetadata {
        update_authority: expected
            .update_authority
            .map(|p| p.into())
            .unwrap_or_else(|| light_compressed_account::Pubkey::from([0u8; 32])),
        mint: actual_metadata.mint, // Copy from actual since mint address is derived
        name: expected.name.clone(),
        symbol: expected.symbol.clone(),
        uri: expected.uri.clone(),
        additional_metadata: expected.additional_metadata.clone(),
    };

    // Single comprehensive assertion comparing complete structs
    assert_eq!(
        *actual_metadata, expected_metadata,
        "Complete metadata state mismatch.\nExpected: {:#?}\nActual: {:#?}",
        expected_metadata, actual_metadata
    );

    actual_metadata.clone()
}

pub fn assert_sha_account_hash(account: &CompressedAccount) -> Result<(), HasherError> {
    let data = account.data.as_ref().ok_or(HasherError::EmptyInput)?;
    let data_hash = Sha256BE::hash(data.data.as_slice())?;
    if data_hash != data.data_hash {
        println!(
            "compressed account expected data hash {:?} != {:?}",
            data_hash, data.data_hash
        );
        Err(HasherError::BorshError)
    } else {
        Ok(())
    }
}

/// Assert that a mint operation produced the expected state transition by modifying before state
///
/// # Arguments
/// * `rpc` - RPC client to fetch current state
/// * `compressed_mint_address` - Address of the compressed mint
/// * `mint_before` - Complete mint state before the operation
/// * `expected_changes` - Function that applies expected changes to the before state
///
/// # Assertions
/// * Current complete mint state equals the before state with expected changes applied
pub async fn assert_mint_operation_result<R: Rpc + Indexer, F>(
    rpc: &mut R,
    compressed_mint_address: [u8; 32],
    mint_before: &CompressedMint,
    expected_changes: F,
) where
    F: FnOnce(&mut CompressedMint),
{
    // Apply expected changes to the before state
    let mut expected_mint_after = mint_before.clone();
    expected_changes(&mut expected_mint_after);

    // Fetch current complete mint state
    let actual_mint_after = get_actual_mint_state(rpc, compressed_mint_address).await;

    // Assert current state equals before state with expected changes applied
    assert_eq!(
        actual_mint_after,
        expected_mint_after,
        "Complete mint state transition mismatch.\nExpected (before + changes): {:#?}\nActual: {:#?}",
        expected_mint_after,
        actual_mint_after
    );
}

/// Get the complete CompressedMint state from account using borsh deserialization
pub async fn get_actual_mint_state<R: Rpc + Indexer>(
    rpc: &mut R,
    compressed_mint_address: [u8; 32],
) -> CompressedMint {
    let compressed_mint_account = rpc
        .indexer()
        .unwrap()
        .get_compressed_account(compressed_mint_address, None)
        .await
        .expect("Failed to get compressed mint account")
        .value
        .ok_or(RpcError::AccountDoesNotExist(format!(
            "{:?}",
            compressed_mint_address
        )))
        .expect("Compressed mint account not found");
    println!(
        "compressed_mint_account.data {:?}",
        compressed_mint_account.data
    );
    BorshDeserialize::deserialize(&mut compressed_mint_account.data.unwrap().data.as_slice())
        .expect("Failed to deserialize CompressedMint")
}

/// Assert that an operation fails with the expected error code
#[track_caller]
pub fn assert_metadata_error(result: Result<Signature, RpcError>, expected_error_code: u32) {
    // Use the existing error assertion pattern from light-test-utils
    crate::assert_custom_error_or_program_error(result, expected_error_code)
        .expect("Failed to verify expected error");
}

/// Helper to create ExpectedMetadataState for testing
pub fn create_expected_metadata_state(
    update_authority: Option<Pubkey>,
    name: &str,
    symbol: &str,
    uri: &str,
    additional_metadata: Vec<AdditionalMetadata>,
) -> ExpectedMetadataState {
    ExpectedMetadataState {
        update_authority,
        name: name.as_bytes().to_vec(),
        symbol: symbol.as_bytes().to_vec(),
        uri: uri.as_bytes().to_vec(),
        additional_metadata,
    }
}

/// Helper to create additional metadata entries for testing
pub fn create_additional_metadata(key: &str, value: &str) -> AdditionalMetadata {
    AdditionalMetadata {
        key: key.as_bytes().to_vec(),
        value: value.as_bytes().to_vec(),
    }
}

/// Assert that metadata extensions exist and return the TokenMetadata
pub async fn assert_metadata_exists<R: Rpc + Indexer>(
    rpc: &mut R,
    compressed_mint_address: [u8; 32],
) -> TokenMetadata {
    let compressed_mint_account = rpc
        .indexer()
        .unwrap()
        .get_compressed_account(compressed_mint_address, None)
        .await
        .expect("Failed to get compressed mint account")
        .value
        .ok_or(RpcError::AccountDoesNotExist(format!(
            "{:?}",
            compressed_mint_address
        )))
        .expect("Compressed mint account not found");

    let mint_data: CompressedMint =
        BorshDeserialize::deserialize(&mut compressed_mint_account.data.unwrap().data.as_slice())
            .expect("Failed to deserialize CompressedMint");

    assert!(
        mint_data.extensions.is_some(),
        "Expected mint to have extensions but found none"
    );

    let extensions = mint_data.extensions.unwrap();
    assert!(
        !extensions.is_empty(),
        "Extensions array should not be empty"
    );

    match &extensions[0] {
        ExtensionStruct::TokenMetadata(metadata) => metadata.clone(),
        _ => panic!("Expected first extension to be TokenMetadata"),
    }
}

/// Assert that a mint does NOT have metadata extensions
pub async fn assert_metadata_not_exists<R: Rpc + Indexer>(
    rpc: &mut R,
    compressed_mint_address: [u8; 32],
) {
    let compressed_mint_account = rpc
        .indexer()
        .unwrap()
        .get_compressed_account(compressed_mint_address, None)
        .await
        .expect("Failed to get compressed mint account")
        .value
        .ok_or(RpcError::AccountDoesNotExist(format!(
            "{:?}",
            compressed_mint_address
        )))
        .expect("Compressed mint account not found");

    let mint_data: CompressedMint =
        BorshDeserialize::deserialize(&mut compressed_mint_account.data.unwrap().data.as_slice())
            .expect("Failed to deserialize CompressedMint");

    // Assert that either extensions is None or doesn't contain TokenMetadata
    if let Some(extensions) = mint_data.extensions {
        for extension in extensions {
            if matches!(extension, ExtensionStruct::TokenMetadata(_)) {
                panic!("Expected mint to not have TokenMetadata extension but found one");
            }
        }
    }
    // If extensions is None, that's also valid - no metadata exists
}
