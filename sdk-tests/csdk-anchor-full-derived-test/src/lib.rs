#![allow(deprecated)]
#![allow(clippy::useless_asref)] // Testing macro handling of .as_ref() patterns

use anchor_lang::prelude::*;
use light_instruction_decoder_derive::instruction_decoder;
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
pub mod d10_token_accounts;
pub mod d11_zero_copy;
pub use d10_token_accounts::*;
pub use d11_zero_copy::*;
pub use instruction_accounts::*;
pub use instructions::{
    d7_infra_names::{
        D7_ALL_AUTH_SEED, D7_ALL_VAULT_SEED, D7_LIGHT_TOKEN_AUTH_SEED, D7_LIGHT_TOKEN_VAULT_SEED,
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
    derive_light_rent_sponsor_pda!("FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah");

#[inline]
pub fn program_rent_sponsor() -> Pubkey {
    Pubkey::from(PROGRAM_RENT_SPONSOR_DATA.0)
}

pub const GAME_SESSION_SEED: &str = "game_session";

#[instruction_decoder]
#[light_program]
#[program]
pub mod csdk_anchor_full_derived_test {
    #![allow(clippy::too_many_arguments)]

    use super::{
        amm_test::{Deposit, InitializeParams, InitializePool, Swap, TradeDirection, Withdraw},
        d5_markers::{
            D5AllMarkers, D5AllMarkersParams, D5LightToken, D5LightTokenParams, D5RentfreeBare,
            D5RentfreeBareParams, D5_ALL_VAULT_SEED, D5_VAULT_SEED,
        },
        d6_account_types::{D6Account, D6AccountParams, D6Boxed, D6BoxedParams},
        d7_infra_names::{
            D7AllNames, D7AllNamesParams, D7Creator, D7CreatorParams, D7LightTokenConfig,
            D7LightTokenConfigParams, D7Payer, D7PayerParams, D7_ALL_VAULT_SEED,
            D7_LIGHT_TOKEN_VAULT_SEED,
        },
        d8_builder_paths::{
            D8All, D8AllParams, D8MultiRentfree, D8MultiRentfreeParams, D8PdaOnly, D8PdaOnlyParams,
        },
        d9_seeds::{
            const_seed,
            identity_seed,
            // Helper types for const patterns
            AnotherHolder,
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
            // Instruction data tests (various params struct patterns)
            D9BigEndianParams,
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
            D9ChainedAsRefParams,
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
            D9ComplexMixedParams,
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
            D9ConstMixedParams,
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
            D9InstrBigEndian,
            D9InstrChainedAsRef,
            D9InstrComplexMixed,
            D9InstrConstMixed,
            D9InstrMixedCtx,
            D9InstrMultiField,
            D9InstrMultiU64,
            D9InstrSinglePubkey,
            D9InstrTriple,
            D9InstrU64,
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
            D9MixedCtxParams,
            D9MixedParams,
            D9MultiAssocConst,
            D9MultiAssocConstParams,
            D9MultiFieldParams,
            D9MultiU64Params,
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
            D9SinglePubkeyParams,
            D9Static,
            D9StaticParams,
            D9TraitAssocConst,
            D9TraitAssocConstParams,
            D9TripleParams,
            D9U64Params,
            HasSeed,
            SeedHolder,
            // Constant for qualified paths
            D9_QUALIFIED_LOCAL,
        },
        instruction_accounts::{
            CreateMintWithMetadata, CreateMintWithMetadataParams, CreatePdasAndMintAuto,
            CreateThreeMints, CreateThreeMintsParams, CreateTwoMints, CreateTwoMintsParams,
            VAULT_SEED,
        },
        instructions::d10_token_accounts::{
            D10SingleAta, D10SingleAtaMarkonly, D10SingleAtaMarkonlyParams, D10SingleAtaParams,
            D10SingleVault, D10SingleVaultParams,
        },
        instructions::d11_zero_copy::{
            // mixed_zc_borsh
            D11MixedZcBorsh,
            D11MixedZcBorshParams,
            // multiple_zc
            D11MultipleZc,
            D11MultipleZcParams,
            // with_ata
            D11ZcWithAta,
            D11ZcWithAtaParams,
            // with_ctx_seeds
            D11ZcWithCtxSeeds,
            D11ZcWithCtxSeedsParams,
            // with_mint_to
            D11ZcWithMintTo,
            D11ZcWithMintToParams,
            // with_params_seeds
            D11ZcWithParamsSeeds,
            D11ZcWithParamsSeedsParams,
            // with_vault
            D11ZcWithVault,
            D11ZcWithVaultParams,
        },
        FullAutoWithMintParams, LIGHT_CPI_SIGNER,
    };

    pub fn create_pdas_and_mint_auto<'info>(
        ctx: Context<'_, '_, '_, 'info, CreatePdasAndMintAuto<'info>>,
        params: FullAutoWithMintParams,
    ) -> Result<()> {
        use light_token::instruction::{
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

        // vault is mark-only - create manually via CreateTokenAccountCpi
        CreateTokenAccountCpi {
            payer: ctx.accounts.fee_payer.to_account_info(),
            account: ctx.accounts.vault.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            owner: ctx.accounts.vault_authority.key(),
        }
        .rent_free(
            ctx.accounts.light_token_config.to_account_info(),
            ctx.accounts.light_token_rent_sponsor.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            &crate::ID,
        )
        .invoke_signed(&[
            VAULT_SEED,
            ctx.accounts.mint.to_account_info().key.as_ref(),
            &[ctx.bumps.vault],
        ])?;

        CreateTokenAtaCpi {
            payer: ctx.accounts.fee_payer.to_account_info(),
            owner: ctx.accounts.fee_payer.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            ata: ctx.accounts.user_ata.to_account_info(),
            bump: params.user_ata_bump,
        }
        .idempotent()
        .rent_free(
            ctx.accounts.light_token_config.to_account_info(),
            ctx.accounts.light_token_rent_sponsor.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        )
        .invoke()?;

        if params.vault_mint_amount > 0 {
            CTokenMintToCpi {
                mint: ctx.accounts.mint.to_account_info(),
                destination: ctx.accounts.vault.to_account_info(),
                amount: params.vault_mint_amount,
                authority: ctx.accounts.mint_authority.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                max_top_up: None,
                fee_payer: None,
            }
            .invoke()?;
        }

        if params.user_ata_mint_amount > 0 {
            CTokenMintToCpi {
                mint: ctx.accounts.mint.to_account_info(),
                destination: ctx.accounts.user_ata.to_account_info(),
                amount: params.user_ata_mint_amount,
                authority: ctx.accounts.mint_authority.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                max_top_up: None,
                fee_payer: None,
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
    /// Also tests dynamic context name detection using "context" instead of "ctx".
    #[allow(unused_variables)]
    pub fn create_two_mints<'info>(
        context: Context<'_, '_, '_, 'info, CreateTwoMints<'info>>,
        params: CreateTwoMintsParams,
    ) -> Result<()> {
        // Both mints are created by the RentFree macro in pre_init
        // Nothing to do here - just verify both mints exist
        Ok(())
    }

    /// Test instruction that creates 3 mints in a single transaction.
    /// Tests the multi-mint support in the RentFree macro scales beyond 2.
    /// Also tests dynamic context name detection using "anchor_ctx" instead of "ctx".
    #[allow(unused_variables)]
    pub fn create_three_mints<'info>(
        anchor_ctx: Context<'_, '_, '_, 'info, CreateThreeMints<'info>>,
        params: CreateThreeMintsParams,
    ) -> Result<()> {
        // All 3 mints are created by the RentFree macro in pre_init
        // Nothing to do here - just verify all mints exist
        Ok(())
    }

    /// Test instruction that creates a mint with metadata.
    /// Tests the metadata support in the RentFree macro.
    /// Also tests dynamic context name detection using "c" (single letter) instead of "ctx".
    #[allow(unused_variables)]
    pub fn create_mint_with_metadata<'info>(
        c: Context<'_, '_, '_, 'info, CreateMintWithMetadata<'info>>,
        params: CreateMintWithMetadataParams,
    ) -> Result<()> {
        // Mint with metadata is created by the RentFree macro in pre_init
        // Nothing to do here - metadata is part of the mint creation
        Ok(())
    }

    /// AMM initialize instruction with all light account markers.
    /// Tests: 2x #[light_account(init)], 2x #[light_account(token)], 1x #[light_account(init)],
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

    /// AMM swap instruction with directional vault aliases.
    /// Tests divergent naming: input_vault/output_vault are aliases for token_0_vault/token_1_vault
    pub fn swap(
        ctx: Context<Swap>,
        amount_in: u64,
        minimum_amount_out: u64,
        direction: TradeDirection,
    ) -> Result<()> {
        crate::amm_test::process_swap(ctx, amount_in, minimum_amount_out, direction)
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

    /// D7: "light_token_config" naming variant for token accounts
    pub fn d7_light_token_config<'info>(
        ctx: Context<'_, '_, '_, 'info, D7LightTokenConfig<'info>>,
        params: D7LightTokenConfigParams,
    ) -> Result<()> {
        use light_token::instruction::CreateTokenAccountCpi;

        // Token vault is mark-only - create manually via CreateTokenAccountCpi
        CreateTokenAccountCpi {
            payer: ctx.accounts.fee_payer.to_account_info(),
            account: ctx.accounts.d7_light_token_vault.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            owner: ctx.accounts.d7_light_token_authority.key(),
        }
        .rent_free(
            ctx.accounts.light_token_config.to_account_info(),
            ctx.accounts.light_token_rent_sponsor.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            &crate::ID,
        )
        .invoke_signed(&[
            D7_LIGHT_TOKEN_VAULT_SEED,
            ctx.accounts.mint.to_account_info().key.as_ref(),
            &[params.vault_bump],
        ])?;

        Ok(())
    }

    /// D7: All naming variants combined (payer + light_token config/sponsor)
    pub fn d7_all_names<'info>(
        ctx: Context<'_, '_, '_, 'info, D7AllNames<'info>>,
        params: D7AllNamesParams,
    ) -> Result<()> {
        use light_token::instruction::CreateTokenAccountCpi;

        // Set up the PDA record
        ctx.accounts.d7_all_record.owner = params.owner;

        // Token vault is mark-only - create manually via CreateTokenAccountCpi
        CreateTokenAccountCpi {
            payer: ctx.accounts.payer.to_account_info(),
            account: ctx.accounts.d7_all_vault.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            owner: ctx.accounts.d7_all_authority.key(),
        }
        .rent_free(
            ctx.accounts.light_token_config.to_account_info(),
            ctx.accounts.light_token_rent_sponsor.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            &crate::ID,
        )
        .invoke_signed(&[
            D7_ALL_VAULT_SEED,
            ctx.accounts.mint.to_account_info().key.as_ref(),
            &[ctx.bumps.d7_all_vault],
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
        ctx.accounts.d9_qualified_bare_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: self:: prefix path
    pub fn d9_qualified_self<'info>(
        ctx: Context<'_, '_, '_, 'info, D9QualifiedSelf<'info>>,
        _params: D9QualifiedSelfParams,
    ) -> Result<()> {
        ctx.accounts.d9_qualified_self_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: crate:: prefix path
    pub fn d9_qualified_crate<'info>(
        ctx: Context<'_, '_, '_, 'info, D9QualifiedCrate<'info>>,
        _params: D9QualifiedCrateParams,
    ) -> Result<()> {
        ctx.accounts.d9_qualified_crate_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Deep nested crate path
    pub fn d9_qualified_deep<'info>(
        ctx: Context<'_, '_, '_, 'info, D9QualifiedDeep<'info>>,
        _params: D9QualifiedDeepParams,
    ) -> Result<()> {
        ctx.accounts.d9_qualified_deep_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Mixed qualified and bare paths
    pub fn d9_qualified_mixed<'info>(
        ctx: Context<'_, '_, '_, 'info, D9QualifiedMixed<'info>>,
        params: D9QualifiedMixedParams,
    ) -> Result<()> {
        ctx.accounts.d9_qualified_mixed_record.owner = params.owner;
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
        ctx.accounts.d9_method_as_ref_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: string_constant.as_bytes()
    pub fn d9_method_as_bytes<'info>(
        ctx: Context<'_, '_, '_, 'info, D9MethodAsBytes<'info>>,
        _params: D9MethodAsBytesParams,
    ) -> Result<()> {
        ctx.accounts.d9_method_as_bytes_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: crate::path::CONST.as_bytes()
    pub fn d9_method_qualified_as_bytes<'info>(
        ctx: Context<'_, '_, '_, 'info, D9MethodQualifiedAsBytes<'info>>,
        _params: D9MethodQualifiedAsBytesParams,
    ) -> Result<()> {
        ctx.accounts.d9_method_qualified_as_bytes_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: params.field.to_le_bytes().as_ref()
    pub fn d9_method_to_le_bytes<'info>(
        ctx: Context<'_, '_, '_, 'info, D9MethodToLeBytes<'info>>,
        _params: D9MethodToLeBytesParams,
    ) -> Result<()> {
        ctx.accounts.d9_method_to_le_bytes_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: params.field.to_be_bytes().as_ref()
    pub fn d9_method_to_be_bytes<'info>(
        ctx: Context<'_, '_, '_, 'info, D9MethodToBeBytes<'info>>,
        _params: D9MethodToBeBytesParams,
    ) -> Result<()> {
        ctx.accounts.d9_method_to_be_bytes_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Mixed methods in seeds
    pub fn d9_method_mixed<'info>(
        ctx: Context<'_, '_, '_, 'info, D9MethodMixed<'info>>,
        params: D9MethodMixedParams,
    ) -> Result<()> {
        ctx.accounts.d9_method_mixed_record.owner = params.owner;
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
        ctx.accounts.d9_bump_lit_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Constant seed with bump
    pub fn d9_bump_constant<'info>(
        ctx: Context<'_, '_, '_, 'info, D9BumpConstant<'info>>,
        _params: D9BumpConstantParams,
    ) -> Result<()> {
        ctx.accounts.d9_bump_const_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Qualified path with bump
    pub fn d9_bump_qualified<'info>(
        ctx: Context<'_, '_, '_, 'info, D9BumpQualified<'info>>,
        _params: D9BumpQualifiedParams,
    ) -> Result<()> {
        ctx.accounts.d9_bump_qual_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Param seed with bump
    pub fn d9_bump_param<'info>(
        ctx: Context<'_, '_, '_, 'info, D9BumpParam<'info>>,
        params: D9BumpParamParams,
    ) -> Result<()> {
        ctx.accounts.d9_bump_param_record.owner = params.owner;
        Ok(())
    }

    /// D9: Ctx account seed with bump
    pub fn d9_bump_ctx<'info>(
        ctx: Context<'_, '_, '_, 'info, D9BumpCtx<'info>>,
        _params: D9BumpCtxParams,
    ) -> Result<()> {
        ctx.accounts.d9_bump_ctx_record.owner = ctx.accounts.authority.key();
        Ok(())
    }

    /// D9: Multiple seeds with bump
    pub fn d9_bump_mixed<'info>(
        ctx: Context<'_, '_, '_, 'info, D9BumpMixed<'info>>,
        params: D9BumpMixedParams,
    ) -> Result<()> {
        ctx.accounts.d9_bump_mixed_record.owner = params.owner;
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
        ctx.accounts.d9_complex_three_record.owner = params.owner;
        Ok(())
    }

    /// D9: Four seeds
    pub fn d9_complex_four<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ComplexFour<'info>>,
        params: D9ComplexFourParams,
    ) -> Result<()> {
        ctx.accounts.d9_complex_four_record.owner = params.owner;
        Ok(())
    }

    /// D9: Five seeds with ctx account
    pub fn d9_complex_five<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ComplexFive<'info>>,
        params: D9ComplexFiveParams,
    ) -> Result<()> {
        ctx.accounts.d9_complex_five_record.owner = params.owner;
        Ok(())
    }

    /// D9: Qualified paths mixed with local
    pub fn d9_complex_qualified_mix<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ComplexQualifiedMix<'info>>,
        params: D9ComplexQualifiedMixParams,
    ) -> Result<()> {
        ctx.accounts.d9_complex_qualified_mix_record.owner = params.owner;
        Ok(())
    }

    /// D9: Function call combined with other seeds
    pub fn d9_complex_func<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ComplexFunc<'info>>,
        params: D9ComplexFuncParams,
    ) -> Result<()> {
        ctx.accounts.d9_complex_func_record.owner = params.key_a;
        Ok(())
    }

    /// D9: All qualified paths
    pub fn d9_complex_all_qualified<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ComplexAllQualified<'info>>,
        params: D9ComplexAllQualifiedParams,
    ) -> Result<()> {
        ctx.accounts.d9_complex_all_qualified_record.owner = params.owner;
        Ok(())
    }

    /// D9: Program ID as seed
    pub fn d9_complex_program_id<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ComplexProgramId<'info>>,
        params: D9ComplexProgramIdParams,
    ) -> Result<()> {
        ctx.accounts.d9_complex_program_id_record.owner = params.owner;
        Ok(())
    }

    /// D9: id() function call as seed
    pub fn d9_complex_id_func<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ComplexIdFunc<'info>>,
        params: D9ComplexIdFuncParams,
    ) -> Result<()> {
        ctx.accounts.d9_complex_id_func_record.owner = params.owner;
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
        ctx.accounts.d9_edge_empty_record.owner = params.owner;
        Ok(())
    }

    /// D9: Single byte constant
    pub fn d9_edge_single_byte<'info>(
        ctx: Context<'_, '_, '_, 'info, D9EdgeSingleByte<'info>>,
        _params: D9EdgeSingleByteParams,
    ) -> Result<()> {
        ctx.accounts.d9_edge_single_byte_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Single letter constant name
    pub fn d9_edge_single_letter<'info>(
        ctx: Context<'_, '_, '_, 'info, D9EdgeSingleLetter<'info>>,
        _params: D9EdgeSingleLetterParams,
    ) -> Result<()> {
        ctx.accounts.d9_edge_single_letter_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Constant name with digits
    pub fn d9_edge_digits<'info>(
        ctx: Context<'_, '_, '_, 'info, D9EdgeDigits<'info>>,
        _params: D9EdgeDigitsParams,
    ) -> Result<()> {
        ctx.accounts.d9_edge_digits_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Leading underscore constant
    pub fn d9_edge_underscore<'info>(
        ctx: Context<'_, '_, '_, 'info, D9EdgeUnderscore<'info>>,
        _params: D9EdgeUnderscoreParams,
    ) -> Result<()> {
        ctx.accounts.d9_edge_underscore_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Many literals in seeds
    pub fn d9_edge_many_literals<'info>(
        ctx: Context<'_, '_, '_, 'info, D9EdgeManyLiterals<'info>>,
        _params: D9EdgeManyLiteralsParams,
    ) -> Result<()> {
        ctx.accounts.d9_edge_many_literals_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Mixed edge cases
    pub fn d9_edge_mixed<'info>(
        ctx: Context<'_, '_, '_, 'info, D9EdgeMixed<'info>>,
        params: D9EdgeMixedParams,
    ) -> Result<()> {
        ctx.accounts.d9_edge_mixed_record.owner = params.owner;
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
        ctx.accounts.d9_external_sdk_types_record.owner = params.owner;
        Ok(())
    }

    /// D9: External crate (light_ctoken_types)
    pub fn d9_external_ctoken<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ExternalCtoken<'info>>,
        params: D9ExternalCtokenParams,
    ) -> Result<()> {
        ctx.accounts.d9_external_ctoken_record.owner = params.owner;
        Ok(())
    }

    /// D9: Multiple external crates mixed
    pub fn d9_external_mixed<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ExternalMixed<'info>>,
        params: D9ExternalMixedParams,
    ) -> Result<()> {
        ctx.accounts.d9_external_mixed_record.owner = params.owner;
        Ok(())
    }

    /// D9: External with local constant
    pub fn d9_external_with_local<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ExternalWithLocal<'info>>,
        params: D9ExternalWithLocalParams,
    ) -> Result<()> {
        ctx.accounts.d9_external_with_local_record.owner = params.owner;
        Ok(())
    }

    /// D9: External constant with bump
    pub fn d9_external_bump<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ExternalBump<'info>>,
        params: D9ExternalBumpParams,
    ) -> Result<()> {
        ctx.accounts.d9_external_bump_record.owner = params.owner;
        Ok(())
    }

    /// D9: Re-exported external constant
    pub fn d9_external_reexport<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ExternalReexport<'info>>,
        _params: D9ExternalReexportParams,
    ) -> Result<()> {
        ctx.accounts.d9_external_reexport_record.owner = ctx.accounts.fee_payer.key();
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
        ctx.accounts.d9_nested_simple_record.owner = params.nested.owner;
        Ok(())
    }

    /// D9: Double nested struct access
    pub fn d9_nested_double<'info>(
        ctx: Context<'_, '_, '_, 'info, D9NestedDouble<'info>>,
        params: D9NestedDoubleParams,
    ) -> Result<()> {
        ctx.accounts.d9_nested_double_record.owner = params.outer.nested.owner;
        Ok(())
    }

    /// D9: Nested array field access
    pub fn d9_nested_array_field<'info>(
        ctx: Context<'_, '_, '_, 'info, D9NestedArrayField<'info>>,
        params: D9NestedArrayFieldParams,
    ) -> Result<()> {
        ctx.accounts.d9_nested_array_field_record.owner = params.outer.nested.owner;
        Ok(())
    }

    /// D9: Array indexing params.arrays[2].as_slice()
    pub fn d9_array_index<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ArrayIndex<'info>>,
        _params: D9ArrayIndexParams,
    ) -> Result<()> {
        ctx.accounts.d9_array_index_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Nested struct with bytes conversion
    pub fn d9_nested_bytes<'info>(
        ctx: Context<'_, '_, '_, 'info, D9NestedBytes<'info>>,
        params: D9NestedBytesParams,
    ) -> Result<()> {
        ctx.accounts.d9_nested_bytes_record.owner = params.nested.owner;
        Ok(())
    }

    /// D9: Multiple nested seeds combined
    pub fn d9_nested_combined<'info>(
        ctx: Context<'_, '_, '_, 'info, D9NestedCombined<'info>>,
        params: D9NestedCombinedParams,
    ) -> Result<()> {
        ctx.accounts.d9_nested_combined_record.owner = params.outer.nested.owner;
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
        ctx.accounts.d9_assoc_const_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Associated constant with method
    pub fn d9_assoc_const_method<'info>(
        ctx: Context<'_, '_, '_, 'info, D9AssocConstMethod<'info>>,
        _params: D9AssocConstMethodParams,
    ) -> Result<()> {
        ctx.accounts.d9_assoc_const_method_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Multiple associated constants
    pub fn d9_multi_assoc_const<'info>(
        ctx: Context<'_, '_, '_, 'info, D9MultiAssocConst<'info>>,
        params: D9MultiAssocConstParams,
    ) -> Result<()> {
        ctx.accounts.d9_multi_assoc_const_record.owner = params.owner;
        Ok(())
    }

    /// D9: Const fn call
    pub fn d9_const_fn<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ConstFn<'info>>,
        _params: D9ConstFnParams,
    ) -> Result<()> {
        ctx.accounts.d9_const_fn_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Const fn with generic
    pub fn d9_const_fn_generic<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ConstFnGeneric<'info>>,
        _params: D9ConstFnGenericParams,
    ) -> Result<()> {
        ctx.accounts.d9_const_fn_generic_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Trait associated constant
    pub fn d9_trait_assoc_const<'info>(
        ctx: Context<'_, '_, '_, 'info, D9TraitAssocConst<'info>>,
        _params: D9TraitAssocConstParams,
    ) -> Result<()> {
        ctx.accounts.d9_trait_assoc_const_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Static variable
    pub fn d9_static<'info>(
        ctx: Context<'_, '_, '_, 'info, D9Static<'info>>,
        _params: D9StaticParams,
    ) -> Result<()> {
        ctx.accounts.d9_static_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Qualified const fn
    pub fn d9_qualified_const_fn<'info>(
        ctx: Context<'_, '_, '_, 'info, D9QualifiedConstFn<'info>>,
        _params: D9QualifiedConstFnParams,
    ) -> Result<()> {
        ctx.accounts.d9_qualified_const_fn_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Fully qualified associated constant
    pub fn d9_fully_qualified_assoc<'info>(
        ctx: Context<'_, '_, '_, 'info, D9FullyQualifiedAssoc<'info>>,
        _params: D9FullyQualifiedAssocParams,
    ) -> Result<()> {
        ctx.accounts.d9_fully_qualified_assoc_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Fully qualified trait associated constant
    pub fn d9_fully_qualified_trait<'info>(
        ctx: Context<'_, '_, '_, 'info, D9FullyQualifiedTrait<'info>>,
        _params: D9FullyQualifiedTraitParams,
    ) -> Result<()> {
        ctx.accounts.d9_fully_qualified_trait_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Fully qualified const fn with generic
    pub fn d9_fully_qualified_generic<'info>(
        ctx: Context<'_, '_, '_, 'info, D9FullyQualifiedGeneric<'info>>,
        _params: D9FullyQualifiedGenericParams,
    ) -> Result<()> {
        ctx.accounts.d9_fully_qualified_generic_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Combined const patterns
    pub fn d9_const_combined<'info>(
        ctx: Context<'_, '_, '_, 'info, D9ConstCombined<'info>>,
        params: D9ConstCombinedParams,
    ) -> Result<()> {
        ctx.accounts.d9_const_combined_record.owner = params.owner;
        Ok(())
    }

    // =========================================================================
    // D9 Instruction Data Tests (various params struct patterns)
    // =========================================================================

    /// D9: Standard params with single Pubkey field
    pub fn d9_instr_single_pubkey<'info>(
        ctx: Context<'_, '_, '_, 'info, D9InstrSinglePubkey<'info>>,
        params: D9SinglePubkeyParams,
    ) -> Result<()> {
        ctx.accounts.d9_instr_single_pubkey_record.owner = params.owner;
        Ok(())
    }

    /// D9: Params with u64 field requiring to_le_bytes
    pub fn d9_instr_u64<'info>(
        ctx: Context<'_, '_, '_, 'info, D9InstrU64<'info>>,
        _params: D9U64Params,
    ) -> Result<()> {
        ctx.accounts.d9_instr_u64_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Multiple data fields in seeds (owner + amount)
    pub fn d9_instr_multi_field<'info>(
        ctx: Context<'_, '_, '_, 'info, D9InstrMultiField<'info>>,
        params: D9MultiFieldParams,
    ) -> Result<()> {
        ctx.accounts.d9_instr_multi_field_record.owner = params.owner;
        Ok(())
    }

    /// D9: Mixed params and ctx account in seeds
    pub fn d9_instr_mixed_ctx<'info>(
        ctx: Context<'_, '_, '_, 'info, D9InstrMixedCtx<'info>>,
        params: D9MixedCtxParams,
    ) -> Result<()> {
        ctx.accounts.d9_instr_mixed_ctx_record.owner = params.data_key;
        Ok(())
    }

    /// D9: Three data fields with different types
    pub fn d9_instr_triple<'info>(
        ctx: Context<'_, '_, '_, 'info, D9InstrTriple<'info>>,
        params: D9TripleParams,
    ) -> Result<()> {
        ctx.accounts.d9_instr_triple_record.owner = params.key_a;
        Ok(())
    }

    /// D9: to_be_bytes conversion (big endian)
    pub fn d9_instr_big_endian<'info>(
        ctx: Context<'_, '_, '_, 'info, D9InstrBigEndian<'info>>,
        _params: D9BigEndianParams,
    ) -> Result<()> {
        ctx.accounts.d9_instr_big_endian_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Multiple u64 fields with to_le_bytes
    pub fn d9_instr_multi_u64<'info>(
        ctx: Context<'_, '_, '_, 'info, D9InstrMultiU64<'info>>,
        _params: D9MultiU64Params,
    ) -> Result<()> {
        ctx.accounts.d9_instr_multi_u64_record.owner = ctx.accounts.fee_payer.key();
        Ok(())
    }

    /// D9: Pubkey with as_ref chained
    pub fn d9_instr_chained_as_ref<'info>(
        ctx: Context<'_, '_, '_, 'info, D9InstrChainedAsRef<'info>>,
        params: D9ChainedAsRefParams,
    ) -> Result<()> {
        ctx.accounts.d9_instr_chained_as_ref_record.owner = params.key;
        Ok(())
    }

    /// D9: Constant mixed with params
    pub fn d9_instr_const_mixed<'info>(
        ctx: Context<'_, '_, '_, 'info, D9InstrConstMixed<'info>>,
        params: D9ConstMixedParams,
    ) -> Result<()> {
        ctx.accounts.d9_instr_const_mixed_record.owner = params.owner;
        Ok(())
    }

    /// D9: Complex mixed - literal + constant + ctx + params
    pub fn d9_instr_complex_mixed<'info>(
        ctx: Context<'_, '_, '_, 'info, D9InstrComplexMixed<'info>>,
        params: D9ComplexMixedParams,
    ) -> Result<()> {
        ctx.accounts.d9_instr_complex_mixed_record.owner = params.data_owner;
        Ok(())
    }

    // =========================================================================
    // D5 Additional Markers Tests
    // =========================================================================

    /// D5: #[light_account(token)] attribute test
    pub fn d5_light_token<'info>(
        ctx: Context<'_, '_, '_, 'info, D5LightToken<'info>>,
        params: D5LightTokenParams,
    ) -> Result<()> {
        use light_token::instruction::CreateTokenAccountCpi;

        // Token vault is mark-only - create manually via CreateTokenAccountCpi
        CreateTokenAccountCpi {
            payer: ctx.accounts.fee_payer.to_account_info(),
            account: ctx.accounts.d5_token_vault.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            owner: ctx.accounts.vault_authority.key(),
        }
        .rent_free(
            ctx.accounts.light_token_config.to_account_info(),
            ctx.accounts.light_token_rent_sponsor.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            &crate::ID,
        )
        .invoke_signed(&[
            D5_VAULT_SEED,
            ctx.accounts.mint.to_account_info().key.as_ref(),
            &[params.vault_bump],
        ])?;

        Ok(())
    }

    /// D5: All markers combined (#[light_account(init)] + #[light_account(token)])
    pub fn d5_all_markers<'info>(
        ctx: Context<'_, '_, '_, 'info, D5AllMarkers<'info>>,
        params: D5AllMarkersParams,
    ) -> Result<()> {
        use light_token::instruction::CreateTokenAccountCpi;

        // Set up the PDA record
        ctx.accounts.d5_all_record.owner = params.owner;

        // Token vault is mark-only - create manually via CreateTokenAccountCpi
        CreateTokenAccountCpi {
            payer: ctx.accounts.fee_payer.to_account_info(),
            account: ctx.accounts.d5_all_vault.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            owner: ctx.accounts.d5_all_authority.key(),
        }
        .rent_free(
            ctx.accounts.light_token_config.to_account_info(),
            ctx.accounts.light_token_rent_sponsor.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            &crate::ID,
        )
        .invoke_signed(&[
            D5_ALL_VAULT_SEED,
            ctx.accounts.mint.to_account_info().key.as_ref(),
            &[ctx.bumps.d5_all_vault],
        ])?;

        Ok(())
    }

    // =========================================================================
    // D10 Token Account Tests (auto-generated via #[light_account(init, token)])
    // =========================================================================

    /// D10: Single vault with #[light_account(init, token, ...)]
    /// This tests automatic code generation for token account creation.
    /// The macro should generate CreateTokenAccountCpi in LightFinalize.
    /// Also tests dynamic context name detection using "my_ctx" instead of "ctx".
    #[allow(unused_variables)]
    pub fn d10_single_vault<'info>(
        my_ctx: Context<'_, '_, '_, 'info, D10SingleVault<'info>>,
        params: D10SingleVaultParams,
    ) -> Result<()> {
        // Token account creation is handled by the LightFinalize trait implementation
        // generated by the #[light_account(init, token, ...)] macro.
        // This handler can be empty - the macro handles everything.
        Ok(())
    }

    /// D10: Single ATA with #[light_account(init, associated_token, ...)]
    /// This tests automatic code generation for ATA creation.
    /// The macro should generate create_associated_token_account_idempotent in LightFinalize.
    /// Also tests dynamic context name detection using "cx" instead of "ctx".
    #[allow(unused_variables)]
    pub fn d10_single_ata<'info>(
        cx: Context<'_, '_, '_, 'info, D10SingleAta<'info>>,
        params: D10SingleAtaParams,
    ) -> Result<()> {
        // ATA creation is handled by the LightFinalize trait implementation
        // generated by the #[light_account(init, associated_token, ...)] macro.
        // This handler can be empty - the macro handles everything.
        Ok(())
    }

    /// D10: Mark-only ATA with #[light_account(associated_token::...)] (NO init keyword).
    /// Tests that the macro generates seed structs for decompression support while
    /// skipping the CPI call. User manually calls CreateTokenAtaCpi in handler.
    pub fn d10_single_ata_markonly<'info>(
        ctx: Context<'_, '_, '_, 'info, D10SingleAtaMarkonly<'info>>,
        params: D10SingleAtaMarkonlyParams,
    ) -> Result<()> {
        use light_token::instruction::CreateTokenAtaCpi;

        // Mark-only: LightPreInit/LightFinalize are no-ops, we create the ATA manually
        CreateTokenAtaCpi {
            payer: ctx.accounts.fee_payer.to_account_info(),
            owner: ctx.accounts.d10_markonly_ata_owner.to_account_info(),
            mint: ctx.accounts.d10_markonly_ata_mint.to_account_info(),
            ata: ctx.accounts.d10_markonly_ata.to_account_info(),
            bump: params.ata_bump,
        }
        .idempotent()
        .rent_free(
            ctx.accounts.light_token_config.to_account_info(),
            ctx.accounts.light_token_rent_sponsor.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        )
        .invoke()?;

        Ok(())
    }

    // =========================================================================
    // D11 Zero-copy (AccountLoader) Tests
    // =========================================================================

    /// D11: Zero-copy + Token Vault
    /// Tests `#[light_account(init, zero_copy)]` combined with token vault creation.
    /// Token vault creation is handled automatically by the `#[light_account(init, token, ...)]` macro.
    pub fn d11_zc_with_vault<'info>(
        ctx: Context<'_, '_, '_, 'info, D11ZcWithVault<'info>>,
        params: D11ZcWithVaultParams,
    ) -> Result<()> {
        // Initialize zero-copy record
        let mut record = ctx.accounts.zc_vault_record.load_init()?;
        record.owner = params.owner;
        record.counter = 0;
        // Token vault creation is handled by the LightFinalize trait implementation
        // generated by the #[light_account(init, token, ...)] macro.
        Ok(())
    }

    /// D11: Zero-copy + ATA
    /// Tests `#[light_account(init, zero_copy)]` combined with ATA creation.
    /// ATA creation is handled automatically by the `#[light_account(init, associated_token, ...)]` macro.
    pub fn d11_zc_with_ata<'info>(
        ctx: Context<'_, '_, '_, 'info, D11ZcWithAta<'info>>,
        params: D11ZcWithAtaParams,
    ) -> Result<()> {
        // Initialize zero-copy record
        let mut record = ctx.accounts.zc_ata_record.load_init()?;
        record.owner = params.owner;
        record.counter = 0;
        // ATA creation is handled by the LightFinalize trait implementation
        // generated by the #[light_account(init, associated_token, ...)] macro.
        Ok(())
    }

    /// D11: Multiple zero-copy PDAs
    /// Tests `#[light_account(init, zero_copy)]` with multiple AccountLoader fields.
    pub fn d11_multiple_zc<'info>(
        ctx: Context<'_, '_, '_, 'info, D11MultipleZc<'info>>,
        params: D11MultipleZcParams,
    ) -> Result<()> {
        let mut record1 = ctx.accounts.zc_record_1.load_init()?;
        record1.owner = params.owner;
        record1.counter = 1;

        let mut record2 = ctx.accounts.zc_record_2.load_init()?;
        record2.owner = params.owner;
        record2.counter = 2;

        Ok(())
    }

    /// D11: Mixed zero-copy and Borsh accounts
    /// Tests `#[light_account(init, zero_copy)]` alongside regular `#[light_account(init)]`.
    pub fn d11_mixed_zc_borsh<'info>(
        ctx: Context<'_, '_, '_, 'info, D11MixedZcBorsh<'info>>,
        params: D11MixedZcBorshParams,
    ) -> Result<()> {
        // Initialize zero-copy account
        let mut zc = ctx.accounts.zc_mixed_record.load_init()?;
        zc.owner = params.owner;
        zc.counter = 100;

        // Initialize Borsh account
        ctx.accounts.borsh_record.owner = params.owner;
        ctx.accounts.borsh_record.counter = 200;

        Ok(())
    }

    /// D11: Zero-copy with ctx.accounts.* seeds
    /// Tests `#[light_account(init, zero_copy)]` with context account seeds.
    pub fn d11_zc_with_ctx_seeds<'info>(
        ctx: Context<'_, '_, '_, 'info, D11ZcWithCtxSeeds<'info>>,
        params: D11ZcWithCtxSeedsParams,
    ) -> Result<()> {
        let mut record = ctx.accounts.zc_ctx_record.load_init()?;
        record.owner = params.owner;
        record.authority = ctx.accounts.authority.key();
        record.value = 42;

        Ok(())
    }

    /// D11: Zero-copy with params-only seeds
    /// Tests `#[light_account(init, zero_copy)]` with params seeds not on struct.
    pub fn d11_zc_with_params_seeds<'info>(
        ctx: Context<'_, '_, '_, 'info, D11ZcWithParamsSeeds<'info>>,
        params: D11ZcWithParamsSeedsParams,
    ) -> Result<()> {
        let mut record = ctx.accounts.zc_params_record.load_init()?;
        record.owner = params.owner;
        record.data = params.category_id;

        Ok(())
    }

    /// D11: Zero-copy + Vault + MintTo
    /// Tests `#[light_account(init, zero_copy)]` combined with vault and token minting.
    /// Token vault creation is handled automatically by the `#[light_account(init, token, ...)]` macro.
    pub fn d11_zc_with_mint_to<'info>(
        ctx: Context<'_, '_, '_, 'info, D11ZcWithMintTo<'info>>,
        params: D11ZcWithMintToParams,
    ) -> Result<()> {
        use light_token::instruction::MintToCpi;

        // Initialize zero-copy record
        let mut record = ctx.accounts.zc_mint_record.load_init()?;
        record.owner = params.owner;
        record.counter = params.mint_amount;
        // Token vault creation is handled by the LightFinalize trait implementation
        // generated by the #[light_account(init, token, ...)] macro.

        // Mint tokens to vault (this is additional business logic)
        if params.mint_amount > 0 {
            MintToCpi {
                mint: ctx.accounts.d11_mint.to_account_info(),
                destination: ctx.accounts.d11_mint_vault.to_account_info(),
                amount: params.mint_amount,
                authority: ctx.accounts.mint_authority.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                max_top_up: None,
                fee_payer: None,
            }
            .invoke()?;
        }

        Ok(())
    }
}

