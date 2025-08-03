use anchor_lang::solana_program::program_error::ProgramError;
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey};

use crate::shared::AccountIterator;

pub struct CpiContextLightSystemAccounts<'info> {
    pub fee_payer: &'info AccountInfo,
    pub cpi_authority_pda: &'info AccountInfo,
    pub cpi_context: &'info AccountInfo,
}

impl<'info> CpiContextLightSystemAccounts<'info> {
    #[track_caller]
    #[inline(always)]
    pub fn validate_and_parse(
        iter: &mut AccountIterator<'info, AccountInfo>,
    ) -> Result<Self, ProgramError> {
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
    #[track_caller]
    pub fn validate_and_parse(
        iter: &mut AccountIterator<'info, AccountInfo>,
        with_sol_pool: bool,
        decompress_sol: bool,
        with_cpi_context: bool,
    ) -> Result<Self, ProgramError> {
        Ok(Self {
            fee_payer: iter.next_signer_mut("fee_payer")?,
            cpi_authority_pda: iter.next_account("cpi_authority_pda")?,
            registered_program_pda: iter.next_account("registered_program_pda")?,
            account_compression_authority: iter.next_account("account_compression_authority")?,
            account_compression_program: iter.next_account("account_compression_program")?,
            system_program: iter.next_account("system_program")?,
            sol_pool_pda: iter.next_option("sol_pool_pda", with_sol_pool)?,
            sol_decompression_recipient: iter
                .next_option("sol_decompression_recipient", decompress_sol)?,
            cpi_context: iter.next_option_mut("cpi_context", with_cpi_context)?,
        })
    }
}

pub struct UpdateOneCompressedAccountTreeAccounts<'info> {
    pub in_merkle_tree: &'info AccountInfo,
    pub in_output_queue: &'info AccountInfo,
    pub out_output_queue: &'info AccountInfo,
}

impl<'info> UpdateOneCompressedAccountTreeAccounts<'info> {
    #[track_caller]
    pub fn validate_and_parse(
        iter: &mut AccountIterator<'info, AccountInfo>,
    ) -> Result<Self, ProgramError> {
        let in_merkle_tree = iter.next_mut("in_merkle_tree")?;
        let in_output_queue = iter.next_mut("in_output_queue")?;
        let out_output_queue = iter.next_mut("out_output_queue")?;

        Ok(Self {
            in_merkle_tree,
            in_output_queue,
            out_output_queue,
        })
    }

    #[inline(always)]
    pub fn pubkeys(&self) -> [&'info Pubkey; 3] {
        [
            self.in_merkle_tree.key(),
            self.in_output_queue.key(),
            self.out_output_queue.key(),
        ]
    }
}

pub struct CreateCompressedAccountTreeAccounts<'info> {
    pub address_merkle_tree: &'info AccountInfo,
    pub out_output_queue: &'info AccountInfo,
}

impl<'info> CreateCompressedAccountTreeAccounts<'info> {
    #[track_caller]
    pub fn validate_and_parse(
        iter: &mut AccountIterator<'info, AccountInfo>,
    ) -> Result<Self, ProgramError> {
        let address_merkle_tree = iter.next_mut("address_merkle_tree")?;
        let out_output_queue = iter.next_mut("out_output_queue")?;
        Ok(Self {
            address_merkle_tree,
            out_output_queue,
        })
    }

    #[inline(always)]
    pub fn pubkeys(&self) -> [&'info Pubkey; 2] {
        [self.address_merkle_tree.key(), self.out_output_queue.key()]
    }
}
