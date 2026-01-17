//! Seed provider generation for PDA and Light Token accounts.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Result};

use super::instructions::{InstructionDataSpec, SeedElement, TokenSeedSpec};
use crate::rentfree::traits::utils::is_pubkey_type;

/// Helper to add a Pubkey parameter and its .as_ref() expression.
/// This is the default fallback for ctx.accounts.field and similar patterns.
#[inline]
fn push_pubkey_param(
    field_name: &syn::Ident,
    parameters: &mut Vec<TokenStream>,
    expressions: &mut Vec<TokenStream>,
) {
    parameters.push(quote! { #field_name: &solana_pubkey::Pubkey });
    expressions.push(quote! { #field_name.as_ref() });
}

/// Extract ctx.* field names from seed elements (both token seeds and authority seeds)
fn extract_ctx_fields_from_token_spec(spec: &TokenSeedSpec) -> Vec<Ident> {
    let mut ctx_fields = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // Helper to extract ctx.* from a SeedElement
    fn extract_from_seed(
        seed: &SeedElement,
        ctx_fields: &mut Vec<Ident>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        if let SeedElement::Expression(expr) = seed {
            extract_ctx_from_expr(expr, ctx_fields, seen);
        }
    }

    fn extract_ctx_from_expr(
        expr: &syn::Expr,
        ctx_fields: &mut Vec<Ident>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        if let syn::Expr::Field(field_expr) = expr {
            if let syn::Member::Named(field_name) = &field_expr.member {
                // Check for ctx.accounts.field pattern
                if let syn::Expr::Field(nested_field) = &*field_expr.base {
                    if let syn::Member::Named(base_name) = &nested_field.member {
                        if base_name == "accounts" {
                            if let syn::Expr::Path(path) = &*nested_field.base {
                                if let Some(segment) = path.path.segments.first() {
                                    if segment.ident == "ctx" {
                                        let field_name_str = field_name.to_string();
                                        // Skip standard fields
                                        if !matches!(
                                            field_name_str.as_str(),
                                            "fee_payer"
                                                | "rent_sponsor"
                                                | "config"
                                                | "compression_authority"
                                        ) && seen.insert(field_name_str)
                                        {
                                            ctx_fields.push(field_name.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // Check for ctx.field pattern (shorthand)
                else if let syn::Expr::Path(path) = &*field_expr.base {
                    if let Some(segment) = path.path.segments.first() {
                        if segment.ident == "ctx" {
                            let field_name_str = field_name.to_string();
                            if !matches!(
                                field_name_str.as_str(),
                                "fee_payer" | "rent_sponsor" | "config" | "compression_authority"
                            ) && seen.insert(field_name_str)
                            {
                                ctx_fields.push(field_name.clone());
                            }
                        }
                    }
                }
            }
        }
        // Recursively check method calls like max_key(&ctx.field.key(), ...)
        else if let syn::Expr::Call(call_expr) = expr {
            for arg in &call_expr.args {
                extract_ctx_from_expr(arg, ctx_fields, seen);
            }
        } else if let syn::Expr::Reference(ref_expr) = expr {
            extract_ctx_from_expr(&ref_expr.expr, ctx_fields, seen);
        } else if let syn::Expr::MethodCall(method_call) = expr {
            extract_ctx_from_expr(&method_call.receiver, ctx_fields, seen);
        }
    }

    // Extract from seeds
    for seed in &spec.seeds {
        extract_from_seed(seed, &mut ctx_fields, &mut seen);
    }

    // Extract from authority seeds too
    if let Some(auth_seeds) = &spec.authority {
        for seed in auth_seeds {
            extract_from_seed(seed, &mut ctx_fields, &mut seen);
        }
    }

    ctx_fields
}

pub fn generate_ctoken_account_variant_enum(token_seeds: &[TokenSeedSpec]) -> Result<TokenStream> {
    // Phase 8: Generate struct variants with ctx.* seed fields

    // Unpacked variants (with Pubkeys)
    let unpacked_variants = token_seeds.iter().map(|spec| {
        let variant_name = &spec.variant;
        let ctx_fields = extract_ctx_fields_from_token_spec(spec);

        let fields = ctx_fields.iter().map(|field| {
            quote! { #field: Pubkey }
        });

        if ctx_fields.is_empty() {
            quote! { #variant_name, }
        } else {
            quote! { #variant_name { #(#fields,)* }, }
        }
    });

    // Packed variants (with u8 indices)
    let packed_variants = token_seeds.iter().map(|spec| {
        let variant_name = &spec.variant;
        let ctx_fields = extract_ctx_fields_from_token_spec(spec);

        let fields = ctx_fields.iter().map(|field| {
            let idx_field = format_ident!("{}_idx", field);
            quote! { #idx_field: u8 }
        });

        if ctx_fields.is_empty() {
            quote! { #variant_name, }
        } else {
            quote! { #variant_name { #(#fields,)* }, }
        }
    });

    // Pack impl match arms
    let pack_arms = token_seeds.iter().map(|spec| {
        let variant_name = &spec.variant;
        let ctx_fields = extract_ctx_fields_from_token_spec(spec);

        if ctx_fields.is_empty() {
            quote! {
                TokenAccountVariant::#variant_name => PackedTokenAccountVariant::#variant_name,
            }
        } else {
            let field_bindings: Vec<_> = ctx_fields.iter().collect();
            let idx_fields: Vec<_> = ctx_fields
                .iter()
                .map(|f| format_ident!("{}_idx", f))
                .collect();
            let pack_stmts: Vec<_> = ctx_fields
                .iter()
                .zip(idx_fields.iter())
                .map(|(field, idx)| {
                    quote! { let #idx = remaining_accounts.insert_or_get(*#field); }
                })
                .collect();

            quote! {
                TokenAccountVariant::#variant_name { #(#field_bindings,)* } => {
                    #(#pack_stmts)*
                    PackedTokenAccountVariant::#variant_name { #(#idx_fields,)* }
                }
            }
        }
    });

    // Unpack impl match arms
    let unpack_arms = token_seeds.iter().map(|spec| {
        let variant_name = &spec.variant;
        let ctx_fields = extract_ctx_fields_from_token_spec(spec);

        if ctx_fields.is_empty() {
            quote! {
                PackedTokenAccountVariant::#variant_name => Ok(TokenAccountVariant::#variant_name),
            }
        } else {
            let idx_fields: Vec<_> = ctx_fields
                .iter()
                .map(|f| format_ident!("{}_idx", f))
                .collect();
            let unpack_stmts: Vec<_> = ctx_fields
                .iter()
                .zip(idx_fields.iter())
                .map(|(field, idx)| {
                    // Dereference idx since match pattern gives us &u8
                    quote! {
                        let #field = *remaining_accounts
                            .get(*#idx as usize)
                            .ok_or(solana_program_error::ProgramError::InvalidAccountData)?
                            .key;
                    }
                })
                .collect();
            let field_names: Vec<_> = ctx_fields.iter().collect();

            quote! {
                PackedTokenAccountVariant::#variant_name { #(#idx_fields,)* } => {
                    #(#unpack_stmts)*
                    Ok(TokenAccountVariant::#variant_name { #(#field_names,)* })
                }
            }
        }
    });

    Ok(quote! {
        #[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy)]
        pub enum TokenAccountVariant {
            #(#unpacked_variants)*
        }

        #[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy)]
        pub enum PackedTokenAccountVariant {
            #(#packed_variants)*
        }

        impl light_token_sdk::pack::Pack for TokenAccountVariant {
            type Packed = PackedTokenAccountVariant;

            fn pack(&self, remaining_accounts: &mut light_sdk::instruction::PackedAccounts) -> Self::Packed {
                match self {
                    #(#pack_arms)*
                }
            }
        }

        impl light_token_sdk::pack::Unpack for PackedTokenAccountVariant {
            type Unpacked = TokenAccountVariant;

            fn unpack(
                &self,
                remaining_accounts: &[solana_account_info::AccountInfo],
            ) -> std::result::Result<Self::Unpacked, solana_program_error::ProgramError> {
                match self {
                    #(#unpack_arms)*
                }
            }
        }

        impl light_sdk::compressible::IntoCTokenVariant<RentFreeAccountVariant, light_token_sdk::compat::TokenData> for TokenAccountVariant {
            fn into_ctoken_variant(self, token_data: light_token_sdk::compat::TokenData) -> RentFreeAccountVariant {
                RentFreeAccountVariant::CTokenData(light_token_sdk::compat::CTokenData {
                    variant: self,
                    token_data,
                })
            }
        }
    })
}

/// Convert a SeedElement to a TokenStream representing the seed reference expression.
/// Used by generate_ctoken_seed_provider_implementation for both token and authority seeds.
fn seed_element_to_ref_expr(seed: &SeedElement) -> TokenStream {
    match seed {
        SeedElement::Literal(lit) => {
            let value = lit.value();
            quote! { #value.as_bytes() }
        }
        SeedElement::Expression(expr) => {
            // Handle byte string literals
            if let syn::Expr::Lit(lit_expr) = &**expr {
                if let syn::Lit::ByteStr(byte_str) = &lit_expr.lit {
                    let bytes = byte_str.value();
                    return quote! { &[#(#bytes),*] };
                }
            }

            // Handle uppercase constants
            if let syn::Expr::Path(path_expr) = &**expr {
                if let Some(ident) = path_expr.path.get_ident() {
                    let ident_str = ident.to_string();
                    if ident_str
                        .chars()
                        .all(|c| c.is_uppercase() || c == '_' || c.is_ascii_digit())
                    {
                        if ident_str == "LIGHT_CPI_SIGNER" {
                            return quote! { crate::#ident.cpi_signer.as_ref() };
                        } else {
                            return quote! { { let __seed: &[u8] = crate::#ident.as_ref(); __seed } };
                        }
                    }
                }
            }

            // Handle ctx.accounts.field or ctx.field - use the destructured field directly
            if let Some(field_name) = extract_ctx_field_name(expr) {
                return quote! { #field_name.as_ref() };
            }

            // Fallback
            quote! { (#expr).as_ref() }
        }
    }
}

/// Phase 8: Generate TokenSeedProvider impl that uses self.field instead of ctx.accounts.field
pub fn generate_ctoken_seed_provider_implementation(
    token_seeds: &[TokenSeedSpec],
) -> Result<TokenStream> {
    let mut get_seeds_match_arms = Vec::new();
    let mut get_authority_seeds_match_arms = Vec::new();

    for spec in token_seeds {
        let variant_name = &spec.variant;
        let ctx_fields = extract_ctx_fields_from_token_spec(spec);

        // Build match pattern with destructuring if there are ctx fields
        let pattern = if ctx_fields.is_empty() {
            quote! { TokenAccountVariant::#variant_name }
        } else {
            let field_names: Vec<_> = ctx_fields.iter().collect();
            quote! { TokenAccountVariant::#variant_name { #(#field_names,)* } }
        };

        // Build seed refs for get_seeds - use self.field directly for ctx.* seeds
        let token_seed_refs: Vec<TokenStream> =
            spec.seeds.iter().map(seed_element_to_ref_expr).collect();

        let get_seeds_arm = quote! {
            #pattern => {
                let seeds: &[&[u8]] = &[#(#token_seed_refs),*];
                let (token_account_pda, bump) = solana_pubkey::Pubkey::find_program_address(seeds, program_id);
                let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
                seeds_vec.extend(seeds.iter().map(|s| s.to_vec()));
                seeds_vec.push(vec![bump]);
                Ok((seeds_vec, token_account_pda))
            }
        };
        get_seeds_match_arms.push(get_seeds_arm);

        // Build authority seeds
        if let Some(authority_seeds) = &spec.authority {
            let auth_seed_refs: Vec<TokenStream> =
                authority_seeds.iter().map(seed_element_to_ref_expr).collect();

            let authority_arm = quote! {
                #pattern => {
                    let seeds: &[&[u8]] = &[#(#auth_seed_refs),*];
                    let (authority_pda, bump) = solana_pubkey::Pubkey::find_program_address(seeds, program_id);
                    let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
                    seeds_vec.extend(seeds.iter().map(|s| s.to_vec()));
                    seeds_vec.push(vec![bump]);
                    Ok((seeds_vec, authority_pda))
                }
            };
            get_authority_seeds_match_arms.push(authority_arm);
        } else {
            let authority_arm = quote! {
                #pattern => {
                    Err(solana_program_error::ProgramError::Custom(
                        RentFreeInstructionError::MissingSeedAccount.into()
                    ))
                }
            };
            get_authority_seeds_match_arms.push(authority_arm);
        }
    }

    // Phase 8: New trait signature - no ctx/accounts parameter needed
    Ok(quote! {
        impl light_sdk::compressible::TokenSeedProvider for TokenAccountVariant {
            fn get_seeds(
                &self,
                program_id: &solana_pubkey::Pubkey,
            ) -> std::result::Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey), solana_program_error::ProgramError> {
                match self {
                    #(#get_seeds_match_arms)*
                }
            }

            fn get_authority_seeds(
                &self,
                program_id: &solana_pubkey::Pubkey,
            ) -> std::result::Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey), solana_program_error::ProgramError> {
                match self {
                    #(#get_authority_seeds_match_arms)*
                }
            }
        }
    })
}

/// Extract the field name from a ctx.field or ctx.accounts.field expression
fn extract_ctx_field_name(expr: &syn::Expr) -> Option<Ident> {
    if let syn::Expr::Field(field_expr) = expr {
        if let syn::Member::Named(field_name) = &field_expr.member {
            // Check for ctx.accounts.field pattern
            if let syn::Expr::Field(nested_field) = &*field_expr.base {
                if let syn::Member::Named(base_name) = &nested_field.member {
                    if base_name == "accounts" {
                        if let syn::Expr::Path(path) = &*nested_field.base {
                            if let Some(segment) = path.path.segments.first() {
                                if segment.ident == "ctx" {
                                    return Some(field_name.clone());
                                }
                            }
                        }
                    }
                }
            }
            // Check for ctx.field pattern (shorthand)
            else if let syn::Expr::Path(path) = &*field_expr.base {
                if let Some(segment) = path.path.segments.first() {
                    if segment.ident == "ctx" {
                        return Some(field_name.clone());
                    }
                }
            }
        }
    }
    None
}

/// Generate the body of a seed function that computes a PDA address.
/// `program_id_expr` should be either `&crate::ID` or a variable like `_program_id`.
fn generate_seed_fn_body(
    seed_count: usize,
    seed_expressions: &[TokenStream],
    program_id_expr: TokenStream,
) -> TokenStream {
    quote! {
        let mut seed_values = Vec::with_capacity(#seed_count + 1);
        #(
            seed_values.push((#seed_expressions).to_vec());
        )*
        let seed_slices: Vec<&[u8]> = seed_values.iter().map(|v| v.as_slice()).collect();
        let (pda, bump) = solana_pubkey::Pubkey::find_program_address(&seed_slices, #program_id_expr);
        seed_values.push(vec![bump]);
        (seed_values, pda)
    }
}

#[inline(never)]
pub fn generate_client_seed_functions(
    _account_types: &[Ident],
    pda_seeds: &Option<Vec<TokenSeedSpec>>,
    token_seeds: &Option<Vec<TokenSeedSpec>>,
    instruction_data: &[InstructionDataSpec],
) -> Result<TokenStream> {
    let mut functions = Vec::new();

    if let Some(pda_seed_specs) = pda_seeds {
        for spec in pda_seed_specs {
            let variant_name = &spec.variant;
            let snake_case = camel_to_snake_case(&variant_name.to_string());
            let function_name = format_ident!("get_{}_seeds", snake_case);

            let (parameters, seed_expressions) =
                analyze_seed_spec_for_client(spec, instruction_data)?;

            let seed_count = seed_expressions.len();
            let fn_body = generate_seed_fn_body(seed_count, &seed_expressions, quote! { &crate::ID });
            let function = quote! {
                pub fn #function_name(#(#parameters),*) -> (Vec<Vec<u8>>, solana_pubkey::Pubkey) {
                    #fn_body
                }
            };
            functions.push(function);
        }
    }

    if let Some(token_seed_specs) = token_seeds {
        for spec in token_seed_specs {
            let variant_name = &spec.variant;

            let function_name =
                format_ident!("get_{}_seeds", variant_name.to_string().to_lowercase());

            let (parameters, seed_expressions) =
                analyze_seed_spec_for_client(spec, instruction_data)?;

            let seed_count = seed_expressions.len();
            let fn_body = generate_seed_fn_body(seed_count, &seed_expressions, quote! { &crate::ID });
            let function = quote! {
                pub fn #function_name(#(#parameters),*) -> (Vec<Vec<u8>>, solana_pubkey::Pubkey) {
                    #fn_body
                }
            };
            functions.push(function);

            if let Some(authority_seeds) = &spec.authority {
                let authority_function_name = format_ident!(
                    "get_{}_authority_seeds",
                    variant_name.to_string().to_lowercase()
                );

                let mut authority_spec = TokenSeedSpec {
                    variant: spec.variant.clone(),
                    _eq: spec._eq,
                    is_token: spec.is_token,
                    seeds: syn::punctuated::Punctuated::new(),
                    authority: None,
                };

                for auth_seed in authority_seeds {
                    authority_spec.seeds.push(auth_seed.clone());
                }

                let (auth_parameters, auth_seed_expressions) =
                    analyze_seed_spec_for_client(&authority_spec, instruction_data)?;

                let auth_seed_count = auth_seed_expressions.len();
                let (fn_params, fn_body) = if auth_parameters.is_empty() {
                    (
                        quote! { _program_id: &solana_pubkey::Pubkey },
                        generate_seed_fn_body(auth_seed_count, &auth_seed_expressions, quote! { _program_id }),
                    )
                } else {
                    (
                        quote! { #(#auth_parameters),* },
                        generate_seed_fn_body(auth_seed_count, &auth_seed_expressions, quote! { &crate::ID }),
                    )
                };
                let authority_function = quote! {
                    pub fn #authority_function_name(#fn_params) -> (Vec<Vec<u8>>, solana_pubkey::Pubkey) {
                        #fn_body
                    }
                };
                functions.push(authority_function);
            }
        }
    }

    Ok(quote! {
        mod __client_seed_functions {
            use super::*;
            #(#functions)*
        }

        pub use __client_seed_functions::*;
    })
}

#[inline(never)]
fn analyze_seed_spec_for_client(
    spec: &TokenSeedSpec,
    instruction_data: &[InstructionDataSpec],
) -> Result<(Vec<TokenStream>, Vec<TokenStream>)> {
    let mut parameters = Vec::new();
    let mut expressions = Vec::new();

    for seed in &spec.seeds {
        match seed {
            SeedElement::Literal(lit) => {
                let value = lit.value();
                expressions.push(quote! { #value.as_bytes() });
            }
            SeedElement::Expression(expr) => {
                match &**expr {
                    syn::Expr::Field(field_expr) => {
                        if let syn::Member::Named(field_name) = &field_expr.member {
                            // Check for data.field pattern which uses instruction_data types
                            if let syn::Expr::Path(path) = &*field_expr.base {
                                if let Some(segment) = path.path.segments.first() {
                                    if segment.ident == "data" {
                                        if let Some(data_spec) = instruction_data
                                            .iter()
                                            .find(|d| d.field_name == *field_name)
                                        {
                                            let param_type = &data_spec.field_type;
                                            let param_with_ref = if is_pubkey_type(param_type) {
                                                quote! { #field_name: &#param_type }
                                            } else {
                                                quote! { #field_name: #param_type }
                                            };
                                            parameters.push(param_with_ref);
                                            expressions.push(quote! { #field_name.as_ref() });
                                            continue;
                                        } else {
                                            return Err(syn::Error::new_spanned(
                                                field_name,
                                                format!("data.{} used in seeds but no type specified", field_name),
                                            ));
                                        }
                                    }
                                }
                            }
                            // Default: ctx.accounts.field, ctx.field, or other field patterns
                            push_pubkey_param(field_name, &mut parameters, &mut expressions);
                        }
                    }
                    syn::Expr::MethodCall(method_call) => {
                        if let syn::Expr::Field(field_expr) = &*method_call.receiver {
                            if let syn::Member::Named(field_name) = &field_expr.member {
                                if let syn::Expr::Path(path) = &*field_expr.base {
                                    if let Some(segment) = path.path.segments.first() {
                                        if segment.ident == "data" {
                                            if let Some(data_spec) = instruction_data
                                                .iter()
                                                .find(|d| d.field_name == *field_name)
                                            {
                                                let param_type = &data_spec.field_type;
                                                let param_with_ref = if is_pubkey_type(param_type) {
                                                    quote! { #field_name: &#param_type }
                                                } else {
                                                    quote! { #field_name: #param_type }
                                                };
                                                parameters.push(param_with_ref);

                                                let method_name = &method_call.method;
                                                expressions.push(
                                                    quote! { #field_name.#method_name().as_ref() },
                                                );
                                            } else {
                                                return Err(syn::Error::new_spanned(
                                                    field_name,
                                                    format!("data.{} used in seeds but no type specified", field_name),
                                                ));
                                            }
                                        } else if segment.ident == "ctx" {
                                            // ctx.field.method() -> add field as Pubkey parameter
                                            parameters.push(
                                                quote! { #field_name: &solana_pubkey::Pubkey },
                                            );
                                            let method_name = &method_call.method;
                                            expressions.push(
                                                quote! { #field_name.#method_name().as_ref() },
                                            );
                                        }
                                    }
                                }
                            }
                        } else if let syn::Expr::Path(path_expr) = &*method_call.receiver {
                            if let Some(ident) = path_expr.path.get_ident() {
                                parameters.push(quote! { #ident: &solana_pubkey::Pubkey });
                                expressions.push(quote! { #ident.as_ref() });
                            }
                        }
                    }
                    syn::Expr::Lit(lit_expr) => {
                        // Handle byte string literals: b"seed" -> use directly
                        if let syn::Lit::ByteStr(byte_str) = &lit_expr.lit {
                            let bytes = byte_str.value();
                            expressions.push(quote! { &[#(#bytes),*] });
                        }
                    }
                    syn::Expr::Path(path_expr) => {
                        if let Some(ident) = path_expr.path.get_ident() {
                            let ident_str = ident.to_string();
                            if ident_str
                                .chars()
                                .all(|c| c.is_uppercase() || c == '_' || c.is_ascii_digit())
                            {
                                if ident_str == "LIGHT_CPI_SIGNER" {
                                    expressions.push(quote! { crate::#ident.cpi_signer.as_ref() });
                                } else {
                                    // Use crate:: prefix and explicit type annotation
                                    expressions.push(quote! { { let __seed: &[u8] = crate::#ident.as_ref(); __seed } });
                                }
                            } else {
                                parameters.push(quote! { #ident: &solana_pubkey::Pubkey });
                                expressions.push(quote! { #ident.as_ref() });
                            }
                        } else {
                            expressions.push(quote! { (#expr).as_ref() });
                        }
                    }
                    syn::Expr::Call(call_expr) => {
                        // Recursively map data.* to parameter names in function call arguments
                        fn map_client_call_arg(
                            arg: &syn::Expr,
                            instruction_data: &[InstructionDataSpec],
                            parameters: &mut Vec<TokenStream>,
                        ) -> TokenStream {
                            match arg {
                                syn::Expr::Reference(ref_expr) => {
                                    let inner = map_client_call_arg(
                                        &ref_expr.expr,
                                        instruction_data,
                                        parameters,
                                    );
                                    quote! { &#inner }
                                }
                                syn::Expr::Field(field_expr) => {
                                    if let syn::Member::Named(field_name) = &field_expr.member {
                                        if let syn::Expr::Path(path) = &*field_expr.base {
                                            if let Some(segment) = path.path.segments.first() {
                                                if segment.ident == "data" {
                                                    // Add parameter if needed
                                                    if let Some(data_spec) = instruction_data
                                                        .iter()
                                                        .find(|d| d.field_name == *field_name)
                                                    {
                                                        let param_type = &data_spec.field_type;
                                                        let param_with_ref =
                                                            if is_pubkey_type(param_type) {
                                                                quote! { #field_name: &#param_type }
                                                            } else {
                                                                quote! { #field_name: #param_type }
                                                            };
                                                        if !parameters.iter().any(|p| {
                                                            p.to_string()
                                                                .contains(&field_name.to_string())
                                                        }) {
                                                            parameters.push(param_with_ref);
                                                        }
                                                    }
                                                    return quote! { #field_name };
                                                } else if segment.ident == "ctx" {
                                                    // ctx.field -> add as Pubkey parameter
                                                    if !parameters.iter().any(|p| {
                                                        p.to_string()
                                                            .contains(&field_name.to_string())
                                                    }) {
                                                        parameters.push(quote! { #field_name: &solana_pubkey::Pubkey });
                                                    }
                                                    return quote! { #field_name };
                                                }
                                            }
                                        }
                                    }
                                    quote! { #field_expr }
                                }
                                syn::Expr::MethodCall(method_call) => {
                                    let receiver = map_client_call_arg(
                                        &method_call.receiver,
                                        instruction_data,
                                        parameters,
                                    );
                                    let method = &method_call.method;
                                    let args: Vec<_> = method_call
                                        .args
                                        .iter()
                                        .map(|a| {
                                            map_client_call_arg(a, instruction_data, parameters)
                                        })
                                        .collect();
                                    quote! { (#receiver).#method(#(#args),*) }
                                }
                                syn::Expr::Call(nested_call) => {
                                    let func = &nested_call.func;
                                    let args: Vec<_> = nested_call
                                        .args
                                        .iter()
                                        .map(|a| {
                                            map_client_call_arg(a, instruction_data, parameters)
                                        })
                                        .collect();
                                    quote! { (#func)(#(#args),*) }
                                }
                                _ => quote! { #arg },
                            }
                        }

                        let mut mapped_args: Vec<TokenStream> = Vec::new();
                        for arg in &call_expr.args {
                            let mapped =
                                map_client_call_arg(arg, instruction_data, &mut parameters);
                            mapped_args.push(mapped);
                        }
                        let func = &call_expr.func;
                        expressions.push(quote! { (#func)(#(#mapped_args),*).as_ref() });
                    }
                    syn::Expr::Reference(ref_expr) => {
                        let (ref_params, ref_exprs) =
                            analyze_seed_spec_for_client_expr(&ref_expr.expr, instruction_data)?;
                        parameters.extend(ref_params);
                        if let Some(first_expr) = ref_exprs.first() {
                            expressions.push(quote! { (#first_expr).as_ref() });
                        }
                    }
                    _ => {
                        expressions.push(quote! { (#expr).as_ref() });
                    }
                }
            }
        }
    }

    Ok((parameters, expressions))
}

#[inline(never)]
fn analyze_seed_spec_for_client_expr(
    expr: &syn::Expr,
    instruction_data: &[InstructionDataSpec],
) -> Result<(Vec<TokenStream>, Vec<TokenStream>)> {
    let mut parameters = Vec::new();
    let mut expressions = Vec::new();

    match expr {
        syn::Expr::Field(field_expr) => {
            if let syn::Member::Named(field_name) = &field_expr.member {
                if let syn::Expr::Field(nested_field) = &*field_expr.base {
                    if let syn::Member::Named(base_name) = &nested_field.member {
                        if base_name == "accounts" {
                            parameters.push(quote! { #field_name: &solana_pubkey::Pubkey });
                            expressions.push(quote! { #field_name });
                        } else if base_name == "data" {
                            // Use declared instruction_data types to determine parameter type
                            if let Some(data_spec) = instruction_data
                                .iter()
                                .find(|d| d.field_name == *field_name)
                            {
                                let param_type = &data_spec.field_type;
                                let param_with_ref = if is_pubkey_type(param_type) {
                                    quote! { #field_name: &#param_type }
                                } else {
                                    quote! { #field_name: #param_type }
                                };
                                parameters.push(param_with_ref);
                                expressions.push(quote! { #field_name });
                            } else {
                                return Err(syn::Error::new_spanned(
                                    field_name,
                                    format!(
                                        "data.{} used in seeds but no type specified",
                                        field_name
                                    ),
                                ));
                            }
                        }
                    }
                } else if let syn::Expr::Path(path) = &*field_expr.base {
                    if let Some(segment) = path.path.segments.first() {
                        if segment.ident == "ctx" {
                            parameters.push(quote! { #field_name: &solana_pubkey::Pubkey });
                            expressions.push(quote! { #field_name });
                        }
                    }
                }
            }
        }
        syn::Expr::MethodCall(method_call) => {
            let (recv_params, _) =
                analyze_seed_spec_for_client_expr(&method_call.receiver, instruction_data)?;
            parameters.extend(recv_params);
        }
        syn::Expr::Call(call_expr) => {
            for arg in &call_expr.args {
                let (arg_params, _) = analyze_seed_spec_for_client_expr(arg, instruction_data)?;
                parameters.extend(arg_params);
            }
        }
        syn::Expr::Reference(ref_expr) => {
            let (ref_params, _) =
                analyze_seed_spec_for_client_expr(&ref_expr.expr, instruction_data)?;
            parameters.extend(ref_params);
        }
        syn::Expr::Path(path_expr) => {
            if let Some(ident) = path_expr.path.get_ident() {
                let name = ident.to_string();
                if !(name == "ctx"
                    || name == "data"
                    || name
                        .chars()
                        .all(|c| c.is_uppercase() || c == '_' || c.is_ascii_digit()))
                {
                    parameters.push(quote! { #ident: &solana_pubkey::Pubkey });
                }
            }
        }
        _ => {}
    }

    Ok((parameters, expressions))
}

fn camel_to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(c.to_lowercase().next().unwrap());
    }
    result
}
