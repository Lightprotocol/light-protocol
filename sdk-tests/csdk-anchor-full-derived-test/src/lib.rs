#![allow(deprecated)]

use anchor_lang::prelude::*;
use light_sdk::{derive_light_cpi_signer, derive_light_rent_sponsor_pda};
// Using the new `compressible` alias (equivalent to add_compressible_instructions)
use light_sdk_macros::compressible;
// LightFinalize approach imports
use light_sdk_macros::light_instruction;
use light_sdk_types::CpiSigner;

pub mod errors;
pub mod instruction_accounts;
pub mod state;

pub use instruction_accounts::*;
pub use state::{
    GameSession, PackedGameSession, PackedPlaceholderRecord, PackedUserRecord, PlaceholderRecord,
    UserRecord,
};

// Example helper expression usable in seeds
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

/// Derive a program-owned rent sponsor PDA (version = 1 by default).
pub const PROGRAM_RENT_SPONSOR_DATA: ([u8; 32], u8) =
    derive_light_rent_sponsor_pda!("FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah", 1);

/// Returns the program's rent sponsor PDA as a Pubkey.
#[inline]
pub fn program_rent_sponsor() -> Pubkey {
    Pubkey::from(PROGRAM_RENT_SPONSOR_DATA.0)
}

// Using the new `#[compressible]` attribute (alias for add_compressible_instructions)
#[compressible(
    // Complex PDA account types with seed specifications using BOTH ctx.accounts.* AND data.*
    // UserRecord: uses ctx accounts (authority, mint_authority) + data fields (owner, category_id)
    UserRecord = ("user_record", ctx.authority, ctx.mint_authority, data.owner, data.category_id.to_le_bytes()),
    // GameSession: uses max_key expression with ctx.accounts + data.session_id
    GameSession = ("game_session", max_key(&ctx.user.key(), &ctx.authority.key()), data.session_id.to_le_bytes()),
    // PlaceholderRecord: mixes ctx accounts and data for seeds
    PlaceholderRecord = ("placeholder_record", ctx.authority, ctx.some_account, data.placeholder_id.to_le_bytes(), data.counter.to_le_bytes()),
    // Token variant (CToken account) with authority for compression signing
    CTokenSigner = (is_token, "ctoken_signer", ctx.fee_payer, ctx.mint, authority = LIGHT_CPI_SIGNER),
    // Program-owned CToken vault: seeds = "vault" + cmint pubkey (like cp-swap token vaults)
    // Authority = vault_authority PDA that owns the vault (like cp-swap's authority)
    Vault = (is_token, "vault", ctx.cmint, authority = ("vault_authority")),
    // User-owned ATA: uses ctoken's standard ATA derivation (wallet + ctoken_program + mint)
    // is_ata flag indicates the wallet signs (not the program)
    UserAta = (is_token, is_ata, ctx.wallet, ctx.cmint),
    // CMint: for decompressing a light mint
    CMint = (is_token, "cmint", ctx.mint_signer, authority = LIGHT_CPI_SIGNER),
    // Instruction data fields used in seed expressions above
    owner = Pubkey,
    category_id = u64,
    session_id = u64,
    placeholder_id = u64,
    counter = u32,
)]
#[program]
pub mod csdk_anchor_full_derived_test {
    #![allow(clippy::too_many_arguments)]

