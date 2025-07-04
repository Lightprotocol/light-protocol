use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use spl_token_2022::{
    extension::{BaseStateWithExtensions, ExtensionType, PodStateWithExtensions},
    pod::PodMint,
};

use crate::{
    constants::{NUM_MAX_POOL_ACCOUNTS, POOL_SEED},
    spl_compression::is_valid_token_pool_pda,
};

/// Creates an SPL or token-2022 token pool account, which is owned by the token authority PDA.
#[derive(Accounts)]
pub struct CreateTokenPoolInstruction<'info> {
    /// UNCHECKED: only pays fees.
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    #[account(
        init,
        seeds = [
        POOL_SEED, &mint.key().to_bytes(),
        ],
        bump,
        payer = fee_payer,
          token::mint = mint,
          token::authority = cpi_authority_pda,
    )]
    pub token_pool_pda: InterfaceAccount<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    /// CHECK: is mint account.
    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    /// CHECK: (seeds anchor constraint).
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub cpi_authority_pda: AccountInfo<'info>,
}

pub fn get_token_pool_pda(mint: &Pubkey) -> Pubkey {
    get_token_pool_pda_with_index(mint, 0)
}

pub fn find_token_pool_pda_with_index(mint: &Pubkey, token_pool_index: u8) -> (Pubkey, u8) {
    let seeds = &[POOL_SEED, mint.as_ref(), &[token_pool_index]];
    let seeds = if token_pool_index == 0 {
        &seeds[..2]
    } else {
        &seeds[..]
    };
    Pubkey::find_program_address(seeds, &crate::ID)
}

pub fn get_token_pool_pda_with_index(mint: &Pubkey, token_pool_index: u8) -> Pubkey {
    find_token_pool_pda_with_index(mint, token_pool_index).0
}

const ALLOWED_EXTENSION_TYPES: [ExtensionType; 7] = [
    ExtensionType::MetadataPointer,
    ExtensionType::TokenMetadata,
    ExtensionType::InterestBearingConfig,
    ExtensionType::GroupPointer,
    ExtensionType::GroupMemberPointer,
    ExtensionType::TokenGroup,
    ExtensionType::TokenGroupMember,
];

pub fn assert_mint_extensions(account_data: &[u8]) -> Result<()> {
    let mint = PodStateWithExtensions::<PodMint>::unpack(account_data).unwrap();
    let mint_extensions = mint.get_extension_types().unwrap();
    if !mint_extensions
        .iter()
        .all(|item| ALLOWED_EXTENSION_TYPES.contains(item))
    {
        return err!(crate::ErrorCode::MintWithInvalidExtension);
    }
    Ok(())
}

/// Creates an SPL or token-2022 token pool account, which is owned by the token authority PDA.
#[derive(Accounts)]
#[instruction(token_pool_index: u8)]
pub struct AddTokenPoolInstruction<'info> {
    /// UNCHECKED: only pays fees.
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    #[account(
        init,
        seeds = [
        POOL_SEED, &mint.key().to_bytes(), &[token_pool_index],
        ],
        bump,
        payer = fee_payer,
          token::mint = mint,
          token::authority = cpi_authority_pda,
    )]
    pub token_pool_pda: InterfaceAccount<'info, TokenAccount>,
    pub existing_token_pool_pda: InterfaceAccount<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    /// CHECK: is mint account.
    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    /// CHECK: (seeds anchor constraint).
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub cpi_authority_pda: AccountInfo<'info>,
}

/// Checks if the token pool PDA is valid.
/// Iterates over all possible bump seeds to check if the token pool PDA is valid.
#[inline(always)]
pub fn check_spl_token_pool_derivation(token_pool_pda: &Pubkey, mint: &Pubkey) -> Result<()> {
    let mint_bytes = mint.to_bytes();
    let is_valid = (0..NUM_MAX_POOL_ACCOUNTS).any(|i| {
        is_valid_token_pool_pda(mint_bytes.as_slice(), token_pool_pda, &[i], None).unwrap_or(false)
    });
    if !is_valid {
        err!(crate::ErrorCode::InvalidTokenPoolPda)
    } else {
        Ok(())
    }
}

#[inline(always)]
pub fn check_spl_token_pool_derivation_with_index(
    token_pool_pda: &Pubkey,
    mint: &Pubkey,
    index: u8,
    bump: Option<u8>,
) -> Result<()> {
    let mint_bytes = mint.to_bytes();
    let is_valid = is_valid_token_pool_pda(mint_bytes.as_slice(), token_pool_pda, &[index], bump)?;
    if !is_valid {
        err!(crate::ErrorCode::InvalidTokenPoolPda)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /// Test:
    /// 1. Functional: test_check_spl_token_pool_derivation
    /// 2. Failing: test_check_spl_token_pool_derivation_invalid_derivation
    /// 3. Failing: test_check_spl_token_pool_derivation_bump_seed_equal_to_num_max_accounts
    /// 4. Failing: test_check_spl_token_pool_derivation_bump_seed_larger_than_num_max_accounts
    #[test]
    fn test_check_spl_token_pool_derivation() {
        // 1. Functional: test_check_spl_token_pool_derivation_valid
        let mint = Pubkey::new_unique();
        for i in 0..NUM_MAX_POOL_ACCOUNTS {
            let valid_pda = get_token_pool_pda_with_index(&mint, i);
            assert!(check_spl_token_pool_derivation(&valid_pda, &mint).is_ok());
        }

        // 2. Failing: test_check_spl_token_pool_derivation_invalid_derivation
        let mint = Pubkey::new_unique();
        let invalid_pda = Pubkey::new_unique();
        assert!(check_spl_token_pool_derivation(&invalid_pda, &mint).is_err());

        // 3. Failing: test_check_spl_token_pool_derivation_bump_seed_equal_to_num_max_accounts
        let mint = Pubkey::new_unique();
        let invalid_pda = get_token_pool_pda_with_index(&mint, NUM_MAX_POOL_ACCOUNTS);
        assert!(check_spl_token_pool_derivation(&invalid_pda, &mint).is_err());

        // 4. Failing: test_check_spl_token_pool_derivation_bump_seed_larger_than_num_max_accounts
        let mint = Pubkey::new_unique();
        let invalid_pda = get_token_pool_pda_with_index(&mint, NUM_MAX_POOL_ACCOUNTS + 1);
        assert!(check_spl_token_pool_derivation(&invalid_pda, &mint).is_err());
    }
}
