use anchor_lang::prelude::{AccountInfo, ProgramError};
use anchor_lang::solana_program::pubkey::Pubkey;
use anchor_lang::solana_program::program_pack::IsInitialized;
use light_account_checks::checks::{check_mut, check_non_mut, check_signer};
use spl_pod::bytemuck::pod_from_bytes;
use spl_token_2022::pod::PodMint;

pub struct CreateAssociatedTokenAccountAccounts<'a, 'info> {
    pub fee_payer: &'a AccountInfo<'info>,
    pub associated_token_account: &'a AccountInfo<'info>,
    pub mint: Option<&'a AccountInfo<'info>>,
    pub system_program: &'a AccountInfo<'info>,
}

impl<'a, 'info> CreateAssociatedTokenAccountAccounts<'a, 'info> {
    pub fn new(
        accounts: &'a [AccountInfo<'info>],
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
        accounts: &'a [AccountInfo<'info>],
        mint: &Pubkey,
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
            if *mint_account_info.key != *mint {
                return Err(ProgramError::InvalidAccountData);
            }
            
            // Check if owned by either spl-token or spl-token-2022 program
            if mint_account_info.owner != &spl_token::id() && mint_account_info.owner != &spl_token_2022::id() {
                return Err(ProgramError::IncorrectProgramId);
            }
            
            let mint_data = mint_account_info.try_borrow_data()?;
            let pod_mint = pod_from_bytes::<PodMint>(&mint_data)
                .map_err(|_| ProgramError::InvalidAccountData)?;
            
            if !pod_mint.is_initialized() {
                return Err(ProgramError::UninitializedAccount);
            }
        }

        Ok(accounts_struct)
    }
}
