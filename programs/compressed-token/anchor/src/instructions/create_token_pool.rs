use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{TokenAccount, TokenInterface};
use light_ctoken_interface::{is_restricted_extension, ALLOWED_EXTENSION_TYPES};
use spl_token_2022::{
    extension::{
        transfer_fee::TransferFeeConfig, transfer_hook::TransferHook, BaseStateWithExtensions,
        ExtensionType, PodStateWithExtensions,
    },
    pod::PodMint,
};

use crate::{
    constants::{NUM_MAX_POOL_ACCOUNTS, POOL_SEED, RESTRICTED_POOL_SEED},
    spl_compression::is_valid_token_pool_pda,
};

/// Returns RESTRICTED_POOL_SEED if mint has restricted extensions, empty vec otherwise.
/// For mints with restricted extensions (Pausable, PermanentDelegate, TransferFeeConfig, TransferHook),
/// returns the restricted seed to include in PDA derivation.
pub fn restricted_seed(mint: &AccountInfo) -> Vec<u8> {
    let mint_data = mint.try_borrow_data().unwrap();
    let has_restricted =
        if let Ok(mint_state) = PodStateWithExtensions::<PodMint>::unpack(&mint_data) {
            mint_state
                .get_extension_types()
                .unwrap_or_default()
                .iter()
                .any(is_restricted_extension)
        } else {
            false
        };

    if has_restricted {
        RESTRICTED_POOL_SEED.to_vec()
    } else {
        vec![]
    }
}

/// Creates an SPL or token-2022 token pool account, which is owned by the token authority PDA.
/// We use manual token account initialization via CPI instead of Anchor's `token::mint` constraint
/// because Anchor's constraint internally deserializes the mint account, which fails for Token 2022
/// mints with variable-length extensions like ConfidentialTransferMint.
#[derive(Accounts)]
pub struct CreateTokenPoolInstruction<'info> {
    /// UNCHECKED: only pays fees.
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    /// CHECK: Token pool account. Initialized manually via CPI because Anchor's token::mint
    /// constraint cannot handle Token 2022 mints with variable-length extensions.
    #[account(
        init,
        seeds = [POOL_SEED, &mint.key().to_bytes(), restricted_seed(&mint).as_slice()],
        bump,
        payer = fee_payer,
        space = get_token_account_space(&mint)?,
        owner = token_program.key(),
    )]
    pub token_pool_pda: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: Mint account. We use AccountInfo instead of InterfaceAccount<Mint> because
    /// Anchor's InterfaceAccount cannot deserialize Token 2022 mints with variable-length
    /// extensions like ConfidentialTransferMint. The mint is validated manually using
    /// PodStateWithExtensions<PodMint>::unpack() in assert_mint_extensions().
    #[account(owner = token_program.key())]
    pub mint: AccountInfo<'info>,
    pub token_program: Interface<'info, TokenInterface>,
    /// CHECK: (seeds anchor constraint).
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub cpi_authority_pda: AccountInfo<'info>,
}

/// Calculates the space needed for a token account based on the mint's extensions.
/// Uses `get_required_init_account_extensions` to map mint extensions to required token account extensions.
pub fn get_token_account_space(mint: &AccountInfo) -> Result<usize> {
    let mint_data = mint.try_borrow_data()?;
    let mint_state = PodStateWithExtensions::<PodMint>::unpack(&mint_data)
        .map_err(|_| crate::ErrorCode::InvalidMint)?;
    let mint_extensions = mint_state.get_extension_types().unwrap_or_default();
    let account_extensions = ExtensionType::get_required_init_account_extensions(&mint_extensions);
    ExtensionType::try_calculate_account_len::<spl_token_2022::state::Account>(&account_extensions)
        .map_err(|_| crate::ErrorCode::InvalidMint.into())
}

/// Initializes a token account via CPI to the token program.
pub fn initialize_token_account<'info>(
    token_account: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
) -> Result<()> {
    let ix = spl_token_2022::instruction::initialize_account3(
        token_program.key,
        token_account.key,
        mint.key,
        authority.key,
    )?;
    anchor_lang::solana_program::program::invoke(
        &ix,
        &[
            token_account.clone(),
            mint.clone(),
            authority.clone(),
            token_program.clone(),
        ],
    )?;
    Ok(())
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

pub fn assert_mint_extensions(account_data: &[u8]) -> Result<()> {
    let mint = PodStateWithExtensions::<PodMint>::unpack(account_data)
        .map_err(|_| crate::ErrorCode::InvalidMint)?;
    let mint_extensions = mint.get_extension_types().unwrap_or_default();

    // Check all extensions are in the allowed list
    if !mint_extensions
        .iter()
        .all(|item| ALLOWED_EXTENSION_TYPES.contains(item))
    {
        return err!(crate::ErrorCode::MintWithInvalidExtension);
    }

    // TransferFeeConfig: fees must be zero
    if let Ok(transfer_fee_config) = mint.get_extension::<TransferFeeConfig>() {
        let older_fee = &transfer_fee_config.older_transfer_fee;
        let newer_fee = &transfer_fee_config.newer_transfer_fee;
        if u16::from(older_fee.transfer_fee_basis_points) != 0
            || u64::from(older_fee.maximum_fee) != 0
            || u16::from(newer_fee.transfer_fee_basis_points) != 0
            || u64::from(newer_fee.maximum_fee) != 0
        {
            return err!(crate::ErrorCode::NonZeroTransferFeeNotSupported);
        }
    }

    // TransferHook: program_id must be nil
    if let Ok(transfer_hook) = mint.get_extension::<TransferHook>() {
        if Option::<spl_token_2022::solana_program::pubkey::Pubkey>::from(transfer_hook.program_id)
            .is_some()
        {
            return err!(crate::ErrorCode::TransferHookNotSupported);
        }
    }

    Ok(())
}

/// Creates an additional SPL or token-2022 token pool account, which is owned by the token authority PDA.
/// We use manual token account initialization via CPI instead of Anchor's `token::mint` constraint
/// because Anchor's constraint internally deserializes the mint account, which fails for Token 2022
/// mints with variable-length extensions like ConfidentialTransferMint.
#[derive(Accounts)]
#[instruction(token_pool_index: u8)]
pub struct AddTokenPoolInstruction<'info> {
    /// UNCHECKED: only pays fees.
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    /// CHECK: Token pool account. Initialized manually via CPI because Anchor's token::mint
    /// constraint cannot handle Token 2022 mints with variable-length extensions.
    #[account(
        init,
        seeds = [POOL_SEED, &mint.key().to_bytes(), &[token_pool_index]],
        bump,
        payer = fee_payer,
        space = get_token_account_space(&mint)?,
        owner = token_program.key(),
    )]
    pub token_pool_pda: AccountInfo<'info>,
    pub existing_token_pool_pda: InterfaceAccount<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    /// CHECK: Mint account. We use AccountInfo instead of InterfaceAccount<Mint> because
    /// Anchor's InterfaceAccount cannot deserialize Token 2022 mints with variable-length
    /// extensions like ConfidentialTransferMint. The mint is validated manually using
    /// PodStateWithExtensions<PodMint>::unpack() in assert_mint_extensions().
    #[account(owner = token_program.key())]
    pub mint: AccountInfo<'info>,
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