// =============================================================================
// Custom Instruction Decoder with Account Names and Params Decoding
// =============================================================================

/// Custom instruction decoder enum that provides account names and decoded params.
/// This uses the enhanced `#[derive(InstructionDecoder)]` macro with variant-level attributes
/// that reference Anchor's generated types.
#[derive(light_instruction_decoder_derive::InstructionDecoder)]
#[instruction_decoder(
    program_id = "FAMipfVEhN4hjCLpKCvjDXXfzLsoVTqQccXzePz1L1ah",
    program_name = "Csdk Anchor Full Derived Test"
)]
pub enum CsdkTestInstruction {
    /// Create PDAs and mint in auto mode
    #[instruction_decoder(
        accounts = instruction_accounts::CreatePdasAndMintAuto,
        params = instruction_accounts::FullAutoWithMintParams
    )]
    CreatePdasAndMintAuto,

    /// Create two mints in a single transaction
    #[instruction_decoder(
        accounts = instruction_accounts::CreateTwoMints,
        params = instruction_accounts::CreateTwoMintsParams
    )]
    CreateTwoMints,

    /// Create three mints in a single transaction
    #[instruction_decoder(
        accounts = instruction_accounts::CreateThreeMints,
        params = instruction_accounts::CreateThreeMintsParams
    )]
    CreateThreeMints,

    /// Create mint with metadata
    #[instruction_decoder(
        accounts = instruction_accounts::CreateMintWithMetadata,
        params = instruction_accounts::CreateMintWithMetadataParams
    )]
    CreateMintWithMetadata,
}
