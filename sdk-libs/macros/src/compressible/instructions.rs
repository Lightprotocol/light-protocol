//! Compressible instructions generation.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Expr, Ident, Item, ItemMod, LitStr, Result, Token,
};

/// Convert PascalCase to snake_case (e.g., UserRecord -> user_record)
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

macro_rules! macro_error {
    ($span:expr, $msg:expr) => {
        syn::Error::new_spanned(
            $span,
            format!(
                "{}\n  --> macro location: {}:{}",
                $msg,
                file!(),
                line!()
            )
        )
    };
    ($span:expr, $fmt:expr, $($arg:tt)*) => {
        syn::Error::new_spanned(
            $span,
            format!(
                concat!($fmt, "\n  --> macro location: {}:{}"),
                $($arg)*,
                file!(),
                line!()
            )
        )
    };
}

#[derive(Debug, Clone, Copy)]
pub enum InstructionVariant {
    PdaOnly,
    TokenOnly,
    Mixed,
}

#[derive(Clone)]
pub struct TokenSeedSpec {
    pub variant: Ident,
    pub _eq: Token![=],
    pub is_token: Option<bool>,
    pub seeds: Punctuated<SeedElement, Token![,]>,
    pub authority: Option<Vec<SeedElement>>,
}

impl Parse for TokenSeedSpec {
    fn parse(input: ParseStream) -> Result<Self> {
        let variant: Ident = input.parse()?;
        let _eq: Token![=] = input.parse()?;

        let content;
        syn::parenthesized!(content in input);

        // New explicit syntax:
        //   PDA:   TypeName = (seeds = (...))
        //   Token: TypeName = (is_token, seeds = (...), authority = (...))
        let mut is_token = None;
        let mut seeds = Punctuated::new();
        let mut authority = None;

        while !content.is_empty() {
            if content.peek(Ident) {
                let ident: Ident = content.parse()?;
                let ident_str = ident.to_string();

                match ident_str.as_str() {
                    "is_token" | "true" => {
                        is_token = Some(true);
                    }
                    "is_pda" | "false" => {
                        is_token = Some(false);
                    }
                    "seeds" => {
                        let _eq: Token![=] = content.parse()?;
                        let seeds_content;
                        syn::parenthesized!(seeds_content in content);
                        seeds = parse_seed_elements(&seeds_content)?;
                    }
                    "authority" => {
                        let _eq: Token![=] = content.parse()?;
                        authority = Some(parse_authority_seeds(&content)?);
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            &ident,
                            format!(
                                "Unknown keyword '{}'. Expected: is_token, seeds, or authority.\n\
                                 Use explicit syntax: TypeName = (seeds = (\"seed\", ctx.account, ...))\n\
                                 For tokens: TypeName = (is_token, seeds = (...), authority = (...))",
                                ident_str
                            ),
                        ));
                    }
                }
            } else {
                return Err(syn::Error::new(
                    content.span(),
                    "Expected keyword (is_token, seeds, or authority). Use explicit syntax:\n\
                     - PDA: TypeName = (seeds = (\"seed\", ctx.account, ...))\n\
                     - Token: TypeName = (is_token, seeds = (...), authority = (...))",
                ));
            }

            if content.peek(Token![,]) {
                let _comma: Token![,] = content.parse()?;
            } else {
                break;
            }
        }

        if seeds.is_empty() {
            return Err(syn::Error::new_spanned(
                &variant,
                format!(
                    "Missing seeds for '{}'. Use: {} = (seeds = (\"seed\", ctx.account, ...))",
                    variant, variant
                ),
            ));
        }

        Ok(TokenSeedSpec {
            variant,
            _eq,
            is_token,
            seeds,
            authority,
        })
    }
}

/// Parse seed elements from within seeds = (...)
fn parse_seed_elements(content: ParseStream) -> Result<Punctuated<SeedElement, Token![,]>> {
    let mut seeds = Punctuated::new();

    while !content.is_empty() {
        seeds.push(content.parse::<SeedElement>()?);

        if content.peek(Token![,]) {
            let _: Token![,] = content.parse()?;
            if content.is_empty() {
                break;
            }
        } else {
            break;
        }
    }

    Ok(seeds)
}

/// Parse authority seeds - either parenthesized tuple or single expression
fn parse_authority_seeds(content: ParseStream) -> Result<Vec<SeedElement>> {
    if content.peek(syn::token::Paren) {
        let auth_content;
        syn::parenthesized!(auth_content in content);
        let mut auth_seeds = Vec::new();

        while !auth_content.is_empty() {
            auth_seeds.push(auth_content.parse::<SeedElement>()?);
            if auth_content.peek(Token![,]) {
                let _: Token![,] = auth_content.parse()?;
            } else {
                break;
            }
        }
        Ok(auth_seeds)
    } else {
        // Single expression (e.g., LIGHT_CPI_SIGNER)
        Ok(vec![content.parse::<SeedElement>()?])
    }
}

#[derive(Clone)]
pub enum SeedElement {
    Literal(LitStr),
    Expression(Box<Expr>),
}

impl Parse for SeedElement {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(LitStr) {
            Ok(SeedElement::Literal(input.parse()?))
        } else {
            Ok(SeedElement::Expression(input.parse()?))
        }
    }
}

