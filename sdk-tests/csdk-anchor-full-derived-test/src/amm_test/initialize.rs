//! Initialize instruction with all light account markers.
//!
//! Tests:
//! - 2x #[light_account(init)] (pool_state, observation_state)
//! - 2x #[light_account(token::...)] mark-only mode (token_0_vault, token_1_vault) - manual CreateTokenAccountCpi
//! - 1x #[light_account(init, mint::...)] (lp_mint)
//! - CreateTokenAtaCpi.rent_free()
//! - MintToCpi

use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;
use light_account::{
    CreateAccountsProof, CreateTokenAccountCpi, CreateTokenAtaCpi, LightAccounts,
    LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR,
};
use light_anchor_spl::token_interface::{TokenAccount, TokenInterface};
use light_token::instruction::MintToCpi;

use super::states::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct InitializeParams {
    pub init_amount_0: u64,
    pub init_amount_1: u64,
    pub open_time: u64,
    pub create_accounts_proof: CreateAccountsProof,
    pub lp_mint_signer_bump: u8,
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
    // Mark-only: seeds and owner_seeds only (no mint/owner)
    #[light_account(token::seeds = [POOL_VAULT_SEED.as_bytes(), self.pool_state.key(), self.token_0_mint.key()], token::owner_seeds = [AUTH_SEED.as_bytes()])]
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
    // Mark-only: seeds and owner_seeds only (no mint/owner)
    #[light_account(token::seeds = [POOL_VAULT_SEED.as_bytes(), self.pool_state.key(), self.token_1_mint.key()], token::owner_seeds = [AUTH_SEED.as_bytes()])]
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

    #[account(address = LIGHT_TOKEN_CONFIG)]
    pub light_token_config: AccountInfo<'info>,

    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    pub light_token_program: AccountInfo<'info>,

    /// CHECK: CToken CPI authority.
    pub light_token_cpi_authority: AccountInfo<'info>,
}

/// Initialize instruction handler.
/// Token vaults (token_0_vault, token_1_vault) are manually created via CreateTokenAccountCpi.
pub fn process_initialize_pool<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializePool<'info>>,
    params: InitializeParams,
) -> Result<()> {
    // Create token_0_vault using CreateTokenAccountCpi (mark-only field)
    {
        let payer_info = ctx.accounts.creator.to_account_info();
        let account_info = ctx.accounts.token_0_vault.to_account_info();
        let mint_info = ctx.accounts.token_0_mint.to_account_info();
        let config_info = ctx.accounts.light_token_config.to_account_info();
        let sponsor_info = ctx.accounts.light_token_rent_sponsor.to_account_info();
        let system_info = ctx.accounts.system_program.to_account_info();
        CreateTokenAccountCpi {
            payer: &payer_info,
            account: &account_info,
            mint: &mint_info,
            owner: ctx.accounts.authority.key().to_bytes(),
        }
        .rent_free(
            &config_info,
            &sponsor_info,
            &system_info,
            &crate::ID.to_bytes(),
        )
        .invoke_signed(&[
            POOL_VAULT_SEED.as_bytes(),
            ctx.accounts.pool_state.to_account_info().key.as_ref(),
            ctx.accounts.token_0_mint.to_account_info().key.as_ref(),
            &[ctx.bumps.token_0_vault],
        ])?;
    }

    // Create token_1_vault using CreateTokenAccountCpi (mark-only field)
    {
        let payer_info = ctx.accounts.creator.to_account_info();
        let account_info = ctx.accounts.token_1_vault.to_account_info();
        let mint_info = ctx.accounts.token_1_mint.to_account_info();
        let config_info = ctx.accounts.light_token_config.to_account_info();
        let sponsor_info = ctx.accounts.light_token_rent_sponsor.to_account_info();
        let system_info = ctx.accounts.system_program.to_account_info();
        CreateTokenAccountCpi {
            payer: &payer_info,
            account: &account_info,
            mint: &mint_info,
            owner: ctx.accounts.authority.key().to_bytes(),
        }
        .rent_free(
            &config_info,
            &sponsor_info,
            &system_info,
            &crate::ID.to_bytes(),
        )
        .invoke_signed(&[
            POOL_VAULT_SEED.as_bytes(),
            ctx.accounts.pool_state.to_account_info().key.as_ref(),
            ctx.accounts.token_1_mint.to_account_info().key.as_ref(),
            &[ctx.bumps.token_1_vault],
        ])?;
    }

    // Create creator LP token ATA using CreateTokenAtaCpi.rent_free()
    {
        let payer_info = ctx.accounts.creator.to_account_info();
        let owner_info = ctx.accounts.creator.to_account_info();
        let mint_info = ctx.accounts.lp_mint.to_account_info();
        let ata_info = ctx.accounts.creator_lp_token.to_account_info();
        let config_info = ctx.accounts.light_token_config.to_account_info();
        let sponsor_info = ctx.accounts.light_token_rent_sponsor.to_account_info();
        let system_info = ctx.accounts.system_program.to_account_info();
        CreateTokenAtaCpi {
            payer: &payer_info,
            owner: &owner_info,
            mint: &mint_info,
            ata: &ata_info,
        }
        .idempotent()
        .rent_free(&config_info, &sponsor_info, &system_info)
        .invoke()?;
    }

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
