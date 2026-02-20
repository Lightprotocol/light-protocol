//! Compressible instructions generation - orchestration module.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Item, ItemMod, Result};

// Re-export types from parsing, compress, and variant_enum for external use
pub use super::compress::CompressibleAccountInfo;
use super::{
    compress::CompressBuilder,
    decompress::DecompressBuilder,
    parsing::{
        convert_classified_to_seed_elements, convert_classified_to_seed_elements_vec,
        extract_context_and_params, macro_error, wrap_function_with_light,
    },
    variant_enum::LightVariantBuilder,
};
pub use super::{
    parsing::{
        extract_ctx_seed_fields, extract_data_seed_fields, InstructionDataSpec, InstructionVariant,
        SeedElement, TokenSeedSpec,
    },
    variant_enum::PdaCtxSeedInfo,
};
use crate::{
    light_pdas::{
        backend::{AnchorBackend, CodegenBackend, PinocchioBackend},
        shared_utils::{ident_to_type, qualify_type_with_crate},
    },
    utils::to_snake_case,
};

// =============================================================================
// MAIN CODEGEN
// =============================================================================

/// Shared code generation used by both `#[light_program]` and `#[derive(LightProgram)]`.
///
/// Returns a `Vec<TokenStream>` of all generated items (enums, structs, trait impls,
/// instruction handlers, etc.) that can be injected into a module or returned directly.
#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_light_program_items(
    compressible_accounts: Vec<CompressibleAccountInfo>,
    pda_seeds: Option<Vec<TokenSeedSpec>>,
    token_seeds: Option<Vec<TokenSeedSpec>>,
    instruction_data: Vec<InstructionDataSpec>,
    crate_ctx: &crate::light_pdas::parsing::CrateContext,
    has_mint_fields: bool,
    has_ata_fields: bool,
    pda_variant_code: TokenStream,
    enum_name: Option<&syn::Ident>,
) -> Result<Vec<TokenStream>> {
    generate_light_program_items_with_backend(
        compressible_accounts,
        pda_seeds,
        token_seeds,
        instruction_data,
        crate_ctx,
        has_mint_fields,
        has_ata_fields,
        pda_variant_code,
        enum_name,
        &AnchorBackend,
    )
}