/// Recursively extract all ctx.XXX or ctx.accounts.XXX field names from an expression.
/// Handles nested expressions like function calls: max_key(&ctx.user.key(), &ctx.authority.key())
fn extract_ctx_fields_from_expr(expr: &syn::Expr, fields: &mut Vec<Ident>) {
    match expr {
        syn::Expr::Field(field_expr) => {
            if let syn::Member::Named(field_name) = &field_expr.member {
                // Check for ctx.XXX pattern (direct field access)
                if let syn::Expr::Path(path) = &*field_expr.base {
                    if let Some(segment) = path.path.segments.first() {
                        if segment.ident == "ctx" {
                            fields.push(field_name.clone());
                            return;
                        }
                    }
                }
                // Check for ctx.accounts.XXX pattern (nested field access)
                if let syn::Expr::Field(nested_field) = &*field_expr.base {
                    if let syn::Member::Named(base_name) = &nested_field.member {
                        if base_name == "accounts" {
                            if let syn::Expr::Path(path) = &*nested_field.base {
                                if let Some(segment) = path.path.segments.first() {
                                    if segment.ident == "ctx" {
                                        fields.push(field_name.clone());
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Recurse into base expression
            extract_ctx_fields_from_expr(&field_expr.base, fields);
        }
        syn::Expr::MethodCall(method) => {
            // Recurse into receiver and args
            extract_ctx_fields_from_expr(&method.receiver, fields);
            for arg in &method.args {
                extract_ctx_fields_from_expr(arg, fields);
            }
        }
        syn::Expr::Call(call) => {
            // Recurse into function args
            for arg in &call.args {
                extract_ctx_fields_from_expr(arg, fields);
            }
        }
        syn::Expr::Reference(ref_expr) => {
            extract_ctx_fields_from_expr(&ref_expr.expr, fields);
        }
        syn::Expr::Paren(paren) => {
            extract_ctx_fields_from_expr(&paren.expr, fields);
        }
        _ => {}
    }
}

/// Extract ctx.XXX or ctx.accounts.XXX field names from a seed element.
fn extract_ctx_account_fields(seed: &SeedElement) -> Vec<Ident> {
    let mut fields = Vec::new();
    if let SeedElement::Expression(expr) = seed {
        extract_ctx_fields_from_expr(expr, &mut fields);
    }
    fields
}

/// Extract all ctx.accounts.XXX field names from a list of seed elements.
/// Deduplicates the fields.
pub fn extract_ctx_seed_fields(seeds: &syn::punctuated::Punctuated<SeedElement, Token![,]>) -> Vec<Ident> {
    let mut all_fields = Vec::new();
    for seed in seeds {
        all_fields.extend(extract_ctx_account_fields(seed));
    }
    // Deduplicate while preserving order
    let mut seen = std::collections::HashSet::new();
    all_fields
        .into_iter()
        .filter(|f| seen.insert(f.to_string()))
        .collect()
}

/// Phase 5: Extract data.XXX field names from an expression recursively.
fn extract_data_fields_from_expr(expr: &syn::Expr, fields: &mut Vec<Ident>) {
    match expr {
        syn::Expr::Field(field_expr) => {
            if let syn::Member::Named(field_name) = &field_expr.member {
                // Check for data.XXX pattern
                if let syn::Expr::Path(path) = &*field_expr.base {
                    if let Some(segment) = path.path.segments.first() {
                        if segment.ident == "data" {
                            fields.push(field_name.clone());
                            return;
                        }
                    }
                }
            }
            // Recurse into base expression
            extract_data_fields_from_expr(&field_expr.base, fields);
        }
        syn::Expr::MethodCall(method) => {
            extract_data_fields_from_expr(&method.receiver, fields);
            for arg in &method.args {
                extract_data_fields_from_expr(arg, fields);
            }
        }
        syn::Expr::Call(call) => {
            for arg in &call.args {
                extract_data_fields_from_expr(arg, fields);
            }
        }
        syn::Expr::Reference(ref_expr) => {
            extract_data_fields_from_expr(&ref_expr.expr, fields);
        }
        syn::Expr::Paren(paren) => {
            extract_data_fields_from_expr(&paren.expr, fields);
        }
        _ => {}
    }
}

/// Phase 5: Extract all data.XXX field names from a list of seed elements.
pub fn extract_data_seed_fields(seeds: &syn::punctuated::Punctuated<SeedElement, Token![,]>) -> Vec<Ident> {
    let mut all_fields = Vec::new();
    for seed in seeds {
        if let SeedElement::Expression(expr) = seed {
            extract_data_fields_from_expr(expr, &mut all_fields);
        }
    }
    // Deduplicate while preserving order
    let mut seen = std::collections::HashSet::new();
    all_fields
        .into_iter()
        .filter(|f| seen.insert(f.to_string()))
        .collect()
}

pub struct InstructionDataSpec {
    pub field_name: Ident,
    pub field_type: syn::Type,
}

impl Parse for InstructionDataSpec {
    fn parse(input: ParseStream) -> Result<Self> {
        let field_name: Ident = input.parse()?;
        let _eq: Token![=] = input.parse()?;
        let field_type: syn::Type = input.parse()?;

        Ok(InstructionDataSpec {
            field_name,
            field_type,
        })
    }
}

<<<<<<< HEAD
struct EnhancedMacroArgs {
    account_types: Vec<Ident>,
    pda_seeds: Vec<TokenSeedSpec>,
    token_seeds: Vec<TokenSeedSpec>,
    instruction_data: Vec<InstructionDataSpec>,
}

impl Parse for EnhancedMacroArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut account_types = Vec::new();
        let mut pda_seeds = Vec::new();
        let mut token_seeds = Vec::new();
        let mut instruction_data = Vec::new();

        let mut _item_count = 0;
        while !input.is_empty() {
            let ident: Ident = input.parse()?;

            if input.peek(Token![=]) {
                let _eq: Token![=] = input.parse()?;

                if input.peek(syn::token::Paren) {
                    let content;
                    syn::parenthesized!(content in input);
                    let inside: TokenStream = content.parse()?;
                    let seed_spec: TokenSeedSpec = syn::parse2(quote! { #ident = (#inside) })?;

                    let is_token_account = seed_spec.is_token.unwrap_or(false);
                    if is_token_account {
                        token_seeds.push(seed_spec);
                    } else {
                        pda_seeds.push(seed_spec);
                        account_types.push(ident);
                    }
                } else {
                    let field_type: syn::Type = input.parse()?;
                    instruction_data.push(InstructionDataSpec {
                        field_name: ident,
                        field_type,
                    });
                }
            } else {
                account_types.push(ident);
            }

            if input.peek(Token![,]) {
                let _comma: Token![,] = input.parse()?;
            } else {
                break;
            }
            _item_count += 1;
        }
        Ok(EnhancedMacroArgs {
            account_types,
            pda_seeds,
            token_seeds,
            instruction_data,
        })
    }
}

#[allow(clippy::too_many_arguments)]
#[inline(never)]
pub fn add_compressible_instructions(
    args: TokenStream,
    mut module: ItemMod,
) -> Result<TokenStream> {
    let enhanced_args = match syn::parse2::<EnhancedMacroArgs>(args.clone()) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("ERROR: Failed to parse macro args: {}", e);
            eprintln!("Args were: {}", args);
            return Err(e);
        }
    };

    let account_types = enhanced_args.account_types;
    let pda_seeds = Some(enhanced_args.pda_seeds);
    let token_seeds = Some(enhanced_args.token_seeds);
    let instruction_data = enhanced_args.instruction_data;

    if module.content.is_none() {
        return Err(macro_error!(&module, "Module must have a body"));
    }

    if account_types.is_empty() {
        return Err(macro_error!(
            &module,
            "At least one account type must be specified"
        ));
    }

    let size_validation_checks = validate_compressed_account_sizes(&account_types)?;

    let content = module.content.as_mut().unwrap();

    let ctoken_enum = if let Some(ref token_seed_specs) = token_seeds {
        if !token_seed_specs.is_empty() {
            crate::compressible::seed_providers::generate_token_account_variant_enum(
                token_seed_specs,
            )?
        } else {
            crate::compressible::utils::generate_empty_ctoken_enum()
        }
    } else {
        crate::compressible::utils::generate_empty_ctoken_enum()
    };

    if let Some(ref token_seed_specs) = token_seeds {
        for spec in token_seed_specs {
            if spec.is_ata {
                if !spec.seeds.is_empty() {
                    return Err(macro_error!(
                        &spec.variant,
                        "ATA variant '{}' must not have seeds - ATAs are derived from owner+mint only",
                        spec.variant
                    ));
                }
                if spec.authority.is_some() {
                    return Err(macro_error!(
                        &spec.variant,
                        "ATA variant '{}' must not have authority - ATAs are owned by user wallets",
                        spec.variant
                    ));
                }
            } else if spec.authority.is_none() {
                return Err(macro_error!(
                    &spec.variant,
                    "Program-owned token account '{}' must specify authority = <seed_expr> for compression signing. For user-owned ATAs, use is_ata flag instead.",
                    spec.variant
                ));
            }
        }
    }

    let mut account_types_stream = TokenStream::new();
    for (i, account_type) in account_types.iter().enumerate() {
        if i > 0 {
            account_types_stream.extend(quote! { , });
        }
        account_types_stream.extend(quote! { #account_type });
    }
    let enum_and_traits =
        crate::compressible::variant_enum::compressed_account_variant(account_types_stream)?;

    // Generate SeedParams struct for instruction data fields
    let seed_params_struct = {
        let param_fields: Vec<_> = instruction_data
            .iter()
            .map(|spec| {
                let field_name = &spec.field_name;
                let field_type = &spec.field_type;
                quote! {
                    pub #field_name: #field_type
                }
            })
            .collect();

        quote! {
            #[derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize, Clone, Debug, Default)]
            pub struct SeedParams {
                #(#param_fields,)*
            }
        }
    };

    let has_pda_seeds = pda_seeds.as_ref().map(|p| !p.is_empty()).unwrap_or(false);
    let has_token_seeds = token_seeds.as_ref().map(|t| !t.is_empty()).unwrap_or(false);

    let instruction_variant = match (has_pda_seeds, has_token_seeds) {
        (true, true) => InstructionVariant::Mixed,
        (true, false) => InstructionVariant::PdaOnly,
        (false, true) => InstructionVariant::TokenOnly,
        (false, false) => {
            return Err(macro_error!(
                &module,
                "At least one PDA or token seed specification must be provided"
            ))
        }
    };

    let error_codes = generate_error_codes(instruction_variant)?;

    let required_accounts = extract_required_accounts_from_seeds(&pda_seeds, &token_seeds)?;

    let decompress_accounts =
        generate_decompress_accounts_struct(&required_accounts, instruction_variant)?;

    let pda_seed_provider_impls: Result<Vec<_>> = account_types
        .iter()
        .map(|name| {
            let name_str = name.to_string();
            let spec = if let Some(ref pda_seed_specs) = pda_seeds {
                pda_seed_specs
                    .iter()
                    .find(|s| s.variant == name_str)
                    .ok_or_else(|| {
                        macro_error!(
                            name,
                            "No seed specification for account type '{}'. All accounts must have seed specifications.",
                            name_str
                        )
                    })?
            } else {
                return Err(macro_error!(
                    name,
                    "No seed specifications provided. Use: AccountType = (\"seed\", data.field)"
                ));
            };
            let seed_derivation =
                generate_pda_seed_derivation_for_trait(spec, &instruction_data)?;
            Ok(quote! {
                impl<'info> light_sdk::compressible::PdaSeedDerivation<DecompressAccountsIdempotent<'info>, SeedParams> for #name {
                    fn derive_pda_seeds_with_accounts(
                        &self,
                        program_id: &solana_pubkey::Pubkey,
                        accounts: &DecompressAccountsIdempotent<'info>,
                        seed_params: &SeedParams,
                    ) -> std::result::Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey), solana_program_error::ProgramError> {
                        #seed_derivation
                    }
                }
            })
        })
        .collect();
    let pda_seed_provider_impls = pda_seed_provider_impls?;

    let helper_packed_fns: Vec<_> = account_types.iter().map(|name| {
        let packed_name = format_ident!("Packed{}", name);
        let func_name = format_ident!("handle_packed_{}", name);
        quote! {
            #[inline(never)]
            #[allow(clippy::too_many_arguments)]
            fn #func_name<'a, 'b, 'info>(
                accounts: &DecompressAccountsIdempotent<'info>,
                cpi_accounts: &light_sdk::cpi::v2::CpiAccounts<'b, 'info>,
                address_space: solana_pubkey::Pubkey,
                solana_accounts: &[solana_account_info::AccountInfo<'info>],
                i: usize,
                packed: &#packed_name,
                meta: &light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                post_system_accounts: &[solana_account_info::AccountInfo<'info>],
                compressed_pda_infos: &mut Vec<light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo>,
                seed_accounts: &DecompressAccountsIdempotent<'info>,
                seed_params: &SeedParams,
            ) -> std::result::Result<(), solana_program_error::ProgramError> {
                light_sdk::compressible::handle_packed_pda_variant::<#name, #packed_name, DecompressAccountsIdempotent<'info>, SeedParams>(
                    accounts.rent_sponsor.as_ref(),
                    cpi_accounts,
                    address_space,
                    &solana_accounts[i],
                    i,
                    packed,
                    meta,
                    post_system_accounts,
                    compressed_pda_infos,
                    &crate::ID,
                    seed_accounts,
                    std::option::Option::Some(seed_params),
                )
            }
        }
    }).collect();

    let call_unpacked_arms: Vec<_> = account_types.iter().map(|name| {
        quote! {
            CompressedAccountVariant::#name(_) => {
                unreachable!("Unpacked variants should not be present during decompression - accounts are always packed in-flight");
            }
        }
    }).collect();
    let call_packed_arms: Vec<_> = account_types.iter().map(|name| {
        let packed_name = format_ident!("Packed{}", name);
        let func_name = format_ident!("handle_packed_{}", name);
        quote! {
                            CompressedAccountVariant::#packed_name(packed) => {
                                match #func_name(accounts, &cpi_accounts, address_space, solana_accounts, i, &packed, &meta, post_system_accounts, &mut compressed_pda_infos, accounts, seed_params) {
                                    std::result::Result::Ok(()) => {},
                                    std::result::Result::Err(e) => return std::result::Result::Err(e),
                                }
                            }
        }
    }).collect();

    let trait_impls: syn::ItemMod = syn::parse_quote! {
        mod __trait_impls {
            use super::*;

            impl light_sdk::compressible::HasTokenVariant for CompressedAccountData {
                fn is_packed_token(&self) -> bool {
                    matches!(self.data, CompressedAccountVariant::PackedCTokenData(_))
                }
            }

            impl light_sdk::compressible::TokenSeedProvider for CTokenAccountVariant {
                type Accounts<'info> = DecompressAccountsIdempotent<'info>;

                fn get_seeds<'a, 'info>(
                    &self,
                    accounts: &'a Self::Accounts<'info>,
                    remaining_accounts: &'a [solana_account_info::AccountInfo<'info>],
                ) -> std::result::Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey), anchor_lang::prelude::ProgramError> {
                    use super::ctoken_seed_system::{
                        CTokenSeedContext,
                        CTokenSeedProvider as LocalProvider,
                    };
                    let ctx = CTokenSeedContext {
                        accounts,
                        remaining_accounts,
                    };
                    LocalProvider::get_seeds(self, &ctx).map_err(|e: anchor_lang::error::Error| -> anchor_lang::prelude::ProgramError { e.into() })
                }

                fn get_authority_seeds<'a, 'info>(
                    &self,
                    accounts: &'a Self::Accounts<'info>,
                    remaining_accounts: &'a [solana_account_info::AccountInfo<'info>],
                ) -> std::result::Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey), anchor_lang::prelude::ProgramError> {
                    use super::ctoken_seed_system::{
                        CTokenSeedContext,
                        CTokenSeedProvider as LocalProvider,
                    };
                    let ctx = CTokenSeedContext {
                        accounts,
                        remaining_accounts,
                    };
                    LocalProvider::get_authority_seeds(self, &ctx).map_err(|e: anchor_lang::error::Error| -> anchor_lang::prelude::ProgramError { e.into() })
                }
            }

            impl light_token_sdk::compressible::TokenSeedProvider for CTokenAccountVariant {
                type Accounts<'info> = DecompressAccountsIdempotent<'info>;

                fn get_seeds<'a, 'info>(
                    &self,
                    accounts: &'a Self::Accounts<'info>,
                    remaining_accounts: &'a [solana_account_info::AccountInfo<'info>],
                ) -> std::result::Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey), solana_program_error::ProgramError> {
                    use super::ctoken_seed_system::{
                        CTokenSeedContext,
                        CTokenSeedProvider as LocalProvider,
                    };
                    let ctx = CTokenSeedContext {
                        accounts,
                        remaining_accounts,
                    };
                    LocalProvider::get_seeds(self, &ctx)
                        .map_err(|e: anchor_lang::error::Error| {
                            let program_error: anchor_lang::prelude::ProgramError = e.into();
                            let code = match program_error {
                                anchor_lang::prelude::ProgramError::Custom(code) => code,
                                _ => 0,
                            };
                            solana_program_error::ProgramError::Custom(code)
                        })
                }

                fn get_authority_seeds<'a, 'info>(
                    &self,
                    accounts: &'a Self::Accounts<'info>,
                    remaining_accounts: &'a [solana_account_info::AccountInfo<'info>],
                ) -> std::result::Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey), solana_program_error::ProgramError> {
                    use super::ctoken_seed_system::{
                        CTokenSeedContext,
                        CTokenSeedProvider as LocalProvider,
                    };
                    let ctx = CTokenSeedContext {
                        accounts,
                        remaining_accounts,
                    };
                    LocalProvider::get_authority_seeds(self, &ctx)
                        .map_err(|e: anchor_lang::error::Error| {
                            let program_error: anchor_lang::prelude::ProgramError = e.into();
                            let code = match program_error {
                                anchor_lang::prelude::ProgramError::Custom(code) => code,
                                _ => 0,
                            };
                            solana_program_error::ProgramError::Custom(code)
                        })
                }
            }
        }
    };

    let ctoken_trait_system: syn::ItemMod = syn::parse_quote! {
        pub mod ctoken_seed_system {
            use super::*;

            pub struct CTokenSeedContext<'a, 'info> {
                pub accounts: &'a DecompressAccountsIdempotent<'info>,
                pub remaining_accounts: &'a [solana_account_info::AccountInfo<'info>],
            }

            pub trait CTokenSeedProvider {
                fn get_seeds<'a, 'info>(
                    &self,
                    ctx: &CTokenSeedContext<'a, 'info>,
                ) -> Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey)>;

                fn get_authority_seeds<'a, 'info>(
                    &self,
                    ctx: &CTokenSeedContext<'a, 'info>,
                ) -> Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey)>;
            }
        }
    };

    let helpers_module: syn::ItemMod = {
        let helper_packed_fns = helper_packed_fns.clone();
        let call_unpacked_arms = call_unpacked_arms.clone();
        let call_packed_arms = call_packed_arms.clone();
        syn::parse_quote! {
        mod __macro_helpers {
            use super::*;
            use crate::state::*;  // Import Packed* types from state module
            #(#helper_packed_fns)*
                #[inline(never)]
                pub fn collect_pda_and_token<'a, 'b, 'info>(
                    accounts: &DecompressAccountsIdempotent<'info>,
                    cpi_accounts: &light_sdk::cpi::v2::CpiAccounts<'b, 'info>,
                    address_space: solana_pubkey::Pubkey,
                    compressed_accounts: Vec<CompressedAccountData>,
                    solana_accounts: &[solana_account_info::AccountInfo<'info>],
                    seed_params: &SeedParams,
                ) -> std::result::Result<(
                    Vec<light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo>,
                    Vec<(
                        light_token_sdk::compat::PackedCTokenData<CTokenAccountVariant>,
                        light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                    )>,
                ), solana_program_error::ProgramError> {
                    let post_system_offset = cpi_accounts.system_accounts_end_offset();
                    let all_infos = cpi_accounts.account_infos();
                    let post_system_accounts = &all_infos[post_system_offset..];
                    let estimated_capacity = compressed_accounts.len();
                    let mut compressed_pda_infos = Vec::with_capacity(estimated_capacity);
                    let mut compressed_token_accounts: Vec<(
                        light_token_sdk::compat::PackedCTokenData<CTokenAccountVariant>,
                        light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                    )> = Vec::with_capacity(estimated_capacity);

                    for (i, compressed_data) in compressed_accounts.into_iter().enumerate() {
                        let meta = compressed_data.meta;
                        match compressed_data.data {
                            #(#call_unpacked_arms)*
                            #(#call_packed_arms)*
                            CompressedAccountVariant::PackedCTokenData(mut data) => {
                                data.token_data.version = 3;
                                compressed_token_accounts.push((data, meta));
                            }
                            CompressedAccountVariant::CTokenData(_) => {
                                unreachable!();
                            }
                        }
                    }

                    std::result::Result::Ok((compressed_pda_infos, compressed_token_accounts))
                }
            }
        }
    };

    let token_variant_name = format_ident!("CTokenAccountVariant");

    let decompress_context_impl = generate_decompress_context_impl(
        instruction_variant,
        account_types.clone(),
        token_variant_name,
    )?;
    let decompress_processor_fn =
        generate_process_decompress_accounts_idempotent(instruction_variant, &instruction_data)?;
    let decompress_instruction =
        generate_decompress_instruction_entrypoint(instruction_variant, &instruction_data)?;

    let compress_accounts: syn::ItemStruct = match instruction_variant {
        InstructionVariant::PdaOnly => unreachable!(),
        InstructionVariant::TokenOnly => unreachable!(),
        InstructionVariant::Mixed => syn::parse_quote! {
        #[derive(Accounts)]
        pub struct CompressAccountsIdempotent<'info> {
            #[account(mut)]
            pub fee_payer: Signer<'info>,
            /// CHECK: Checked by SDK
            pub config: AccountInfo<'info>,
            /// CHECK: Checked by SDK
            #[account(mut)]
            pub rent_sponsor: AccountInfo<'info>,
            /// CHECK: Checked by SDK
            #[account(mut)]
            pub compression_authority: AccountInfo<'info>,
        }
        },
    };

    let compress_context_impl =
        generate_compress_context_impl(instruction_variant, account_types.clone())?;
    let compress_processor_fn = generate_process_compress_accounts_idempotent(instruction_variant)?;
    let compress_instruction = generate_compress_instruction_entrypoint(instruction_variant)?;

    let processor_module: syn::ItemMod = syn::parse_quote! {
        mod __processor_functions {
            use super::*;
            #decompress_processor_fn
            #compress_processor_fn
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
            /// CHECK: Checked by SDK
            pub authority: Signer<'info>,
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
            light_sdk::compressible::process_initialize_compression_config_checked(
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
            light_sdk::compressible::process_update_compression_config(
                ctx.accounts.config.as_ref(),
                ctx.accounts.authority.as_ref(),
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

    // Insert SeedParams struct
    let seed_params_item: Item = syn::parse2(seed_params_struct)?;
    content.1.push(seed_params_item);

    content.1.push(Item::Struct(decompress_accounts));
    content.1.push(Item::Mod(helpers_module));
    content.1.push(Item::Mod(ctoken_trait_system));
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

    if let Some(ref seeds) = token_seeds {
        if !seeds.is_empty() {
            let impl_code =
                crate::compressible::seed_providers::generate_token_seed_provider_implementation(
                    seeds,
                )?;
            let ctoken_impl: syn::ItemImpl = syn::parse2(impl_code).map_err(|e| {
                syn::Error::new_spanned(
                    &seeds[0].variant,
                    format!("Failed to parse ctoken implementation: {}", e),
                )
            })?;
            content.1.push(Item::Impl(ctoken_impl));
        }
    }

    let client_seed_functions =
        crate::compressible::seed_providers::generate_client_seed_functions(
            &account_types,
            &pda_seeds,
            &token_seeds,
            &instruction_data,
        )?;

    // Add allow attribute to module itself to suppress clippy warnings
    module.attrs.push(syn::parse_quote! {
        #[allow(clippy::too_many_arguments)]
    });

    Ok(quote! {
        #size_validation_checks
        #error_codes
        #ctoken_enum
        #enum_and_traits
        #(#pda_seed_provider_impls)*
        #[allow(non_snake_case)]
        #module
        #client_seed_functions
    })
}

=======
>>>>>>> a606eb113 (wip)
pub fn generate_decompress_context_impl(
    _variant: InstructionVariant,
    pda_ctx_seeds: Vec<crate::compressible::variant_enum::PdaCtxSeedInfo>,
    token_variant_ident: Ident,
) -> Result<syn::ItemMod> {
    let lifetime: syn::Lifetime = syn::parse_quote!('info);

    let trait_impl =
        crate::compressible::decompress_context::generate_decompress_context_trait_impl(
            pda_ctx_seeds,
            token_variant_ident,
            lifetime,
        )?;

    Ok(syn::parse_quote! {
        mod __decompress_context_impl {
            use super::*;

            #trait_impl
        }
    })
}

pub fn generate_process_decompress_accounts_idempotent(
    _variant: InstructionVariant,
    _instruction_data: &[InstructionDataSpec],
) -> Result<syn::ItemFn> {
    // Phase 4: seed_data removed - data.* seeds come from unpacked account data, ctx.* from variant idx
    Ok(syn::parse_quote! {
        #[inline(never)]
        pub fn process_decompress_accounts_idempotent<'info>(
            accounts: &DecompressAccountsIdempotent<'info>,
            remaining_accounts: &[solana_account_info::AccountInfo<'info>],
            proof: light_sdk::instruction::ValidityProof,
            compressed_accounts: Vec<RentFreeAccountData>,
            system_accounts_offset: u8,
        ) -> Result<()> {
            light_sdk::compressible::process_decompress_accounts_idempotent(
                accounts,
                remaining_accounts,
                compressed_accounts,
                proof,
                system_accounts_offset,
                LIGHT_CPI_SIGNER,
                &crate::ID,
                std::option::Option::None::<&()>,
            )
            .map_err(|e: solana_program_error::ProgramError| -> anchor_lang::error::Error { e.into() })
        }
    })
}

pub fn generate_decompress_instruction_entrypoint(
    _variant: InstructionVariant,
    _instruction_data: &[InstructionDataSpec],
) -> Result<syn::ItemFn> {
    // Phase 4: seed_data removed - data.* seeds come from unpacked account data, ctx.* from variant idx

    Ok(syn::parse_quote! {
        #[inline(never)]
        pub fn decompress_accounts_idempotent<'info>(
            ctx: Context<'_, '_, '_, 'info, DecompressAccountsIdempotent<'info>>,
            proof: light_sdk::instruction::ValidityProof,
            compressed_accounts: Vec<RentFreeAccountData>,
            system_accounts_offset: u8,
        ) -> Result<()> {
            __processor_functions::process_decompress_accounts_idempotent(
                &ctx.accounts,
                &ctx.remaining_accounts,
                proof,
                compressed_accounts,
                system_accounts_offset,
            )
        }
    })
}

pub fn generate_compress_context_impl(
    _variant: InstructionVariant,
    account_types: Vec<Ident>,
) -> Result<syn::ItemMod> {
    let lifetime: syn::Lifetime = syn::parse_quote!('info);

    let compress_arms: Vec<_> = account_types.iter().map(|name| {
        quote! {
            d if d == #name::LIGHT_DISCRIMINATOR => {
                drop(data);
                let data_borrow = account_info.try_borrow_data().map_err(|e| {
                    let err: anchor_lang::error::Error = e.into();
                    let program_error: anchor_lang::prelude::ProgramError = err.into();
                    let code = match program_error {
                        anchor_lang::prelude::ProgramError::Custom(code) => code,
                        _ => 0,
                    };
                    solana_program_error::ProgramError::Custom(code)
                })?;
                let mut account_data = #name::try_deserialize(&mut &data_borrow[..]).map_err(|e| {
                    let err: anchor_lang::error::Error = e.into();
                    let program_error: anchor_lang::prelude::ProgramError = err.into();
                    let code = match program_error {
                        anchor_lang::prelude::ProgramError::Custom(code) => code,
                        _ => 0,
                    };
                    solana_program_error::ProgramError::Custom(code)
                })?;
                drop(data_borrow);

                let compressed_info = light_sdk::compressible::compress_account::prepare_account_for_compression::<#name>(
                    program_id,
                    account_info,
                    &mut account_data,
                    meta,
                    cpi_accounts,
                    &compression_config.address_space,
                )?;
                // Lamport transfers are handled by close() in process_compress_pda_accounts_idempotent
                // All lamports go to rent_sponsor for simplicity
                Ok(Some(compressed_info))
            }
        }
    }).collect();

    Ok(syn::parse_quote! {
        mod __compress_context_impl {
            use super::*;
            use light_sdk::LightDiscriminator;
            use light_sdk::compressible::HasCompressionInfo;

            impl<#lifetime> light_sdk::compressible::CompressContext<#lifetime> for CompressAccountsIdempotent<#lifetime> {
                fn fee_payer(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                    &*self.fee_payer
                }

                fn config(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                    &self.config
                }

                fn rent_sponsor(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                    &self.rent_sponsor
                }

                fn compression_authority(&self) -> &solana_account_info::AccountInfo<#lifetime> {
                    &self.compression_authority
                }

                fn compress_pda_account(
                    &self,
                    account_info: &solana_account_info::AccountInfo<#lifetime>,
                    meta: &light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress,
                    cpi_accounts: &light_sdk::cpi::v2::CpiAccounts<'_, #lifetime>,
                    compression_config: &light_sdk::compressible::CompressibleConfig,
                    program_id: &solana_pubkey::Pubkey,
                ) -> std::result::Result<Option<light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo>, solana_program_error::ProgramError> {
                    let data = account_info.try_borrow_data().map_err(|e| {
                        let err: anchor_lang::error::Error = e.into();
                        let program_error: anchor_lang::prelude::ProgramError = err.into();
                        let code = match program_error {
                            anchor_lang::prelude::ProgramError::Custom(code) => code,
                            _ => 0,
                        };
                        solana_program_error::ProgramError::Custom(code)
                    })?;
                    let discriminator = &data[0..8];

                    match discriminator {
                        #(#compress_arms)*
                        _ => {
                            let err: anchor_lang::error::Error = anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch.into();
                            let program_error: anchor_lang::prelude::ProgramError = err.into();
                            let code = match program_error {
                                anchor_lang::prelude::ProgramError::Custom(code) => code,
                                _ => 0,
                            };
                            Err(solana_program_error::ProgramError::Custom(code))
                        }
                    }
                }
            }
        }
    })
}

