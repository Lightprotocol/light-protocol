use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::checks::{check_mut, check_signer};
use light_ctoken_types::instructions::multi_transfer::ZCompressedTokenInstructionDataMultiTransfer;
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};

use crate::shared::AccountIterator;

/// Validated system accounts for multi-transfer instruction
/// Accounts are ordered to match light-system-program CPI expectation
pub struct MultiTransferValidatedAccounts<'info> {
    /// Fee payer account (index 0) - signer, mutable
    pub fee_payer: &'info AccountInfo,
    /// CPI authority PDA (index 1) - signer (via CPI)
    pub authority: &'info AccountInfo,
    /// Registered program PDA (index 2) - non-mutable
    pub registered_program_pda: &'info AccountInfo,
    /// Noop program (index 3) - non-mutable
    pub noop_program: &'info AccountInfo,
    /// Account compression authority (index 4) - non-mutable
    pub account_compression_authority: &'info AccountInfo,
    /// Account compression program (index 5) - non-mutable
    pub account_compression_program: &'info AccountInfo,
    /// Invoking program (index 6) - self program, non-mutable
    pub invoking_program: &'info AccountInfo,
    /// Sol pool PDA (index 7) - optional, mutable if present
    pub sol_pool_pda: Option<&'info AccountInfo>,
    /// SOL decompression recipient (index 8) - optional, mutable, for SOL decompression
    pub sol_decompression_recipient: Option<&'info AccountInfo>,
    /// System program (index 9) - non-mutable
    pub system_program: &'info AccountInfo,
    /// CPI context account (index 10) - optional, non-mutable
    pub cpi_context_account: Option<&'info AccountInfo>,
}

/// Dynamic accounts slice for index-based access
/// Contains mint, owner, delegate, merkle tree, and queue accounts
pub struct MultiTransferPackedAccounts<'info> {
    /// Packed accounts slice starting at index 11
    pub accounts: &'info [AccountInfo],
}

impl MultiTransferPackedAccounts<'_> {
    /// Get account by index with bounds checking
    pub fn get(&self, index: usize) -> Result<&AccountInfo, ProgramError> {
        self.accounts
            .get(index)
            .ok_or(ProgramError::NotEnoughAccountKeys)
    }

    /// Get account by u8 index with bounds checking
    pub fn get_u8(&self, index: u8) -> Result<&AccountInfo, ProgramError> {
        self.get(index as usize)
    }
}

impl MultiTransferValidatedAccounts<'_> {
    // The offset of 1 skips the light-system-program account (index 0)
    pub const CPI_ACCOUNTS_OFFSET: usize = 1;
}

impl<'info> MultiTransferValidatedAccounts<'info> {
    /// Validate and parse accounts from the instruction accounts slice
    pub fn validate_and_parse(
        accounts: &'info [AccountInfo],
        with_sol_pool: bool,
        with_cpi_context: bool,
    ) -> Result<(Self, MultiTransferPackedAccounts<'info>), ProgramError> {
        // Parse system accounts from fixed positions
        let mut iter = AccountIterator::new(accounts);
        let fee_payer = iter.next_account()?;
        let authority = iter.next_account()?;
        let registered_program_pda = iter.next_account()?;
        let noop_program = iter.next_account()?;
        let account_compression_authority = iter.next_account()?;
        let account_compression_program = iter.next_account()?;
        let invoking_program = iter.next_account()?;

        let sol_pool_pda = if with_sol_pool {
            Some(iter.next_account()?)
        } else {
            None
        };

        let sol_decompression_recipient = if with_sol_pool {
            Some(iter.next_account()?)
        } else {
            None
        };

        let system_program = iter.next_account()?;
        let cpi_context_account = if with_cpi_context {
            let cpi_context_account = iter.next_account()?;
            check_mut(cpi_context_account)?;
            Some(cpi_context_account)
        } else {
            None
        };

        // Validate fee_payer: must be signer and mutable
        check_signer(fee_payer)?;
        check_mut(fee_payer)?;
        // Extract remaining accounts slice for dynamic indexing
        let remaining_accounts = iter.remaining();

        let validated_accounts = MultiTransferValidatedAccounts {
            fee_payer,
            authority,
            registered_program_pda,
            noop_program,
            account_compression_authority,
            account_compression_program,
            invoking_program,
            sol_pool_pda,
            sol_decompression_recipient,
            system_program,
            cpi_context_account,
        };

        let packed_accounts = MultiTransferPackedAccounts {
            accounts: remaining_accounts,
        };

        Ok((validated_accounts, packed_accounts))
    }

    /// Calculate static accounts count after skipping index 0 (system accounts only)
    /// Returns the count of fixed accounts based on optional features
    #[inline(always)]
    pub fn static_accounts_count(&self) -> usize {
        let with_sol_pool = self.sol_pool_pda.is_some();
        let with_cpi_context = self.cpi_context_account.is_some();
        8 + if with_sol_pool { 2 } else { 0 } + if with_cpi_context { 1 } else { 0 }
    }

    /// Extract CPI accounts slice for light-system-program invocation
    /// Includes static accounts + tree accounts based on highest tree index
    /// Returns (cpi_accounts_slice, tree_accounts)
    #[inline(always)]
    pub fn cpi_accounts(
        &self,
        all_accounts: &'info [AccountInfo],
        inputs: &ZCompressedTokenInstructionDataMultiTransfer,
        packed_accounts: &'info MultiTransferPackedAccounts<'info>,
    ) -> (&'info [AccountInfo], Vec<&'info Pubkey>) {
        // Extract tree accounts using highest index approach
        let (tree_accounts, tree_accounts_count) = extract_tree_accounts(inputs, packed_accounts);

        // Calculate static accounts count after skipping index 0 (system accounts only)
        let static_accounts_count = self.static_accounts_count();

        // Include static CPI accounts + tree accounts based on highest tree index
        let cpi_accounts_end =
            Self::CPI_ACCOUNTS_OFFSET + static_accounts_count + tree_accounts_count;
        let cpi_accounts_slice = &all_accounts[Self::CPI_ACCOUNTS_OFFSET..cpi_accounts_end];

        (cpi_accounts_slice, tree_accounts)
    }
}

// TODO: unit test.
/// Extract tree accounts by finding the highest tree index and using it as closing offset
pub fn extract_tree_accounts<'info>(
    inputs: &ZCompressedTokenInstructionDataMultiTransfer,
    packed_accounts: &'info MultiTransferPackedAccounts<'info>,
) -> (Vec<&'info Pubkey>, usize) {
    // Find highest tree index from input and output data to determine tree accounts range
    let mut highest_tree_index = 0u8;
    for input_data in inputs.in_token_data.iter() {
        highest_tree_index =
            highest_tree_index.max(input_data.merkle_context.merkle_tree_pubkey_index);
        highest_tree_index = highest_tree_index.max(input_data.merkle_context.queue_pubkey_index);
    }
    for output_data in inputs.out_token_data.iter() {
        highest_tree_index = highest_tree_index.max(output_data.merkle_tree);
    }

    // Tree accounts span from index 0 to highest_tree_index in remaining accounts
    let tree_accounts_count = highest_tree_index as usize + 1;

    // Extract tree account pubkeys from the determined range
    let mut tree_accounts = Vec::new();
    for i in 0..tree_accounts_count {
        if let Some(account) = packed_accounts.accounts.get(i) {
            tree_accounts.push(account.key());
        }
    }

    (tree_accounts, tree_accounts_count)
}
