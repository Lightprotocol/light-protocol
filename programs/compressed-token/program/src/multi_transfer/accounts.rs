use crate::shared::AccountIterator;
use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::checks::{check_mut, check_signer};
use pinocchio::account_info::AccountInfo;

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
    /// Remaining accounts slice starting at index 11
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

impl<'info> MultiTransferValidatedAccounts<'info> {
    /// Validate and parse accounts from the instruction accounts slice
    pub fn validate_and_parse(
        accounts: &'info [AccountInfo],
        with_sol_pool: bool,
        with_cpi_context: bool,
    ) -> Result<(Self, MultiTransferPackedAccounts<'info>), ProgramError> {
        // Calculate minimum required accounts
        let min_accounts =
            11 + if with_sol_pool { 1 } else { 0 } + if with_cpi_context { 1 } else { 0 };

        if accounts.len() < min_accounts {
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        // Parse system accounts from fixed positions
        let mut iter = AccountIterator::new(accounts);
        let fee_payer = iter.next()?;
        let authority = iter.next()?;
        let registered_program_pda = iter.next()?;
        let noop_program = iter.next()?;
        let account_compression_authority = iter.next()?;
        let account_compression_program = iter.next()?;
        let invoking_program = iter.next()?;

        let sol_pool_pda = if with_sol_pool {
            Some(iter.next()?)
        } else {
            None
        };

        let sol_decompression_recipient = if with_sol_pool {
            Some(iter.next()?)
        } else {
            None
        };

        let system_program = iter.next()?;

        let cpi_context_account = if with_cpi_context {
            Some(iter.next()?)
        } else {
            None
        };

        // Validate fee_payer: must be signer and mutable
        check_signer(fee_payer).map_err(ProgramError::from)?;
        check_mut(fee_payer).map_err(ProgramError::from)?;
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
}
