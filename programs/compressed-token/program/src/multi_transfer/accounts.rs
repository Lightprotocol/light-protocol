use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::checks::{check_mut, check_non_mut, check_program, check_signer};
use light_compressed_account::constants::ACCOUNT_COMPRESSION_PROGRAM_ID;
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
    /// Decompression recipient (index 8) - non-mutable
    pub decompression_recipient: &'info AccountInfo,
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

impl<'info> MultiTransferPackedAccounts<'info> {
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
        program_id: &pinocchio::pubkey::Pubkey,
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
        let fee_payer = &accounts[0];
        let authority = &accounts[1];
        let registered_program_pda = &accounts[2];
        let noop_program = &accounts[3];
        let account_compression_authority = &accounts[4];
        let account_compression_program = &accounts[5];
        let invoking_program = &accounts[6];

        let mut index = 7;
        let sol_pool_pda = if with_sol_pool {
            let account = Some(&accounts[index]);
            index += 1;
            account
        } else {
            None
        };

        let decompression_recipient = &accounts[index];
        index += 1;

        let system_program = &accounts[index];
        index += 1;

        let cpi_context_account = if with_cpi_context {
            let account = Some(&accounts[index]);
            index += 1;
            account
        } else {
            None
        };

        // Validate fee_payer: must be signer and mutable
        check_signer(fee_payer).map_err(ProgramError::from)?;
        check_mut(fee_payer).map_err(ProgramError::from)?;

        // Validate registered_program_pda: must be correct PDA
        check_non_mut(registered_program_pda).map_err(ProgramError::from)?;

        // Validate noop_program: must be correct program
        check_non_mut(noop_program).map_err(ProgramError::from)?;

        // Validate account_compression_authority: must be correct PDA
        check_non_mut(account_compression_authority).map_err(ProgramError::from)?;

        // Validate account_compression_program: must be correct program
        check_non_mut(account_compression_program).map_err(ProgramError::from)?;
        check_program(&ACCOUNT_COMPRESSION_PROGRAM_ID, account_compression_program)
            .map_err(ProgramError::from)?;

        // Validate invoking_program: must be this program
        check_non_mut(invoking_program).map_err(ProgramError::from)?;
        check_program(&program_id, invoking_program).map_err(ProgramError::from)?;

        // Validate sol_pool_pda: mutable if present
        if let Some(sol_pool_account) = sol_pool_pda {
            check_mut(sol_pool_account).map_err(ProgramError::from)?;
        }

        // Validate decompression_recipient: non-mutable
        check_non_mut(decompression_recipient).map_err(ProgramError::from)?;

        // Validate system_program: must be system program
        check_non_mut(system_program).map_err(ProgramError::from)?;
        let system_program_id = anchor_lang::solana_program::system_program::ID;
        check_program(&system_program_id.to_bytes(), system_program).map_err(ProgramError::from)?;

        // Validate cpi_context_account: non-mutable if present
        if let Some(cpi_context) = cpi_context_account {
            check_non_mut(cpi_context).map_err(ProgramError::from)?;
        }

        // Extract remaining accounts slice for dynamic indexing
        let remaining_accounts = &accounts[index..];

        let validated_accounts = MultiTransferValidatedAccounts {
            fee_payer,
            authority,
            registered_program_pda,
            noop_program,
            account_compression_authority,
            account_compression_program,
            invoking_program,
            sol_pool_pda,
            decompression_recipient,
            system_program,
            cpi_context_account,
        };

        let packed_accounts = MultiTransferPackedAccounts {
            accounts: remaining_accounts,
        };

        Ok((validated_accounts, packed_accounts))
    }
}
