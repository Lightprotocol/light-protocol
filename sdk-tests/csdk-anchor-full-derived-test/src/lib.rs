#![allow(deprecated)]

use anchor_lang::prelude::*;
use light_sdk::{derive_light_cpi_signer, derive_light_rent_sponsor_pda};
use light_sdk_macros::rentfree_program;
use light_sdk_types::CpiSigner;

pub mod amm_test;
pub mod d5_markers;
pub mod d6_account_types;
pub mod d7_infra_names;
pub mod d8_builder_paths;
pub mod d9_seeds;
pub mod errors;
pub mod instruction_accounts;
pub mod instructions;
pub mod processors;
pub mod state;
pub use amm_test::*;
pub use d5_markers::*;
pub use d6_account_types::*;
pub use d7_infra_names::*;
pub use d8_builder_paths::*;
pub use d9_seeds::*;
pub use instruction_accounts::*;
pub use instructions::{
    d7_infra_names::{
        D7_ALL_AUTH_SEED, D7_ALL_VAULT_SEED, D7_CTOKEN_AUTH_SEED, D7_CTOKEN_VAULT_SEED,
    },
    d9_seeds::{D9_ALL_SEED, D9_CONSTANT_SEED},
};
pub use state::{
    d1_field_types::{
        all::{AllFieldTypesRecord, PackedAllFieldTypesRecord},
        arrays::ArrayRecord,
        multi_pubkey::{MultiPubkeyRecord, PackedMultiPubkeyRecord},
        no_pubkey::NoPubkeyRecord,
        non_copy::NonCopyRecord,
        option_primitive::OptionPrimitiveRecord,
        option_pubkey::{OptionPubkeyRecord, PackedOptionPubkeyRecord},
        single_pubkey::{PackedSinglePubkeyRecord, SinglePubkeyRecord},
    },
    d2_compress_as::{
        absent::{NoCompressAsRecord, PackedNoCompressAsRecord},
        all::{AllCompressAsRecord, PackedAllCompressAsRecord},
        multiple::{MultipleCompressAsRecord, PackedMultipleCompressAsRecord},
        option_none::{OptionNoneCompressAsRecord, PackedOptionNoneCompressAsRecord},
        single::{PackedSingleCompressAsRecord, SingleCompressAsRecord},
    },
    d4_composition::{
        all::{AllCompositionRecord, PackedAllCompositionRecord},
        info_last::{InfoLastRecord, PackedInfoLastRecord},
        large::LargeRecord,
        minimal::MinimalRecord,
    },
    GameSession, PackedGameSession, PackedPlaceholderRecord, PackedUserRecord, PlaceholderRecord,
    UserRecord,
};
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

    use super::{
        amm_test::{Deposit, InitializeParams, InitializePool, Withdraw},
        d5_markers::{
            D5AllMarkers, D5AllMarkersParams, D5RentfreeBare, D5RentfreeBareParams,
            D5RentfreeToken, D5RentfreeTokenParams,
        },
        d6_account_types::{D6Account, D6AccountParams, D6Boxed, D6BoxedParams},
        d7_infra_names::{
            D7AllNames, D7AllNamesParams, D7Creator, D7CreatorParams, D7CtokenConfig,
            D7CtokenConfigParams, D7Payer, D7PayerParams,
        },
        d8_builder_paths::{
            D8All, D8AllParams, D8MultiRentfree, D8MultiRentfreeParams, D8PdaOnly, D8PdaOnlyParams,
        },
        d9_seeds::{
            D9All, D9AllParams, D9Constant, D9ConstantParams, D9CtxAccount, D9CtxAccountParams,
            D9FunctionCall, D9FunctionCallParams, D9Literal, D9LiteralParams, D9Mixed,
            D9MixedParams, D9Param, D9ParamBytes, D9ParamBytesParams, D9ParamParams,
        },
        instruction_accounts::{
            CreateFourMints, CreateFourMintsParams, CreateMintWithMetadata,
            CreateMintWithMetadataParams, CreatePdasAndMintAuto, CreateTwoMints,
            CreateTwoMintsParams,
        },
        FullAutoWithMintParams, LIGHT_CPI_SIGNER,
    };

    pub fn create_pdas_and_mint_auto<'info>(
        ctx: Context<'_, '_, '_, 'info, CreatePdasAndMintAuto<'info>>,
        params: FullAutoWithMintParams,
    ) -> Result<()> {
        use light_token_sdk::token::{
            CreateTokenAccountCpi, CreateTokenAtaCpi, MintToCpi as CTokenMintToCpi,
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
        game_session.start_time = 2; // Hardcoded non-zero for compress_as test
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

        CreateTokenAtaCpi {
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

    /// Second instruction to test #[rentfree_program] with multiple instructions.
    /// Delegates to nested processor in separate module.
    pub fn create_single_record<'info>(
        ctx: Context<'_, '_, '_, 'info, D5RentfreeBare<'info>>,
        params: D5RentfreeBareParams,
    ) -> Result<()> {
        crate::processors::process_create_single_record(ctx, params)
    }

    /// Test instruction that creates 2 mints in a single transaction.
    /// Tests the multi-mint support in the RentFree macro.
    #[allow(unused_variables)]
    pub fn create_two_mints<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateTwoMints<'info>>,
        params: CreateTwoMintsParams,
    ) -> Result<()> {
        // Both mints are created by the RentFree macro in pre_init
        // Nothing to do here - just verify both mints exist
        Ok(())
    }

    /// Test instruction that creates 4 mints in a single transaction.
    /// Tests the multi-mint support in the RentFree macro scales beyond 2.
    #[allow(unused_variables)]
    pub fn create_four_mints<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateFourMints<'info>>,
        params: CreateFourMintsParams,
    ) -> Result<()> {
        // All 4 mints are created by the RentFree macro in pre_init
        // Nothing to do here - just verify all mints exist
        Ok(())
    }

    /// Test instruction that creates a mint with metadata.
    /// Tests the metadata support in the RentFree macro.
    #[allow(unused_variables)]
    pub fn create_mint_with_metadata<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateMintWithMetadata<'info>>,
        params: CreateMintWithMetadataParams,
    ) -> Result<()> {
        // Mint with metadata is created by the RentFree macro in pre_init
        // Nothing to do here - metadata is part of the mint creation
        Ok(())
    }

    /// AMM initialize instruction with all rentfree markers.
    /// Tests: 2x #[rentfree], 2x #[rentfree_token], 1x #[light_mint],
    /// CreateTokenAccountCpi.rent_free(), CreateTokenAtaCpi.rent_free(), MintToCpi
    pub fn initialize_pool<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializePool<'info>>,
        params: InitializeParams,
    ) -> Result<()> {
        crate::amm_test::process_initialize_pool(ctx, params)
    }

    /// AMM deposit instruction with MintToCpi.
    pub fn deposit(ctx: Context<Deposit>, lp_token_amount: u64) -> Result<()> {
        crate::amm_test::process_deposit(ctx, lp_token_amount)
    }

    /// AMM withdraw instruction with BurnCpi.
    pub fn withdraw(ctx: Context<Withdraw>, lp_token_amount: u64) -> Result<()> {
        crate::amm_test::process_withdraw(ctx, lp_token_amount)
    }

    // =========================================================================
    // D6 Account Types: Account type extraction
    // =========================================================================

    /// D6: Direct Account<'info, T> type
    pub fn d6_account<'info>(
        ctx: Context<'_, '_, '_, 'info, D6Account<'info>>,
        params: D6AccountParams,
    ) -> Result<()> {
        ctx.accounts.d6_account_record.owner = params.owner;
        Ok(())
    }

    /// D6: Box<Account<'info, T>> type
    pub fn d6_boxed<'info>(
        ctx: Context<'_, '_, '_, 'info, D6Boxed<'info>>,
        params: D6BoxedParams,
    ) -> Result<()> {
        ctx.accounts.d6_boxed_record.owner = params.owner;
        Ok(())
    }

    // =========================================================================
    // D8 Builder Paths: Builder code generation paths
    // =========================================================================

    /// D8: Only #[rentfree] fields (no token accounts)
    pub fn d8_pda_only<'info>(
        ctx: Context<'_, '_, '_, 'info, D8PdaOnly<'info>>,
        params: D8PdaOnlyParams,
    ) -> Result<()> {
        ctx.accounts.d8_pda_only_record.owner = params.owner;
        Ok(())
    }

    /// D8: Multiple #[rentfree] fields of same type
    pub fn d8_multi_rentfree<'info>(
        ctx: Context<'_, '_, '_, 'info, D8MultiRentfree<'info>>,
        params: D8MultiRentfreeParams,
    ) -> Result<()> {
        ctx.accounts.d8_multi_record1.owner = params.owner;
        ctx.accounts.d8_multi_record2.owner = params.owner;
        Ok(())
    }

    /// D8: Multiple #[rentfree] fields of different types
    pub fn d8_all<'info>(
        ctx: Context<'_, '_, '_, 'info, D8All<'info>>,
        params: D8AllParams,
    ) -> Result<()> {
        ctx.accounts.d8_all_single.owner = params.owner;
        ctx.accounts.d8_all_multi.owner = params.owner;
        Ok(())
    }

    // =========================================================================
    // D9 Seeds: Seed expression classification
    // =========================================================================

    /// D9: Literal seed expression
    pub fn d9_literal<'info>(
        ctx: Context<'_, '_, '_, 'info, D9Literal<'info>>,
        _params: D9LiteralParams,
    ) -> Result<()> {
        ctx.accounts.d9_literal_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Constant seed expression
    pub fn d9_constant<'info>(
        ctx: Context<'_, '_, '_, 'info, D9Constant<'info>>,
        _params: D9ConstantParams,
    ) -> Result<()> {
        ctx.accounts.d9_constant_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Context account seed expression
    pub fn d9_ctx_account<'info>(
        ctx: Context<'_, '_, '_, 'info, D9CtxAccount<'info>>,
        _params: D9CtxAccountParams,
    ) -> Result<()> {
        ctx.accounts.d9_ctx_record.owner = ctx.accounts.authority.key();
        Ok(())
    }

    /// D9: Param seed expression (Pubkey)
    pub fn d9_param<'info>(
        ctx: Context<'_, '_, '_, 'info, D9Param<'info>>,
        params: D9ParamParams,
    ) -> Result<()> {
        ctx.accounts.d9_param_record.owner = params.owner;
        Ok(())
    }

    /// D9: Param bytes seed expression (u64)
    pub fn d9_param_bytes<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ParamBytes<'info>>,
        _params: D9ParamBytesParams,
    ) -> Result<()> {
        ctx.accounts.d9_param_bytes_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Mixed seed expression types
    pub fn d9_mixed<'info>(
        ctx: Context<'_, '_, '_, 'info, D9Mixed<'info>>,
        params: D9MixedParams,
    ) -> Result<()> {
        ctx.accounts.d9_mixed_record.owner = params.owner;
        Ok(())
    }

    // =========================================================================
    // D7 Infrastructure Names: Field naming convention tests
    // =========================================================================

    /// D7: "payer" field name variant (instead of fee_payer)
    pub fn d7_payer<'info>(
        ctx: Context<'_, '_, '_, 'info, D7Payer<'info>>,
        params: D7PayerParams,
    ) -> Result<()> {
        ctx.accounts.d7_payer_record.owner = params.owner;
        Ok(())
    }

    /// D7: "creator" field name variant (instead of fee_payer)
    pub fn d7_creator<'info>(
        ctx: Context<'_, '_, '_, 'info, D7Creator<'info>>,
        params: D7CreatorParams,
    ) -> Result<()> {
        ctx.accounts.d7_creator_record.owner = params.owner;
        Ok(())
    }

    /// D7: "ctoken_config" naming variant for token accounts
    pub fn d7_ctoken_config<'info>(
        ctx: Context<'_, '_, '_, 'info, D7CtokenConfig<'info>>,
        _params: D7CtokenConfigParams,
    ) -> Result<()> {
        use light_token_sdk::token::CreateTokenAccountCpi;

        let mint_key = ctx.accounts.mint.key();
        // Derive the vault bump at runtime
        let (_, vault_bump) = Pubkey::find_program_address(
            &[
                crate::d7_infra_names::D7_CTOKEN_VAULT_SEED,
                mint_key.as_ref(),
            ],
            &crate::ID,
        );

        CreateTokenAccountCpi {
            payer: ctx.accounts.fee_payer.to_account_info(),
            account: ctx.accounts.d7_ctoken_vault.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            owner: ctx.accounts.d7_ctoken_authority.key(),
        }
        .rent_free(
            ctx.accounts.ctoken_compressible_config.to_account_info(),
            ctx.accounts.ctoken_rent_sponsor.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            &crate::ID,
        )
        .invoke_signed(&[
            crate::d7_infra_names::D7_CTOKEN_VAULT_SEED,
            mint_key.as_ref(),
            &[vault_bump],
        ])?;
        Ok(())
    }

    /// D7: All naming variants combined (payer + ctoken config/sponsor)
    pub fn d7_all_names<'info>(
        ctx: Context<'_, '_, '_, 'info, D7AllNames<'info>>,
        params: D7AllNamesParams,
    ) -> Result<()> {
        use light_token_sdk::token::CreateTokenAccountCpi;

        // Set up the PDA record
        ctx.accounts.d7_all_record.owner = params.owner;

        // Create token vault
        let mint_key = ctx.accounts.mint.key();
        // Derive the vault bump at runtime
        let (_, vault_bump) = Pubkey::find_program_address(
            &[crate::d7_infra_names::D7_ALL_VAULT_SEED, mint_key.as_ref()],
            &crate::ID,
        );

        CreateTokenAccountCpi {
            payer: ctx.accounts.payer.to_account_info(),
            account: ctx.accounts.d7_all_vault.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            owner: ctx.accounts.d7_all_authority.key(),
        }
        .rent_free(
            ctx.accounts.ctoken_compressible_config.to_account_info(),
            ctx.accounts.ctoken_rent_sponsor.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            &crate::ID,
        )
        .invoke_signed(&[
            crate::d7_infra_names::D7_ALL_VAULT_SEED,
            mint_key.as_ref(),
            &[vault_bump],
        ])?;
        Ok(())
    }

    // =========================================================================
    // D9 Additional Seeds Tests
    // =========================================================================

    /// D9: Function call seed expression
    pub fn d9_function_call<'info>(
        ctx: Context<'_, '_, '_, 'info, D9FunctionCall<'info>>,
        params: D9FunctionCallParams,
    ) -> Result<()> {
        ctx.accounts.d9_func_record.owner = params.key_a;
        Ok(())
    }

    /// D9: All seed expression types (6 PDAs)
    pub fn d9_all<'info>(
        ctx: Context<'_, '_, '_, 'info, D9All<'info>>,
        params: D9AllParams,
    ) -> Result<()> {
        ctx.accounts.d9_all_lit.owner = params.owner;
        ctx.accounts.d9_all_const.owner = params.owner;
        ctx.accounts.d9_all_ctx.owner = params.owner;
        ctx.accounts.d9_all_param.owner = params.owner;
        ctx.accounts.d9_all_bytes.owner = params.owner;
        ctx.accounts.d9_all_func.owner = params.owner;
        Ok(())
    }

    // =========================================================================
    // D5 Additional Markers Tests
    // =========================================================================

    /// D5: #[rentfree_token] attribute test
    pub fn d5_rentfree_token<'info>(
        ctx: Context<'_, '_, '_, 'info, D5RentfreeToken<'info>>,
        params: D5RentfreeTokenParams,
    ) -> Result<()> {
        use light_token_sdk::token::CreateTokenAccountCpi;

        let mint_key = ctx.accounts.mint.key();
        CreateTokenAccountCpi {
            payer: ctx.accounts.fee_payer.to_account_info(),
            account: ctx.accounts.d5_token_vault.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            owner: ctx.accounts.vault_authority.key(),
        }
        .rent_free(
            ctx.accounts.ctoken_compressible_config.to_account_info(),
            ctx.accounts.ctoken_rent_sponsor.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            &crate::ID,
        )
        .invoke_signed(&[
            crate::d5_markers::D5_VAULT_SEED,
            mint_key.as_ref(),
            &[params.vault_bump],
        ])?;
        Ok(())
    }

    /// D5: All markers combined (#[rentfree] + #[rentfree_token])
    pub fn d5_all_markers<'info>(
        ctx: Context<'_, '_, '_, 'info, D5AllMarkers<'info>>,
        params: D5AllMarkersParams,
    ) -> Result<()> {
        use light_token_sdk::token::CreateTokenAccountCpi;

        // Set up the PDA record
        ctx.accounts.d5_all_record.owner = params.owner;

        // Create token vault
        let mint_key = ctx.accounts.mint.key();
        // Derive the vault bump at runtime
        let (_, vault_bump) = Pubkey::find_program_address(
            &[crate::d5_markers::D5_ALL_VAULT_SEED, mint_key.as_ref()],
            &crate::ID,
        );

        CreateTokenAccountCpi {
            payer: ctx.accounts.fee_payer.to_account_info(),
            account: ctx.accounts.d5_all_vault.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            owner: ctx.accounts.d5_all_authority.key(),
        }
        .rent_free(
            ctx.accounts.ctoken_compressible_config.to_account_info(),
            ctx.accounts.ctoken_rent_sponsor.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            &crate::ID,
        )
        .invoke_signed(&[
            crate::d5_markers::D5_ALL_VAULT_SEED,
            mint_key.as_ref(),
            &[vault_bump],
        ])?;
        Ok(())
    }
}
