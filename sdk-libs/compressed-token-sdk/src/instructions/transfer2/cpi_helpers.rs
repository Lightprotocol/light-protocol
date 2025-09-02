use arrayvec::ArrayVec;
use light_sdk::cpi::CpiAccounts;
use solana_account_info::AccountInfo;
use solana_instruction::AccountMeta;
use std::panic::Location;

use crate::{account2::CTokenAccount2, error::TokenSdkError};

/// ArrayVecSet implementation using two ArrayVecs for maximum efficiency in Solana programs
struct ArrayVecSet<T, const N: usize> {
    indices: ArrayVec<u8, N>, // Track which positions are used
    values: ArrayVec<T, N>,   // Store the actual values
}

impl<T: PartialEq + Copy + Default, const N: usize> ArrayVecSet<T, N> {
    fn new() -> Self {
        Self {
            indices: ArrayVec::new(),
            values: ArrayVec::new(),
        }
    }

    /// Insert value only if it doesn't already exist
    /// Returns true if inserted, false if already existed
    fn insert(&mut self, value: T) -> Result<bool, TokenSdkError> {
        // Check if value already exists using cheap comparisons
        for existing_value in self.values.iter() {
            if *existing_value == value {
                return Ok(false); // Already exists
            }
        }

        // Insert new value
        let next_index = self.values.len() as u8;
        self.indices
            .try_push(next_index)
            .map_err(|_| TokenSdkError::TooManyAccounts)?;
        self.values
            .try_push(value)
            .map_err(|_| TokenSdkError::TooManyAccounts)?;

        Ok(true)
    }

    fn into_values(self) -> ArrayVec<T, N> {
        self.values
    }
}

/// Generate packed AccountMetas efficiently using manual HashSet implementation
/// Returns ArrayVec for optimal Solana program performance
#[track_caller]
pub fn generate_packed_metas_from_token_accounts(
    token_accounts: &[&CTokenAccount2],
    cpi_accounts: &CpiAccounts,
) -> Result<Vec<AccountMeta>, TokenSdkError> {
    let tree_accounts = cpi_accounts.tree_accounts()?;

    let mut unique_indices: ArrayVecSet<u8, 32> = ArrayVecSet::new();

    // Collect all unique indices efficiently
    for token_account in token_accounts {
        // Process input indices
        for input in token_account.input_metas().iter() {
            check_packed_account_bounds(tree_accounts, input.owner, "input.owner")?;
            check_packed_account_bounds(tree_accounts, input.mint, "input.mint")?;
            check_packed_account_bounds(tree_accounts, input.delegate, "input.delegate")?;
            check_packed_account_bounds(
                tree_accounts,
                input.merkle_context.merkle_tree_pubkey_index,
                "input.merkle_tree_pubkey_index",
            )?;
            check_packed_account_bounds(
                tree_accounts,
                input.merkle_context.queue_pubkey_index,
                "input.queue_pubkey_index",
            )?;
            unique_indices.insert(input.owner)?;
            unique_indices.insert(input.mint)?;
            unique_indices.insert(input.delegate)?;
            unique_indices.insert(input.merkle_context.merkle_tree_pubkey_index)?;
            unique_indices.insert(input.merkle_context.queue_pubkey_index)?;
        }

        // Process output indices
        check_packed_account_bounds(
            tree_accounts,
            token_account.output.merkle_tree,
            "output.merkle_tree",
        )?;
        check_packed_account_bounds(tree_accounts, token_account.output.owner, "output.owner")?;
        check_packed_account_bounds(tree_accounts, token_account.output.mint, "output.mint")?;
        check_packed_account_bounds(
            tree_accounts,
            token_account.output.delegate,
            "output.delegate",
        )?;
        unique_indices.insert(token_account.output.merkle_tree)?;
        unique_indices.insert(token_account.output.owner)?;
        unique_indices.insert(token_account.output.mint)?;
        unique_indices.insert(token_account.output.delegate)?;

        // Process compression indices if present
        if let Some(compression) = &token_account.compression() {
            check_packed_account_bounds(
                tree_accounts,
                compression.source_or_recipient,
                "compression.source_or_recipient",
            )?;
            check_packed_account_bounds(
                tree_accounts,
                compression.authority,
                "compression.authority",
            )?;
            check_packed_account_bounds(tree_accounts, compression.mint, "compression.mint")?;
            check_packed_account_bounds(
                tree_accounts,
                compression.pool_account_index,
                "compression.pool_account_index",
            )?;
            unique_indices.insert(compression.source_or_recipient)?;
            unique_indices.insert(compression.authority)?;
            unique_indices.insert(compression.mint)?;
            unique_indices.insert(compression.pool_account_index)?;
        }
    }

    let mut indices = unique_indices.into_values();
    indices.sort();

    // Check that indices are continuous (0, 1, 2, ... n-1) and within bounds
    for (i, &index) in indices.iter().enumerate() {
        if index != i as u8 {
            solana_msg::msg!("missing index {}", i);
            return Err(TokenSdkError::NonContinuousIndices);
        }
    }

    // Convert indices to AccountMetas using ArrayVec
    let mut packed_accounts: Vec<AccountMeta> = Vec::with_capacity(indices.len());
    for index in indices {
        // Bounds already checked above
        let account_info = &tree_accounts[index as usize];

        packed_accounts.push(AccountMeta {
            pubkey: *account_info.key,
            is_signer: account_info.is_signer,
            is_writable: account_info.is_writable,
        });
    }

    Ok(packed_accounts)
}

/// Check bounds for tree account access and print detailed error on failure
#[track_caller]
#[inline(always)]
fn check_packed_account_bounds(
    tree_accounts: &[AccountInfo],
    index: u8,
    account_name: &str,
) -> Result<(), TokenSdkError> {
    if (index as usize) >= tree_accounts.len() {
        return handle_packed_account_bounds_error(account_name, index, tree_accounts.len());
    }
    Ok(())
}
#[cold]
fn handle_packed_account_bounds_error(
    account_name: &str,
    index: u8,
    tree_accounts_len: usize,
) -> Result<(), TokenSdkError> {
    let location = Location::caller();
    solana_msg::msg!(
        "ERROR: PackedAccount index out of bounds for account '{}' at index {} (max: {}) {}:{}:{}",
        account_name,
        index,
        tree_accounts_len.saturating_sub(1),
        location.file(),
        location.line(),
        location.column()
    );
    Err(TokenSdkError::PackedAccountIndexOutOfBounds)
}
