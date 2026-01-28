//! Initialize instruction with all light account markers.
//!
//! Tests:
//! - 2x #[light_account(init)] (pool_state, observation_state)
//! - 2x #[light_account(token, authority = [...])] (token_0_vault, token_1_vault)
//! - 1x #[light_account(init, mint,...)] (lp_mint)
//! - CreateTokenAccountCpi.rent_free()
//! - CreateTokenAtaCpi.rent_free()
//! - MintToCpi

use anchor_lang::prelude::*;
use light_anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use light_compressible::CreateAccountsProof;
use light_sdk_macros::LightAccounts;
use light_token::instruction::{
    CreateTokenAccountCpi, CreateTokenAtaCpi, MintToCpi, COMPRESSIBLE_CONFIG_V1,
    RENT_SPONSOR as LIGHT_TOKEN_RENT_SPONSOR,
};

use super::states::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct InitializeParams {
    pub init_amount_0: u64,
    pub init_amount_1: u64,
    pub open_time: u64,
    pub create_accounts_proof: CreateAccountsProof,
    pub lp_mint_signer_bump: u8,
    pub creator_lp_token_bump: u8,
    pub authority_bump: u8,
}

#[derive(Accounts, LightAccounts)]
#[instruction(params: InitializeParams)]
pub struct InitializePool<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    /// CHECK: AMM config account
    pub amm_config: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [AUTH_SEED.as_bytes()],
        bump,
    )]
    pub authority: UncheckedAccount<'info>,

    #[account(
        init,
        seeds = [
            POOL_SEED.as_bytes(),
            amm_config.key().as_ref(),
            token_0_mint.key().as_ref(),
            token_1_mint.key().as_ref(),
        ],
        bump,
        payer = creator,
        space = 8 + PoolState::INIT_SPACE
    )]
    #[light_account(init)]
    pub pool_state: Box<Account<'info, PoolState>>,

    #[account(
        constraint = token_0_mint.key() < token_1_mint.key(),
        mint::token_program = token_0_program,
    )]
    pub token_0_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mint::token_program = token_1_program)]
    pub token_1_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        seeds = [POOL_LP_MINT_SIGNER_SEED, pool_state.key().as_ref()],
        bump,
    )]
    pub lp_mint_signer: UncheckedAccount<'info>, // TODO: check where the cpi gets the seeds from

    #[account(mut)]
    #[light_account(init,
        mint::signer = lp_mint_signer,
        mint::authority = authority,
        mint::decimals = 9,
        mint::seeds = &[POOL_LP_MINT_SIGNER_SEED, self.pool_state.to_account_info().key.as_ref()],
        mint::bump = params.lp_mint_signer_bump,
        mint::authority_seeds = &[AUTH_SEED.as_bytes()],
        mint::authority_bump = params.authority_bump
    )]
    pub lp_mint: UncheckedAccount<'info>,

    #[account(
        mut,
        token::mint = token_0_mint,
        token::authority = creator,
    )]
    pub creator_token_0: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = token_1_mint,
        token::authority = creator,
    )]
    pub creator_token_1: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut)]
    pub creator_lp_token: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [
            POOL_VAULT_SEED.as_bytes(),
            pool_state.key().as_ref(),
            token_0_mint.key().as_ref()
        ],
        bump,
    )]
    #[light_account(token::authority = [AUTH_SEED.as_bytes()])]
    pub token_0_vault: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [
            POOL_VAULT_SEED.as_bytes(),
            pool_state.key().as_ref(),
            token_1_mint.key().as_ref()
        ],
        bump,
    )]
    #[light_account(token::authority = [AUTH_SEED.as_bytes()])]
    pub token_1_vault: UncheckedAccount<'info>,

    #[account(
        init,
        seeds = [OBSERVATION_SEED.as_bytes(), pool_state.key().as_ref()],
        bump,
        payer = creator,
        space = 8 + ObservationState::INIT_SPACE
    )]
    #[light_account(init)]
    pub observation_state: Box<Account<'info, ObservationState>>,

    pub token_program: Interface<'info, TokenInterface>,
    pub token_0_program: Interface<'info, TokenInterface>,
    pub token_1_program: Interface<'info, TokenInterface>,
    /// CHECK: Associated token program (SPL ATA or Light Token).
    pub associated_token_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,

    pub compression_config: AccountInfo<'info>,

    /// CHECK: PDA rent sponsor for reimbursement
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    #[account(address = COMPRESSIBLE_CONFIG_V1)]
    pub light_token_compressible_config: AccountInfo<'info>,

    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub rent_sponsor: AccountInfo<'info>,

    pub light_token_program: AccountInfo<'info>,

    /// CHECK: CToken CPI authority.
    pub light_token_cpi_authority: AccountInfo<'info>,
}

