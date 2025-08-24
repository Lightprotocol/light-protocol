use anchor_compressed_token::ErrorCode;
use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_ctoken_types::instructions::transfer2::ZCompressedTokenInstructionDataTransfer2;
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};
use spl_pod::solana_msg::msg;

use crate::{
    shared::{
        accounts::{CpiContextLightSystemAccounts, LightSystemAccounts},
        AccountIterator,
    },
    transfer2::config::Transfer2Config,
};

pub struct Transfer2Accounts<'info> {
    //_light_system_program: &'info AccountInfo,
    pub system: Option<LightSystemAccounts<'info>>,
    pub write_to_cpi_context_system: Option<CpiContextLightSystemAccounts<'info>>,
    pub decompressed_only_cpi_authority_pda: Option<&'info AccountInfo>,
    /// Contains mint, owner, delegate, merkle tree, and queue accounts
    /// tree and queue accounts come last.
    pub packed_accounts: ProgramPackedAccounts<'info, AccountInfo>,
}

impl<'info> Transfer2Accounts<'info> {
    /// Validate and parse accounts from the instruction accounts slice
    pub fn validate_and_parse(
        accounts: &'info [AccountInfo],
        config: &Transfer2Config,
    ) -> Result<Self, ProgramError> {
        let mut iter = AccountIterator::new(accounts);
        // Unused, just for readability
        let _light_system_program =
            iter.next_option("light_system_program", !config.no_compressed_accounts)?;
        let system = if config.cpi_context_write_required || config.no_compressed_accounts {
            None
        } else {
            Some(LightSystemAccounts::validate_and_parse(
                &mut iter,
                config.sol_pool_required,
                config.sol_decompression_required,
                config.cpi_context_required,
            )?)
        };
        let write_to_cpi_context_system =
            if config.cpi_context_write_required && !config.no_compressed_accounts {
                Some(CpiContextLightSystemAccounts::validate_and_parse(
                    &mut iter,
                )?)
            } else {
                None
            };
        let decompressed_only_cpi_authority_pda =
            iter.next_option("cpi authority pda", config.no_compressed_accounts)?;
        // Extract remaining accounts slice for dynamic indexing
        let packed_accounts = iter.remaining()?;
        Ok(Transfer2Accounts {
            system,
            write_to_cpi_context_system,
            decompressed_only_cpi_authority_pda,
            packed_accounts: ProgramPackedAccounts {
                accounts: packed_accounts,
            },
        })
    }

    /// Calculate static accounts count after skipping index 0 (system accounts only)
    /// Returns the count of fixed accounts based on optional features
    #[inline(always)]
    pub fn static_accounts_count(&self) -> Result<usize, ProgramError> {
        let system = self
            .system
            .as_ref()
            .ok_or(ErrorCode::Transfer2CpiContextWriteInvalidAccess)?;

        let with_sol_pool = system.sol_pool_pda.is_some();
        let decompressing_sol = system.sol_decompression_recipient.is_some();
        let with_cpi_context = system.cpi_context.is_some();

        Ok(6 + if with_sol_pool { 1 } else { 0 }
            + if decompressing_sol { 1 } else { 0 }
            + if with_cpi_context { 1 } else { 0 })
    }

    /// Extract CPI accounts slice for light-system-program invocation
    /// Includes static accounts + tree accounts based on highest tree index
    /// Returns (cpi_accounts_slice, tree_accounts)
    #[inline(always)]
    pub fn cpi_accounts(
        &self,
        all_accounts: &'info [AccountInfo],
        inputs: &ZCompressedTokenInstructionDataTransfer2,
        packed_accounts: &'info ProgramPackedAccounts<'info, AccountInfo>,
    ) -> Result<(&'info [AccountInfo], Vec<&'info Pubkey>), ProgramError> {
        // Extract tree accounts using highest index approach
        let (tree_accounts, tree_accounts_count) = extract_tree_accounts(inputs, packed_accounts)?;

        // Calculate static accounts count after skipping index 0 (system accounts only)
        let static_accounts_count = self.static_accounts_count()?;

        // Include static CPI accounts + tree accounts based on highest tree index
        let cpi_accounts_end = 1 + static_accounts_count + tree_accounts_count;
        if all_accounts.len() < cpi_accounts_end {
            msg!(
                "Accounts len {} < expected cpi accounts len {}",
                all_accounts.len(),
                cpi_accounts_end
            );
            return Err(ProgramError::NotEnoughAccountKeys);
        }
        let cpi_accounts_slice = &all_accounts[1..cpi_accounts_end];

        Ok((cpi_accounts_slice, tree_accounts))
    }
}

// TODO: unit test.
/// Extract tree accounts by finding the highest tree index and using it as closing offset
pub fn extract_tree_accounts<'info>(
    inputs: &ZCompressedTokenInstructionDataTransfer2,
    packed_accounts: &'info ProgramPackedAccounts<'info, AccountInfo>,
) -> Result<(Vec<&'info Pubkey>, usize), ProgramError> {
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
    let tree_accounts_count = highest_tree_index + 1;
    // Extract tree account pubkeys from the determined range
    // Note: Don't switch to ArrayVec it results in weird memory access with non deterministic values.
    let mut tree_accounts = Vec::with_capacity(tree_accounts_count.into());
    for i in 0..tree_accounts_count {
        let account_key = packed_accounts.get_u8(i, "tree account")?.key();
        tree_accounts.push(account_key);
    }

    Ok((tree_accounts, tree_accounts_count.into()))
}
