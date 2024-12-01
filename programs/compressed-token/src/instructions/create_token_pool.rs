use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use anchor_lang::prelude::*;
use anchor_spl::{
    token::{Mint, Token, TokenAccount},
    token_2022::Token2022,
    token_interface::{Mint as Mint22, TokenAccount as Token22Account},
};
use spl_token_2022::{
    extension::{BaseStateWithExtensions, ExtensionType, PodStateWithExtensions},
    pod::PodMint,
};

pub const POOL_SEED: &[u8] = b"pool";

/// Creates a spl token pool account which is owned by the token authority pda.
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
    pub token_pool_pda: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    /// CHECK: is mint account.
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    /// CHECK: (seeds anchor constraint).
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub cpi_authority_pda: AccountInfo<'info>,
}

/// Creates a token22 token pool account which is owned by the token authority pda.
#[derive(Accounts)]
pub struct CreateTokenPoolInstruction2022<'info> {
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
    pub token_pool_pda: InterfaceAccount<'info, Token22Account>,
    pub system_program: Program<'info, System>,
    /// CHECK: is mint account.
    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint22>,
    pub token_program: Program<'info, Token2022>,
    /// CHECK: (seeds anchor constraint).
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub cpi_authority_pda: AccountInfo<'info>,
}

pub fn get_token_pool_pda(mint: &Pubkey) -> Pubkey {
    let seeds = &[POOL_SEED, mint.as_ref()];
    let (address, _) = Pubkey::find_program_address(seeds, &crate::ID);
    address
}

// cpi guard could be ok but should be tested
const ALLOWED_EXTENSION_TYPES: [ExtensionType; 6] = [
    ExtensionType::MetadataPointer,
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