pub fn generate_process_compress_accounts_idempotent(
    _variant: InstructionVariant,
) -> Result<syn::ItemFn> {
    Ok(syn::parse_quote! {
        #[inline(never)]
        pub fn process_compress_accounts_idempotent<'info>(
            accounts: &CompressAccountsIdempotent<'info>,
            remaining_accounts: &[solana_account_info::AccountInfo<'info>],
            compressed_accounts: Vec<light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress>,
            system_accounts_offset: u8,
        ) -> Result<()> {
            light_sdk::compressible::compress_runtime::process_compress_pda_accounts_idempotent(
                accounts,
                remaining_accounts,
                compressed_accounts,
                system_accounts_offset,
                LIGHT_CPI_SIGNER,
                &crate::ID,
            )
            .map_err(|e: solana_program_error::ProgramError| -> anchor_lang::error::Error { e.into() })
        }
    })
}

pub fn generate_compress_instruction_entrypoint(
    _variant: InstructionVariant,
) -> Result<syn::ItemFn> {
    Ok(syn::parse_quote! {
        #[inline(never)]
        #[allow(clippy::too_many_arguments)]
        pub fn compress_accounts_idempotent<'info>(
            ctx: Context<'_, '_, '_, 'info, CompressAccountsIdempotent<'info>>,
            proof: light_sdk::instruction::ValidityProof,
            compressed_accounts: Vec<light_sdk::instruction::account_meta::CompressedAccountMetaNoLamportsNoAddress>,
            system_accounts_offset: u8,
        ) -> Result<()> {
            __processor_functions::process_compress_accounts_idempotent(
                &ctx.accounts,
                &ctx.remaining_accounts,
                compressed_accounts,
                system_accounts_offset,
            )
        }
    })
}

