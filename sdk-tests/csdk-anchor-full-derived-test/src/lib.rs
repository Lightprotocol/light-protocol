#![allow(deprecated)]
#![allow(clippy::useless_asref)] // Testing macro handling of .as_ref() patterns

use anchor_lang::prelude::*;
use light_sdk::{derive_light_cpi_signer, derive_light_rent_sponsor_pda};
use light_sdk_macros::light_program;
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

#[light_program]
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
            // Original tests
            D9All,
            D9AllParams,
            D9ArrayIndex,
            D9ArrayIndexParams,
            // Const patterns tests
            D9AssocConst,
            D9AssocConstMethod,
            D9AssocConstMethodParams,
            D9AssocConstParams,
            D9BumpConstant,
            D9BumpConstantParams,
            D9BumpCtx,
            D9BumpCtxParams,
            // Array bumps tests
            D9BumpLiteral,
            D9BumpLiteralParams,
            D9BumpMixed,
            D9BumpMixedParams,
            D9BumpParam,
            D9BumpParamParams,
            D9BumpQualified,
            D9BumpQualifiedParams,
            D9ComplexAllQualified,
            D9ComplexAllQualifiedParams,
            D9ComplexFive,
            D9ComplexFiveParams,
            D9ComplexFour,
            D9ComplexFourParams,
            D9ComplexFunc,
            D9ComplexFuncParams,
            D9ComplexIdFunc,
            D9ComplexIdFuncParams,
            D9ComplexProgramId,
            D9ComplexProgramIdParams,
            D9ComplexQualifiedMix,
            D9ComplexQualifiedMixParams,
            // Complex mixed tests
            D9ComplexThree,
            D9ComplexThreeParams,
            D9ConstCombined,
            D9ConstCombinedParams,
            D9ConstFn,
            D9ConstFnGeneric,
            D9ConstFnGenericParams,
            D9ConstFnParams,
            D9Constant,
            D9ConstantParams,
            D9CtxAccount,
            D9CtxAccountParams,
            D9EdgeDigits,
            D9EdgeDigitsParams,
            // Edge cases tests
            D9EdgeEmpty,
            D9EdgeEmptyParams,
            D9EdgeManyLiterals,
            D9EdgeManyLiteralsParams,
            D9EdgeMixed,
            D9EdgeMixedParams,
            D9EdgeSingleByte,
            D9EdgeSingleByteParams,
            D9EdgeSingleLetter,
            D9EdgeSingleLetterParams,
            D9EdgeUnderscore,
            D9EdgeUnderscoreParams,
            D9ExternalBump,
            D9ExternalBumpParams,
            D9ExternalCtoken,
            D9ExternalCtokenParams,
            D9ExternalMixed,
            D9ExternalMixedParams,
            D9ExternalReexport,
            D9ExternalReexportParams,
            // External paths tests
            D9ExternalSdkTypes,
            D9ExternalSdkTypesParams,
            D9ExternalWithLocal,
            D9ExternalWithLocalParams,
            D9FullyQualifiedAssoc,
            D9FullyQualifiedAssocParams,
            D9FullyQualifiedGeneric,
            D9FullyQualifiedGenericParams,
            D9FullyQualifiedTrait,
            D9FullyQualifiedTraitParams,
            D9FunctionCall,
            D9FunctionCallParams,
            D9Literal,
            D9LiteralParams,
            D9MethodAsBytes,
            D9MethodAsBytesParams,
            // Method chains tests
            D9MethodAsRef,
            D9MethodAsRefParams,
            D9MethodMixed,
            D9MethodMixedParams,
            D9MethodQualifiedAsBytes,
            D9MethodQualifiedAsBytesParams,
            D9MethodToBeBytes,
            D9MethodToBeBytesParams,
            D9MethodToLeBytes,
            D9MethodToLeBytesParams,
            D9Mixed,
            D9MixedParams,
            D9MultiAssocConst,
            D9MultiAssocConstParams,
            D9NestedArrayField,
            D9NestedArrayFieldParams,
            D9NestedBytes,
            D9NestedBytesParams,
            D9NestedCombined,
            D9NestedCombinedParams,
            D9NestedDouble,
            D9NestedDoubleParams,
            // Nested seeds tests
            D9NestedSimple,
            D9NestedSimpleParams,
            D9Param,
            D9ParamBytes,
            D9ParamBytesParams,
            D9ParamParams,
            // Qualified paths tests
            D9QualifiedBare,
            D9QualifiedBareParams,
            D9QualifiedConstFn,
            D9QualifiedConstFnParams,
            D9QualifiedCrate,
            D9QualifiedCrateParams,
            D9QualifiedDeep,
            D9QualifiedDeepParams,
            D9QualifiedMixed,
            D9QualifiedMixedParams,
            D9QualifiedSelf,
            D9QualifiedSelfParams,
            D9Static,
            D9StaticParams,
            D9TraitAssocConst,
            D9TraitAssocConstParams,
        },
        instruction_accounts::{
            CreateMintWithMetadata, CreateMintWithMetadataParams, CreatePdasAndMintAuto,
            CreateThreeMints, CreateThreeMintsParams, CreateTwoMints, CreateTwoMintsParams,
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

    /// Second instruction to test #[light_program] with multiple instructions.
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

    /// Test instruction that creates 3 mints in a single transaction.
    /// Tests the multi-mint support in the RentFree macro scales beyond 2.
    #[allow(unused_variables)]
    pub fn create_three_mints<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateThreeMints<'info>>,
        params: CreateThreeMintsParams,
    ) -> Result<()> {
        // All 3 mints are created by the RentFree macro in pre_init
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
    /// Tests: 2x #[light_account(init)], 2x #[rentfree_token], 1x #[light_mint],
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

    /// D8: Only #[light_account(init)] fields (no token accounts)
    pub fn d8_pda_only<'info>(
        ctx: Context<'_, '_, '_, 'info, D8PdaOnly<'info>>,
        params: D8PdaOnlyParams,
    ) -> Result<()> {
        ctx.accounts.d8_pda_only_record.owner = params.owner;
        Ok(())
    }

    /// D8: Multiple #[light_account(init)] fields of same type
    pub fn d8_multi_rentfree<'info>(
        ctx: Context<'_, '_, '_, 'info, D8MultiRentfree<'info>>,
        params: D8MultiRentfreeParams,
    ) -> Result<()> {
        ctx.accounts.d8_multi_record1.owner = params.owner;
        ctx.accounts.d8_multi_record2.owner = params.owner;
        Ok(())
    }

    /// D8: Multiple #[light_account(init)] fields of different types
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
    // D9 Qualified Paths Tests
    // =========================================================================

    /// D9: Bare constant (no path prefix)
    pub fn d9_qualified_bare<'info>(
        ctx: Context<'_, '_, '_, 'info, D9QualifiedBare<'info>>,
        _params: D9QualifiedBareParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: self:: prefix path
    pub fn d9_qualified_self<'info>(
        ctx: Context<'_, '_, '_, 'info, D9QualifiedSelf<'info>>,
        _params: D9QualifiedSelfParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: crate:: prefix path
    pub fn d9_qualified_crate<'info>(
        ctx: Context<'_, '_, '_, 'info, D9QualifiedCrate<'info>>,
        _params: D9QualifiedCrateParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Deep nested crate path
    pub fn d9_qualified_deep<'info>(
        ctx: Context<'_, '_, '_, 'info, D9QualifiedDeep<'info>>,
        _params: D9QualifiedDeepParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Mixed qualified and bare paths
    pub fn d9_qualified_mixed<'info>(
        ctx: Context<'_, '_, '_, 'info, D9QualifiedMixed<'info>>,
        params: D9QualifiedMixedParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = params.owner;
        Ok(())
    }

    // =========================================================================
    // D9 Method Chains Tests
    // =========================================================================

    /// D9: constant.as_ref()
    pub fn d9_method_as_ref<'info>(
        ctx: Context<'_, '_, '_, 'info, D9MethodAsRef<'info>>,
        _params: D9MethodAsRefParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: string_constant.as_bytes()
    pub fn d9_method_as_bytes<'info>(
        ctx: Context<'_, '_, '_, 'info, D9MethodAsBytes<'info>>,
        _params: D9MethodAsBytesParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: crate::path::CONST.as_bytes()
    pub fn d9_method_qualified_as_bytes<'info>(
        ctx: Context<'_, '_, '_, 'info, D9MethodQualifiedAsBytes<'info>>,
        _params: D9MethodQualifiedAsBytesParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: params.field.to_le_bytes().as_ref()
    pub fn d9_method_to_le_bytes<'info>(
        ctx: Context<'_, '_, '_, 'info, D9MethodToLeBytes<'info>>,
        _params: D9MethodToLeBytesParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: params.field.to_be_bytes().as_ref()
    pub fn d9_method_to_be_bytes<'info>(
        ctx: Context<'_, '_, '_, 'info, D9MethodToBeBytes<'info>>,
        _params: D9MethodToBeBytesParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Mixed methods in seeds
    pub fn d9_method_mixed<'info>(
        ctx: Context<'_, '_, '_, 'info, D9MethodMixed<'info>>,
        params: D9MethodMixedParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = params.owner;
        Ok(())
    }

    // =========================================================================
    // D9 Array Bumps Tests (seed combinations with bump)
    // =========================================================================

    /// D9: Literal seed with bump
    pub fn d9_bump_literal<'info>(
        ctx: Context<'_, '_, '_, 'info, D9BumpLiteral<'info>>,
        _params: D9BumpLiteralParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Constant seed with bump
    pub fn d9_bump_constant<'info>(
        ctx: Context<'_, '_, '_, 'info, D9BumpConstant<'info>>,
        _params: D9BumpConstantParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Qualified path with bump
    pub fn d9_bump_qualified<'info>(
        ctx: Context<'_, '_, '_, 'info, D9BumpQualified<'info>>,
        _params: D9BumpQualifiedParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Param seed with bump
    pub fn d9_bump_param<'info>(
        ctx: Context<'_, '_, '_, 'info, D9BumpParam<'info>>,
        params: D9BumpParamParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = params.owner;
        Ok(())
    }

    /// D9: Ctx account seed with bump
    pub fn d9_bump_ctx<'info>(
        ctx: Context<'_, '_, '_, 'info, D9BumpCtx<'info>>,
        _params: D9BumpCtxParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.authority.key();
        Ok(())
    }

    /// D9: Multiple seeds with bump
    pub fn d9_bump_mixed<'info>(
        ctx: Context<'_, '_, '_, 'info, D9BumpMixed<'info>>,
        params: D9BumpMixedParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = params.owner;
        Ok(())
    }

    // =========================================================================
    // D9 Complex Mixed Tests
    // =========================================================================

    /// D9: Three seeds
    pub fn d9_complex_three<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ComplexThree<'info>>,
        params: D9ComplexThreeParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = params.owner;
        Ok(())
    }

    /// D9: Four seeds
    pub fn d9_complex_four<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ComplexFour<'info>>,
        params: D9ComplexFourParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = params.owner;
        Ok(())
    }

    /// D9: Five seeds with ctx account
    pub fn d9_complex_five<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ComplexFive<'info>>,
        params: D9ComplexFiveParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = params.owner;
        Ok(())
    }

    /// D9: Qualified paths mixed with local
    pub fn d9_complex_qualified_mix<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ComplexQualifiedMix<'info>>,
        params: D9ComplexQualifiedMixParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = params.owner;
        Ok(())
    }

    /// D9: Function call combined with other seeds
    pub fn d9_complex_func<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ComplexFunc<'info>>,
        params: D9ComplexFuncParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = params.key_a;
        Ok(())
    }

    /// D9: All qualified paths
    pub fn d9_complex_all_qualified<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ComplexAllQualified<'info>>,
        params: D9ComplexAllQualifiedParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = params.owner;
        Ok(())
    }

    /// D9: Program ID as seed
    pub fn d9_complex_program_id<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ComplexProgramId<'info>>,
        params: D9ComplexProgramIdParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = params.owner;
        Ok(())
    }

    /// D9: id() function call as seed
    pub fn d9_complex_id_func<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ComplexIdFunc<'info>>,
        params: D9ComplexIdFuncParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = params.owner;
        Ok(())
    }

    // =========================================================================
    // D9 Edge Cases Tests
    // =========================================================================

    /// D9: Empty literal
    pub fn d9_edge_empty<'info>(
        ctx: Context<'_, '_, '_, 'info, D9EdgeEmpty<'info>>,
        params: D9EdgeEmptyParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = params.owner;
        Ok(())
    }

    /// D9: Single byte constant
    pub fn d9_edge_single_byte<'info>(
        ctx: Context<'_, '_, '_, 'info, D9EdgeSingleByte<'info>>,
        _params: D9EdgeSingleByteParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Single letter constant name
    pub fn d9_edge_single_letter<'info>(
        ctx: Context<'_, '_, '_, 'info, D9EdgeSingleLetter<'info>>,
        _params: D9EdgeSingleLetterParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Constant name with digits
    pub fn d9_edge_digits<'info>(
        ctx: Context<'_, '_, '_, 'info, D9EdgeDigits<'info>>,
        _params: D9EdgeDigitsParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Leading underscore constant
    pub fn d9_edge_underscore<'info>(
        ctx: Context<'_, '_, '_, 'info, D9EdgeUnderscore<'info>>,
        _params: D9EdgeUnderscoreParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Many literals in seeds
    pub fn d9_edge_many_literals<'info>(
        ctx: Context<'_, '_, '_, 'info, D9EdgeManyLiterals<'info>>,
        _params: D9EdgeManyLiteralsParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Mixed edge cases
    pub fn d9_edge_mixed<'info>(
        ctx: Context<'_, '_, '_, 'info, D9EdgeMixed<'info>>,
        params: D9EdgeMixedParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = params.owner;
        Ok(())
    }

    // =========================================================================
    // D9 External Paths Tests
    // =========================================================================

    /// D9: External crate (light_sdk_types)
    pub fn d9_external_sdk_types<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ExternalSdkTypes<'info>>,
        params: D9ExternalSdkTypesParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = params.owner;
        Ok(())
    }

    /// D9: External crate (light_ctoken_types)
    pub fn d9_external_ctoken<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ExternalCtoken<'info>>,
        params: D9ExternalCtokenParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = params.owner;
        Ok(())
    }

    /// D9: Multiple external crates mixed
    pub fn d9_external_mixed<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ExternalMixed<'info>>,
        params: D9ExternalMixedParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = params.owner;
        Ok(())
    }

    /// D9: External with local constant
    pub fn d9_external_with_local<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ExternalWithLocal<'info>>,
        params: D9ExternalWithLocalParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = params.owner;
        Ok(())
    }

    /// D9: External constant with bump
    pub fn d9_external_bump<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ExternalBump<'info>>,
        params: D9ExternalBumpParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = params.owner;
        Ok(())
    }

    /// D9: Re-exported external constant
    pub fn d9_external_reexport<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ExternalReexport<'info>>,
        _params: D9ExternalReexportParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    // =========================================================================
    // D9 Nested Seeds Tests
    // =========================================================================

    /// D9: Simple nested struct access
    pub fn d9_nested_simple<'info>(
        ctx: Context<'_, '_, '_, 'info, D9NestedSimple<'info>>,
        params: D9NestedSimpleParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = params.nested.owner;
        Ok(())
    }

    /// D9: Double nested struct access
    pub fn d9_nested_double<'info>(
        ctx: Context<'_, '_, '_, 'info, D9NestedDouble<'info>>,
        params: D9NestedDoubleParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = params.outer.nested.owner;
        Ok(())
    }

    /// D9: Nested array field access
    pub fn d9_nested_array_field<'info>(
        ctx: Context<'_, '_, '_, 'info, D9NestedArrayField<'info>>,
        params: D9NestedArrayFieldParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = params.outer.nested.owner;
        Ok(())
    }

    /// D9: Array indexing params.arrays[2].as_slice()
    pub fn d9_array_index<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ArrayIndex<'info>>,
        _params: D9ArrayIndexParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Nested struct with bytes conversion
    pub fn d9_nested_bytes<'info>(
        ctx: Context<'_, '_, '_, 'info, D9NestedBytes<'info>>,
        params: D9NestedBytesParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = params.nested.owner;
        Ok(())
    }

    /// D9: Multiple nested seeds combined
    pub fn d9_nested_combined<'info>(
        ctx: Context<'_, '_, '_, 'info, D9NestedCombined<'info>>,
        params: D9NestedCombinedParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = params.outer.nested.owner;
        Ok(())
    }

    // =========================================================================
    // D9 Const Patterns Tests
    // =========================================================================

    /// D9: Associated constant
    pub fn d9_assoc_const<'info>(
        ctx: Context<'_, '_, '_, 'info, D9AssocConst<'info>>,
        _params: D9AssocConstParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Associated constant with method
    pub fn d9_assoc_const_method<'info>(
        ctx: Context<'_, '_, '_, 'info, D9AssocConstMethod<'info>>,
        _params: D9AssocConstMethodParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Multiple associated constants
    pub fn d9_multi_assoc_const<'info>(
        ctx: Context<'_, '_, '_, 'info, D9MultiAssocConst<'info>>,
        params: D9MultiAssocConstParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = params.owner;
        Ok(())
    }

    /// D9: Const fn call
    pub fn d9_const_fn<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ConstFn<'info>>,
        _params: D9ConstFnParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Const fn with generic
    pub fn d9_const_fn_generic<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ConstFnGeneric<'info>>,
        _params: D9ConstFnGenericParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Trait associated constant
    pub fn d9_trait_assoc_const<'info>(
        ctx: Context<'_, '_, '_, 'info, D9TraitAssocConst<'info>>,
        _params: D9TraitAssocConstParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Static variable
    pub fn d9_static<'info>(
        ctx: Context<'_, '_, '_, 'info, D9Static<'info>>,
        _params: D9StaticParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Qualified const fn
    pub fn d9_qualified_const_fn<'info>(
        ctx: Context<'_, '_, '_, 'info, D9QualifiedConstFn<'info>>,
        _params: D9QualifiedConstFnParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Fully qualified associated constant
    pub fn d9_fully_qualified_assoc<'info>(
        ctx: Context<'_, '_, '_, 'info, D9FullyQualifiedAssoc<'info>>,
        _params: D9FullyQualifiedAssocParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Fully qualified trait associated constant
    pub fn d9_fully_qualified_trait<'info>(
        ctx: Context<'_, '_, '_, 'info, D9FullyQualifiedTrait<'info>>,
        _params: D9FullyQualifiedTraitParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Fully qualified const fn with generic
    pub fn d9_fully_qualified_generic<'info>(
        ctx: Context<'_, '_, '_, 'info, D9FullyQualifiedGeneric<'info>>,
        _params: D9FullyQualifiedGenericParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Combined const patterns
    pub fn d9_const_combined<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ConstCombined<'info>>,
        params: D9ConstCombinedParams,
    ) -> Result<()> {
        ctx.accounts.record.owner = params.owner;
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

    /// D5: All markers combined (#[light_account(init)] + #[rentfree_token])
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
