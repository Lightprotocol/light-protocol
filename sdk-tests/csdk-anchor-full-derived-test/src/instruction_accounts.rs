use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::RentFree;

use crate::state::*;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct FullAutoWithMintParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
    pub category_id: u64,
    pub session_id: u64,
    pub mint_signer_bump: u8,
    pub vault_bump: u8,
    pub user_ata_bump: u8,
    pub vault_mint_amount: u64,
    pub user_ata_mint_amount: u64,
}

pub const LP_MINT_SIGNER_SEED: &[u8] = b"lp_mint_signer";
pub const AUTO_VAULT_SEED: &[u8] = b"auto_vault";
pub const AUTO_VAULT_AUTHORITY_SEED: &[u8] = b"auto_vault_authority";

#[derive(Accounts, RentFree)]
#[instruction(params: FullAutoWithMintParams)]
pub struct CreatePdasAndMintAuto<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    pub authority: Signer<'info>,

    #[account(mut)]
    pub mint_authority: Signer<'info>,

    /// CHECK: PDA derived from authority
    #[account(
        seeds = [LP_MINT_SIGNER_SEED, authority.key().as_ref()],
        bump,
    )]
    pub mint_signer: UncheckedAccount<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + UserRecord::INIT_SPACE,
        seeds = [
            b"user_record",
            authority.key().as_ref(),
            mint_authority.key().as_ref(),
            params.owner.as_ref(),
            params.category_id.to_le_bytes().as_ref()
        ],
        bump,
    )]
    #[rentfree]
    pub user_record: Account<'info, UserRecord>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + GameSession::INIT_SPACE,
        seeds = [
            b"game_session",
            crate::max_key(&fee_payer.key(), &authority.key()).as_ref(),
            params.session_id.to_le_bytes().as_ref()
        ],
        bump,
    )]
    #[rentfree]
    pub game_session: Account<'info, GameSession>,

    /// CHECK: Initialized by mint_action
    #[account(mut)]
    #[light_mint(
        mint_signer = mint_signer,
        authority = mint_authority,
        decimals = 9,
        signer_seeds = &[LP_MINT_SIGNER_SEED, self.authority.to_account_info().key.as_ref(), &[params.mint_signer_bump]]
    )]
    pub cmint: UncheckedAccount<'info>,

    /// CHECK: Initialized via CToken CPI
    #[account(
        mut,
        seeds = [VAULT_SEED, cmint.key().as_ref()],
        bump,
    )]
    #[rentfree_token(Vault, authority = [b"vault_authority"])]
    pub vault: UncheckedAccount<'info>,

    /// CHECK: PDA used as vault owner
    #[account(seeds = [b"vault_authority"], bump)]
    pub vault_authority: UncheckedAccount<'info>,

    /// CHECK: Initialized via CToken CPI
    #[account(mut)]
    pub user_ata: UncheckedAccount<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    /// CHECK: CToken config
    pub ctoken_compressible_config: AccountInfo<'info>,

    /// CHECK: CToken rent sponsor
    #[account(mut)]
    pub ctoken_rent_sponsor: AccountInfo<'info>,

    /// CHECK: CToken program
    pub ctoken_program: AccountInfo<'info>,

    /// CHECK: CToken CPI authority
    pub ctoken_cpi_authority: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

pub const VAULT_SEED: &[u8] = b"vault";
