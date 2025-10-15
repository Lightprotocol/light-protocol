use anchor_compressed_token::ErrorCode;
use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_program_profiler::profile;
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};
use spl_pod::solana_msg::msg;

use crate::{
    shared::{
        accounts::{CpiContextLightSystemAccounts, LightSystemAccounts},
        AccountIterator,
    },
    transfer2::config::Transfer2Config,
};

/// 3 Scenarios:
/// 1. Some compressed accounts
/// 2. Some compressed accounts and write to cpi context account.
/// 3. no compressed accounts.
pub struct Transfer2Accounts<'info> {
    //_light_system_program: &'info AccountInfo,
    pub system: Option<LightSystemAccounts<'info>>,
    pub write_to_cpi_context_system: Option<CpiContextLightSystemAccounts<'info>>,
    pub compressions_only_fee_payer: Option<&'info AccountInfo>,
    pub compressions_only_cpi_authority_pda: Option<&'info AccountInfo>,
    /// Contains mint, owner, delegate, merkle tree, and queue accounts
    /// tree and queue accounts come last.
    pub packed_accounts: ProgramPackedAccounts<'info, AccountInfo>,
}

impl<'info> Transfer2Accounts<'info> {
    /// Validate and parse accounts from the instruction accounts slice
    #[profile]
    #[inline(always)]
    pub fn validate_and_parse(
        accounts: &'info [AccountInfo],
        config: &Transfer2Config,
    ) -> Result<Self, ProgramError> {
        let mut iter = AccountIterator::new(accounts);

        if config.no_compressed_accounts {
            let compressions_only_cpi_authority_pda =
                iter.next_account("compressions only cpi authority pda")?;
            let compressions_only_fee_payer = iter.next_signer("compressions only fee payer")?;
            Ok(Transfer2Accounts {
                compressions_only_fee_payer: Some(compressions_only_fee_payer),
                compressions_only_cpi_authority_pda: Some(compressions_only_cpi_authority_pda),
                packed_accounts: ProgramPackedAccounts {
                    accounts: iter.remaining()?,
                },
                system: None,
                write_to_cpi_context_system: None,
            })
        } else if config.cpi_context_write_required {
            // Unused, just for readability and cpi
            let _light_system_program = iter.next_non_mut("light_system_program")?;
            Ok(Transfer2Accounts {
                system: None,
                write_to_cpi_context_system: Some(CpiContextLightSystemAccounts::new(&mut iter)?),
                compressions_only_fee_payer: None,
                compressions_only_cpi_authority_pda: None,
                packed_accounts: ProgramPackedAccounts {
                    accounts: iter.remaining()?,
                },
            })
        } else {
            // Unused, just for readability and cpi
            let _light_system_program = iter.next_non_mut("light_system_program")?;
            let system = LightSystemAccounts::validate_and_parse(
                &mut iter,
                config.sol_pool_required,
                config.sol_decompression_required,
                config.cpi_context_required,
            )?;

            Ok(Transfer2Accounts {
                system: Some(system),
                write_to_cpi_context_system: None,
                compressions_only_fee_payer: None,
                compressions_only_cpi_authority_pda: None,
                packed_accounts: ProgramPackedAccounts {
                    accounts: iter.remaining()?,
                },
            })
        }
    }

    /// Calculate static accounts count after skipping index 0 (system accounts only)
    /// Returns the count of fixed accounts based on optional features
    #[profile]
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
    #[profile]
    #[inline(always)]
    pub fn cpi_accounts(
        &self,
        all_accounts: &'info [AccountInfo],
        packed_accounts: &'info ProgramPackedAccounts<'info, AccountInfo>,
    ) -> Result<(&'info [AccountInfo], Vec<&'info Pubkey>), ProgramError> {
        // Extract tree accounts using highest index approach
        let tree_accounts = extract_tree_accounts(packed_accounts);

        // Calculate static accounts count after skipping index 0 (system accounts only)
        let static_accounts_count = self.static_accounts_count()?;

        // Include static CPI accounts + tree accounts
        let cpi_accounts_end = 1 + static_accounts_count + tree_accounts.len();
        if all_accounts.len() < cpi_accounts_end {
            msg!(
                "Accounts len {} < expected cpi accounts len {}",
                all_accounts.len(),
                cpi_accounts_end
            );
            return Err(ProgramError::NotEnoughAccountKeys);
        }
        // Exclude light system program in index 0.
        let cpi_accounts_slice = &all_accounts[1..cpi_accounts_end];

        Ok((cpi_accounts_slice, tree_accounts))
    }
}

/// Extract tree accounts by finding the highest tree index and using it as closing offset
#[profile]
#[inline(always)]
pub fn extract_tree_accounts<'info>(
    packed_accounts: &'info ProgramPackedAccounts<'info, AccountInfo>,
) -> Vec<&'info Pubkey> {
    let mut tree_accounts = Vec::with_capacity(8);
    for account_info in packed_accounts.accounts {
        // As heuristic which accounts are tree or queue accounts we
        // check that the first 8 bytes of the account compression program
        // equal the first 8 bytes of the account owner.
        if account_info.owner()[0..8] == [9, 44, 54, 236, 34, 245, 23, 131] {
            tree_accounts.push(account_info.key());
        }
    }
    tree_accounts
}
