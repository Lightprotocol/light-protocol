use anchor_lang::solana_program::program_error::ProgramError;
use pinocchio::account_info::AccountInfo;

use crate::shared::AccountIterator;

pub struct CpiContextLightSystemAccounts<'info> {
    pub fee_payer: &'info AccountInfo,
    pub cpi_authority_pda: &'info AccountInfo,
    pub cpi_context: &'info AccountInfo,
}

impl<'info> CpiContextLightSystemAccounts<'info> {
    /// Returns the number of accounts in the CPI context light system accounts slice
    pub const fn cpi_len() -> usize {
        3 // fee_payer, cpi_authority_pda, cpi_context
    }

    #[track_caller]
    #[inline(always)]
    pub fn new(iter: &mut AccountIterator<'info, AccountInfo>) -> Result<Self, ProgramError> {
        Ok(Self {
            fee_payer: iter.next_signer_mut("fee_payer")?,
            cpi_authority_pda: iter.next_account("cpi_authority_pda")?,
            cpi_context: iter.next_account("cpi_context")?,
        })
    }
}

pub struct LightSystemAccounts<'info> {
    /// Fee payer account (index 0) - signer, mutable
    pub fee_payer: &'info AccountInfo,
    /// CPI authority PDA (index 1) - signer (via CPI)
    pub cpi_authority_pda: &'info AccountInfo,
    /// Registered program PDA (index 2) - non-mutable
    pub registered_program_pda: &'info AccountInfo,
    /// Account compression authority (index 4) - non-mutable
    pub account_compression_authority: &'info AccountInfo,
    /// Account compression program (index 5) - non-mutable
    pub account_compression_program: &'info AccountInfo,
    /// System program (index 9) - non-mutable
    pub system_program: &'info AccountInfo,
    /// Sol pool PDA (index 7) - optional, mutable if present
    pub sol_pool_pda: Option<&'info AccountInfo>,
    /// SOL decompression recipient (index 8) - optional, mutable, for SOL decompression
    pub sol_decompression_recipient: Option<&'info AccountInfo>,
    /// CPI context account (index 10) - optional, non-mutable
    pub cpi_context: Option<&'info AccountInfo>,
}

impl<'info> LightSystemAccounts<'info> {
    /// Returns the number of required accounts in the light system accounts slice (excludes optional accounts)
    pub const fn cpi_len() -> usize {
        6 // fee_payer, cpi_authority_pda, registered_program_pda, account_compression_authority, account_compression_program, system_program
    }

    #[track_caller]
    pub fn validate_and_parse(
        iter: &mut AccountIterator<'info, AccountInfo>,
        with_sol_pool: bool,
        decompress_sol: bool,
        with_cpi_context: bool,
    ) -> Result<Self, ProgramError> {
        Ok(Self {
            fee_payer: iter.next_signer_mut("fee_payer")?,
            cpi_authority_pda: iter.next_non_mut("cpi_authority_pda")?,
            registered_program_pda: iter.next_non_mut("registered_program_pda")?,
            account_compression_authority: iter.next_non_mut("account_compression_authority")?,
            account_compression_program: iter.next_non_mut("account_compression_program")?,
            system_program: iter.next_non_mut("system_program")?,
            sol_pool_pda: iter.next_option("sol_pool_pda", with_sol_pool)?,
            sol_decompression_recipient: iter
                .next_option("sol_decompression_recipient", decompress_sol)?,
            cpi_context: iter.next_option_mut("cpi_context", with_cpi_context)?,
        })
    }
}
