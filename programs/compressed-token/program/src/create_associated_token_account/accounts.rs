use anchor_lang::solana_program::{program_error::ProgramError, program_pack::IsInitialized};
use light_account_checks::{
    checks::{check_mut, check_non_mut, check_signer},
    AccountInfoTrait,
};
use pinocchio::account_info::AccountInfo;
use spl_pod::bytemuck::pod_from_bytes;
use spl_token_2022::pod::PodMint;

use crate::shared::AccountIterator;

pub struct CreateAssociatedTokenAccountAccounts<'info> {
    pub fee_payer: &'info AccountInfo,
    pub associated_token_account: &'info AccountInfo,
    pub mint: Option<&'info AccountInfo>,
    pub system_program: &'info AccountInfo,
}

impl<'info> CreateAssociatedTokenAccountAccounts<'info> {
    pub fn validate_and_parse(
        accounts: &'info [AccountInfo],
        mint: &[u8; 32],
        mint_is_decompressed: bool,
    ) -> Result<Self, ProgramError> {
        let mut iter = AccountIterator::new(accounts);

        let fee_payer = iter.next_account("fee_payer")?;
        let associated_token_account = iter.next_account("associated_token_account")?;
        let mint_account = if mint_is_decompressed {
            let mint_account_info = iter.next_account("mint_account")?;
            if AccountInfoTrait::key(mint_account_info) != *mint {
                return Err(ProgramError::InvalidAccountData);
            }

            // Check if owned by either spl-token or spl-token-2022 program
            let spl_token_id = spl_token::id().to_bytes();
            let spl_token_2022_id = spl_token_2022::id().to_bytes();
            let owner = unsafe { *mint_account_info.owner() };
            if owner != spl_token_id && owner != spl_token_2022_id {
                return Err(ProgramError::IncorrectProgramId);
            }

            let mint_data = AccountInfoTrait::try_borrow_data(mint_account_info)
                .map_err(|_| ProgramError::InvalidAccountData)?;
            let pod_mint = pod_from_bytes::<PodMint>(&mint_data)
                .map_err(|_| ProgramError::InvalidAccountData)?;

            if !pod_mint.is_initialized() {
                return Err(ProgramError::UninitializedAccount);
            }
            Some(mint_account_info)
        } else {
            None
        };
        let system_program = iter.next_account("system_program")?;

        // Basic validations using light_account_checks
        check_signer(fee_payer)?;
        check_mut(fee_payer)?;
        check_mut(associated_token_account)?;
        check_non_mut(system_program)?;

        Ok(CreateAssociatedTokenAccountAccounts {
            fee_payer,
            associated_token_account,
            mint: mint_account,
            system_program,
        })
    }
}
