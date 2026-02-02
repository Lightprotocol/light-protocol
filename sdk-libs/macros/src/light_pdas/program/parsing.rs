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
    /// Owner PDA seeds - used when the token owner is a PDA that needs to sign.
    /// Must contain only constant values (byte literals, const references).
    pub owner_seeds: Option<Vec<SeedElement>>,
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
        //   Token: TypeName = (is_token, seeds = (...), owner_seeds = (...))
        let mut is_token = None;
        let mut seeds = Punctuated::new();
        let mut owner_seeds = None;

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
                    "owner_seeds" => {
                        let _eq: Token![=] = content.parse()?;
                        owner_seeds = Some(parse_owner_seeds(&content)?);
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            &ident,
                            format!(
                                "Unknown keyword '{}'. Expected: is_token, seeds, or owner_seeds.\n\
                                 Use explicit syntax: TypeName = (seeds = (\"seed\", ctx.account, ...))\n\
                                 For tokens: TypeName = (is_token, seeds = (...), owner_seeds = (...))",
                                ident_str
                            ),
                        ));
                    }
                }
            } else {
                return Err(syn::Error::new(
                    content.span(),
                    "Expected keyword (is_token, seeds, or owner_seeds). Use explicit syntax:\n\
                     - PDA: TypeName = (seeds = (\"seed\", ctx.account, ...))\n\
                     - Token: TypeName = (is_token, seeds = (...), owner_seeds = (...))",
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
            owner_seeds,
            inner_type: None,    // Set by caller for #[light_account(init)] fields
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

/// Parse owner seeds - either parenthesized tuple or single expression
fn parse_owner_seeds(content: ParseStream) -> Result<Vec<SeedElement>> {
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
/// - Constant: single-segment constants are qualified with their definition module path
/// - FunctionCall: bare function names are qualified with their definition module path
/// - Passthrough: uses expression as-is (for complex patterns)
///
/// `module_path` is the module where the Accounts struct was found (used as fallback
/// for function calls). `crate_ctx` is used to look up where constants and functions
/// are actually defined, to generate fully qualified paths.
pub fn convert_classified_to_seed_elements(
    seeds: &[crate::light_pdas::seeds::ClassifiedSeed],
    module_path: &str,
    crate_ctx: &crate::light_pdas::parsing::CrateContext,
) -> Punctuated<SeedElement, Token![,]> {
    use crate::light_pdas::seeds::{extract_data_field_info, ClassifiedSeed};

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
            ClassifiedSeed::Constant { path, expr } => {
                // Single-segment bare constant names (e.g., POOL_SEED, A) need to be
                // fully qualified because the generated code lives in the program module,
                // not where the Accounts struct is defined.
                //
                // Resolution strategy:
                // 1. Look up where the constant is defined in the crate (CrateContext)
                // 2. If found AND the module path is publicly accessible, use it
                //    (e.g., crate::instructions::edge_cases::A)
                // 3. Otherwise fall back to crate:: prefix (e.g., crate::POOL_SEED)
                //    which works for constants re-exported at the crate root
                //
                // Multi-segment paths are left as-is because they may be:
                // - Already qualified: crate::state::CONSTANT
                // - External crate paths: light_sdk_types::constants::X
                // - Self-qualified: self::CONSTANT
                //
                // Important: We must preserve any trailing method calls (e.g., .as_bytes())
                // from the original expression.
                let is_single_segment = path.segments.len() == 1;
                let qualified_expr: Expr = if is_single_segment {
                    let const_name = path.segments[0].ident.to_string();
                    let resolved = crate_ctx
                        .find_const_module_path(&const_name)
                        .filter(|p| crate_ctx.is_module_path_public(p))
                        .unwrap_or("crate");
                    let mod_path: syn::Path =
                        syn::parse_str(resolved).unwrap_or_else(|_| syn::parse_quote!(crate));
                    // Qualify the constant in the expression, preserving method calls
                    qualify_constant_in_expr(expr, &mod_path, path)
                } else {
                    // Multi-segment paths: use expr as-is
                    (**expr).clone()
                };
                SeedElement::Expression(Box::new(qualified_expr))
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
            ClassifiedSeed::FunctionCall {
                func_expr,
                args: fn_args,
                has_as_ref,
            } => {
                // Reconstruct the function call with rewritten args for ctx/data scope.
                // Each classified arg gets rewritten:
                // - CtxAccount `field` -> `ctx.field`
                // - DataField `field` -> `data.field`
                // Bare function names are qualified via CrateContext lookup.
                let rewritten_call =
                    rewrite_fn_call_for_scope(func_expr, fn_args, module_path, crate_ctx);
                let expr: Expr = if *has_as_ref {
                    syn::parse_quote!(#rewritten_call.as_ref())
                } else {
                    rewritten_call
                };
                SeedElement::Expression(Box::new(expr))
            }
            ClassifiedSeed::Passthrough(expr) => SeedElement::Expression(expr.clone()),
        };
        result.push(elem);
    }
    result
}

/// Qualify a constant in an expression, preserving any trailing method calls.
///
/// For example, `AUTH_SEED.as_bytes()` with `mod_path = crate` becomes `crate::AUTH_SEED.as_bytes()`.
fn qualify_constant_in_expr(expr: &Expr, mod_path: &syn::Path, const_path: &syn::Path) -> Expr {
    match expr {
        Expr::MethodCall(method_call) => {
            // Recursively qualify the receiver, then rebuild the method call
            let qualified_receiver =
                qualify_constant_in_expr(&method_call.receiver, mod_path, const_path);
            Expr::MethodCall(syn::ExprMethodCall {
                attrs: method_call.attrs.clone(),
                receiver: Box::new(qualified_receiver),
                dot_token: method_call.dot_token,
                method: method_call.method.clone(),
                turbofish: method_call.turbofish.clone(),
                paren_token: method_call.paren_token,
                args: method_call.args.clone(),
            })
        }
        Expr::Path(_) => {
            // This is the constant itself - qualify it
            syn::parse_quote!(#mod_path::#const_path)
        }
        _ => {
            // For other expression types, just use the qualified constant
            // (shouldn't normally happen for constant seeds)
            syn::parse_quote!(#mod_path::#const_path)
        }
    }
}

/// Rewrite a FunctionCall expression's arguments for the program scope.
///
/// Each classified arg gets rewritten:
/// - CtxAccount `field` -> `&ctx.field`
/// - DataField `field` -> `&data.field`
///
/// Bare function names (single-segment paths) are qualified by looking up
/// the function's definition module in CrateContext, falling back to `module_path`.
/// Non-classified args are passed through unchanged.
fn rewrite_fn_call_for_scope(
    func_expr: &Expr,
    fn_args: &[crate::light_pdas::seeds::ClassifiedFnArg],
    module_path: &str,
    crate_ctx: &crate::light_pdas::parsing::CrateContext,
) -> Expr {
    use quote::quote;

    use crate::light_pdas::seeds::FnArgKind;

    if let Expr::Call(call) = func_expr {
        // Qualify bare function names via CrateContext lookup.
        // Use definition path if found in a public module, else fall back to module_path.
        let func_path: Expr = if let Expr::Path(path_expr) = &*call.func {
            if path_expr.path.segments.len() == 1 {
                let fn_name = path_expr.path.segments[0].ident.to_string();
                let resolved = crate_ctx
                    .find_fn_module_path(&fn_name)
                    .filter(|p| crate_ctx.is_module_path_public(p))
                    .unwrap_or(module_path);
                let mod_path: syn::Path =
                    syn::parse_str(resolved).unwrap_or_else(|_| syn::parse_quote!(crate));
                let ident = &path_expr.path.segments[0].ident;
                syn::parse_quote!(#mod_path::#ident)
            } else {
                Expr::Path(path_expr.clone())
            }
        } else {
            (*call.func).clone()
        };

        let rewritten_args: Vec<Expr> = call
            .args
            .iter()
            .map(|arg| {
                // Check if this arg matches any classified arg
                let arg_str = quote!(#arg).to_string();
                for classified in fn_args {
                    let field = &classified.field_name;
                    let field_str = field.to_string();
                    if arg_str.contains(&field_str) {
                        return match classified.kind {
                            FnArgKind::CtxAccount => syn::parse_quote!(&ctx.#field),
                            FnArgKind::DataField => syn::parse_quote!(&data.#field),
                        };
                    }
                }
                // Non-dynamic arg: pass through
                arg.clone()
            })
            .collect();

        syn::parse_quote!(#func_path(#(#rewritten_args),*))
    } else {
        // Shouldn't happen -- FunctionCall always wraps an Expr::Call
        func_expr.clone()
    }
}

pub fn convert_classified_to_seed_elements_vec(
    seeds: &[crate::light_pdas::seeds::ClassifiedSeed],
    module_path: &str,
    crate_ctx: &crate::light_pdas::parsing::CrateContext,
) -> Vec<SeedElement> {
    convert_classified_to_seed_elements(seeds, module_path, crate_ctx)
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
pub(crate) fn call_has_ctx_arg(
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
                use light_account::{LightPreInit, LightFinalize};
                let _ = #ctx_name.accounts.light_pre_init(#ctx_name.remaining_accounts, &#params_ident)
                    .map_err(|e| anchor_lang::error::Error::from(solana_program_error::ProgramError::from(e)))?;

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
                use light_account::{LightPreInit, LightFinalize};
                let __has_pre_init = #ctx_name.accounts.light_pre_init(#ctx_name.remaining_accounts, &#params_ident)
                    .map_err(|e| anchor_lang::error::Error::from(solana_program_error::ProgramError::from(e)))?;

                // Execute the original handler body and capture result
                let __user_result: anchor_lang::Result<()> = #fn_block;
                // Propagate any errors from user code
                __user_result?;

                // Phase 2: Finalize (creates token accounts/ATAs via CPI)
                #ctx_name.accounts.light_finalize(#ctx_name.remaining_accounts, &#params_ident, __has_pre_init)
                    .map_err(|e| anchor_lang::error::Error::from(solana_program_error::ProgramError::from(e)))?;

                Ok(())
            }
        }
    }
}