    use super::*;
    use crate::{
        instruction_accounts::CreatePdasAndMintAuto,
        state::{GameSession, PlaceholderRecord, UserRecord},
        FullAutoWithMintParams, LIGHT_CPI_SIGNER,
    };
    /// FULL AUTOMATIC WITH MINT: Creates 2 PDAs + 1 CMint + vault + user_ata in ONE instruction.
    /// - 2 PDAs with #[compressible] (UserRecord, GameSession)
    /// - 1 CMint with #[light_mint] (creates + decompresses atomically in pre_init)
    /// - 1 Program-owned CToken vault (created in instruction body)
    /// - 1 User CToken ATA (created in instruction body)
    /// - MintTo both vault and user_ata (in instruction body)
    ///
    /// All batched together with a single proof execution!
    ///
    /// This is the pattern used by protocols like Raydium cp-swap:
    /// - Pool state PDA (compressible)
    /// - Observation state PDA (compressible)
    /// - LP mint (light_mint - created and immediately decompressed)
    /// - Token vaults (CToken accounts)
    /// - Creator LP token (user's ATA)
    #[light_instruction(params)]
    pub fn create_pdas_and_mint_auto<'info>(
        ctx: Context<'_, '_, '_, 'info, CreatePdasAndMintAuto<'info>>,
        params: FullAutoWithMintParams,
    ) -> Result<()> {
        use anchor_lang::solana_program::sysvar::clock::Clock;
        use light_ctoken_sdk::ctoken::{
            CTokenMintToCpi, CreateAssociatedCTokenAccountCpi, CreateCTokenAccountCpi,
        };

        // Populate UserRecord - compression handled by macro
        let user_record = &mut ctx.accounts.user_record;
        user_record.owner = params.owner;
        user_record.name = "Auto Created User With Mint".to_string();
        user_record.score = 0;
        user_record.category_id = params.category_id;

        // Populate GameSession - compression handled by macro
        let game_session = &mut ctx.accounts.game_session;
        game_session.session_id = params.session_id;
        game_session.player = ctx.accounts.fee_payer.key();
        game_session.game_type = "Auto Game With Mint".to_string();
        game_session.start_time = Clock::get()?.unix_timestamp as u64;
        game_session.end_time = None;
        game_session.score = 0;

        // At this point, the CMint is already created and decompressed ("hot")
        // by the #[light_instruction] macro's pre_init phase.
        // Now we can use it to create CToken accounts and mint tokens.

        // 1. Create program-owned CToken vault (like cp-swap's token vaults)
        // Pattern: owner = vault_authority PDA, compress_to_account_pubkey derived from signer seeds
        // This ensures compressed TokenData.owner = vault address (not authority)
        let cmint_key = ctx.accounts.cmint.key();
        CreateCTokenAccountCpi::new_v2_signed(
            ctx.accounts.fee_payer.to_account_info(),
            ctx.accounts.vault.to_account_info(),
            ctx.accounts.cmint.to_account_info(),
            ctx.accounts.vault_authority.key(), // Authority owns vault (like cp-swap)
            ctx.accounts.ctoken_compressible_config.to_account_info(),
            ctx.accounts.ctoken_rent_sponsor.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            &crate::ID,
            &[
                crate::instruction_accounts::VAULT_SEED,
                cmint_key.as_ref(),
                &[params.vault_bump],
            ],
        )?;

        // 2. Create user's ATA (like cp-swap's creator_lp_token)
        CreateAssociatedCTokenAccountCpi::new_v2_idempotent(
            ctx.accounts.fee_payer.to_account_info(),
            ctx.accounts.cmint.to_account_info(),
            ctx.accounts.fee_payer.to_account_info(),
            ctx.accounts.user_ata.to_account_info(),
            params.user_ata_bump,
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.ctoken_compressible_config.to_account_info(),
            ctx.accounts.ctoken_rent_sponsor.to_account_info(),
        )?;

        // 3. Mint tokens to vault
        if params.vault_mint_amount > 0 {
            CTokenMintToCpi {
                cmint: ctx.accounts.cmint.to_account_info(),
                destination: ctx.accounts.vault.to_account_info(),
                amount: params.vault_mint_amount,
                authority: ctx.accounts.mint_authority.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                max_top_up: None,
            }
            .invoke()?;
        }

        // 4. Mint tokens to user's ATA
        if params.user_ata_mint_amount > 0 {
            CTokenMintToCpi {
                cmint: ctx.accounts.cmint.to_account_info(),
                destination: ctx.accounts.user_ata.to_account_info(),
                amount: params.user_ata_mint_amount,
                authority: ctx.accounts.mint_authority.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                max_top_up: None,
            }
            .invoke()?;
        }

        // The #[light_instruction] macro handles:
        // - pre_init(): Created+Decompressed CMint (already done by now)
        // - finalize(): no-op (all work done above and in pre_init)
        //
        // After this instruction:
        // - UserRecord and GameSession PDAs have compressed addresses registered
        // - LP mint is created AND decompressed (hot/active state)
        // - Vault exists with vault_mint_amount tokens, owned by vault_authority
        // - User ATA exists with user_ata_mint_amount tokens, owned by fee_payer
        Ok(())
    }
}
