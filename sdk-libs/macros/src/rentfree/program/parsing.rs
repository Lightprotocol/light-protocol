//! Parsing types, expression analysis, seed conversion, and function wrapping.

use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Expr, Ident, ItemFn, LitStr, Result, Token,
};

// =============================================================================
// MACRO ERROR HELPER
// =============================================================================

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

pub(crate) use macro_error;

// =============================================================================
// CORE TYPES
// =============================================================================

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

// =============================================================================
// EXPRESSION ANALYSIS
// =============================================================================

/// Recursively extract field names from expressions matching `base.field` or `base.nested.field`.
/// Handles nested expressions like function calls: max_key(&ctx.user.key(), &ctx.authority.key())
///
/// Parameters:
/// - `base_ident`: The base identifier to match (e.g., "ctx" or "data")
/// - `nested_prefix`: Optional nested field name (e.g., "accounts" for ctx.accounts.XXX)
fn extract_fields_by_base(
    expr: &syn::Expr,
    base_ident: &str,
    nested_prefix: Option<&str>,
    fields: &mut Vec<Ident>,
) {
    match expr {
        syn::Expr::Field(field_expr) => {
            if let syn::Member::Named(field_name) = &field_expr.member {
                // Check for base.XXX pattern (direct field access)
                if let syn::Expr::Path(path) = &*field_expr.base {
                    if let Some(segment) = path.path.segments.first() {
                        if segment.ident == base_ident {
                            fields.push(field_name.clone());
                            return;
                        }
                    }
                }
                // Check for base.nested.XXX pattern (nested field access) if nested_prefix is provided
                if let Some(nested) = nested_prefix {
                    if let syn::Expr::Field(nested_field) = &*field_expr.base {
                        if let syn::Member::Named(base_name) = &nested_field.member {
                            if base_name == nested {
                                if let syn::Expr::Path(path) = &*nested_field.base {
                                    if let Some(segment) = path.path.segments.first() {
                                        if segment.ident == base_ident {
                                            fields.push(field_name.clone());
                                            return;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Recurse into base expression
            extract_fields_by_base(&field_expr.base, base_ident, nested_prefix, fields);
        }
        syn::Expr::MethodCall(method) => {
            // Recurse into receiver and args
            extract_fields_by_base(&method.receiver, base_ident, nested_prefix, fields);
            for arg in &method.args {
                extract_fields_by_base(arg, base_ident, nested_prefix, fields);
            }
        }
        syn::Expr::Call(call) => {
            // Recurse into function args
            for arg in &call.args {
                extract_fields_by_base(arg, base_ident, nested_prefix, fields);
            }
        }
        syn::Expr::Reference(ref_expr) => {
            extract_fields_by_base(&ref_expr.expr, base_ident, nested_prefix, fields);
        }
        syn::Expr::Paren(paren) => {
            extract_fields_by_base(&paren.expr, base_ident, nested_prefix, fields);
        }
        _ => {}
    }
}

/// Recursively extract all ctx.XXX or ctx.accounts.XXX field names from an expression.
fn extract_ctx_fields_from_expr(expr: &syn::Expr, fields: &mut Vec<Ident>) {
    extract_fields_by_base(expr, "ctx", Some("accounts"), fields);
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
pub fn extract_ctx_seed_fields(
    seeds: &syn::punctuated::Punctuated<SeedElement, Token![,]>,
) -> Vec<Ident> {
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

/// Extract data.XXX field names from an expression recursively.
fn extract_data_fields_from_expr(expr: &syn::Expr, fields: &mut Vec<Ident>) {
    extract_fields_by_base(expr, "data", None, fields);
}

/// Extract all data.XXX field names from a list of seed elements.
pub fn extract_data_seed_fields(
    seeds: &syn::punctuated::Punctuated<SeedElement, Token![,]>,
) -> Vec<Ident> {
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

// =============================================================================
// SEED CONVERSION
// =============================================================================

/// Convert ClassifiedSeed to SeedElement (Punctuated)
pub fn convert_classified_to_seed_elements(
    seeds: &[crate::rentfree::traits::anchor_seeds::ClassifiedSeed],
) -> Punctuated<SeedElement, Token![,]> {
    use crate::rentfree::traits::anchor_seeds::ClassifiedSeed;

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
            ClassifiedSeed::DataField {
                field_name,
                conversion: None,
            } => {
                let expr: Expr = syn::parse_quote!(data.#field_name);
                SeedElement::Expression(Box::new(expr))
            }
            ClassifiedSeed::DataField {
                field_name,
                conversion: Some(method),
            } => {
                let expr: Expr = syn::parse_quote!(data.#field_name.#method());
                SeedElement::Expression(Box::new(expr))
            }
            ClassifiedSeed::FunctionCall { func, ctx_args } => {
                let args: Vec<Expr> = ctx_args
                    .iter()
                    .map(|arg| syn::parse_quote!(&ctx.#arg.key()))
                    .collect();
                let expr: Expr = syn::parse_quote!(#func(#(#args),*));
                SeedElement::Expression(Box::new(expr))
            }
        };
        result.push(elem);
    }
    result
}

pub fn convert_classified_to_seed_elements_vec(
    seeds: &[crate::rentfree::traits::anchor_seeds::ClassifiedSeed],
) -> Vec<SeedElement> {
    convert_classified_to_seed_elements(seeds)
        .into_iter()
        .collect()
}

// =============================================================================
// FUNCTION WRAPPING
// =============================================================================

/// Extract the Context<T> type name from a function's parameters.
/// Returns (struct_name, params_ident) if found.
pub fn extract_context_and_params(fn_item: &ItemFn) -> Option<(String, Ident)> {
    let mut context_type = None;
    let mut params_ident = None;

    for input in &fn_item.sig.inputs {
        if let syn::FnArg::Typed(pat_type) = input {
            if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                // Check if this is a Context<T> parameter
                if let syn::Type::Path(type_path) = &*pat_type.ty {
                    if let Some(segment) = type_path.path.segments.last() {
                        if segment.ident == "Context" {
                            // Extract T from Context<'_, '_, '_, 'info, T<'info>> or Context<T>
                            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                                // Find the last type argument (T or T<'info>)
                                for arg in args.args.iter().rev() {
                                    if let syn::GenericArgument::Type(syn::Type::Path(inner_path)) =
                                        arg
                                    {
                                        if let Some(inner_seg) = inner_path.path.segments.last() {
                                            context_type = Some(inner_seg.ident.to_string());
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Track potential params argument (not ctx, not signer-like names)
                let name = pat_ident.ident.to_string();
                if name != "ctx" && !name.contains("signer") && !name.contains("bump") {
                    // Prefer "params" but accept others
                    if name == "params" || params_ident.is_none() {
                        params_ident = Some(pat_ident.ident.clone());
                    }
                }
            }
        }
    }

    match (context_type, params_ident) {
        (Some(ctx), Some(params)) => Some((ctx, params)),
        _ => None,
    }
}

/// Wrap a function with pre_init/finalize logic.
pub fn wrap_function_with_rentfree(fn_item: &ItemFn, params_ident: &Ident) -> ItemFn {
    let fn_vis = &fn_item.vis;
    let fn_sig = &fn_item.sig;
    let fn_block = &fn_item.block;
    let fn_attrs = &fn_item.attrs;

    let wrapped: ItemFn = syn::parse_quote! {
        #(#fn_attrs)*
        #fn_vis #fn_sig {
            // Phase 1: Pre-init (creates mints via CPI context write, registers compressed addresses)
            use light_sdk::compressible::{LightPreInit, LightFinalize};
            let __has_pre_init = ctx.accounts.light_pre_init(ctx.remaining_accounts, &#params_ident)
                .map_err(|e| {
                    let pe: solana_program_error::ProgramError = e.into();
                    pe
                })?;

            // Execute the original handler body in a closure
            let __light_handler_result = (|| #fn_block)();

            // Phase 2: On success, finalize compression
            if __light_handler_result.is_ok() {
                ctx.accounts.light_finalize(ctx.remaining_accounts, &#params_ident, __has_pre_init)
                    .map_err(|e| {
                        let pe: solana_program_error::ProgramError = e.into();
                        pe
                    })?;
            }

            __light_handler_result
        }
    };

    wrapped
}
