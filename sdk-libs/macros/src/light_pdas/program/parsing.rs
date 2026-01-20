//! Parsing types, expression analysis, seed conversion, and function wrapping.

use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Expr, Ident, ItemFn, LitStr, Result, Token,
};

use super::visitors::FieldExtractor;

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
    /// The variant name (derived from field name, used for enum variant naming)
    pub variant: Ident,
    pub _eq: Token![=],
    pub is_token: Option<bool>,
    pub seeds: Punctuated<SeedElement, Token![,]>,
    pub authority: Option<Vec<SeedElement>>,
    /// The inner type (e.g., crate::state::SinglePubkeyRecord - used for type references)
    /// Preserves the full type path for code generation.
    /// Only set for PDAs extracted from #[light_account(init)] fields; None for parsed specs
    pub inner_type: Option<syn::Type>,
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
            inner_type: None, // Set by caller for #[light_account(init)] fields
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

#[derive(Clone, Debug)]
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

/// Extract all ctx.accounts.XXX and ctx.XXX field names from a list of seed elements.
/// Deduplicates the fields using visitor-based extraction.
pub fn extract_ctx_seed_fields(
    seeds: &syn::punctuated::Punctuated<SeedElement, Token![,]>,
) -> Vec<Ident> {
    let mut all_fields = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for seed in seeds {
        if let SeedElement::Expression(expr) = seed {
            let fields = FieldExtractor::ctx_fields(&[]).extract(expr);
            for field in fields {
                let name = field.to_string();
                if seen.insert(name) {
                    all_fields.push(field);
                }
            }
        }
    }

    all_fields
}

/// Extract all data.XXX field names from a list of seed elements.
/// Deduplicates the fields using visitor-based extraction.
pub fn extract_data_seed_fields(
    seeds: &syn::punctuated::Punctuated<SeedElement, Token![,]>,
) -> Vec<Ident> {
    let mut all_fields = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for seed in seeds {
        if let SeedElement::Expression(expr) = seed {
            let fields = FieldExtractor::data_fields().extract(expr);
            for field in fields {
                let name = field.to_string();
                if seen.insert(name) {
                    all_fields.push(field);
                }
            }
        }
    }

    all_fields
}

// =============================================================================
// SEED CONVERSION
// =============================================================================

