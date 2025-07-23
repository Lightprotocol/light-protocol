use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::checks::{check_mut, check_signer};
use pinocchio::account_info::AccountInfo;

use crate::shared::AccountIterator;

pub struct LightSystemAccounts<'info> {
    pub fee_payer: &'info AccountInfo,
    pub cpi_authority_pda: &'info AccountInfo,
    pub registered_program_pda: &'info AccountInfo,
    pub noop_program: &'info AccountInfo,
    pub account_compression_authority: &'info AccountInfo,
    pub account_compression_program: &'info AccountInfo,
    pub system_program: &'info AccountInfo,
    pub self_program: &'info AccountInfo,
}

impl<'info> LightSystemAccounts<'info> {
    pub fn validate_and_parse(iter: &mut AccountIterator<'info>) -> Result<Self, ProgramError> {
        let fee_payer: &AccountInfo = iter.next_account()?;
        // Validate fee_payer: must be signer and mutable
        check_signer(fee_payer).map_err(ProgramError::from)?;
        check_mut(fee_payer).map_err(ProgramError::from)?;

        Ok(Self {
            fee_payer,
            cpi_authority_pda: iter.next_account()?,
            registered_program_pda: iter.next_account()?,
            noop_program: iter.next_account()?,
            account_compression_authority: iter.next_account()?,
            account_compression_program: iter.next_account()?,
            system_program: iter.next_account()?,
            self_program: iter.next_account()?,
        })
    }
}

pub struct UpdateOneCompressedAccountTreeAccounts<'info> {
    pub in_merkle_tree: &'info AccountInfo,
    pub in_output_queue: &'info AccountInfo,
    pub out_output_queue: &'info AccountInfo,
}

impl<'info> UpdateOneCompressedAccountTreeAccounts<'info> {
    pub fn validate_and_parse(iter: &mut AccountIterator<'info>) -> Result<Self, ProgramError> {
        let in_merkle_tree = iter.next_account()?;
        let in_output_queue = iter.next_account()?;
        let out_output_queue = iter.next_account()?;
        check_mut(in_merkle_tree).map_err(ProgramError::from)?;
        check_mut(in_output_queue).map_err(ProgramError::from)?;
        check_mut(out_output_queue).map_err(ProgramError::from)?;

        Ok(Self {
            in_merkle_tree,
            in_output_queue,
            out_output_queue,
        })
    }
}
