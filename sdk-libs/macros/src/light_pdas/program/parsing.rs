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
    /// Programs that only create Light mints without compressed state accounts
    MintOnly,
    /// Programs that only create Light ATAs without compressed state accounts
    AtaOnly,
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
    /// True if the field uses zero-copy serialization (AccountLoader).
    /// Only set for PDAs extracted from #[light_account(init, zero_copy)] fields; false by default.
    pub is_zero_copy: bool,
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
            inner_type: None,   // Set by caller for #[light_account(init)] fields
            is_zero_copy: false, // Set by caller for #[light_account(init, zero_copy)] fields
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

/// Convert ClassifiedSeed to SeedElement (Punctuated).
///
/// Produces simplified expressions for downstream processing:
/// - CtxRooted: generates `ctx.account` (not the full expression)
/// - DataRooted: generates `data.field` with optional conversion method
/// - Passthrough: uses expression as-is (for complex patterns)
pub fn convert_classified_to_seed_elements(
    seeds: &[crate::light_pdas::account::seed_extraction::ClassifiedSeed],
) -> Punctuated<SeedElement, Token![,]> {
    use crate::light_pdas::account::seed_extraction::{extract_data_field_info, ClassifiedSeed};

    let mut result = Punctuated::new();
    for seed in seeds {
        let elem = match seed {
            ClassifiedSeed::Literal(bytes) => {
                // Convert to string literal if valid UTF-8
                if let Ok(s) = std::str::from_utf8(bytes) {
                    SeedElement::Literal(syn::LitStr::new(s, proc_macro2::Span::call_site()))
                } else {
                    // Non-UTF8 byte array - use expression
                    let byte_values: Vec<_> = bytes.iter().map(|b| quote!(#b)).collect();
                    let expr: Expr = syn::parse_quote!(&[#(#byte_values),*]);
                    SeedElement::Expression(Box::new(expr))
                }
            }
            ClassifiedSeed::Constant(path) => {
                let expr: Expr = syn::parse_quote!(#path);
                SeedElement::Expression(Box::new(expr))
            }
            ClassifiedSeed::CtxRooted { account, .. } => {
                // Generate simplified ctx.account expression
                let expr: Expr = syn::parse_quote!(ctx.#account);
                SeedElement::Expression(Box::new(expr))
            }
            ClassifiedSeed::DataRooted { expr, .. } => {
                // Extract the field name and optional conversion method
                if let Some((field_name, conversion)) = extract_data_field_info(expr) {
                    let expr: Expr = if let Some(method) = conversion {
                        syn::parse_quote!(data.#field_name.#method())
                    } else {
                        syn::parse_quote!(data.#field_name)
                    };
                    SeedElement::Expression(Box::new(expr))
                } else {
                    // Fallback: pass through as-is
                    SeedElement::Expression(expr.clone())
                }
            }
            ClassifiedSeed::Passthrough(expr) => {
                SeedElement::Expression(expr.clone())
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

/// Result from extracting context and params from a function signature.
pub enum ExtractResult {
    /// Successfully extracted context type, params ident, and context ident
    Success {
        context_type: String,
        params_ident: Ident,
        ctx_ident: Ident,
    },
    /// Multiple params arguments detected (format-2 case) - caller decides if this is an error
    MultipleParams {
        context_type: String,
        param_names: Vec<String>,
    },
    /// No valid context/params combination found
    None,
}

/// Extract the Context<T> type name and context parameter name from a function's parameters.
/// Returns ExtractResult indicating success, multiple params, or none found.
/// The ctx_ident is the actual parameter name (e.g., "ctx", "context", "anchor_ctx").
pub fn extract_context_and_params(fn_item: &ItemFn) -> ExtractResult {
    let mut context_type = None;
    let mut ctx_ident = None;
    // Collect ALL potential params arguments to detect multi-arg cases
    let mut params_candidates: Vec<Ident> = Vec::new();

    for input in &fn_item.sig.inputs {
        if let syn::FnArg::Typed(pat_type) = input {
            if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                // Check if this is a Context<T> parameter
                if let syn::Type::Path(type_path) = &*pat_type.ty {
                    if let Some(segment) = type_path.path.segments.last() {
                        if segment.ident == "Context" {
                            // Capture the context parameter name (e.g., ctx, context, anchor_ctx)
                            ctx_ident = Some(pat_ident.ident.clone());

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
                            continue; // Don't consider ctx as params
                        }
                    }
                }

                // Track potential params argument (not the context param, not signer-like names)
                let name = pat_ident.ident.to_string();
                if !name.contains("signer") && !name.contains("bump") {
                    params_candidates.push(pat_ident.ident.clone());
                }
            }
        }
    }

    match (context_type, ctx_ident) {
        (Some(ctx_type), Some(ctx_name)) => {
            if params_candidates.len() > 1 {
                // Multiple params detected - let caller decide if this is an error
                ExtractResult::MultipleParams {
                    context_type: ctx_type,
                    param_names: params_candidates.iter().map(|id| id.to_string()).collect(),
                }
            } else if let Some(params) = params_candidates.into_iter().next() {
                ExtractResult::Success {
                    context_type: ctx_type,
                    params_ident: params,
                    ctx_ident: ctx_name,
                }
            } else {
                ExtractResult::None
            }
        }
        _ => ExtractResult::None,
    }
}

/// Check if a function body is a simple delegation (single expression that moves ctx).
/// Returns true for patterns like `crate::module::function(ctx, params)`.
/// Does NOT match simple returns like `Ok(())` since those don't consume ctx.
/// `ctx_name` is the context parameter name to look for (e.g., "ctx", "context").
fn is_delegation_body(block: &syn::Block, ctx_name: &str) -> bool {
    // Check if block has exactly one statement that's an expression
    if block.stmts.len() != 1 {
        return false;
    }
    match &block.stmts[0] {
        syn::Stmt::Expr(expr, _) => {
            // Check if it's a function call that takes ctx as an argument
            match expr {
                syn::Expr::Call(call) => call_has_ctx_arg(&call.args, ctx_name),
                syn::Expr::MethodCall(call) => call_has_ctx_arg(&call.args, ctx_name),
                _ => false,
            }
        }
        _ => false,
    }
}

/// Check if any argument in the call is the context param (moving the context).
/// Detects: ctx, &ctx, &mut ctx, ctx.clone(), ctx.into(), etc.
/// `ctx_name` is the context parameter name to look for (e.g., "ctx", "context").
fn call_has_ctx_arg(
    args: &syn::punctuated::Punctuated<syn::Expr, syn::token::Comma>,
    ctx_name: &str,
) -> bool {
    for arg in args {
        match arg {
            // Direct ctx identifier
            syn::Expr::Path(path) if path.path.is_ident(ctx_name) => return true,
            // Reference patterns: &ctx, &mut ctx
            syn::Expr::Reference(ref_expr) => {
                if let syn::Expr::Path(p) = &*ref_expr.expr {
                    if p.path.is_ident(ctx_name) {
                        return true;
                    }
                }
            }
            // Method call patterns: ctx.clone(), ctx.into()
            syn::Expr::MethodCall(method_call) => {
                if let syn::Expr::Path(p) = &*method_call.receiver {
                    if p.path.is_ident(ctx_name) {
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
/// `ctx_name` is the parameter name used for the Context (e.g., "ctx", "context", "anchor_ctx").
pub fn wrap_function_with_light(
    fn_item: &ItemFn,
    params_ident: &Ident,
    ctx_name: &Ident,
) -> ItemFn {
    let fn_vis = &fn_item.vis;
    let fn_sig = &fn_item.sig;
    let fn_block = &fn_item.block;
    let fn_attrs = &fn_item.attrs;

    // Check if this handler delegates to another function (which moves ctx)
    // In that case, skip finalize since the delegated function handles everything
    let ctx_name_str = ctx_name.to_string();
    let is_delegation = is_delegation_body(fn_block, &ctx_name_str);

    if is_delegation {
        // For delegation handlers, just add pre_init before the delegation call
        syn::parse_quote! {
            #(#fn_attrs)*
            #fn_vis #fn_sig {
                // Phase 1: Pre-init (creates mints via CPI context write, registers compressed addresses)
                use light_sdk::interface::{LightPreInit, LightFinalize};
                let _ = #ctx_name.accounts.light_pre_init(#ctx_name.remaining_accounts, &#params_ident)
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
                let __has_pre_init = #ctx_name.accounts.light_pre_init(#ctx_name.remaining_accounts, &#params_ident)
                    .map_err(|e: light_sdk::error::LightSdkError| -> solana_program_error::ProgramError {
                        e.into()
                    })?;

                // Execute the original handler body and capture result
                let __user_result: anchor_lang::Result<()> = #fn_block;
                // Propagate any errors from user code
                __user_result?;

                // Phase 2: Finalize (creates token accounts/ATAs via CPI)
                #ctx_name.accounts.light_finalize(#ctx_name.remaining_accounts, &#params_ident, __has_pre_init)
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
        assert!(call_has_ctx_arg(&args, "ctx"));
    }

    #[test]
    fn test_call_has_ctx_arg_reference() {
        // F001: Reference pattern &ctx
        let args = parse_args("&ctx");
        assert!(call_has_ctx_arg(&args, "ctx"));
    }

    #[test]
    fn test_call_has_ctx_arg_mut_reference() {
        // F001: Mutable reference pattern &mut ctx
        let args = parse_args("&mut ctx");
        assert!(call_has_ctx_arg(&args, "ctx"));
    }

    #[test]
    fn test_call_has_ctx_arg_clone() {
        // F001: Method call ctx.clone()
        let args = parse_args("ctx.clone()");
        assert!(call_has_ctx_arg(&args, "ctx"));
    }

    #[test]
    fn test_call_has_ctx_arg_into() {
        // F001: Method call ctx.into()
        let args = parse_args("ctx.into()");
        assert!(call_has_ctx_arg(&args, "ctx"));
    }

    #[test]
    fn test_call_has_ctx_arg_other_name() {
        // Non-ctx identifier should return false when looking for "ctx"
        let args = parse_args("context");
        assert!(!call_has_ctx_arg(&args, "ctx"));
    }

    #[test]
    fn test_call_has_ctx_arg_method_on_other() {
        // Method call on non-ctx receiver
        let args = parse_args("other.clone()");
        assert!(!call_has_ctx_arg(&args, "ctx"));
    }

    #[test]
    fn test_call_has_ctx_arg_multiple_args() {
        // F001: ctx among multiple arguments
        let args = parse_args("foo, ctx.clone(), bar");
        assert!(call_has_ctx_arg(&args, "ctx"));
    }

    #[test]
    fn test_call_has_ctx_arg_empty() {
        // Empty args should return false
        let args = parse_args("");
        assert!(!call_has_ctx_arg(&args, "ctx"));
    }

    // Tests for dynamic context name detection
    #[test]
    fn test_call_has_ctx_arg_custom_name_context() {
        // Direct identifier with custom name "context"
        let args = parse_args("context");
        assert!(call_has_ctx_arg(&args, "context"));
    }

    #[test]
    fn test_call_has_ctx_arg_custom_name_anchor_ctx() {
        // Direct identifier with custom name "anchor_ctx"
        let args = parse_args("anchor_ctx");
        assert!(call_has_ctx_arg(&args, "anchor_ctx"));
    }

    #[test]
    fn test_call_has_ctx_arg_custom_name_reference() {
        // Reference pattern with custom name
        let args = parse_args("&my_context");
        assert!(call_has_ctx_arg(&args, "my_context"));
    }

    #[test]
    fn test_call_has_ctx_arg_custom_name_method_call() {
        // Method call with custom name
        let args = parse_args("c.clone()");
        assert!(call_has_ctx_arg(&args, "c"));
    }

    #[test]
    fn test_call_has_ctx_arg_wrong_custom_name() {
        // Looking for wrong name should return false
        let args = parse_args("ctx");
        assert!(!call_has_ctx_arg(&args, "context"));
    }

    #[test]
    fn test_extract_context_and_params_standard() {
        let fn_item: syn::ItemFn = syn::parse_quote! {
            pub fn handler(ctx: Context<MyAccounts>, params: Params) -> Result<()> {
                Ok(())
            }
        };
        match extract_context_and_params(&fn_item) {
            ExtractResult::Success {
                context_type,
                params_ident,
                ctx_ident,
            } => {
                assert_eq!(context_type, "MyAccounts");
                assert_eq!(params_ident.to_string(), "params");
                assert_eq!(ctx_ident.to_string(), "ctx");
            }
            _ => panic!("Expected ExtractResult::Success"),
        }
    }

    #[test]
    fn test_extract_context_and_params_custom_context_name() {
        let fn_item: syn::ItemFn = syn::parse_quote! {
            pub fn handler(context: Context<MyAccounts>, params: Params) -> Result<()> {
                Ok(())
            }
        };
        match extract_context_and_params(&fn_item) {
            ExtractResult::Success {
                context_type,
                params_ident,
                ctx_ident,
            } => {
                assert_eq!(context_type, "MyAccounts");
                assert_eq!(params_ident.to_string(), "params");
                assert_eq!(ctx_ident.to_string(), "context");
            }
            _ => panic!("Expected ExtractResult::Success"),
        }
    }

    #[test]
    fn test_extract_context_and_params_anchor_ctx_name() {
        let fn_item: syn::ItemFn = syn::parse_quote! {
            pub fn handler(anchor_ctx: Context<MyAccounts>, data: Data) -> Result<()> {
                Ok(())
            }
        };
        match extract_context_and_params(&fn_item) {
            ExtractResult::Success {
                context_type,
                params_ident,
                ctx_ident,
            } => {
                assert_eq!(context_type, "MyAccounts");
                assert_eq!(params_ident.to_string(), "data");
                assert_eq!(ctx_ident.to_string(), "anchor_ctx");
            }
            _ => panic!("Expected ExtractResult::Success"),
        }
    }

    #[test]
    fn test_extract_context_and_params_single_letter_name() {
        let fn_item: syn::ItemFn = syn::parse_quote! {
            pub fn handler(c: Context<MyAccounts>, p: Params) -> Result<()> {
                Ok(())
            }
        };
        match extract_context_and_params(&fn_item) {
            ExtractResult::Success {
                context_type,
                params_ident,
                ctx_ident,
            } => {
                assert_eq!(context_type, "MyAccounts");
                assert_eq!(params_ident.to_string(), "p");
                assert_eq!(ctx_ident.to_string(), "c");
            }
            _ => panic!("Expected ExtractResult::Success"),
        }
    }

    #[test]
    fn test_extract_context_and_params_multiple_args_detected() {
        // Format-2 case: multiple instruction arguments should be detected
        let fn_item: syn::ItemFn = syn::parse_quote! {
            pub fn handler(ctx: Context<MyAccounts>, amount: u64, owner: Pubkey) -> Result<()> {
                Ok(())
            }
        };
        match extract_context_and_params(&fn_item) {
            ExtractResult::MultipleParams {
                context_type,
                param_names,
            } => {
                assert_eq!(context_type, "MyAccounts");
                assert!(param_names.contains(&"amount".to_string()));
                assert!(param_names.contains(&"owner".to_string()));
            }
            _ => panic!("Expected ExtractResult::MultipleParams"),
        }
    }
}
