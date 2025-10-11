use light_client::rpc::{Rpc, RpcError};
use light_ctoken_types::COMPRESSIBLE_TOKEN_ACCOUNT_SIZE;
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone, PartialEq)]
pub struct AccountInfo {
    pub exists: bool,
    pub pubkey: Pubkey,
    pub lamports: u64,
    pub data: Vec<u8>,
    pub owner: Pubkey,
    pub executable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DestinationState {
    pub pubkey: Pubkey,
    pub lamports: u64,
}

impl AccountInfo {
    pub fn nonexistent(pubkey: Pubkey) -> Self {
        Self {
            exists: false,
            pubkey,
            lamports: 0,
            data: vec![],
            owner: Pubkey::default(),
            executable: false,
        }
    }

    pub fn from_account_info(pubkey: Pubkey, account: &solana_sdk::account::Account) -> Self {
        Self {
            exists: true,
            pubkey,
            lamports: account.lamports,
            data: account.data.clone(),
            owner: account.owner,
            executable: account.executable,
        }
    }
}

/// Get complete account state before an operation
pub async fn get_account_state_before<R: Rpc>(
    rpc: &mut R,
    account_pubkey: Pubkey,
) -> Result<AccountInfo, RpcError> {
    match rpc.get_account(account_pubkey).await? {
        Some(account) => Ok(AccountInfo::from_account_info(account_pubkey, &account)),
        None => Ok(AccountInfo::nonexistent(account_pubkey)),
    }
}

/// Get complete account state after an operation
pub async fn get_account_state_after<R: Rpc>(
    rpc: &mut R,
    account_pubkey: Pubkey,
) -> Result<AccountInfo, RpcError> {
    get_account_state_before(rpc, account_pubkey).await
}

/// Get destination account state for lamport transfer validation
pub async fn get_destination_state<R: Rpc>(
    rpc: &mut R,
    destination_pubkey: Pubkey,
) -> Result<DestinationState, RpcError> {
    let account = rpc
        .get_account(destination_pubkey)
        .await?
        .ok_or_else(|| RpcError::AssertRpcError("Destination account must exist".to_string()))?;

    Ok(DestinationState {
        pubkey: destination_pubkey,
        lamports: account.lamports,
    })
}

/// Assert account creation operation using ideal before + changes = after pattern
pub async fn assert_account_creation_result<R: Rpc, F>(
    rpc: &mut R,
    account_pubkey: Pubkey,
    account_state_before: &AccountInfo,
    expected_changes: F,
) -> Result<(), RpcError>
where
    F: FnOnce(&mut AccountInfo),
{
    let mut expected_state_after = account_state_before.clone();
    expected_changes(&mut expected_state_after);

    let actual_state_after = get_account_state_after(rpc, account_pubkey).await?;

    assert_eq!(
        actual_state_after, expected_state_after,
        "Account creation state transition mismatch.\nExpected: {:#?}\nActual: {:#?}",
        expected_state_after, actual_state_after
    );

    Ok(())
}

/// Assert account closure operation using ideal before + changes = after pattern
pub async fn assert_account_closure_result<R: Rpc, F, G>(
    rpc: &mut R,
    account_pubkey: Pubkey,
    destination_pubkey: Pubkey,
    account_state_before: &AccountInfo,
    destination_state_before: &DestinationState,
    expected_account_changes: F,
    expected_destination_changes: G,
) -> Result<(), RpcError>
where
    F: FnOnce(&mut AccountInfo),
    G: FnOnce(&mut DestinationState),
{
    // Apply expected changes to account state
    let mut expected_account_after = account_state_before.clone();
    expected_account_changes(&mut expected_account_after);

    // Apply expected changes to destination state
    let mut expected_destination_after = destination_state_before.clone();
    expected_destination_changes(&mut expected_destination_after);

    // Get actual states after operation
    let actual_account_after = get_account_state_after(rpc, account_pubkey).await?;
    let actual_destination_after = get_destination_state(rpc, destination_pubkey).await?;

    // Assert complete state transitions
    assert_eq!(
        actual_account_after, expected_account_after,
        "Account closure state transition mismatch.\nExpected: {:#?}\nActual: {:#?}",
        expected_account_after, actual_account_after
    );

    assert_eq!(
        actual_destination_after, expected_destination_after,
        "Destination account state transition mismatch.\nExpected: {:#?}\nActual: {:#?}",
        expected_destination_after, actual_destination_after
    );

    Ok(())
}

/// Create expected token account data for basic SPL token account
pub fn create_basic_token_account_data(
    mint_pubkey: Pubkey,
    owner_pubkey: Pubkey,
) -> Result<Vec<u8>, RpcError> {
    // For basic token accounts, we need to work with the existing COption structure
    // The simplest approach is to create a minimal account structure
    // This is used for test expectations only
    let mut data = vec![0u8; 165]; // SPL token account size

    // Basic structure: mint (32) + owner (32) + amount (8) + delegate (36) + state (1) + ...
    // For test purposes, we'll set the basic fields and leave COptions as zero-initialized
    data[0..32].copy_from_slice(&mint_pubkey.to_bytes()); // mint
    data[32..64].copy_from_slice(&owner_pubkey.to_bytes()); // owner
                                                            // amount = 0 (already zero)
                                                            // delegate COption = None (already zero)
    data[100] = 1; // state = Initialized
                   // is_native COption = None (already zero)
                   // delegated_amount = 0 (already zero)
                   // close_authority COption = None (already zero)

    Ok(data)
}

/// Create expected compressible token account data (placeholder)
pub async fn create_compressible_token_account_data<R: Rpc>(
    _rpc: &mut R,
    _mint_pubkey: Pubkey,
    _owner_pubkey: Pubkey,
    _compression_authority: Pubkey,
    _rent_sponsor: Pubkey,
    _slots_until_compression: u64,
) -> Result<Vec<u8>, RpcError> {
    // Return placeholder data for now - compressible accounts are complex
    // In a real implementation, this would serialize a CompressedToken properly
    Ok(vec![0u8; COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize])
}

/// Calculate rent exemption for account size
pub async fn calculate_rent_exemption<R: Rpc>(
    rpc: &mut R,
    account_size: usize,
) -> Result<u64, RpcError> {
    rpc.get_minimum_balance_for_rent_exemption(account_size)
        .await
}
