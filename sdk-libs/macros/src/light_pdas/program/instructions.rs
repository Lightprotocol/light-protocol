//! Compressible instructions generation - orchestration module.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Item, ItemMod, Result};

// Re-export types from parsing for external use
pub use super::parsing::{
    extract_ctx_seed_fields, extract_data_seed_fields, InstructionDataSpec, InstructionVariant,
    SeedElement, TokenSeedSpec,
};
use super::{
    compress::{CompressBuilder, CompressibleAccountInfo},
    decompress::DecompressBuilder,
    parsing::{
        convert_classified_to_seed_elements, convert_classified_to_seed_elements_vec,
        extract_context_and_params, macro_error, wrap_function_with_light,
    },
    variant_enum::{LightVariantBuilder, PdaCtxSeedInfo},
};
use crate::{
    light_pdas::shared_utils::{ident_to_type, qualify_type_with_crate},
    utils::to_snake_case,
};

// =============================================================================
// MAIN CODEGEN
// =============================================================================

/// Orchestrates all code generation for the rentfree module.
#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn codegen(
    module: &mut ItemMod,
    compressible_accounts: Vec<CompressibleAccountInfo>,
    pda_seeds: Option<Vec<TokenSeedSpec>>,
    token_seeds: Option<Vec<TokenSeedSpec>>,
    instruction_data: Vec<InstructionDataSpec>,
    crate_ctx: &super::crate_context::CrateContext,
    has_mint_fields: bool,
    has_ata_fields: bool,
) -> Result<TokenStream> {
    let content = match module.content.as_mut() {
        Some(content) => content,
        None => return Err(macro_error!(module, "Module must have a body")),
    };

    // Insert anchor_lang::prelude::* import at the beginning of the module
    // This ensures Accounts, Signer, AccountInfo, Result, error_code etc. are in scope
    // for the generated code (structs, enums, functions).
    let anchor_import: syn::Item = syn::parse_quote! {
        use anchor_lang::prelude::*;
    };
    content.1.insert(0, anchor_import);

    if let Some(ref token_seed_specs) = token_seeds {
        for spec in token_seed_specs {
            if spec.authority.is_none() {
                return Err(macro_error!(
                    &spec.variant,
                    "Token account '{}' must specify authority = <seed_expr> for compression signing.",
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
                    let params_only_seed_fields = crate::light_pdas::seeds::get_params_only_seed_fields_from_spec(spec, &state_field_names);

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
    let enum_and_traits = if pda_ctx_seeds.is_empty() {
        // Generate minimal code for mint-only programs that matches trait signatures
        quote! {
            /// Placeholder enum for programs that only use Light mints without state accounts.
            #[derive(Clone, Debug, anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize)]
            pub enum LightAccountVariant {
                /// Placeholder variant for mint-only programs
                Empty,
            }

            impl Default for LightAccountVariant {
                fn default() -> Self {
                    Self::Empty
                }
            }

            impl ::light_sdk::hasher::DataHasher for LightAccountVariant {
                fn hash<H: ::light_sdk::hasher::Hasher>(&self) -> std::result::Result<[u8; 32], ::light_sdk::hasher::HasherError> {
                    match self {
                        Self::Empty => Err(::light_sdk::hasher::HasherError::EmptyInput),
                    }
                }
            }

            impl light_sdk::LightDiscriminator for LightAccountVariant {
                const LIGHT_DISCRIMINATOR: [u8; 8] = [0; 8];
                const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
            }

            impl light_sdk::interface::HasCompressionInfo for LightAccountVariant {
                fn compression_info(&self) -> std::result::Result<&light_sdk::interface::CompressionInfo, solana_program_error::ProgramError> {
                    Err(solana_program_error::ProgramError::InvalidAccountData)
                }

                fn compression_info_mut(&mut self) -> std::result::Result<&mut light_sdk::interface::CompressionInfo, solana_program_error::ProgramError> {
                    Err(solana_program_error::ProgramError::InvalidAccountData)
                }

                fn compression_info_mut_opt(&mut self) -> &mut Option<light_sdk::interface::CompressionInfo> {
                    panic!("compression_info_mut_opt not supported for mint-only programs")
                }

                fn set_compression_info_none(&mut self) -> std::result::Result<(), solana_program_error::ProgramError> {
                    Err(solana_program_error::ProgramError::InvalidAccountData)
                }
            }

            impl light_sdk::account::Size for LightAccountVariant {
                fn size(&self) -> std::result::Result<usize, solana_program_error::ProgramError> {
                    Err(solana_program_error::ProgramError::InvalidAccountData)
                }
            }

            // Pack trait is only available off-chain (client-side)
            #[cfg(not(target_os = "solana"))]
            impl light_sdk::Pack for LightAccountVariant {
                type Packed = Self;
                fn pack(&self, _remaining_accounts: &mut light_sdk::instruction::PackedAccounts) -> std::result::Result<Self::Packed, solana_program_error::ProgramError> {
                    Ok(Self::Empty)
                }
            }

            impl light_sdk::Unpack for LightAccountVariant {
                type Unpacked = Self;
                fn unpack(&self, _remaining_accounts: &[solana_account_info::AccountInfo]) -> std::result::Result<Self::Unpacked, solana_program_error::ProgramError> {
                    Ok(Self::Empty)
                }
            }

            /// Wrapper for compressed account data (mint-only placeholder).
            #[derive(Clone, Debug, anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize)]
            pub struct LightAccountData {
                pub meta: light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                pub data: LightAccountVariant,
            }

            impl Default for LightAccountData {
                fn default() -> Self {
                    Self {
                        meta: light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress::default(),
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
        builder.build()?
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
            #[derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize, Clone, Debug, Default)]
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
            #[derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize, Clone, Debug)]
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

    let _instruction_data_types: std::collections::HashMap<String, &syn::Type> = instruction_data
        .iter()
        .map(|spec| (spec.field_name.to_string(), &spec.field_type))
        .collect();

    // Generate pub use re-exports for per-field variant types from LightAccounts.
    // These types are generated at crate root by #[derive(LightAccounts)] and need
    // to be re-exported from the program module so tests/clients can access them.
    let variant_reexports: Vec<TokenStream> = pda_ctx_seeds
        .iter()
        .flat_map(|info| {
            let variant_name = &info.variant_name;
            let seeds_name = format_ident!("{}Seeds", variant_name);
            let packed_seeds_name = format_ident!("Packed{}Seeds", variant_name);
            let variant_struct_name = format_ident!("{}Variant", variant_name);
            let packed_variant_name = format_ident!("Packed{}Variant", variant_name);
            vec![
                quote! { pub use super::#seeds_name; },
                quote! { pub use super::#packed_seeds_name; },
                quote! { pub use super::#variant_struct_name; },
                quote! { pub use super::#packed_variant_name; },
            ]
        })
        .collect();

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
                // Only generate verifications for data fields that exist on the state struct
                let data_verifications: Vec<_> = data_fields.iter().filter_map(|field| {
                    let field_str = field.to_string();
                    // Skip fields that don't exist on the state struct (e.g., params-only seeds)
                    if !ctx_info.state_field_names.contains(&field_str) {
                        return None;
                    }
                    Some(quote! {
                        if data.#field != seeds.#field {
                            return std::result::Result::Err(LightInstructionError::SeedMismatch.into());
                        }
                    })
                }).collect();

                // Both zero_copy and Borsh accounts use AnchorDeserialize on the full
                // compressed data (which includes CompressionInfo::compressed()).
                let (deserialize_code, variant_data) = (
                    quote! {
                        use anchor_lang::AnchorDeserialize;
                        let data: #inner_type = AnchorDeserialize::deserialize(&mut &account_data[..])
                            .map_err(|_| anchor_lang::error::Error::from(anchor_lang::error::ErrorCode::AccountDidNotDeserialize))?;
                    },
                    quote! { data },
                );

                let variant_struct_name = format_ident!("{}Variant", variant_name);

                let generated = quote! {
                    impl LightAccountVariant {
                        /// Construct a #variant_name variant from account data and seeds.
                        pub fn #constructor_name(
                            account_data: &[u8],
                            seeds: #seeds_struct_name,
                        ) -> std::result::Result<Self, anchor_lang::error::Error> {
                            #deserialize_code

                            #(#data_verifications)*

                            // Create the variant struct using the seeds directly
                            let variant = #variant_struct_name {
                                seeds,
                                data: #variant_data,
                            };
                            std::result::Result::Ok(Self::#variant_name(variant))
                        }
                    }
                    impl light_sdk::interface::IntoVariant<LightAccountVariant> for #seeds_struct_name {
                        fn into_variant(self, data: &[u8]) -> std::result::Result<LightAccountVariant, anchor_lang::error::Error> {
                            LightAccountVariant::#constructor_name(data, self)
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
            return Err(macro_error!(
                module,
                "No #[light_account(init)], #[light_account(init, mint::...)], #[light_account(init, associated_token::...)], or #[light_account(token::...)] fields found.\n\
                 At least one light account field must be provided."
            ))
        }
    };

    // Create CompressBuilder to generate all compress-related code
    let compress_builder = CompressBuilder::new(compressible_accounts.clone(), instruction_variant);
    compress_builder.validate()?;

    let size_validation_checks = compress_builder.generate_size_validation()?;
    let error_codes = compress_builder.generate_error_codes()?;

    // Create DecompressBuilder to generate all decompress-related code
    let decompress_builder = DecompressBuilder::new(pda_ctx_seeds.clone(), pda_seeds.clone());
    // Note: DecompressBuilder validation is optional for now since pda_seeds may be empty for TokenOnly

    let decompress_accounts = decompress_builder.generate_accounts_struct()?;
    let pda_seed_provider_impls = decompress_builder.generate_seed_provider_impls()?;

    // Generate trait impls and decompress processor/instruction based on program type.
    // v2 interface: no DecompressContext trait needed - uses DecompressVariant on PackedLightAccountVariant.
    let (trait_impls, decompress_processor_fn, decompress_instruction) =
        if !pda_ctx_seeds.is_empty() && has_token_seeds_early {
            // Mixed program: PDAs + Tokens - generate full impl with token checking.
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

                    impl light_sdk::interface::HasTokenVariant for LightAccountData {
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
            // PDA-only program: simplified impl without token checking
            let trait_impls: syn::ItemMod = syn::parse_quote! {
                mod __trait_impls {
                    use super::*;

                    impl light_sdk::interface::HasTokenVariant for LightAccountData {
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
            // Mint-only programs: placeholder impl
            let trait_impls: syn::ItemMod = syn::parse_quote! {
                mod __trait_impls {
                    use super::*;

                    impl light_sdk::interface::HasTokenVariant for LightAccountData {
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

    let compress_accounts = compress_builder.generate_accounts_struct()?;
    let compress_dispatch_fn = compress_builder.generate_dispatch_fn()?;
    let compress_processor_fn = compress_builder.generate_processor()?;
    let compress_instruction = compress_builder.generate_entrypoint()?;

    // Generate processor module - includes dispatch fn + processor fns.
    // The compress dispatch function must be inside the module so it can
    // access `use super::*` imports.
    let processor_module: syn::ItemMod =
        if let Some(decompress_processor_fn) = decompress_processor_fn {
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
        #[allow(clippy::too_many_arguments)]
        pub fn initialize_compression_config<'info>(
            ctx: Context<'_, '_, '_, 'info, InitializeCompressionConfig<'info>>,
            write_top_up: u32,
            rent_sponsor: Pubkey,
            compression_authority: Pubkey,
            rent_config: ::light_sdk::interface::rent::RentConfig,
            address_space: Vec<Pubkey>,
        ) -> Result<()> {
            light_sdk::interface::process_initialize_light_config_checked(
                &ctx.accounts.config.to_account_info(),
                &ctx.accounts.authority.to_account_info(),
                &ctx.accounts.program_data.to_account_info(),
                &rent_sponsor,
                &compression_authority,
                rent_config,
                write_top_up,
                address_space,
                0,
                &ctx.accounts.payer.to_account_info(),
                &ctx.accounts.system_program.to_account_info(),
                &crate::ID,
            )?;
            Ok(())
        }
    };

    let update_config_instruction: syn::ItemFn = syn::parse_quote! {
        #[inline(never)]
        #[allow(clippy::too_many_arguments)]
        pub fn update_compression_config<'info>(
            ctx: Context<'_, '_, '_, 'info, UpdateCompressionConfig<'info>>,
            new_rent_sponsor: Option<Pubkey>,
            new_compression_authority: Option<Pubkey>,
            new_rent_config: Option<::light_sdk::interface::rent::RentConfig>,
            new_write_top_up: Option<u32>,
            new_address_space: Option<Vec<Pubkey>>,
            new_update_authority: Option<Pubkey>,
        ) -> Result<()> {
            light_sdk::interface::process_update_light_config(
                ctx.accounts.config.as_ref(),
                ctx.accounts.update_authority.as_ref(),
                new_update_authority.as_ref(),
                new_rent_sponsor.as_ref(),
                new_compression_authority.as_ref(),
                new_rent_config,
                new_write_top_up,
                new_address_space,
                &crate::ID,
            )?;
            Ok(())
        }
    };

    let client_functions = super::seed_codegen::generate_client_seed_functions(
        &pda_seeds,
        &token_seeds,
        &instruction_data,
    )?;

    // Insert SeedParams struct and impl
    let seed_params_file: syn::File = syn::parse2(seed_params_struct)?;
    for item in seed_params_file.items {
        content.1.push(item);
    }

    // Insert XxxSeeds structs and LightAccountVariant constructors
    for seeds_tokens in seeds_structs_and_constructors.into_iter() {
        let wrapped: syn::File = syn::parse2(seeds_tokens)?;
        for item in wrapped.items {
            content.1.push(item);
        }
    }

    // Add pub use re-exports for per-field variant types from LightAccounts.
    // These make {Field}Seeds, {Field}Variant, etc. accessible from the program module.
    for reexport in variant_reexports {
        let reexport_item: syn::Item = syn::parse2(reexport)?;
        content.1.push(reexport_item);
    }

    content.1.push(Item::Verbatim(size_validation_checks));
    content.1.push(Item::Verbatim(enum_and_traits));
    content.1.push(Item::Struct(decompress_accounts));
    content.1.push(Item::Verbatim(
        decompress_builder.generate_accounts_trait_impls()?,
    ));
    if let Some(trait_impls) = trait_impls {
        content.1.push(Item::Mod(trait_impls));
    }
    content.1.push(Item::Mod(processor_module));
    if let Some(decompress_instruction) = decompress_instruction {
        content.1.push(Item::Fn(decompress_instruction));
    }
    content.1.push(Item::Struct(compress_accounts));
    content.1.push(Item::Verbatim(
        compress_builder.generate_accounts_trait_impls()?,
    ));
    content.1.push(Item::Fn(compress_instruction));
    content.1.push(Item::Struct(init_config_accounts));
    content.1.push(Item::Struct(update_config_accounts));
    content.1.push(Item::Fn(init_config_instruction));
    content.1.push(Item::Fn(update_config_instruction));

    // Add pda seed provider impls
    for pda_impl in pda_seed_provider_impls.into_iter() {
        let wrapped: syn::File = syn::parse2(pda_impl)?;
        for item in wrapped.items {
            content.1.push(item);
        }
    }

    // Add ctoken seed provider impls (one per token variant)
    if let Some(ref seeds) = token_seeds {
        if !seeds.is_empty() {
            let impl_code =
                super::seed_codegen::generate_ctoken_seed_provider_implementation(seeds)?;
            let impl_file: syn::File = syn::parse2(impl_code)?;
            for item in impl_file.items {
                content.1.push(item);
            }
        }
    }

    // Add error codes
    let error_item: syn::ItemEnum = syn::parse2(error_codes)?;
    content.1.push(Item::Enum(error_item));

    // Add client functions (module + pub use statement)
    let client_file: syn::File = syn::parse2(client_functions)?;
    for item in client_file.items {
        content.1.push(item);
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
    use super::crate_context::CrateContext;
    use crate::light_pdas::account::seed_extraction::{
        extract_from_accounts_struct, ExtractedSeedSpec, ExtractedTokenSpec,
    };
    use crate::light_pdas::seeds::{get_data_fields, parse_instruction_arg_names};

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
            authority: None,
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
        let authority_elements = token.authority_seeds.as_ref().map(|seeds| {
            convert_classified_to_seed_elements_vec(seeds, &token.module_path, &crate_ctx)
        });

        found_token_seeds.push(TokenSeedSpec {
            variant: token.variant_name.clone(),
            _eq: syn::parse_quote!(=),
            is_token: Some(true),
            seeds: seed_elements,
            authority: authority_elements,
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
    )
}
