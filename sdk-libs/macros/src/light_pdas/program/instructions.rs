//! Compressible instructions generation - orchestration module.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Item, ItemMod, Result, Type};

// Re-export types from parsing for external use
pub use super::parsing::{
    extract_ctx_seed_fields, extract_data_seed_fields, InstructionDataSpec, InstructionVariant,
    SeedElement, TokenSeedSpec,
};
use super::{
    compress::CompressBuilder,
    decompress::DecompressBuilder,
    parsing::{
        convert_classified_to_seed_elements, convert_classified_to_seed_elements_vec,
        extract_context_and_params, macro_error, wrap_function_with_light,
    },
    variant_enum::{LightVariantBuilder, PdaCtxSeedInfo, TokenVariantBuilder},
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
fn codegen(
    module: &mut ItemMod,
    account_types: Vec<Type>,
    pda_seeds: Option<Vec<TokenSeedSpec>>,
    token_seeds: Option<Vec<TokenSeedSpec>>,
    instruction_data: Vec<InstructionDataSpec>,
    crate_ctx: &super::crate_context::CrateContext,
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
    let ctoken_enum = if let Some(ref token_seed_specs) = token_seeds {
        if !token_seed_specs.is_empty() {
            TokenVariantBuilder::new(token_seed_specs).build()?
        } else {
            crate::light_pdas::account::utils::generate_empty_ctoken_enum()
        }
    } else {
        crate::light_pdas::account::utils::generate_empty_ctoken_enum()
    };

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
                    let params_only_seed_fields = crate::light_pdas::account::seed_extraction::get_params_only_seed_fields_from_spec(spec, &state_field_names);

                    PdaCtxSeedInfo::with_state_fields(
                        spec.variant.clone(),
                        inner_type,
                        ctx_fields,
                        state_field_names,
                        params_only_seed_fields,
                    )
                })
                .collect()
        })
        .unwrap_or_default();

    let enum_and_traits = LightVariantBuilder::new(&pda_ctx_seeds).build()?;

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

    let instruction_data_types: std::collections::HashMap<String, &syn::Type> = instruction_data
        .iter()
        .map(|spec| (spec.field_name.to_string(), &spec.field_type))
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
                let seeds_struct_name = format_ident!("{}Seeds", variant_name);
                let constructor_name = format_ident!("{}", to_snake_case(&variant_name.to_string()));
                let ctx_fields = &ctx_info.ctx_seed_fields;
                let params_only_fields = &ctx_info.params_only_seed_fields;
                let ctx_field_decls: Vec<_> = ctx_fields.iter().map(|field| {
                    quote! { pub #field: solana_pubkey::Pubkey }
                }).collect();
                let data_fields = extract_data_seed_fields(&spec.seeds);
                let data_field_decls: Vec<_> = data_fields.iter().filter_map(|field| {
                    let field_str = field.to_string();
                    instruction_data_types.get(&field_str).map(|ty| {
                        quote! { pub #field: #ty }
                    })
                }).collect();
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

                // Extract params-only field names from ctx_info for variant construction
                let params_only_field_names: Vec<_> = params_only_fields.iter().map(|(f, _, _)| f).collect();

                quote! {
                    #[derive(Clone, Debug)]
                    pub struct #seeds_struct_name {
                        #(#ctx_field_decls,)*
                        #(#data_field_decls,)*
                    }
                    impl LightAccountVariant {
                        pub fn #constructor_name(
                            account_data: &[u8],
                            seeds: #seeds_struct_name,
                        ) -> std::result::Result<Self, anchor_lang::error::Error> {
                            use anchor_lang::AnchorDeserialize;
                            // Deserialize using inner_type
                            let data = #inner_type::deserialize(&mut &account_data[..])?;

                            #(#data_verifications)*

                            // Use variant_name for the enum variant
                            // Include ctx fields and params-only fields from seeds
                            std::result::Result::Ok(Self::#variant_name {
                                data,
                                #(#ctx_fields: seeds.#ctx_fields,)*
                                #(#params_only_field_names: seeds.#params_only_field_names,)*
                            })
                        }
                    }
                    impl light_sdk::interface::IntoVariant<LightAccountVariant> for #seeds_struct_name {
                        fn into_variant(self, data: &[u8]) -> std::result::Result<LightAccountVariant, anchor_lang::error::Error> {
                            LightAccountVariant::#constructor_name(data, self)
                        }
                    }
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    let has_pda_seeds = pda_seeds.as_ref().map(|p| !p.is_empty()).unwrap_or(false);
    let has_token_seeds = token_seeds.as_ref().map(|t| !t.is_empty()).unwrap_or(false);

    let instruction_variant = match (has_pda_seeds, has_token_seeds) {
        (true, true) => InstructionVariant::Mixed,
        (true, false) => InstructionVariant::PdaOnly,
        (false, true) => InstructionVariant::TokenOnly,
        (false, false) => {
            return Err(macro_error!(
                module,
                "At least one PDA or token seed specification must be provided"
            ))
        }
    };

    // Create CompressBuilder to generate all compress-related code
    let compress_builder = CompressBuilder::new(account_types.clone(), instruction_variant);
    compress_builder.validate()?;

    let size_validation_checks = compress_builder.generate_size_validation()?;
    let error_codes = compress_builder.generate_error_codes()?;

    let token_variant_name = format_ident!("TokenAccountVariant");

    // Create DecompressBuilder to generate all decompress-related code
    let decompress_builder = DecompressBuilder::new(
        pda_ctx_seeds.clone(),
        token_variant_name,
        account_types.clone(),
        pda_seeds.clone(),
    );
    // Note: DecompressBuilder validation is optional for now since pda_seeds may be empty for TokenOnly

    let decompress_accounts = decompress_builder.generate_accounts_struct()?;
    let pda_seed_provider_impls = decompress_builder.generate_seed_provider_impls()?;

    let trait_impls: syn::ItemMod = syn::parse_quote! {
        mod __trait_impls {
            use super::*;

            impl light_sdk::interface::HasTokenVariant for LightAccountData {
                fn is_packed_token(&self) -> bool {
                    matches!(self.data, LightAccountVariant::PackedCTokenData(_))
                }
            }
        }
    };

    let decompress_context_impl = decompress_builder.generate_context_impl()?;
    let decompress_processor_fn = decompress_builder.generate_processor()?;
    let decompress_instruction = decompress_builder.generate_entrypoint()?;

    let compress_accounts = compress_builder.generate_accounts_struct()?;
    let compress_context_impl = compress_builder.generate_context_impl()?;
    let compress_processor_fn = compress_builder.generate_processor()?;
    let compress_instruction = compress_builder.generate_entrypoint()?;

    let module_tokens = quote! {
        mod __processor_functions {
            use super::*;
            #decompress_processor_fn
            #compress_processor_fn
        }
    };
    let processor_module: syn::ItemMod = syn::parse2(module_tokens)?;

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
            rent_config: light_compressible::rent::RentConfig,
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
            new_rent_config: Option<light_compressible::rent::RentConfig>,
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

    content.1.push(Item::Verbatim(size_validation_checks));
    content.1.push(Item::Verbatim(enum_and_traits));
    content.1.push(Item::Verbatim(ctoken_enum));
    content.1.push(Item::Struct(decompress_accounts));
    content.1.push(Item::Mod(trait_impls));
    content.1.push(Item::Mod(decompress_context_impl));
    content.1.push(Item::Mod(processor_module));
    content.1.push(Item::Fn(decompress_instruction));
    content.1.push(Item::Struct(compress_accounts));
    content.1.push(Item::Mod(compress_context_impl));
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

    // Add ctoken seed provider impl
    if let Some(ref seeds) = token_seeds {
        if !seeds.is_empty() {
            let impl_code =
                super::seed_codegen::generate_ctoken_seed_provider_implementation(seeds)?;
            let ctoken_impl: syn::ItemImpl = syn::parse2(impl_code)?;
            content.1.push(Item::Impl(ctoken_impl));
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
        extract_from_accounts_struct, get_data_fields, ExtractedSeedSpec, ExtractedTokenSpec,
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

    for item_struct in crate_ctx.structs_with_derive("Accounts") {
        if let Some(info) = extract_from_accounts_struct(item_struct)? {
            if !info.pda_fields.is_empty()
                || !info.token_fields.is_empty()
                || info.has_light_mint_fields
            {
                rentfree_struct_names.insert(info.struct_name.to_string());
                pda_specs.extend(info.pda_fields);
                token_specs.extend(info.token_fields);
            }
        }
    }

    // Check if we found anything
    if pda_specs.is_empty() && token_specs.is_empty() {
        return Err(macro_error!(
            &module,
            "No #[light_account(init)] or #[light_account(token)] fields found in any Accounts struct.\n\
             Ensure your Accounts structs are in modules declared with `pub mod xxx;`"
        ));
    }

    // Auto-wrap instruction handlers that use rentfree Accounts structs
    if let Some((_, ref mut items)) = module.content {
        for item in items.iter_mut() {
            if let Item::Fn(fn_item) = item {
                // Check if this function uses a rentfree Accounts struct
                if let Some((context_type, params_ident)) = extract_context_and_params(fn_item) {
                    if rentfree_struct_names.contains(&context_type) {
                        // Wrap the function with pre_init/finalize logic
                        *fn_item = wrap_function_with_light(fn_item, &params_ident);
                    }
                }
            }
        }
    }

    // Convert extracted specs to the format expected by codegen
    // Deduplicate based on variant_name (field name) - field names must be globally unique
    let mut found_pda_seeds: Vec<TokenSeedSpec> = Vec::new();
    let mut found_data_fields: Vec<InstructionDataSpec> = Vec::new();
    let mut account_types: Vec<Type> = Vec::new();
    let mut seen_variants: std::collections::HashSet<String> = std::collections::HashSet::new();

    for pda in &pda_specs {
        // Deduplicate based on variant_name (derived from field name)
        // If same field name is used in multiple instruction structs, only add once
        let variant_str = pda.variant_name.to_string();
        if !seen_variants.insert(variant_str) {
            continue; // Skip duplicate field names
        }

        account_types.push(pda.inner_type.clone());

        let seed_elements = convert_classified_to_seed_elements(&pda.seeds);

        // Extract data field types from seeds
        for (field_name, conversion) in get_data_fields(&pda.seeds) {
            let field_type: syn::Type = if conversion.is_some() {
                syn::parse_quote!(u64)
            } else {
                syn::parse_quote!(solana_pubkey::Pubkey)
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
        });
    }

    // Convert token specs
    let mut found_token_seeds: Vec<TokenSeedSpec> = Vec::new();
    for token in &token_specs {
        let seed_elements = convert_classified_to_seed_elements(&token.seeds);
        let authority_elements = token
            .authority_seeds
            .as_ref()
            .map(|seeds| convert_classified_to_seed_elements_vec(seeds));

        found_token_seeds.push(TokenSeedSpec {
            variant: token.variant_name.clone(),
            _eq: syn::parse_quote!(=),
            is_token: Some(true),
            seeds: seed_elements,
            authority: authority_elements,
            inner_type: None, // Token specs don't have inner type
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
        account_types,
        pda_seeds,
        token_seeds,
        found_data_fields,
        &crate_ctx,
    )
}