/// Phase 3: Generate PDA seed derivation that uses CtxSeeds struct instead of DecompressAccountsIdempotent.
/// Maps ctx.field -> ctx_seeds.field (direct Pubkey access, no Option unwrapping needed)
#[inline(never)]
fn generate_pda_seed_derivation_for_trait_with_ctx_seeds(
    spec: &TokenSeedSpec,
    _instruction_data: &[InstructionDataSpec],
    ctx_seed_fields: &[syn::Ident],
) -> Result<TokenStream> {
    let mut bindings: Vec<TokenStream> = Vec::new();
    let mut seed_refs = Vec::new();

    // Convert ctx_seed_fields to a set for quick lookup
    let ctx_field_names: std::collections::HashSet<String> =
        ctx_seed_fields.iter().map(|f| f.to_string()).collect();

    // Recursively rewrite expressions:
    // - `data.<field>` -> `self.<field>` (from unpacked compressed account data - Phase 4)
    // - `ctx.accounts.<account>` -> `ctx_seeds.<account>` (direct Pubkey on CtxSeeds struct)
    // - `ctx.<field>` -> `ctx_seeds.<field>` (direct Pubkey on CtxSeeds struct)
    fn map_pda_expr_to_ctx_seeds(
        expr: &syn::Expr,
        ctx_field_names: &std::collections::HashSet<String>,
    ) -> syn::Expr {
        match expr {
            syn::Expr::Field(field_expr) => {
                if let syn::Member::Named(field_name) = &field_expr.member {
                    // Handle nested field access: ctx.accounts.field_name -> ctx_seeds.field_name
                    if let syn::Expr::Field(nested_field) = &*field_expr.base {
                        if let syn::Member::Named(base_name) = &nested_field.member {
                            if base_name == "accounts" {
                                if let syn::Expr::Path(path) = &*nested_field.base {
                                    if let Some(segment) = path.path.segments.first() {
                                        if segment.ident == "ctx" {
                                            // ctx.accounts.field -> ctx_seeds.field (direct Pubkey)
                                            return syn::parse_quote! { ctx_seeds.#field_name };
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // Handle direct field access
                    if let syn::Expr::Path(path) = &*field_expr.base {
                        if let Some(segment) = path.path.segments.first() {
                            if segment.ident == "data" {
                                // Phase 4: data.field -> self.field (from unpacked compressed account data)
                                return syn::parse_quote! { self.#field_name };
                            } else if segment.ident == "ctx" {
                                let field_str = field_name.to_string();
                                if ctx_field_names.contains(&field_str) {
                                    // ctx.field -> ctx_seeds.field (direct Pubkey)
                                    return syn::parse_quote! { ctx_seeds.#field_name };
                                }
                            }
                        }
                    }
                }
                expr.clone()
            }
            syn::Expr::MethodCall(method_call) => {
                let mut new_method_call = method_call.clone();
                new_method_call.receiver =
                    Box::new(map_pda_expr_to_ctx_seeds(&method_call.receiver, ctx_field_names));
                new_method_call.args = method_call
                    .args
                    .iter()
                    .map(|a| map_pda_expr_to_ctx_seeds(a, ctx_field_names))
                    .collect();
                syn::Expr::MethodCall(new_method_call)
            }
            syn::Expr::Call(call_expr) => {
                let mut new_call_expr = call_expr.clone();
                new_call_expr.args = call_expr
                    .args
                    .iter()
                    .map(|a| map_pda_expr_to_ctx_seeds(a, ctx_field_names))
                    .collect();
                syn::Expr::Call(new_call_expr)
            }
            syn::Expr::Reference(ref_expr) => {
                let mut new_ref_expr = ref_expr.clone();
                new_ref_expr.expr =
                    Box::new(map_pda_expr_to_ctx_seeds(&ref_expr.expr, ctx_field_names));
                syn::Expr::Reference(new_ref_expr)
            }
            _ => expr.clone(),
        }
    }

    for (i, seed) in spec.seeds.iter().enumerate() {
        match seed {
            SeedElement::Literal(lit) => {
                let value = lit.value();
                seed_refs.push(quote! { #value.as_bytes() });
            }
            SeedElement::Expression(expr) => {
                // Handle byte string literals: b"seed" -> use directly (no .as_bytes())
                if let syn::Expr::Lit(lit_expr) = &**expr {
                    if let syn::Lit::ByteStr(byte_str) = &lit_expr.lit {
                        let bytes = byte_str.value();
                        seed_refs.push(quote! { &[#(#bytes),*] });
                        continue;
                    }
                }

                // Handle uppercase constants
                if let syn::Expr::Path(path_expr) = &**expr {
                    if let Some(ident) = path_expr.path.get_ident() {
                        let ident_str = ident.to_string();
                        if ident_str.chars().all(|c| c.is_uppercase() || c == '_') {
                            seed_refs.push(
                                quote! { { let __seed: &[u8] = crate::#ident.as_ref(); __seed } },
                            );
                            continue;
                        }
                    }
                }

                let binding_name =
                    syn::Ident::new(&format!("seed_{}", i), proc_macro2::Span::call_site());
                let mapped_expr = map_pda_expr_to_ctx_seeds(expr, &ctx_field_names);
                bindings.push(quote! {
                    let #binding_name = #mapped_expr;
                });
                seed_refs.push(quote! { (#binding_name).as_ref() });
            }
        }
    }

    let indices: Vec<usize> = (0..seed_refs.len()).collect();

    Ok(quote! {
        #(#bindings)*
        let seeds: &[&[u8]] = &[#(#seed_refs,)*];
        let (pda, bump) = solana_pubkey::Pubkey::find_program_address(seeds, program_id);
        let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
        #(
            seeds_vec.push(seeds[#indices].to_vec());
        )*
        seeds_vec.push(vec![bump]);
        Ok((seeds_vec, pda))
    })
}

#[inline(never)]
fn generate_decompress_accounts_struct(
    required_accounts: &[String],
    variant: InstructionVariant,
) -> Result<syn::ItemStruct> {
    let mut account_fields = vec![
        quote! {
            #[account(mut)]
            pub fee_payer: Signer<'info>
        },
        quote! {
            /// CHECK: Checked by SDK
            pub config: AccountInfo<'info>
        },
    ];

    match variant {
        InstructionVariant::PdaOnly => {
            unreachable!()
        }
        InstructionVariant::TokenOnly => {
            unreachable!()
        }
        InstructionVariant::Mixed => {
            account_fields.extend(vec![
                quote! {
                    /// CHECK: anyone can pay
                    #[account(mut)]
                    pub rent_sponsor: UncheckedAccount<'info>
                },
                quote! {
                    /// CHECK: optional - only needed if decompressing tokens
                    #[account(mut)]
                    pub ctoken_rent_sponsor: Option<AccountInfo<'info>>
                },
            ]);
        }
    }

    match variant {
        InstructionVariant::TokenOnly => {
            unreachable!()
        }
        InstructionVariant::Mixed => {
            account_fields.extend(vec![
                quote! {
                    /// CHECK:
                    #[account(address = solana_pubkey::pubkey!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m"))]
                    pub ctoken_program: Option<UncheckedAccount<'info>>
                },
                quote! {
                    /// CHECK:
                    #[account(address = solana_pubkey::pubkey!("GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy"))]
                    pub ctoken_cpi_authority: Option<UncheckedAccount<'info>>
                },
                quote! {
                    /// CHECK: Checked by SDK
                    pub ctoken_config: Option<UncheckedAccount<'info>>
                },
            ]);
        }
        InstructionVariant::PdaOnly => {
            unreachable!()
        }
    }

    let standard_fields = [
        "fee_payer",
        "rent_sponsor",
        "ctoken_rent_sponsor",
        "config",
        "ctoken_program",
        "ctoken_cpi_authority",
        "ctoken_config",
    ];

    for account_name in required_accounts {
        if !standard_fields.contains(&account_name.as_str()) {
            let account_ident = syn::Ident::new(account_name, proc_macro2::Span::call_site());
            // Mark seed accounts as writable to support CPI calls that may need them writable
            account_fields.push(quote! {
                /// CHECK: optional seed account - may be used in CPIs
                #[account(mut)]
                pub #account_ident: Option<UncheckedAccount<'info>>
            });
        }
    }

    let struct_def = quote! {
        #[derive(Accounts)]
        pub struct DecompressAccountsIdempotent<'info> {
            #(#account_fields,)*
        }
    };

    syn::parse2(struct_def)
}

#[inline(never)]
fn validate_compressed_account_sizes(account_types: &[Ident]) -> Result<TokenStream> {
    let size_checks: Vec<_> = account_types.iter().map(|account_type| {
        quote! {
            const _: () = {
                const COMPRESSED_SIZE: usize = 8 + <#account_type as light_sdk::compressible::compression_info::CompressedInitSpace>::COMPRESSED_INIT_SPACE;
                if COMPRESSED_SIZE > 800 {
                    panic!(concat!(
                        "Compressed account '", stringify!(#account_type), "' exceeds 800-byte compressible account size limit. If you need support for larger accounts, send a message to team@lightprotocol.com"
                    ));
                }
            };
        }
    }).collect();

    Ok(quote! { #(#size_checks)* })
}

#[inline(never)]
fn generate_error_codes(variant: InstructionVariant) -> Result<TokenStream> {
    let base_errors = quote! {
            #[msg("Rent sponsor mismatch")]
            InvalidRentSponsor,
        #[msg("Missing seed account")]
        MissingSeedAccount,
        #[msg("Seed value does not match account data")]
        SeedMismatch,
    };

    let variant_specific_errors = match variant {
        InstructionVariant::PdaOnly => unreachable!(),
        InstructionVariant::TokenOnly => unreachable!(),
        InstructionVariant::Mixed => quote! {
            #[msg("Not implemented")]
            CTokenDecompressionNotImplemented,
            #[msg("Not implemented")]
            PdaDecompressionNotImplemented,
            #[msg("Not implemented")]
            TokenCompressionNotImplemented,
            #[msg("Not implemented")]
            PdaCompressionNotImplemented,
        },
    };

    Ok(quote! {
        #[error_code]
        pub enum CompressibleInstructionError {
            #base_errors
            #variant_specific_errors
        }
    })
}

/// Convert ClassifiedSeed to SeedElement (Punctuated)
fn convert_classified_to_seed_elements(
    seeds: &[crate::compressible::anchor_seeds::ClassifiedSeed],
) -> Punctuated<SeedElement, Token![,]> {
    use crate::compressible::anchor_seeds::ClassifiedSeed;
    
    let mut result = Punctuated::new();
    for seed in seeds {
        let elem = match seed {
            ClassifiedSeed::Literal(bytes) => {
                // Convert to string literal
                if let Ok(s) = std::str::from_utf8(bytes) {
                    SeedElement::Literal(syn::LitStr::new(s, proc_macro2::Span::call_site()))
                } else {
                    // Byte array - use expression
                    let byte_values: Vec<_> = bytes.iter().map(|b| quote!(#b)).collect();
                    let expr: Expr = syn::parse_quote!(&[#(#byte_values),*]);
                    SeedElement::Expression(Box::new(expr))
                }
            }
            ClassifiedSeed::Constant(path) => {
                let expr: Expr = syn::parse_quote!(#path);
                SeedElement::Expression(Box::new(expr))
            }
            ClassifiedSeed::CtxAccount(ident) => {
                let expr: Expr = syn::parse_quote!(ctx.#ident);
                SeedElement::Expression(Box::new(expr))
            }
            ClassifiedSeed::DataField { field_name, conversion: None } => {
                let expr: Expr = syn::parse_quote!(data.#field_name);
                SeedElement::Expression(Box::new(expr))
            }
            ClassifiedSeed::DataField { field_name, conversion: Some(method) } => {
                let expr: Expr = syn::parse_quote!(data.#field_name.#method());
                SeedElement::Expression(Box::new(expr))
            }
            ClassifiedSeed::FunctionCall { func, ctx_args } => {
                let args: Vec<Expr> = ctx_args.iter().map(|arg| {
                    syn::parse_quote!(&ctx.#arg.key())
                }).collect();
                let expr: Expr = syn::parse_quote!(#func(#(#args),*));
                SeedElement::Expression(Box::new(expr))
            }
        };
        result.push(elem);
    }
    result
}

fn convert_classified_to_seed_elements_vec(
    seeds: &[crate::compressible::anchor_seeds::ClassifiedSeed],
) -> Vec<SeedElement> {
    convert_classified_to_seed_elements(seeds).into_iter().collect()
}

/// Generate all code from extracted seeds (shared logic with add_compressible_instructions)
#[inline(never)]
fn generate_from_extracted_seeds(
    module: &mut ItemMod,
    account_types: Vec<Ident>,
    pda_seeds: Option<Vec<TokenSeedSpec>>,
    token_seeds: Option<Vec<TokenSeedSpec>>,
    instruction_data: Vec<InstructionDataSpec>,
) -> Result<TokenStream> {
    let size_validation_checks = validate_compressed_account_sizes(&account_types)?;

    let content = module.content.as_mut().unwrap();
    let ctoken_enum = if let Some(ref token_seed_specs) = token_seeds {
        if !token_seed_specs.is_empty() {
            crate::compressible::seed_providers::generate_ctoken_account_variant_enum(
                token_seed_specs,
            )?
        } else {
            crate::compressible::utils::generate_empty_ctoken_enum()
        }
    } else {
        crate::compressible::utils::generate_empty_ctoken_enum()
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

    let pda_ctx_seeds: Vec<crate::compressible::variant_enum::PdaCtxSeedInfo> = pda_seeds
        .as_ref()
        .map(|specs| {
            specs
                .iter()
                .map(|spec| {
                    let ctx_fields = extract_ctx_seed_fields(&spec.seeds);
                    crate::compressible::variant_enum::PdaCtxSeedInfo::new(
                        spec.variant.clone(),
                        ctx_fields,
                    )
                })
                .collect()
        })
        .unwrap_or_default();

    let account_type_refs: Vec<&Ident> = account_types.iter().collect();
    let enum_and_traits =
        crate::compressible::variant_enum::compressed_account_variant_with_ctx_seeds(
            &account_type_refs,
            &pda_ctx_seeds,
        )?;

    let seed_params_struct = quote! {
        #[derive(anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize, Clone, Debug, Default)]
        pub struct SeedParams;
    };

    let instruction_data_types: std::collections::HashMap<String, &syn::Type> = instruction_data
        .iter()
        .map(|spec| (spec.field_name.to_string(), &spec.field_type))
        .collect();

    let seeds_structs_and_constructors: Vec<TokenStream> = if let Some(ref pda_seed_specs) = pda_seeds {
        pda_seed_specs
            .iter()
            .zip(pda_ctx_seeds.iter())
            .map(|(spec, ctx_info)| {
                let type_name = &spec.variant;
                let seeds_struct_name = format_ident!("{}Seeds", type_name);
                let constructor_name = format_ident!("{}", to_snake_case(&type_name.to_string()));
                
                let ctx_fields = &ctx_info.ctx_seed_fields;
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
                
                let data_verifications: Vec<_> = data_fields.iter().map(|field| {
                    quote! {
                        if data.#field != seeds.#field {
                            return std::result::Result::Err(CompressibleInstructionError::SeedMismatch.into());
                        }
                    }
                }).collect();
                
                quote! {
                    #[derive(Clone, Debug)]
                    pub struct #seeds_struct_name {
                        #(#ctx_field_decls,)*
                        #(#data_field_decls,)*
                    }
                    
                    impl RentFreeAccountVariant {
                        pub fn #constructor_name(
                            account_data: &[u8],
                            seeds: #seeds_struct_name,
                        ) -> std::result::Result<Self, anchor_lang::error::Error> {
                            use anchor_lang::AnchorDeserialize;
                            let data = #type_name::deserialize(&mut &account_data[..])?;
                            
                            #(#data_verifications)*
                            
                            std::result::Result::Ok(Self::#type_name {
                                data,
                                #(#ctx_fields: seeds.#ctx_fields,)*
                            })
                        }
                    }
                    
                    impl light_sdk::compressible::IntoVariant<RentFreeAccountVariant> for #seeds_struct_name {
                        fn into_variant(self, data: &[u8]) -> std::result::Result<RentFreeAccountVariant, anchor_lang::error::Error> {
                            RentFreeAccountVariant::#constructor_name(data, self)
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

    let error_codes = generate_error_codes(instruction_variant)?;
    let decompress_accounts = generate_decompress_accounts_struct(&[], instruction_variant)?;

    let pda_seed_provider_impls: Result<Vec<_>> = account_types
        .iter()
        .zip(pda_ctx_seeds.iter())
        .map(|(name, ctx_info)| {
            let name_str = name.to_string();
            let spec = if let Some(ref pda_seed_specs) = pda_seeds {
                pda_seed_specs
                    .iter()
                    .find(|s| s.variant == name_str)
                    .ok_or_else(|| {
                        macro_error!(name, "No seed specification for account type '{}'", name_str)
                    })?
            } else {
                return Err(macro_error!(name, "No seed specifications provided"));
            };
            
            let ctx_seeds_struct_name = format_ident!("{}CtxSeeds", name);
            let ctx_fields = &ctx_info.ctx_seed_fields;
            let ctx_fields_decl: Vec<_> = ctx_fields.iter().map(|field| {
                quote! { pub #field: solana_pubkey::Pubkey }
            }).collect();
            
            let ctx_seeds_struct = if ctx_fields.is_empty() {
                quote! {
                    #[derive(Default)]
                    pub struct #ctx_seeds_struct_name;
                }
            } else {
                quote! {
                    #[derive(Default)]
                    pub struct #ctx_seeds_struct_name {
                        #(#ctx_fields_decl),*
                    }
                }
            };
            
            let seed_derivation = generate_pda_seed_derivation_for_trait_with_ctx_seeds(spec, &instruction_data, ctx_fields)?;
            Ok(quote! {
                #ctx_seeds_struct
                
                impl light_sdk::compressible::PdaSeedDerivation<#ctx_seeds_struct_name, ()> for #name {
                    fn derive_pda_seeds_with_accounts(
                        &self,
                        program_id: &solana_pubkey::Pubkey,
                        ctx_seeds: &#ctx_seeds_struct_name,
                        _seed_params: &(),
                    ) -> std::result::Result<(Vec<Vec<u8>>, solana_pubkey::Pubkey), solana_program_error::ProgramError> {
                        #seed_derivation
                    }
                }
            })
        })
        .collect();
    let pda_seed_provider_impls = pda_seed_provider_impls?;

    let trait_impls: syn::ItemMod = syn::parse_quote! {
        mod __trait_impls {
            use super::*;

            impl light_sdk::compressible::HasTokenVariant for RentFreeAccountData {
                fn is_packed_ctoken(&self) -> bool {
                    matches!(self.data, RentFreeAccountVariant::PackedCTokenData(_))
                }
            }
        }
    };

    let token_variant_name = format_ident!("CTokenAccountVariant");

    let decompress_context_impl = generate_decompress_context_impl(
        instruction_variant,
        pda_ctx_seeds.clone(),
        token_variant_name,
    )?;
    let decompress_processor_fn = generate_process_decompress_accounts_idempotent(instruction_variant, &instruction_data)?;
    let decompress_instruction = generate_decompress_instruction_entrypoint(instruction_variant, &instruction_data)?;

    let compress_accounts: syn::ItemStruct = match instruction_variant {
        InstructionVariant::PdaOnly => unreachable!(),
        InstructionVariant::TokenOnly => unreachable!(),
        InstructionVariant::Mixed => syn::parse_quote! {
            #[derive(Accounts)]
            pub struct CompressAccountsIdempotent<'info> {
                #[account(mut)]
                pub fee_payer: Signer<'info>,
                /// CHECK: Checked by SDK
                pub config: AccountInfo<'info>,
                /// CHECK: Checked by SDK
                #[account(mut)]
                pub rent_sponsor: AccountInfo<'info>,
                /// CHECK: Checked by SDK
                #[account(mut)]
                pub compression_authority: AccountInfo<'info>,
            }
        },
    };

    let compress_context_impl = generate_compress_context_impl(instruction_variant, account_types.clone())?;
    let compress_processor_fn = generate_process_compress_accounts_idempotent(instruction_variant)?;
    let compress_instruction = generate_compress_instruction_entrypoint(instruction_variant)?;

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
            light_sdk::compressible::process_initialize_compression_config_checked(
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
            light_sdk::compressible::process_update_compression_config(
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

    let client_functions = crate::compressible::seed_providers::generate_client_seed_functions(
        &account_types,
        &pda_seeds,
        &token_seeds,
        &instruction_data,
    )?;

    // Insert SeedParams struct
    let seed_params_item: Item = syn::parse2(seed_params_struct)?;
    content.1.push(seed_params_item);

    // Insert XxxSeeds structs and RentFreeAccountVariant constructors
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
            let impl_code = crate::compressible::seed_providers::generate_ctoken_seed_provider_implementation(seeds)?;
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
// COMPRESSIBLE_PROGRAM: Auto-discovers seeds from external module files
// =============================================================================

/// Main entry point for #[compressible_program] macro.
///
/// This macro reads external module files to extract seed information from
/// Accounts structs with #[compressible] fields. No explicit type list needed!
///
/// Usage:
/// ```ignore
/// #[compressible_program]
/// #[program]
/// pub mod my_program {
///     pub mod instruction_accounts;  // Macro reads this file!
///     pub mod state;
///     
///     use instruction_accounts::*;
///     use state::*;
///     
///     #[light_instruction]
///     pub fn create_user(ctx: Context<CreateUser>, params: Params) -> Result<()> {
///         // ...
///     }
/// }
/// ```
#[inline(never)]
pub fn compressible_program_impl(
    _args: TokenStream,
    mut module: ItemMod,
) -> Result<TokenStream> {
    use crate::compressible::anchor_seeds::get_data_fields;
    use crate::compressible::file_scanner::{resolve_crate_src_path, scan_module_for_compressible};

    if module.content.is_none() {
        return Err(macro_error!(&module, "Module must have a body"));
    }

    // Resolve the crate's src/ directory
    let base_path = resolve_crate_src_path();

    // Scan the module (and external files) for compressible fields
    let scanned = scan_module_for_compressible(&module, &base_path)?;

    // Report any errors from file scanning
    if !scanned.errors.is_empty() {
        let error_msg = scanned.errors.join("\n");
        return Err(macro_error!(
            &module,
            "Errors while scanning for rentfree types:\n{}",
            error_msg
        ));
    }

    // Check if we found anything
    if scanned.pda_specs.is_empty() && scanned.token_specs.is_empty() {
        return Err(macro_error!(
            &module,
            "No #[rentfree] or #[rentfree_token] fields found in any Accounts struct.\n\
             Ensure your Accounts structs are in modules declared with `pub mod xxx;`"
        ));
    }

    // Convert extracted specs to the format expected by generate_from_extracted_seeds
    let mut found_pda_seeds: Vec<TokenSeedSpec> = Vec::new();
    let mut found_data_fields: Vec<InstructionDataSpec> = Vec::new();
    let mut account_types: Vec<Ident> = Vec::new();

    for pda in &scanned.pda_specs {
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
            variant: pda.inner_type.clone(),
            _eq: syn::parse_quote!(=),
            is_token: Some(false),
            seeds: seed_elements,
            authority: None,
        });
    }

    // Convert token specs
    let mut found_token_seeds: Vec<TokenSeedSpec> = Vec::new();
    for token in &scanned.token_specs {
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
    generate_from_extracted_seeds(
        &mut module,
        account_types,
        pda_seeds,
        token_seeds,
        found_data_fields,
    )
}
