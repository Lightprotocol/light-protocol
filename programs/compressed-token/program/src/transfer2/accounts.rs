use anchor_lang::solana_program::program_error::ProgramError;
use light_ctoken_types::instructions::transfer2::ZCompressedTokenInstructionDataTransfer2;
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};
use spl_pod::solana_msg::msg;

use crate::shared::{
    accounts::{CpiContextLightSystemAccounts, LightSystemAccounts},
    AccountIterator,
};

pub struct Transfer2Accounts<'info> {
    pub light_system_program: &'info AccountInfo,
    pub system: Option<LightSystemAccounts<'info>>,
    pub write_to_cpi_context_system: Option<CpiContextLightSystemAccounts<'info>>,
    /// Contains mint, owner, delegate, merkle tree, and queue accounts
    /// tree and queue accounts come last.
    pub packed_accounts: ProgramPackedAccounts<'info>,
}

/// Dynamic accounts slice for index-based access
/// Contains mint, owner, delegate, merkle tree, and queue accounts
pub struct ProgramPackedAccounts<'info> {
    /// Packed accounts slice starting at index 11
    pub accounts: &'info [AccountInfo],
}

impl ProgramPackedAccounts<'_> {
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

impl<'info> Transfer2Accounts<'info> {
    /// Validate and parse accounts from the instruction accounts slice
    pub fn validate_and_parse(
        accounts: &'info [AccountInfo],
        with_sol_pool: bool,
        decompress_sol: bool,
        with_cpi_context: bool,
        write_cpi_context: bool,
    ) -> Result<Self, ProgramError> {
        let mut iter = AccountIterator::new(accounts);
        // Unusedjust for readability
        let light_system_program = iter.next_account("light_system_program")?;
        let system = if write_cpi_context {
            None
        } else {
            Some(LightSystemAccounts::validate_and_parse(
                &mut iter,
                with_sol_pool,
                decompress_sol,
                with_cpi_context,
            )?)
        };
        let write_to_cpi_context_system = if write_cpi_context {
            Some(CpiContextLightSystemAccounts::validate_and_parse(
                &mut iter,
            )?)
        } else {
            None
        };
        // Extract remaining accounts slice for dynamic indexing
        let packed_accounts = iter.remaining()?;
        Ok(Transfer2Accounts {
            light_system_program,
            system,
            write_to_cpi_context_system,
            packed_accounts: ProgramPackedAccounts {
                accounts: packed_accounts,
            },
        })
    }

    /// Calculate static accounts count after skipping index 0 (system accounts only)
    /// Returns the count of fixed accounts based on optional features
    #[inline(always)]
    pub fn static_accounts_count(&self) -> usize {
        // TODO: remove unwrap
        let with_sol_pool = self.system.as_ref().unwrap().sol_pool_pda.is_some();
        let decompressing_sol = self
            .system
            .as_ref()
            .unwrap()
            .sol_decompression_recipient
            .is_some();
        let with_cpi_context = self.system.as_ref().unwrap().cpi_context.is_some();
        6 + if with_sol_pool { 1 } else { 0 }
            + if decompressing_sol { 1 } else { 0 }
            + if with_cpi_context { 1 } else { 0 }
    }

    /// Extract CPI accounts slice for light-system-program invocation
    /// Includes static accounts + tree accounts based on highest tree index
    /// Returns (cpi_accounts_slice, tree_accounts)
    #[inline(always)]
    pub fn cpi_accounts(
        &self,
        all_accounts: &'info [AccountInfo],
        inputs: &ZCompressedTokenInstructionDataTransfer2,
        packed_accounts: &'info ProgramPackedAccounts<'info>,
    ) -> (&'info [AccountInfo], Vec<&'info Pubkey>) {
        // Extract tree accounts using highest index approach
        let (tree_accounts, tree_accounts_count) = extract_tree_accounts(inputs, packed_accounts);

        // Calculate static accounts count after skipping index 0 (system accounts only)
        let static_accounts_count = self.static_accounts_count();

        // Include static CPI accounts + tree accounts based on highest tree index
        let cpi_accounts_end = 1 + static_accounts_count + tree_accounts_count;
        let cpi_accounts_slice = &all_accounts[1..cpi_accounts_end];

        (cpi_accounts_slice, tree_accounts)
    }
}

// TODO: unit test.
/// Extract tree accounts by finding the highest tree index and using it as closing offset
pub fn extract_tree_accounts<'info>(
    inputs: &ZCompressedTokenInstructionDataTransfer2,
    packed_accounts: &'info ProgramPackedAccounts<'info>,
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
    msg!("Tree accounts count: {}", tree_accounts_count);
    // Extract tree account pubkeys from the determined range
    let mut tree_accounts = Vec::new();
    for i in 0..tree_accounts_count {
        if let Some(account) = packed_accounts.accounts.get(i) {
            tree_accounts.push(account.key());
        }
    }

    (tree_accounts, tree_accounts_count)
}
