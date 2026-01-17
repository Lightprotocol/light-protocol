#![allow(deprecated)]

use anchor_lang::prelude::*;
use light_sdk::{derive_light_cpi_signer, derive_light_rent_sponsor_pda};
use light_sdk_macros::rentfree_program;
use light_sdk_types::CpiSigner;

pub mod errors;
pub mod instruction_accounts;
pub mod state;

pub use instruction_accounts::*;
pub use state::{GameSession, PackedGameSession, PackedUserRecord, PlaceholderRecord, UserRecord};

#[inline]
pub fn max_key(left: &Pubkey, right: &Pubkey) -> [u8; 32] {
    if left > right {
        left.to_bytes()
    } else {
        right.to_bytes()
    }
}

declare_id!("FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah");

pub const PROGRAM_RENT_SPONSOR_DATA: ([u8; 32], u8) =
    derive_light_rent_sponsor_pda!("FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah", 1);

#[inline]
pub fn program_rent_sponsor() -> Pubkey {
    Pubkey::from(PROGRAM_RENT_SPONSOR_DATA.0)
}

pub const GAME_SESSION_SEED: &str = "game_session";

#[rentfree_program]
#[program]
pub mod csdk_anchor_full_derived_test {
    #![allow(clippy::too_many_arguments)]

    use super::*;
    use crate::{
        instruction_accounts::CreatePdasAndMintAuto,
        state::{GameSession, UserRecord},
        FullAutoWithMintParams, LIGHT_CPI_SIGNER,
    };

    pub fn create_pdas_and_mint_auto<'info>(
        ctx: Context<'_, '_, '_, 'info, CreatePdasAndMintAuto<'info>>,
        params: FullAutoWithMintParams,
    ) -> Result<()> {
        use anchor_lang::solana_program::sysvar::clock::Clock;
        use light_token_sdk::token::{
            CreateCTokenAtaCpi, CreateTokenAccountCpi, MintToCpi as CTokenMintToCpi,
        };

        let user_record = &mut ctx.accounts.user_record;
        user_record.owner = params.owner;
        user_record.name = "Auto Created User With Mint".to_string();
        user_record.score = 0;
        user_record.category_id = params.category_id;

        let game_session = &mut ctx.accounts.game_session;
        game_session.session_id = params.session_id;
        game_session.player = ctx.accounts.fee_payer.key();
        game_session.game_type = "Auto Game With Mint".to_string();
        game_session.start_time = Clock::get()?.unix_timestamp as u64;
        game_session.end_time = None;
        game_session.score = 0;

        let cmint_key = ctx.accounts.cmint.key();
        CreateTokenAccountCpi {
            payer: ctx.accounts.fee_payer.to_account_info(),
            account: ctx.accounts.vault.to_account_info(),
            mint: ctx.accounts.cmint.to_account_info(),
            owner: ctx.accounts.vault_authority.key(),
        }
        .rent_free(
            ctx.accounts.ctoken_compressible_config.to_account_info(),
            ctx.accounts.ctoken_rent_sponsor.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            &crate::ID,
        )
        .invoke_signed(&[
            crate::instruction_accounts::VAULT_SEED,
            cmint_key.as_ref(),
            &[params.vault_bump],
        ])?;

        CreateCTokenAtaCpi {
            payer: ctx.accounts.fee_payer.to_account_info(),
            owner: ctx.accounts.fee_payer.to_account_info(),
            mint: ctx.accounts.cmint.to_account_info(),
            ata: ctx.accounts.user_ata.to_account_info(),
            bump: params.user_ata_bump,
        }
        .idempotent()
        .rent_free(
            ctx.accounts.ctoken_compressible_config.to_account_info(),
            ctx.accounts.ctoken_rent_sponsor.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        )
        .invoke()?;

        if params.vault_mint_amount > 0 {
            CTokenMintToCpi {
                mint: ctx.accounts.cmint.to_account_info(),
                destination: ctx.accounts.vault.to_account_info(),
                amount: params.vault_mint_amount,
                authority: ctx.accounts.mint_authority.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                max_top_up: None,
            }
            .invoke()?;
        }

        if params.user_ata_mint_amount > 0 {
            CTokenMintToCpi {
                mint: ctx.accounts.cmint.to_account_info(),
                destination: ctx.accounts.user_ata.to_account_info(),
                amount: params.user_ata_mint_amount,
                authority: ctx.accounts.mint_authority.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                max_top_up: None,
            }
            .invoke()?;
        }

        Ok(())
    }
}