/// Convert ClassifiedSeed to SeedElement (Punctuated)
pub fn convert_classified_to_seed_elements(
    seeds: &[crate::light_pdas::account::seed_extraction::ClassifiedSeed],
) -> Punctuated<SeedElement, Token![,]> {
    use crate::light_pdas::account::seed_extraction::ClassifiedSeed;

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
    seeds: &[crate::light_pdas::account::seed_extraction::ClassifiedSeed],
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

/// Check if a function body is a simple delegation (single expression that moves ctx).
/// Returns true for patterns like `crate::module::function(ctx, params)`.
/// Does NOT match simple returns like `Ok(())` since those don't consume ctx.
fn is_delegation_body(block: &syn::Block) -> bool {
    // Check if block has exactly one statement that's an expression
    if block.stmts.len() != 1 {
        return false;
    }
    match &block.stmts[0] {
        syn::Stmt::Expr(expr, _) => {
            // Check if it's a function call that takes ctx as an argument
            match expr {
                syn::Expr::Call(call) => call_has_ctx_arg(&call.args),
                syn::Expr::MethodCall(call) => call_has_ctx_arg(&call.args),
                _ => false,
            }
        }
        _ => false,
    }
}

/// Check if any argument in the call is `ctx` (moving the context).
/// Detects: ctx, &ctx, &mut ctx, ctx.clone(), ctx.into(), etc.
fn call_has_ctx_arg(args: &syn::punctuated::Punctuated<syn::Expr, syn::token::Comma>) -> bool {
    for arg in args {
        match arg {
            // Direct ctx identifier
            syn::Expr::Path(path) if path.path.is_ident("ctx") => return true,
            // Reference patterns: &ctx, &mut ctx
            syn::Expr::Reference(ref_expr) => {
                if let syn::Expr::Path(p) = &*ref_expr.expr {
                    if p.path.is_ident("ctx") {
                        return true;
                    }
                }
            }
            // Method call patterns: ctx.clone(), ctx.into()
            syn::Expr::MethodCall(method_call) => {
                if let syn::Expr::Path(p) = &*method_call.receiver {
                    if p.path.is_ident("ctx") {
                        return true;
                    }
                }
            }
            _ => {}
        }
    }
    false
}

/// Wrap a function with pre_init/finalize logic.
pub fn wrap_function_with_light(fn_item: &ItemFn, params_ident: &Ident) -> ItemFn {
    let fn_vis = &fn_item.vis;
    let fn_sig = &fn_item.sig;
    let fn_block = &fn_item.block;
    let fn_attrs = &fn_item.attrs;

    // Check if this handler delegates to another function (which moves ctx)
    // In that case, skip finalize since the delegated function handles everything
    let is_delegation = is_delegation_body(fn_block);

    if is_delegation {
        // For delegation handlers, just add pre_init before the delegation call
        syn::parse_quote! {
            #(#fn_attrs)*
            #fn_vis #fn_sig {
                // Phase 1: Pre-init (creates mints via CPI context write, registers compressed addresses)
                use light_sdk::interface::{LightPreInit, LightFinalize};
                let _ = ctx.accounts.light_pre_init(ctx.remaining_accounts, &#params_ident)
                    .map_err(|e: light_sdk::error::LightSdkError| -> solana_program_error::ProgramError {
                        e.into()
                    })?;

                // Execute delegation - this handles its own logic including any finalize
                #fn_block
            }
        }
    } else {
        // For non-delegation handlers, add both pre_init and finalize
        syn::parse_quote! {
            #(#fn_attrs)*
            #fn_vis #fn_sig {
                // Phase 1: Pre-init (creates mints via CPI context write, registers compressed addresses)
                use light_sdk::interface::{LightPreInit, LightFinalize};
                let __has_pre_init = ctx.accounts.light_pre_init(ctx.remaining_accounts, &#params_ident)
                    .map_err(|e: light_sdk::error::LightSdkError| -> solana_program_error::ProgramError {
                        e.into()
                    })?;

                // Execute the original handler body and capture result
                let __user_result: anchor_lang::Result<()> = #fn_block;
                // Propagate any errors from user code
                __user_result?;

                // Phase 2: Finalize (creates token accounts/ATAs via CPI)
                ctx.accounts.light_finalize(ctx.remaining_accounts, &#params_ident, __has_pre_init)
                    .map_err(|e: light_sdk::error::LightSdkError| -> solana_program_error::ProgramError {
                        e.into()
                    })?;

                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::punctuated::Punctuated;

    use super::*;

    fn parse_args(code: &str) -> Punctuated<syn::Expr, syn::token::Comma> {
        let call: syn::ExprCall = syn::parse_str(&format!("f({})", code)).unwrap();
        call.args
    }

    #[test]
    fn test_call_has_ctx_arg_direct() {
        // F001: Direct ctx identifier
        let args = parse_args("ctx");
        assert!(call_has_ctx_arg(&args));
    }

    #[test]
    fn test_call_has_ctx_arg_reference() {
        // F001: Reference pattern &ctx
        let args = parse_args("&ctx");
        assert!(call_has_ctx_arg(&args));
    }

    #[test]
    fn test_call_has_ctx_arg_mut_reference() {
        // F001: Mutable reference pattern &mut ctx
        let args = parse_args("&mut ctx");
        assert!(call_has_ctx_arg(&args));
    }

    #[test]
    fn test_call_has_ctx_arg_clone() {
        // F001: Method call ctx.clone()
        let args = parse_args("ctx.clone()");
        assert!(call_has_ctx_arg(&args));
    }

    #[test]
    fn test_call_has_ctx_arg_into() {
        // F001: Method call ctx.into()
        let args = parse_args("ctx.into()");
        assert!(call_has_ctx_arg(&args));
    }

    #[test]
    fn test_call_has_ctx_arg_other_name() {
        // Non-ctx identifier should return false
        let args = parse_args("context");
        assert!(!call_has_ctx_arg(&args));
    }

    #[test]
    fn test_call_has_ctx_arg_method_on_other() {
        // Method call on non-ctx receiver
        let args = parse_args("other.clone()");
        assert!(!call_has_ctx_arg(&args));
    }

    #[test]
    fn test_call_has_ctx_arg_multiple_args() {
        // F001: ctx among multiple arguments
        let args = parse_args("foo, ctx.clone(), bar");
        assert!(call_has_ctx_arg(&args));
    }

    #[test]
    fn test_call_has_ctx_arg_empty() {
        // Empty args should return false
        let args = parse_args("");
        assert!(!call_has_ctx_arg(&args));
    }
}