/// Unified code generation with backend abstraction.
///
/// This function contains all the shared logic between Anchor and Pinocchio code generation,
/// using the `CodegenBackend` trait to handle framework-specific differences.
#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_light_program_items_with_backend(
    compressible_accounts: Vec<CompressibleAccountInfo>,
    pda_seeds: Option<Vec<TokenSeedSpec>>,
    token_seeds: Option<Vec<TokenSeedSpec>>,
    instruction_data: Vec<InstructionDataSpec>,
    crate_ctx: &crate::light_pdas::parsing::CrateContext,
    has_mint_fields: bool,
    has_ata_fields: bool,
    pda_variant_code: TokenStream,
    enum_name: Option<&syn::Ident>,
    backend: &dyn CodegenBackend,
) -> Result<Vec<TokenStream>> {
    // TODO: Unify seed extraction - currently #[light_program] extracts seeds from Anchor's
    // #[account(seeds = [...])] automatically, while #[derive(LightAccounts)] requires
    // explicit token::seeds = [...] in #[light_account]. Consider removing the duplicate
    // seed specification requirement and always using Anchor seeds.
    if let Some(ref token_seed_specs) = token_seeds {
        for spec in token_seed_specs {
            if spec.seeds.is_empty() {
                return Err(macro_error!(
                    &spec.variant,
                    "Token account '{}' must have seeds in #[account(seeds = [...])] for PDA signing.",
                    spec.variant
                ));
            }
        }
    }

    let pda_ctx_seeds: Vec<PdaCtxSeedInfo> = pda_seeds
        .as_ref()
        .map(|specs| {
            specs
                .iter()
                .map(|spec| {
                    let ctx_fields = extract_ctx_seed_fields(&spec.seeds);
                    // Use inner_type if available (from #[light_account(init)] fields), otherwise fall back to variant as type
                    let inner_type = spec
                        .inner_type
                        .clone()
                        .unwrap_or_else(|| ident_to_type(&spec.variant));

                    // Look up the state struct's field names from CrateContext
                    let state_field_names: std::collections::HashSet<String> = crate_ctx
                        .get_struct_fields(&inner_type)
                        .map(|fields| fields.into_iter().collect())
                        .unwrap_or_default();

                    // Extract params-only seed fields (data.* fields that don't exist on state)
                    let params_only_seed_fields =
                        crate::light_pdas::seeds::get_params_only_seed_fields_from_spec(
                            spec,
                            &state_field_names,
                        );

                    // Calculate seed_count = number of seeds + 1 (for bump)
                    let seed_count = spec.seeds.len() + 1;

                    PdaCtxSeedInfo::with_state_fields(
                        spec.variant.clone(),
                        inner_type,
                        ctx_fields,
                        state_field_names,
                        params_only_seed_fields,
                        seed_count,
                    )
                })
                .collect()
        })
        .unwrap_or_default();

    // Determine if we have token seeds early (needed for variant builder)
    let has_token_seeds_early = token_seeds.as_ref().map(|t| !t.is_empty()).unwrap_or(false);

    // Generate variant enum and traits only if there are PDA seeds
    // For mint-only programs (no PDA state accounts), generate minimal placeholder code
    let account_crate = backend.account_crate();
    let serialize_derive = backend.serialize_derive();
    let deserialize_derive = backend.deserialize_derive();

    let enum_and_traits = if pda_ctx_seeds.is_empty() {
        // Generate minimal code for mint-only programs that matches trait signatures
        quote! {
            /// Placeholder enum for programs that only use Light mints without state accounts.
            #[derive(Clone, Debug, #serialize_derive, #deserialize_derive)]
            pub enum LightAccountVariant {
                /// Placeholder variant for mint-only programs
                Empty,
            }

            impl Default for LightAccountVariant {
                fn default() -> Self {
                    Self::Empty
                }
            }

            impl #account_crate::hasher::DataHasher for LightAccountVariant {
                fn hash<H: #account_crate::hasher::Hasher>(&self) -> std::result::Result<[u8; 32], #account_crate::hasher::HasherError> {
                    match self {
                        Self::Empty => Err(#account_crate::hasher::HasherError::EmptyInput),
                    }
                }
            }

            impl #account_crate::LightDiscriminator for LightAccountVariant {
                const LIGHT_DISCRIMINATOR: [u8; 8] = [0; 8];
                const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
            }

            impl #account_crate::HasCompressionInfo for LightAccountVariant {
                fn compression_info(&self) -> std::result::Result<&#account_crate::CompressionInfo, #account_crate::LightSdkTypesError> {
                    Err(#account_crate::LightSdkTypesError::InvalidInstructionData)
                }

                fn compression_info_mut(&mut self) -> std::result::Result<&mut #account_crate::CompressionInfo, #account_crate::LightSdkTypesError> {
                    Err(#account_crate::LightSdkTypesError::InvalidInstructionData)
                }

                fn compression_info_mut_opt(&mut self) -> &mut Option<#account_crate::CompressionInfo> {
                    panic!("compression_info_mut_opt not supported for mint-only programs")
                }

                fn set_compression_info_none(&mut self) -> std::result::Result<(), #account_crate::LightSdkTypesError> {
                    Err(#account_crate::LightSdkTypesError::InvalidInstructionData)
                }
            }

            impl #account_crate::Size for LightAccountVariant {
                fn size(&self) -> std::result::Result<usize, #account_crate::LightSdkTypesError> {
                    Err(#account_crate::LightSdkTypesError::InvalidInstructionData)
                }
            }

            // Pack trait is only available off-chain (client-side)
            #[cfg(not(target_os = "solana"))]
            impl<AM: #account_crate::AccountMetaTrait> #account_crate::Pack<AM> for LightAccountVariant {
                type Packed = Self;
                fn pack(&self, _remaining_accounts: &mut #account_crate::interface::instruction::PackedAccounts<AM>) -> std::result::Result<Self::Packed, #account_crate::LightSdkTypesError> {
                    Ok(Self::Empty)
                }
            }

            impl<AI: #account_crate::AccountInfoTrait> #account_crate::Unpack<AI> for LightAccountVariant {
                type Unpacked = Self;
                fn unpack(&self, _remaining_accounts: &[AI]) -> std::result::Result<Self::Unpacked, #account_crate::LightSdkTypesError> {
                    Ok(Self::Empty)
                }
            }

            /// Wrapper for compressed account data (mint-only placeholder).
            #[derive(Clone, Debug, #serialize_derive, #deserialize_derive)]
            pub struct LightAccountData {
                pub meta: #account_crate::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                pub data: LightAccountVariant,
            }

            impl Default for LightAccountData {
                fn default() -> Self {
                    Self {
                        meta: #account_crate::account_meta::CompressedAccountMetaNoLamportsNoAddress::default(),
                        data: LightAccountVariant::default(),
                    }
                }
            }

            // Note: No DecompressibleAccount impl for mint-only programs
            // since they don't have PDAs to decompress.
        }
    } else {
        // Include token variants as first-class members if the program has token fields
        let builder = LightVariantBuilder::new(&pda_ctx_seeds);
        let builder = if let Some(ref token_seed_specs) = token_seeds {
            if !token_seed_specs.is_empty() {
                builder.with_token_seeds(token_seed_specs)
            } else {
                builder
            }
        } else {
            builder
        };
        builder.build_with_backend(backend)?
    };

    // Collect all unique params-only seed fields across all variants for SeedParams struct
    // Use BTreeMap for deterministic ordering
    let mut all_params_only_fields: std::collections::BTreeMap<String, syn::Type> =
        std::collections::BTreeMap::new();
    for ctx_info in &pda_ctx_seeds {
        for (field_name, field_type, _) in &ctx_info.params_only_seed_fields {
            let field_str = field_name.to_string();
            all_params_only_fields
                .entry(field_str)
                .or_insert_with(|| field_type.clone());
        }
    }

    let seed_params_struct = if all_params_only_fields.is_empty() {
        quote! {
            #[derive(#serialize_derive, #deserialize_derive, Clone, Debug, Default)]
            pub struct SeedParams;
        }
    } else {
        // Collect into Vec for consistent ordering between field declarations and Default impl
        let sorted_fields: Vec<_> = all_params_only_fields.iter().collect();
        let seed_param_fields: Vec<_> = sorted_fields
            .iter()
            .map(|(name, ty)| {
                let field_ident = format_ident!("{}", name);
                quote! { pub #field_ident: Option<#ty> }
            })
            .collect();
        let seed_param_defaults: Vec<_> = sorted_fields
            .iter()
            .map(|(name, _)| {
                let field_ident = format_ident!("{}", name);
                quote! { #field_ident: None }
            })
            .collect();
        quote! {
            #[derive(#serialize_derive, #deserialize_derive, Clone, Debug)]
            pub struct SeedParams {
                #(#seed_param_fields,)*
            }
            impl Default for SeedParams {
                fn default() -> Self {
                    Self {
                        #(#seed_param_defaults,)*
                    }
                }
            }
        }
    };

    let sdk_error_type = backend.sdk_error_type();
    let program_error_type = backend.program_error_type();

    let seeds_structs_and_constructors: Vec<TokenStream> = if let Some(ref pda_seed_specs) =
        pda_seeds
    {
        pda_seed_specs
            .iter()
            .zip(pda_ctx_seeds.iter())
            .map(|(spec, ctx_info)| {
                // Use variant_name for naming (struct, constructor, enum variant)
                let variant_name = &ctx_info.variant_name;
                // Use inner_type for deserialization - qualify with crate:: for accessibility
                let inner_type = qualify_type_with_crate(&ctx_info.inner_type);
                // Use the existing Seeds struct generated by #[derive(LightAccounts)]
                let seeds_struct_name = format_ident!("{}Seeds", variant_name);
                let constructor_name = format_ident!("{}", to_snake_case(&variant_name.to_string()));
                let _ctx_fields = &ctx_info.ctx_seed_fields;
                let _params_only_fields = &ctx_info.params_only_seed_fields;
                let data_fields = extract_data_seed_fields(&spec.seeds);

                // Data verifications and deserialization differ by backend
                let (data_verifications, deserialize_code, variant_data, return_type) =
                    if backend.is_pinocchio() {
                        // Pinocchio: use BorshDeserialize with light_account_pinocchio errors
                        let verifications: Vec<_> = data_fields.iter().filter_map(|field| {
                            let field_str = field.to_string();
                            if !ctx_info.state_field_names.contains(&field_str) {
                                return None;
                            }
                            Some(quote! {
                                if data.#field != seeds.#field {
                                    return std::result::Result::Err(
                                        #sdk_error_type::InvalidInstructionData
                                    );
                                }
                            })
                        }).collect();

                        let deser = quote! {
                            use borsh::BorshDeserialize;
                            let data: #inner_type = BorshDeserialize::deserialize(&mut &account_data[..])
                                .map_err(|_| #sdk_error_type::Borsh)?;
                        };

                        (verifications, deser, quote! { data }, quote! { #sdk_error_type })
                    } else {
                        // Anchor: use AnchorDeserialize with anchor errors
                        let verifications: Vec<_> = data_fields.iter().filter_map(|field| {
                            let field_str = field.to_string();
                            if !ctx_info.state_field_names.contains(&field_str) {
                                return None;
                            }
                            Some(quote! {
                                if data.#field != seeds.#field {
                                    return std::result::Result::Err(LightInstructionError::SeedMismatch.into());
                                }
                            })
                        }).collect();

                        let deser = quote! {
                            use anchor_lang::AnchorDeserialize;
                            let data: #inner_type = AnchorDeserialize::deserialize(&mut &account_data[..])
                                .map_err(|_| anchor_lang::error::Error::from(anchor_lang::error::ErrorCode::AccountDidNotDeserialize))?;
                        };

                        (verifications, deser, quote! { data }, quote! { #program_error_type })
                    };

                // For Pinocchio, the constructor already returns LightSdkTypesError (same
                // as IntoVariant's error type), so pass through directly to preserve
                // specific error variants (e.g. Borsh). For Anchor, the constructor
                // returns anchor_lang::error::Error which needs type conversion.
                let into_variant_body = if backend.is_pinocchio() {
                    quote! {
                        LightAccountVariant::#constructor_name(data, self)
                    }
                } else {
                    quote! {
                        LightAccountVariant::#constructor_name(data, self)
                            .map_err(|_| #sdk_error_type::InvalidInstructionData)
                    }
                };

                let generated = quote! {
                    impl LightAccountVariant {
                        /// Construct a #variant_name variant from account data and seeds.
                        pub fn #constructor_name(
                            account_data: &[u8],
                            seeds: #seeds_struct_name,
                        ) -> std::result::Result<Self, #return_type> {
                            #deserialize_code

                            #(#data_verifications)*

                            // Create the variant using struct syntax
                            std::result::Result::Ok(Self::#variant_name {
                                seeds,
                                data: #variant_data,
                            })
                        }
                    }
                    impl #account_crate::IntoVariant<LightAccountVariant> for #seeds_struct_name {
                        fn into_variant(self, data: &[u8]) -> std::result::Result<LightAccountVariant, #sdk_error_type> {
                            #into_variant_body
                        }
                    }
                };
                generated
            })
            .collect()
    } else {
        Vec::new()
    };

    let has_pda_seeds = pda_seeds.as_ref().map(|p| !p.is_empty()).unwrap_or(false);
    let has_token_seeds = token_seeds.as_ref().map(|t| !t.is_empty()).unwrap_or(false);

    let instruction_variant = match (has_pda_seeds, has_token_seeds, has_mint_fields, has_ata_fields) {
        (true, true, _, _) => InstructionVariant::Mixed,
        (true, false, _, _) => InstructionVariant::PdaOnly,
        (false, true, _, _) => InstructionVariant::TokenOnly,
        (false, false, true, _) => InstructionVariant::MintOnly,
        (false, false, false, true) => InstructionVariant::AtaOnly,
        (false, false, false, false) => {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "No #[light_account(init)], #[light_account(init, mint::...)], #[light_account(init, associated_token::...)], or #[light_account(token::...)] fields found.\n\
                 At least one light account field must be provided."
            ))
        }
    };

    // Create CompressBuilder to generate all compress-related code
    let compress_builder = CompressBuilder::new(compressible_accounts.clone(), instruction_variant);
    compress_builder.validate()?;

    let size_validation_checks = compress_builder.generate_size_validation_with_backend(backend)?;
    let discriminator_collision_checks =
        compress_builder.generate_discriminator_collision_checks(backend)?;
    // Error codes are only generated for Anchor
    let error_codes = if !backend.is_pinocchio() {
        Some(compress_builder.generate_error_codes()?)
    } else {
        None
    };

    // Create DecompressBuilder to generate all decompress-related code
    let decompress_builder = DecompressBuilder::new(
        pda_ctx_seeds.clone(),
        pda_seeds.clone(),
        has_token_seeds_early,
    );
    // Note: DecompressBuilder validation is optional for now since pda_seeds may be empty for TokenOnly

    // Accounts structs and seed provider impls differ by backend
    let decompress_accounts = if !backend.is_pinocchio() {
        Some(decompress_builder.generate_accounts_struct()?)
    } else {
        None
    };
    let pda_seed_provider_impls =
        decompress_builder.generate_seed_provider_impls_with_backend(backend)?;

    // Generate trait impls and decompress processor/instruction based on program type.
    // These are only generated for Anchor - Pinocchio programs use enum associated functions instead.
    // v2 interface: no DecompressContext trait needed - uses DecompressVariant on PackedLightAccountVariant.
    let (trait_impls, decompress_processor_fn, decompress_instruction) = if backend.is_pinocchio() {
        // Pinocchio: no trait impls, processor module, or instruction handlers
        (None, None, None)
    } else if !pda_ctx_seeds.is_empty() && has_token_seeds_early {
        // Anchor Mixed program: PDAs + Tokens - generate full impl with token checking.
        // Token variants are now first-class members of PackedLightAccountVariant,
        // so we match against the individual token variant names.
        let token_variant_names: Vec<_> = token_seeds
            .as_ref()
            .map(|specs| specs.iter().map(|s| &s.variant).collect())
            .unwrap_or_default();

        let token_match_arms: Vec<_> = token_variant_names
            .iter()
            .map(|name| quote! { PackedLightAccountVariant::#name(_) => true, })
            .collect();

        let trait_impls: syn::ItemMod = syn::parse_quote! {
            mod __trait_impls {
                use super::*;

                impl #account_crate::HasTokenVariant for LightAccountData {
                    fn is_packed_token(&self) -> bool {
                        match &self.data {
                            #(#token_match_arms)*
                            _ => false,
                        }
                    }
                }
            }
        };
        let decompress_processor_fn = decompress_builder.generate_processor()?;
        let decompress_instruction = decompress_builder.generate_entrypoint()?;
        (
            Some(trait_impls),
            Some(decompress_processor_fn),
            Some(decompress_instruction),
        )
    } else if !pda_ctx_seeds.is_empty() {
        // Anchor PDA-only program: simplified impl without token checking
        let trait_impls: syn::ItemMod = syn::parse_quote! {
            mod __trait_impls {
                use super::*;

                impl #account_crate::HasTokenVariant for LightAccountData {
                    fn is_packed_token(&self) -> bool {
                        // PDA-only programs have no token variants
                        false
                    }
                }
            }
        };
        let decompress_processor_fn = decompress_builder.generate_processor()?;
        let decompress_instruction = decompress_builder.generate_entrypoint()?;
        (
            Some(trait_impls),
            Some(decompress_processor_fn),
            Some(decompress_instruction),
        )
    } else {
        // Anchor Mint-only programs: placeholder impl
        let trait_impls: syn::ItemMod = syn::parse_quote! {
            mod __trait_impls {
                use super::*;

                impl #account_crate::HasTokenVariant for LightAccountData {
                    fn is_packed_token(&self) -> bool {
                        match &self.data {
                            LightAccountVariant::Empty => false,
                            _ => true,
                        }
                    }
                }
            }
        };
        (Some(trait_impls), None, None)
    };

    // Anchor-only: accounts structs, processor module, and config instructions
    #[allow(unused_variables)]
    let (
        compress_accounts,
        compress_dispatch_fn,
        compress_processor_fn,
        compress_instruction,
        processor_module,
        init_config_accounts,
        update_config_accounts,
        init_config_instruction,
        update_config_instruction,
    ) = if !backend.is_pinocchio() {
        let compress_accounts = compress_builder.generate_accounts_struct()?;
        let compress_dispatch_fn = compress_builder.generate_dispatch_fn()?;
        let compress_processor_fn = compress_builder.generate_processor()?;
        let compress_instruction = compress_builder.generate_entrypoint()?;

        // Generate processor module - includes dispatch fn + processor fns.
        // The compress dispatch function must be inside the module so it can
        // access `use super::*` imports.
        let processor_module: syn::ItemMod =
            if let Some(ref decompress_processor_fn) = decompress_processor_fn {
                syn::parse_quote! {
                    mod __processor_functions {
                        use super::*;
                        #compress_dispatch_fn
                        #decompress_processor_fn
                        #compress_processor_fn
                    }
                }
            } else {
                syn::parse_quote! {
                    mod __processor_functions {
                        use super::*;
                        #compress_dispatch_fn
                        #compress_processor_fn
                    }
                }
            };

        let init_config_accounts: syn::ItemStruct = syn::parse_quote! {
            #[derive(Accounts)]
            pub struct InitializeCompressionConfig<'info> {
                #[account(mut)]
                pub payer: Signer<'info>,
                /// CHECK: Checked by SDK
                #[account(mut)]
                pub config: AccountInfo<'info>,
                /// CHECK: Checked by SDK
                pub program_data: AccountInfo<'info>,
                pub authority: Signer<'info>,
                pub system_program: Program<'info, System>,
            }
        };

        let update_config_accounts: syn::ItemStruct = syn::parse_quote! {
            #[derive(Accounts)]
            pub struct UpdateCompressionConfig<'info> {
                /// CHECK: Checked by SDK
                #[account(mut)]
                pub config: AccountInfo<'info>,
                pub update_authority: Signer<'info>,
            }
        };

        let init_config_instruction: syn::ItemFn = syn::parse_quote! {
            #[inline(never)]
            pub fn initialize_compression_config<'info>(
                ctx: Context<'_, '_, '_, 'info, InitializeCompressionConfig<'info>>,
                params: InitConfigParams,
            ) -> Result<()> {
                #account_crate::process_initialize_light_config(
                    &ctx.accounts.config,
                    &ctx.accounts.authority,
                    &params.rent_sponsor.to_bytes(),
                    &params.compression_authority.to_bytes(),
                    params.rent_config,
                    params.write_top_up,
                    params.address_space.iter().map(|p| p.to_bytes()).collect(),
                    0, // config_bump
                    &ctx.accounts.payer,
                    &ctx.accounts.system_program,
                    &crate::LIGHT_CPI_SIGNER.program_id,
                ).map_err(|e| anchor_lang::error::Error::from(solana_program_error::ProgramError::from(e)))?;
                Ok(())
            }
        };

        let update_config_instruction: syn::ItemFn = syn::parse_quote! {
            #[inline(never)]
            pub fn update_compression_config<'info>(
                ctx: Context<'_, '_, '_, 'info, UpdateCompressionConfig<'info>>,
                instruction_data: Vec<u8>,
            ) -> Result<()> {
                let remaining = [
                    ctx.accounts.config.to_account_info(),
                    ctx.accounts.update_authority.to_account_info(),
                ];
                #account_crate::process_update_light_config(
                    &remaining,
                    &instruction_data,
                    &crate::LIGHT_CPI_SIGNER.program_id,
                ).map_err(|e| anchor_lang::error::Error::from(solana_program_error::ProgramError::from(e)))?;
                Ok(())
            }
        };

        (
            Some(compress_accounts),
            Some(compress_dispatch_fn),
            Some(compress_processor_fn),
            Some(compress_instruction),
            Some(processor_module),
            Some(init_config_accounts),
            Some(update_config_accounts),
            Some(init_config_instruction),
            Some(update_config_instruction),
        )
    } else {
        // Pinocchio: no accounts structs, processor module, or config instructions
        (None, None, None, None, None, None, None, None, None)
    };

    // InitConfigParams struct - generated for both backends, but with different types
    let init_config_params_struct = if backend.is_pinocchio() {
        // Pinocchio: [u8; 32] instead of Pubkey
        quote! {
            #[derive(#serialize_derive, #deserialize_derive, Clone)]
            pub struct InitConfigParams {
                pub write_top_up: u32,
                pub rent_sponsor: [u8; 32],
                pub compression_authority: [u8; 32],
                pub rent_config: #account_crate::rent::RentConfig,
                pub address_space: Vec<[u8; 32]>,
            }
        }
    } else {
        // Anchor: Pubkey type
        quote! {
            /// Configuration parameters for initializing compression config.
            /// Field order matches SDK client's `InitializeCompressionConfigAnchorData`.
            #[derive(AnchorSerialize, AnchorDeserialize, Clone)]
            pub struct InitConfigParams {
                pub write_top_up: u32,
                pub rent_sponsor: Pubkey,
                pub compression_authority: Pubkey,
                pub rent_config: #account_crate::RentConfig,
                pub address_space: Vec<Pubkey>,
            }
        }
    };

    let client_functions = super::seed_codegen::generate_client_seed_functions(
        &pda_seeds,
        &token_seeds,
        &instruction_data,
        backend.is_pinocchio(),
    )?;

    // Collect all generated items into a Vec<TokenStream>
    let mut items: Vec<TokenStream> = Vec::new();

    // SeedParams struct and impl
    items.push(seed_params_struct);

    // XxxSeeds structs and LightAccountVariant constructors
    for seeds_tokens in seeds_structs_and_constructors.into_iter() {
        items.push(seeds_tokens);
    }

    // PDA variant structs (variant code uses fully qualified paths)
    if !pda_variant_code.is_empty() {
        items.push(pda_variant_code);
    }

    items.push(size_validation_checks);
    items.push(discriminator_collision_checks);
    items.push(enum_and_traits);

    // Anchor-only: accounts structs, trait impls, processor module
    if let Some(decompress_accounts) = decompress_accounts {
        items.push(quote! { #decompress_accounts });
        items.push(decompress_builder.generate_accounts_trait_impls()?);
    }
    if let Some(trait_impls) = trait_impls {
        items.push(quote! { #trait_impls });
    }
    if let Some(ref processor_module) = processor_module {
        items.push(quote! { #processor_module });
    }
    if let Some(decompress_instruction) = decompress_instruction {
        items.push(quote! { #decompress_instruction });
    }
    if let Some(ref compress_accounts) = compress_accounts {
        items.push(quote! { #compress_accounts });
        items.push(compress_builder.generate_accounts_trait_impls()?);
    }
    if let Some(ref compress_instruction) = compress_instruction {
        items.push(quote! { #compress_instruction });
    }
    if let Some(ref init_config_accounts) = init_config_accounts {
        items.push(quote! { #init_config_accounts });
    }
    if let Some(ref update_config_accounts) = update_config_accounts {
        items.push(quote! { #update_config_accounts });
    }
    items.push(init_config_params_struct);
    if let Some(ref init_config_instruction) = init_config_instruction {
        items.push(quote! { #init_config_instruction });
    }
    if let Some(ref update_config_instruction) = update_config_instruction {
        items.push(quote! { #update_config_instruction });
    }

    // PDA seed provider impls
    for pda_impl in pda_seed_provider_impls.into_iter() {
        items.push(pda_impl);
    }

    // CToken seed provider impls (one per token variant)
    if let Some(ref seeds) = token_seeds {
        if !seeds.is_empty() {
            let impl_code =
                super::seed_codegen::generate_ctoken_seed_provider_implementation(seeds)?;
            items.push(impl_code);
        }
    }

    // Error codes (Anchor only)
    if let Some(error_codes) = error_codes {
        items.push(error_codes);
    }

    // Client functions (module + pub use statement)
    items.push(client_functions);

    // Generate enum dispatch methods for #[derive(LightProgram)]
    if let Some(enum_name) = enum_name {
        if backend.is_pinocchio() {
            // Pinocchio: enum associated functions for compress/decompress/config
            if compress_builder.has_pdas() {
                items.push(
                    compress_builder
                        .generate_enum_dispatch_method_with_backend(enum_name, backend)?,
                );
                items.push(
                    compress_builder
                        .generate_enum_process_compress_with_backend(enum_name, backend)?,
                );
            }

            if !pda_ctx_seeds.is_empty() {
                items.push(
                    decompress_builder
                        .generate_enum_process_decompress_with_backend(enum_name, backend)?,
                );
            }

            // Config functions as enum associated methods (Pinocchio)
            items.push(quote! {
                impl #enum_name {
                    // SDK-standard discriminators (must match light-client)
                    pub const INITIALIZE_COMPRESSION_CONFIG: [u8; 8] = [133, 228, 12, 169, 56, 76, 222, 61];
                    pub const UPDATE_COMPRESSION_CONFIG: [u8; 8] = [135, 215, 243, 81, 163, 146, 33, 70];
                    pub const COMPRESS_ACCOUNTS_IDEMPOTENT: [u8; 8] = [70, 236, 171, 120, 164, 93, 113, 181];
                    pub const DECOMPRESS_ACCOUNTS_IDEMPOTENT: [u8; 8] = [114, 67, 61, 123, 234, 31, 1, 112];

                    pub fn process_initialize_config(
                        accounts: &[pinocchio::account_info::AccountInfo],
                        data: &[u8],
                    ) -> std::result::Result<(), pinocchio::program_error::ProgramError> {
                        let params = <InitConfigParams as borsh::BorshDeserialize>::try_from_slice(data)
                            .map_err(|_| pinocchio::program_error::ProgramError::BorshIoError)?;

                        if accounts.len() < 5 {
                            return Err(pinocchio::program_error::ProgramError::NotEnoughAccountKeys);
                        }

                        let fee_payer = &accounts[0];
                        let config = &accounts[1];
                        let _program_data = &accounts[2];
                        let authority = &accounts[3];
                        let system_program = &accounts[4];

                        #account_crate::process_initialize_light_config(
                            config,
                            authority,
                            &params.rent_sponsor,
                            &params.compression_authority,
                            params.rent_config,
                            params.write_top_up,
                            params.address_space,
                            0, // config_bump
                            fee_payer,
                            system_program,
                            &crate::LIGHT_CPI_SIGNER.program_id,
                        )
                        .map_err(|e| pinocchio::program_error::ProgramError::Custom(u32::from(e)))
                    }

                    pub fn process_update_config(
                        accounts: &[pinocchio::account_info::AccountInfo],
                        data: &[u8],
                    ) -> std::result::Result<(), pinocchio::program_error::ProgramError> {
                        if accounts.len() < 2 {
                            return Err(pinocchio::program_error::ProgramError::NotEnoughAccountKeys);
                        }

                        let config = &accounts[0];
                        let authority = &accounts[1];

                        let remaining = [*config, *authority];
                        #account_crate::process_update_light_config(
                            &remaining,
                            data,
                            &crate::LIGHT_CPI_SIGNER.program_id,
                        )
                        .map_err(|e| pinocchio::program_error::ProgramError::Custom(u32::from(e)))
                    }
                }
            });
        } else {
            // Anchor: standard enum dispatch methods
            if compress_builder.has_pdas() {
                items.push(
                    compress_builder
                        .generate_enum_dispatch_method_with_backend(enum_name, backend)?,
                );
            }

            if !pda_ctx_seeds.is_empty() {
                items.push(
                    decompress_builder
                        .generate_enum_decompress_dispatch_with_backend(enum_name, backend)?,
                );
            }
        }
    }

    Ok(items)
}

/// Thin wrapper around `generate_light_program_items` that injects items into a module.
///
/// Used by `#[light_program]` attribute macro.
#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn codegen(
    module: &mut ItemMod,
    compressible_accounts: Vec<CompressibleAccountInfo>,
    pda_seeds: Option<Vec<TokenSeedSpec>>,
    token_seeds: Option<Vec<TokenSeedSpec>>,
    instruction_data: Vec<InstructionDataSpec>,
    crate_ctx: &crate::light_pdas::parsing::CrateContext,
    has_mint_fields: bool,
    has_ata_fields: bool,
    pda_variant_code: TokenStream,
) -> Result<TokenStream> {
    let content = match module.content.as_mut() {
        Some(content) => content,
        None => return Err(macro_error!(module, "Module must have a body")),
    };

    // Insert anchor_lang::prelude::* import at the beginning of the module
    let anchor_import: syn::Item = syn::parse_quote! {
        use anchor_lang::prelude::*;
    };
    content.1.insert(0, anchor_import);

    // Generate all items using the shared function
    // #[light_program] attribute macro doesn't have an enum name - pass None
    let generated_items = generate_light_program_items(
        compressible_accounts,
        pda_seeds,
        token_seeds,
        instruction_data,
        crate_ctx,
        has_mint_fields,
        has_ata_fields,
        pda_variant_code,
        None,
    )?;

    // Inject all generated items into the module
    for item_tokens in generated_items {
        let file: syn::File = syn::parse2(item_tokens)?;
        for item in file.items {
            content.1.push(item);
        }
    }

    Ok(quote! { #module })
}

// =============================================================================
// MAIN ENTRY POINT
// =============================================================================

/// Main entry point for #[light_program] macro.
///
/// This macro reads external module files to extract seed information from
/// Accounts structs with #[light_account(init)] fields. It also automatically wraps
/// instruction handlers that use these Accounts structs with pre_init/finalize logic.
///
/// Usage:
/// ```ignore
/// #[light_program]
/// #[program]
/// pub mod my_program {
///     pub mod instruction_accounts;  // Macro reads this file!
///     pub mod state;
///
///     use instruction_accounts::*;
///     use state::*;
///
///     // No #[light_instruction] needed - auto-wrapped!
///     pub fn create_user(ctx: Context<CreateUser>, params: Params) -> Result<()> {
///         // Your business logic
///     }
/// }
/// ```
#[inline(never)]
pub fn light_program_impl(_args: TokenStream, mut module: ItemMod) -> Result<TokenStream> {
    use crate::light_pdas::{
        parsing::{parse_instruction_arg_names, CrateContext},
        seeds::{
            extract_from_accounts_struct, get_data_fields, ExtractedSeedSpec, ExtractedTokenSpec,
        },
    };

    if module.content.is_none() {
        return Err(macro_error!(&module, "Module must have a body"));
    }

    // Parse the crate following mod declarations (Anchor-style)
    let crate_ctx = CrateContext::parse_from_manifest()?;

    // Find all structs with #[derive(Accounts)] and extract rentfree field info
    let mut pda_specs: Vec<ExtractedSeedSpec> = Vec::new();
    let mut token_specs: Vec<ExtractedTokenSpec> = Vec::new();
    let mut rentfree_struct_names = std::collections::HashSet::new();
    let mut has_any_mint_fields = false;
    let mut has_any_ata_fields = false;

    for (module_path, item_struct) in crate_ctx.structs_with_derive_and_path("Accounts") {
        // Parse #[instruction(...)] attribute to get instruction arg names
        let instruction_args = parse_instruction_arg_names(&item_struct.attrs)?;

        if let Some(info) =
            extract_from_accounts_struct(item_struct, &instruction_args, module_path)?
        {
            if !info.pda_fields.is_empty()
                || !info.token_fields.is_empty()
                || info.has_light_mint_fields
                || info.has_light_ata_fields
            {
                rentfree_struct_names.insert(info.struct_name.to_string());
                pda_specs.extend(info.pda_fields);
                token_specs.extend(info.token_fields);
                if info.has_light_mint_fields {
                    has_any_mint_fields = true;
                }
                if info.has_light_ata_fields {
                    has_any_ata_fields = true;
                }
            }
        }
    }

    // Check if we found anything (PDAs, tokens, mint fields, or ATA fields)
    if pda_specs.is_empty() && token_specs.is_empty() && !has_any_mint_fields && !has_any_ata_fields
    {
        return Err(macro_error!(
            &module,
            "No #[light_account(init)], #[light_account(init, mint::...)], #[light_account(init, associated_token::...)], or #[light_account(token::...)] fields found in any Accounts struct.\n\
             Ensure your Accounts structs are in modules declared with `pub mod xxx;`"
        ));
    }

    // Auto-wrap instruction handlers that use rentfree Accounts structs
    if let Some((_, ref mut items)) = module.content {
        for item in items.iter_mut() {
            if let Item::Fn(fn_item) = item {
                // Check if this function uses a rentfree Accounts struct
                use crate::light_pdas::program::parsing::ExtractResult;
                match extract_context_and_params(fn_item) {
                    ExtractResult::Success {
                        context_type,
                        params_ident,
                        ctx_ident,
                    } => {
                        if rentfree_struct_names.contains(&context_type) {
                            // Wrap the function with pre_init/finalize logic
                            *fn_item = wrap_function_with_light(fn_item, &params_ident, &ctx_ident);
                        }
                    }
                    ExtractResult::MultipleParams {
                        context_type,
                        param_names,
                    } => {
                        // Only error if this is a rentfree struct that needs wrapping
                        if rentfree_struct_names.contains(&context_type) {
                            let fn_name = fn_item.sig.ident.to_string();
                            let params_str = param_names.join(", ");
                            return Err(macro_error!(
                                    fn_item,
                                    format!(
                                        "Function '{}' has multiple instruction arguments ({}) which is not supported by #[light_program].\n\
                                         Please consolidate these into a single params struct.\n\
                                         Example: Instead of `fn {}(ctx: Context<T>, {})`,\n\
                                         use: `fn {}(ctx: Context<T>, params: MyParams)` where MyParams contains all fields.",
                                        fn_name,
                                        params_str,
                                        fn_name,
                                        params_str,
                                        fn_name
                                    )
                                ));
                        }
                        // Non-rentfree structs with multiple params are fine - just skip wrapping
                    }
                    ExtractResult::None => {
                        // No context/params found, skip this function
                    }
                }
            }
        }
    }

    // Generate PDA variant structs using existing VariantBuilder
    let pda_variant_code: TokenStream = pda_specs
        .iter()
        .map(|spec| {
            crate::light_pdas::accounts::variant::VariantBuilder::from_extracted_spec(spec).build()
        })
        .collect();

    // Convert extracted specs to the format expected by codegen
    // Check for duplicate field names - each compressible field must have a unique name
    let mut found_pda_seeds: Vec<TokenSeedSpec> = Vec::new();
    let mut found_data_fields: Vec<InstructionDataSpec> = Vec::new();
    let mut compressible_accounts: Vec<CompressibleAccountInfo> = Vec::new();
    let mut seen_variants: std::collections::HashMap<String, &ExtractedSeedSpec> =
        std::collections::HashMap::new();

    for pda in &pda_specs {
        // Check for duplicate field names - each compressible field must be unique across the program
        let variant_str = pda.variant_name.to_string();
        if let Some(existing) = seen_variants.get(&variant_str) {
            return Err(syn::Error::new(
                pda.variant_name.span(),
                format!(
                    "Duplicate compressible field name '{}' found in multiple instruction structs.\n\
                     Each compressible field must have a unique name across the program.\n\
                     \n\
                     First: struct '{}'\n\
                     Second: struct '{}'\n\
                     \n\
                     Rename one of the fields to be unique.",
                    variant_str,
                    existing.struct_name,
                    pda.struct_name,
                ),
            ));
        }
        seen_variants.insert(variant_str, pda);

        compressible_accounts.push(CompressibleAccountInfo {
            account_type: pda.inner_type.clone(),
            is_zero_copy: pda.is_zero_copy,
        });

        let seed_elements =
            convert_classified_to_seed_elements(&pda.seeds, &pda.module_path, &crate_ctx);

        // Extract data field types from seeds
        for (field_name, conversion) in get_data_fields(&pda.seeds) {
            let field_type: syn::Type = if conversion.is_some() {
                syn::parse_quote!(u64)
            } else {
                // Use Pubkey (from anchor_lang::prelude) instead of solana_pubkey::Pubkey
                // because Anchor's IDL build feature requires IdlBuild trait implementations
                syn::parse_quote!(Pubkey)
            };

            if !found_data_fields.iter().any(|f| f.field_name == field_name) {
                found_data_fields.push(InstructionDataSpec {
                    field_name,
                    field_type,
                });
            }
        }

        found_pda_seeds.push(TokenSeedSpec {
            // Use variant_name (from field name) for enum variant naming
            variant: pda.variant_name.clone(),
            _eq: syn::parse_quote!(=),
            is_token: Some(false),
            seeds: seed_elements,
            owner_seeds: None,
            // Store inner_type for type references (deserialization, trait bounds)
            inner_type: Some(pda.inner_type.clone()),
            is_zero_copy: pda.is_zero_copy,
        });
    }

    // Convert token specs
    let mut found_token_seeds: Vec<TokenSeedSpec> = Vec::new();
    for token in &token_specs {
        let seed_elements =
            convert_classified_to_seed_elements(&token.seeds, &token.module_path, &crate_ctx);
        let owner_seeds_elements = token.owner_seeds.as_ref().map(|seeds| {
            convert_classified_to_seed_elements_vec(seeds, &token.module_path, &crate_ctx)
        });

        found_token_seeds.push(TokenSeedSpec {
            variant: token.variant_name.clone(),
            _eq: syn::parse_quote!(=),
            is_token: Some(true),
            seeds: seed_elements,
            owner_seeds: owner_seeds_elements,
            inner_type: None,    // Token specs don't have inner type
            is_zero_copy: false, // Token specs don't use zero-copy
        });
    }

    let pda_seeds = if found_pda_seeds.is_empty() {
        None
    } else {
        Some(found_pda_seeds)
    };

    let token_seeds = if found_token_seeds.is_empty() {
        None
    } else {
        Some(found_token_seeds)
    };

    // Use the shared generation logic
    codegen(
        &mut module,
        compressible_accounts,
        pda_seeds,
        token_seeds,
        found_data_fields,
        &crate_ctx,
        has_any_mint_fields,
        has_any_ata_fields,
        pda_variant_code,
    )
}

// =============================================================================
// PINOCCHIO CODEGEN
// =============================================================================

/// Pinocchio code generation - thin wrapper around `generate_light_program_items_with_backend`.
///
/// Uses the `PinocchioBackend` to generate code with:
/// - `BorshSerialize/BorshDeserialize` instead of `AnchorSerialize/AnchorDeserialize`
/// - `light_account_pinocchio::` instead of `light_account::`
/// - No Anchor accounts structs, trait impls, processor module, or error_code enum
/// - Config/compress/decompress as enum associated functions
/// - `[u8; 32]` instead of `Pubkey` in params
#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_light_program_pinocchio_items(
    compressible_accounts: Vec<CompressibleAccountInfo>,
    pda_seeds: Option<Vec<TokenSeedSpec>>,
    token_seeds: Option<Vec<TokenSeedSpec>>,
    instruction_data: Vec<InstructionDataSpec>,
    crate_ctx: &crate::light_pdas::parsing::CrateContext,
    has_mint_fields: bool,
    has_ata_fields: bool,
    pda_variant_code: TokenStream,
    enum_name: Option<&syn::Ident>,
) -> Result<Vec<TokenStream>> {
    generate_light_program_items_with_backend(
        compressible_accounts,
        pda_seeds,
        token_seeds,
        instruction_data,
        crate_ctx,
        has_mint_fields,
        has_ata_fields,
        pda_variant_code,
        enum_name,
        &PinocchioBackend,
    )
}