/// Initialize instruction handler (noop for compilation test).
pub fn process_initialize_pool<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializePool<'info>>,
    params: InitializeParams,
) -> Result<()> {
    let pool_state_key = ctx.accounts.pool_state.key();

    // Create token_0 vault using CreateTokenAccountCpi.rent_free()
    CreateTokenAccountCpi {
        payer: ctx.accounts.creator.to_account_info(),
        account: ctx.accounts.token_0_vault.to_account_info(),
        mint: ctx.accounts.token_0_mint.to_account_info(),
        owner: ctx.accounts.authority.key(),
    }
    .rent_free(
        ctx.accounts
            .light_token_compressible_config
            .to_account_info(),
        ctx.accounts.rent_sponsor.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
        &crate::ID,
    )
    .invoke_signed(&[
        POOL_VAULT_SEED.as_bytes(),
        pool_state_key.as_ref(),
        ctx.accounts.token_0_mint.key().as_ref(),
        &[ctx.bumps.token_0_vault],
    ])?;

    // Create token_1 vault using CreateTokenAccountCpi.rent_free()
    CreateTokenAccountCpi {
        payer: ctx.accounts.creator.to_account_info(),
        account: ctx.accounts.token_1_vault.to_account_info(),
        mint: ctx.accounts.token_1_mint.to_account_info(),
        owner: ctx.accounts.authority.key(),
    }
    .rent_free(
        ctx.accounts
            .light_token_compressible_config
            .to_account_info(),
        ctx.accounts.rent_sponsor.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
        &crate::ID,
    )
    .invoke_signed(&[
        POOL_VAULT_SEED.as_bytes(),
        pool_state_key.as_ref(),
        ctx.accounts.token_1_mint.key().as_ref(),
        &[ctx.bumps.token_1_vault],
    ])?;

    // Create creator LP token ATA using CreateTokenAtaCpi.rent_free()
    CreateTokenAtaCpi {
        payer: ctx.accounts.creator.to_account_info(),
        owner: ctx.accounts.creator.to_account_info(),
        mint: ctx.accounts.lp_mint.to_account_info(),
        ata: ctx.accounts.creator_lp_token.to_account_info(),
        bump: params.creator_lp_token_bump,
    }
    .idempotent()
    .rent_free(
        ctx.accounts
            .light_token_compressible_config
            .to_account_info(),
        ctx.accounts.rent_sponsor.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
    )
    .invoke()?;

    // Mint LP tokens using MintToCpi
    let lp_amount = 1000u64; // Placeholder amount
    MintToCpi {
        mint: ctx.accounts.lp_mint.to_account_info(),
        destination: ctx.accounts.creator_lp_token.to_account_info(),
        amount: lp_amount,
        authority: ctx.accounts.authority.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
        max_top_up: None,
        fee_payer: None,
    }
    .invoke_signed(&[&[AUTH_SEED.as_bytes(), &[ctx.bumps.authority]]])?;

    // Populate pool state
    let pool_state = &mut ctx.accounts.pool_state;
    pool_state.amm_config = ctx.accounts.amm_config.key();
    pool_state.pool_creator = ctx.accounts.creator.key();
    pool_state.token_0_vault = ctx.accounts.token_0_vault.key();
    pool_state.token_1_vault = ctx.accounts.token_1_vault.key();
    pool_state.lp_mint = ctx.accounts.lp_mint.key();
    pool_state.token_0_mint = ctx.accounts.token_0_mint.key();
    pool_state.token_1_mint = ctx.accounts.token_1_mint.key();
    pool_state.token_0_program = ctx.accounts.token_0_program.key();
    pool_state.token_1_program = ctx.accounts.token_1_program.key();
    pool_state.observation_key = ctx.accounts.observation_state.key();
    pool_state.auth_bump = ctx.bumps.authority;
    pool_state.status = 1; // Active
    pool_state.lp_mint_decimals = 9;
    pool_state.mint_0_decimals = 9;
    pool_state.mint_1_decimals = 9;
    pool_state.lp_supply = lp_amount;
    pool_state.open_time = params.open_time;

    Ok(())
}
