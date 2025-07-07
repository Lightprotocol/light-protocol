use anchor_lang::prelude::ProgramError;
use anchor_lang::solana_program::program_pack::IsInitialized;
use light_account_checks::{checks::{check_mut, check_non_mut, check_signer}, AccountInfoTrait};
use pinocchio::account_info::AccountInfo;
use spl_pod::bytemuck::pod_from_bytes;
use spl_token_2022::pod::PodMint;

pub struct CreateAssociatedTokenAccountAccounts<'a> {
    pub fee_payer: &'a AccountInfo,
    pub associated_token_account: &'a AccountInfo,
    pub mint: Option<&'a AccountInfo>,
    pub system_program: &'a AccountInfo,
}

impl<'a> CreateAssociatedTokenAccountAccounts<'a> {
    pub fn new(
        accounts: &'a [AccountInfo],
        mint_is_decompressed: bool,
    ) -> Result<Self, ProgramError> {
        let (mint, system_program_index) = if mint_is_decompressed {
            (Some(&accounts[2]), 3)
        } else {
            (None, 2)
        };
        Ok(Self {
            fee_payer: &accounts[0],
            associated_token_account: &accounts[1],
            mint,
            system_program: &accounts[system_program_index],
        })
    }

    pub fn get_checked(
        accounts: &'a [AccountInfo],
        mint: &[u8; 32],
        mint_is_decompressed: bool,
    ) -> Result<Self, ProgramError> {
        let accounts_struct = Self::new(accounts, mint_is_decompressed)?;

        // Basic validations using light_account_checks
        check_signer(accounts_struct.fee_payer)?;
        check_mut(accounts_struct.fee_payer)?;
        check_mut(accounts_struct.associated_token_account)?;
        check_non_mut(accounts_struct.system_program)?;
        // ata derivation is checked implicitly by cpi

        if let Some(mint_account_info) = accounts_struct.mint {
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
        }

        Ok(accounts_struct)
    }
}
