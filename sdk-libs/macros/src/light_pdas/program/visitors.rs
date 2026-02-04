//! Visitor-based AST traversal utilities using syn's Visit trait.
//!
//! This module provides:
//! - `FieldExtractor`: A visitor for extracting field names from expressions
//! - `ClientSeedInfo`: Classification of seed elements for client code generation
//! - `classify_seed`: Classify a seed element into a `ClientSeedInfo`
//! - `generate_client_seed_code`: Generate (parameter, expression) from classified seed
//!
//! The implementation stores references during traversal to avoid allocations,
//! only cloning identifiers when producing the final output.

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    visit::{self, Visit},
    Expr, Ident, Member,
};

use super::instructions::{InstructionDataSpec, SeedElement};
use crate::light_pdas::{account::utils::is_pubkey_type, shared_utils::is_constant_identifier};

/// Visitor that extracts field names matching ctx.field, ctx.accounts.field, or data.field patterns.
///
/// Uses syn's Visit trait for efficient read-only traversal. Stores references during
/// traversal to minimize allocations, only cloning when producing final output.
///
/// # Example
/// ```ignore
/// let fields = FieldExtractor::ctx_fields(&["fee_payer"])
///     .extract(&some_expr);
/// ```
pub struct FieldExtractor<'ast, 'cfg> {
    /// Extract ctx.field and ctx.accounts.field patterns
    extract_ctx: bool,
    /// Extract data.field patterns
    extract_data: bool,
    /// Field names to exclude from results
    excluded: &'cfg [&'cfg str],
    /// The context parameter name (e.g., "ctx", "context", "anchor_ctx")
    ctx_name: &'cfg str,
    /// Collected field references (avoids cloning during traversal)
    fields: Vec<&'ast Ident>,
    /// Track seen field names for deduplication
    seen: HashSet<String>,
}

impl<'ast, 'cfg> FieldExtractor<'ast, 'cfg> {
    /// Create an extractor for ctx.field and ctx.accounts.field patterns.
    ///
    /// Uses the default context name "ctx".
    /// Excludes common infrastructure fields like fee_payer, rent_sponsor, etc.
    pub fn ctx_fields(excluded: &'cfg [&'cfg str]) -> Self {
        Self::ctx_fields_with_name(excluded, "ctx")
    }

    /// Create an extractor for ctx.field and ctx.accounts.field patterns with a custom context name.
    ///
    /// `ctx_name` is the context parameter name (e.g., "ctx", "context", "anchor_ctx").
    /// Excludes common infrastructure fields like fee_payer, rent_sponsor, etc.
    pub fn ctx_fields_with_name(excluded: &'cfg [&'cfg str], ctx_name: &'cfg str) -> Self {
        Self {
            extract_ctx: true,
            extract_data: false,
            excluded,
            ctx_name,
            fields: Vec::new(),
            seen: HashSet::new(),
        }
    }

    /// Create an extractor for data.field patterns.
    pub fn data_fields() -> Self {
        Self {
            extract_ctx: false,
            extract_data: true,
            excluded: &[],
            ctx_name: "ctx", // Not used for data extraction, but needed for struct
            fields: Vec::new(),
            seen: HashSet::new(),
        }
    }

    /// Extract field names from the given expression.
    ///
    /// Visits the expression tree and collects all field names matching the configured patterns.
    /// Returns deduplicated field identifiers in order of first occurrence.
    /// Cloning is deferred until this final output stage.
    pub fn extract(mut self, expr: &'ast Expr) -> Vec<Ident> {
        self.visit_expr(expr);
        // Clone only when producing final output
        self.fields.into_iter().cloned().collect()
    }

    /// Try to add a field reference if not excluded and not already seen.
    fn try_add(&mut self, field: &'ast Ident) {
        let name = field.to_string();
        if !self.excluded.contains(&name.as_str()) && self.seen.insert(name) {
            self.fields.push(field);
        }
    }

    /// Check if the base expression is `<ctx_name>.accounts` (e.g., `ctx.accounts`, `context.accounts`).
    /// Uses the default context name "ctx".
    pub fn is_ctx_accounts(base: &Expr) -> bool {
        Self::is_ctx_accounts_with_name(base, "ctx")
    }

    /// Check if the base expression is `<ctx_name>.accounts` with a custom context name.
    pub fn is_ctx_accounts_with_name(base: &Expr, ctx_name: &str) -> bool {
        if let Expr::Field(nested) = base {
            if let Member::Named(member) = &nested.member {
                return member == "accounts" && Self::is_path_ident(&nested.base, ctx_name);
            }
        }
        false
    }

    /// Check if the base expression matches the `<any>.accounts` pattern.
    /// This is flexible and accepts any identifier before `.accounts` (ctx, context, anchor_ctx, etc.).
    /// Returns the identifier name if matched.
    pub fn is_any_ctx_accounts(base: &Expr) -> Option<String> {
        if let Expr::Field(nested) = base {
            if let Member::Named(member) = &nested.member {
                if member == "accounts" {
                    if let Expr::Path(path) = &*nested.base {
                        if let Some(ident) = path.path.get_ident() {
                            return Some(ident.to_string());
                        }
                    }
                }
            }
        }
        None
    }

    /// Check if an expression is a path with the given identifier.
    pub fn is_path_ident(expr: &Expr, ident: &str) -> bool {
        matches!(expr, Expr::Path(p) if p.path.is_ident(ident))
    }
}

impl<'ast, 'cfg> Visit<'ast> for FieldExtractor<'ast, 'cfg> {
    fn visit_expr_field(&mut self, node: &'ast syn::ExprField) {
        if let Member::Named(field_name) = &node.member {
            // Check for ctx.accounts.field pattern (using configured ctx_name)
            if self.extract_ctx && Self::is_ctx_accounts_with_name(&node.base, self.ctx_name) {
                self.try_add(field_name);
                // Don't recurse further - we found our target
                return;
            }

            // Check for ctx.field pattern (direct access, using configured ctx_name)
            if self.extract_ctx && Self::is_path_ident(&node.base, self.ctx_name) {
                self.try_add(field_name);
                return;
            }

            // Check for data.field pattern
            if self.extract_data && Self::is_path_ident(&node.base, "data") {
                self.try_add(field_name);
                return;
            }
        }

        // Continue visiting child expressions
        visit::visit_expr_field(self, node);
    }
}

// =============================================================================
// CLIENT SEED CLASSIFICATION
// =============================================================================

/// Classified seed for client code generation.
///
/// Separates "what kind of seed is this?" from "what code to generate?".
/// Each variant captures all info needed for the generation phase.
#[derive(Debug, Clone)]
pub enum ClientSeedInfo {
    /// String literal: "seed" -> "seed".as_bytes()
    Literal(String),
    /// Byte literal: b"seed" -> &[...]
    ByteLiteral(Vec<u8>),
    /// Constant: fully qualified path -> path.as_ref()
    Constant {
        path: syn::Path,
        is_cpi_signer: bool,
    },
    /// ctx.field or ctx.accounts.field -> Pubkey parameter
    CtxField { field: Ident, method: Option<Ident> },
    /// data.field -> typed parameter from instruction_data
    DataField { field: Ident, method: Option<Ident> },
    /// Function call - stored as original expression for proper AST transformation
    FunctionCall(Box<syn::ExprCall>),
    /// Raw identifier that becomes a Pubkey parameter
    Identifier(Ident),
    /// Fallback for expressions that don't match other patterns
    RawExpr(Box<syn::Expr>),
}

/// Classify a SeedElement for client code generation.
pub fn classify_seed(seed: &SeedElement) -> syn::Result<ClientSeedInfo> {
    match seed {
        SeedElement::Literal(lit) => Ok(ClientSeedInfo::Literal(lit.value())),
        SeedElement::Expression(expr) => classify_seed_expr(expr),
    }
}

/// Classify an expression into a ClientSeedInfo variant.
fn classify_seed_expr(expr: &syn::Expr) -> syn::Result<ClientSeedInfo> {
    match expr {
        syn::Expr::Field(field_expr) => classify_field_expr(field_expr),
        syn::Expr::MethodCall(method_call) => classify_method_call(method_call),
        syn::Expr::Lit(lit_expr) => classify_lit_expr(lit_expr),
        syn::Expr::Path(path_expr) => classify_path_expr(path_expr),
        syn::Expr::Call(call_expr) => classify_call_expr(call_expr),
        syn::Expr::Reference(ref_expr) => classify_seed_expr(&ref_expr.expr),
        _ => Ok(ClientSeedInfo::RawExpr(Box::new(expr.clone()))),
    }
}

/// Classify a field expression (e.g., ctx.field, data.field).
/// Accepts any context name (ctx, context, anchor_ctx, etc.) for `.accounts.field` patterns.
fn classify_field_expr(field_expr: &syn::ExprField) -> syn::Result<ClientSeedInfo> {
    if let Member::Named(field_name) = &field_expr.member {
        // Check for <any>.accounts.field pattern (ctx.accounts.field, context.accounts.field, etc.)
        if FieldExtractor::is_any_ctx_accounts(&field_expr.base).is_some() {
            return Ok(ClientSeedInfo::CtxField {
                field: field_name.clone(),
                method: None,
            });
        }

        // Check for direct base.field patterns
        if let syn::Expr::Path(path) = &*field_expr.base {
            if let Some(segment) = path.path.segments.first() {
                if segment.ident == "data" {
                    return Ok(ClientSeedInfo::DataField {
                        field: field_name.clone(),
                        method: None,
                    });
                }
                // Any other single identifier followed by .field is likely a context field
                // This handles ctx.bumps, context.program_id, etc.
                return Ok(ClientSeedInfo::CtxField {
                    field: field_name.clone(),
                    method: None,
                });
            }
        }

        // Unrecognized field pattern - preserve as raw expression
        // This lets downstream code decide how to handle it
        return Ok(ClientSeedInfo::RawExpr(Box::new(syn::Expr::Field(
            field_expr.clone(),
        ))));
    }

    Ok(ClientSeedInfo::RawExpr(Box::new(syn::Expr::Field(
        field_expr.clone(),
    ))))
}

/// Classify a method call expression (e.g., data.field.method(), ctx.accounts.field.key()).
/// Accepts any context name (ctx, context, anchor_ctx, etc.) for `.accounts.field.method()` patterns.
fn classify_method_call(method_call: &syn::ExprMethodCall) -> syn::Result<ClientSeedInfo> {
    // Check if receiver is a field expression
    if let syn::Expr::Field(field_expr) = &*method_call.receiver {
        if let Member::Named(field_name) = &field_expr.member {
            // Check for <any>.accounts.field.method() pattern
            if FieldExtractor::is_any_ctx_accounts(&field_expr.base).is_some() {
                return Ok(ClientSeedInfo::CtxField {
                    field: field_name.clone(),
                    method: Some(method_call.method.clone()),
                });
            }

            // Check for direct base.field patterns
            if let syn::Expr::Path(path) = &*field_expr.base {
                if let Some(segment) = path.path.segments.first() {
                    if segment.ident == "data" {
                        return Ok(ClientSeedInfo::DataField {
                            field: field_name.clone(),
                            method: Some(method_call.method.clone()),
                        });
                    }
                    // Any other single identifier followed by .field is likely a context field
                    return Ok(ClientSeedInfo::CtxField {
                        field: field_name.clone(),
                        method: Some(method_call.method.clone()),
                    });
                }
            }
        }
    }

    // Check if receiver is a function call (e.g., func(args).as_ref())
    // Strip the trailing method call and classify the inner call as FunctionCall.
    // This handles expressions like crate::max_key(&ctx.fee_payer, &ctx.authority).as_ref()
    // and crate::id().as_ref() in standalone seed helper functions.
    if let syn::Expr::Call(call_expr) = &*method_call.receiver {
        return classify_call_expr(call_expr);
    }

    // Check if receiver is a path identifier (e.g., ident.as_ref())
    if let syn::Expr::Path(path_expr) = &*method_call.receiver {
        if let Some(ident) = path_expr.path.get_ident() {
            return Ok(ClientSeedInfo::Identifier(ident.clone()));
        }
    }

    Ok(ClientSeedInfo::RawExpr(Box::new(syn::Expr::MethodCall(
        method_call.clone(),
    ))))
}

/// Classify a literal expression.
fn classify_lit_expr(lit_expr: &syn::ExprLit) -> syn::Result<ClientSeedInfo> {
    if let syn::Lit::ByteStr(byte_str) = &lit_expr.lit {
        Ok(ClientSeedInfo::ByteLiteral(byte_str.value()))
    } else {
        Ok(ClientSeedInfo::RawExpr(Box::new(syn::Expr::Lit(
            lit_expr.clone(),
        ))))
    }
}

/// Classify a path expression (constant or identifier).
///
/// Constants are detected by checking if the last path segment is an uppercase identifier.
/// After `convert_classified_to_seed_elements`, constants are fully qualified
/// (e.g., `crate::module::CONSTANT`), so we store the full path.
///
/// Type-qualified paths like `<SeedHolder as HasSeed>::TRAIT_SEED` are NOT classified
/// as constants because stripping the qself would lose the type qualification.
fn classify_path_expr(path_expr: &syn::ExprPath) -> syn::Result<ClientSeedInfo> {
    // Type-qualified paths (qself present) must be preserved as raw expressions
    // to keep the <Type as Trait>:: qualification intact.
    if path_expr.qself.is_some() {
        return Ok(ClientSeedInfo::RawExpr(Box::new(syn::Expr::Path(
            path_expr.clone(),
        ))));
    }

    // Check last segment for constant pattern (works for both single and multi-segment paths)
    if let Some(last_seg) = path_expr.path.segments.last() {
        let last_str = last_seg.ident.to_string();
        if is_constant_identifier(&last_str) {
            return Ok(ClientSeedInfo::Constant {
                path: path_expr.path.clone(),
                is_cpi_signer: last_str == "LIGHT_CPI_SIGNER",
            });
        }
    }
    // Single-segment non-constant identifiers become Identifier
    if let Some(ident) = path_expr.path.get_ident() {
        return Ok(ClientSeedInfo::Identifier(ident.clone()));
    }
    Ok(ClientSeedInfo::RawExpr(Box::new(syn::Expr::Path(
        path_expr.clone(),
    ))))
}

/// Classify a function call expression.
/// We store the original expression because function call arguments need AST transformation,
/// not simple classification - we need to preserve method calls like `.key()` while mapping
/// base identifiers to parameters.
fn classify_call_expr(call_expr: &syn::ExprCall) -> syn::Result<ClientSeedInfo> {
    Ok(ClientSeedInfo::FunctionCall(Box::new(call_expr.clone())))
}

// =============================================================================
// CLIENT CODE GENERATION
// =============================================================================

/// Map a function call argument, extracting parameters and transforming ctx/data references.
///
/// This preserves the expression structure (method calls, references) while replacing
/// `ctx.field`, `ctx.accounts.field`, and `data.field` with just the field name.
/// Returns the transformed expression and collects parameters into the provided vectors.
fn map_call_arg(
    arg: &syn::Expr,
    instruction_data: &[InstructionDataSpec],
    seen_params: &mut HashSet<String>,
    parameters: &mut Vec<TokenStream>,
    is_pinocchio: bool,
) -> syn::Result<TokenStream> {
    // Choose the correct pubkey path based on framework
    let pubkey_param = if is_pinocchio {
        quote! { light_account_pinocchio::solana_pubkey::Pubkey }
    } else {
        quote! { solana_pubkey::Pubkey }
    };

    match arg {
        syn::Expr::Reference(ref_expr) => {
            let inner = map_call_arg(&ref_expr.expr, instruction_data, seen_params, parameters, is_pinocchio)?;
            Ok(quote! { &#inner })
        }
        syn::Expr::Field(field_expr) => {
            if let syn::Member::Named(field_name) = &field_expr.member {
                // Check for ctx.accounts.field
                if FieldExtractor::is_ctx_accounts(&field_expr.base) {
                    if seen_params.insert(field_name.to_string()) {
                        parameters.push(quote! { #field_name: &#pubkey_param });
                    }
                    return Ok(quote! { #field_name });
                }
                // Check for ctx.field or data.field
                if let syn::Expr::Path(path) = &*field_expr.base {
                    if let Some(segment) = path.path.segments.first() {
                        if segment.ident == "data" {
                            if let Some(data_spec) = instruction_data
                                .iter()
                                .find(|d| d.field_name == *field_name)
                            {
                                if seen_params.insert(field_name.to_string()) {
                                    let param_type = &data_spec.field_type;
                                    let param_with_ref = if is_pubkey_type(param_type) {
                                        quote! { #field_name: &#param_type }
                                    } else {
                                        quote! { #field_name: #param_type }
                                    };
                                    parameters.push(param_with_ref);
                                }
                                return Ok(quote! { #field_name });
                            }
                            // data.field not in instruction_data (e.g., from FunctionCall args)
                            // Default to Pubkey parameter
                            if seen_params.insert(field_name.to_string()) {
                                parameters.push(quote! { #field_name: &#pubkey_param });
                            }
                            return Ok(quote! { #field_name });
                        } else if segment.ident == "ctx" {
                            if seen_params.insert(field_name.to_string()) {
                                parameters.push(quote! { #field_name: &#pubkey_param });
                            }
                            return Ok(quote! { #field_name });
                        }
                    }
                }
            }
            Ok(quote! { #field_expr })
        }
        syn::Expr::MethodCall(method_call) => {
            let receiver = map_call_arg(
                &method_call.receiver,
                instruction_data,
                seen_params,
                parameters,
                is_pinocchio,
            )?;
            let method = &method_call.method;
            let args: Vec<TokenStream> = method_call
                .args
                .iter()
                .map(|a| map_call_arg(a, instruction_data, seen_params, parameters, is_pinocchio))
                .collect::<syn::Result<_>>()?;
            Ok(quote! { (#receiver).#method(#(#args),*) })
        }
        syn::Expr::Call(nested_call) => {
            let func = &nested_call.func;
            let args: Vec<TokenStream> = nested_call
                .args
                .iter()
                .map(|a| map_call_arg(a, instruction_data, seen_params, parameters, is_pinocchio))
                .collect::<syn::Result<_>>()?;
            Ok(quote! { (#func)(#(#args),*) })
        }
        syn::Expr::Path(path_expr) => {
            // Check if this is a simple identifier that should become a parameter
            if let Some(ident) = path_expr.path.get_ident() {
                let name = ident.to_string();
                if name != "ctx"
                    && name != "data"
                    && !is_constant_identifier(&name)
                    && seen_params.insert(name)
                {
                    parameters.push(quote! { #ident: &#pubkey_param });
                }
            }
            Ok(quote! { #path_expr })
        }
        _ => Ok(quote! { #arg }),
    }
}

/// Generate code for a classified seed, adding to the provided parameter and expression lists.
///
/// This modifies the parameters and expressions vectors directly rather than returning
/// individual values, which allows for proper handling of function call seeds that may
/// contribute multiple parameters.
pub fn generate_client_seed_code(
    info: &ClientSeedInfo,
    instruction_data: &[InstructionDataSpec],
    seen_params: &mut HashSet<String>,
    parameters: &mut Vec<TokenStream>,
    expressions: &mut Vec<TokenStream>,
    is_pinocchio: bool,
) -> syn::Result<()> {
    // Choose the correct pubkey path based on framework
    let pubkey_param = if is_pinocchio {
        quote! { light_account_pinocchio::solana_pubkey::Pubkey }
    } else {
        quote! { solana_pubkey::Pubkey }
    };

    match info {
        ClientSeedInfo::Literal(s) => {
            expressions.push(quote! { #s.as_bytes() });
        }

        ClientSeedInfo::ByteLiteral(bytes) => {
            expressions.push(quote! { &[#(#bytes),*] });
        }

        ClientSeedInfo::Constant {
            path,
            is_cpi_signer,
        } => {
            let expr = if *is_cpi_signer {
                quote! { #path.cpi_signer.as_ref() }
            } else {
                quote! { { let __seed: &[u8] = #path.as_ref(); __seed } }
            };
            expressions.push(expr);
        }

        ClientSeedInfo::CtxField { field, method } => {
            if seen_params.insert(field.to_string()) {
                parameters.push(quote! { #field: &#pubkey_param });
            }
            let expr = match method {
                Some(m) => quote! { #field.#m().as_ref() },
                None => quote! { #field.as_ref() },
            };
            expressions.push(expr);
        }

        ClientSeedInfo::DataField { field, method } => {
            let data_spec = instruction_data
                .iter()
                .find(|d| d.field_name == *field)
                .ok_or_else(|| {
                    syn::Error::new(
                        field.span(),
                        format!("data.{} used in seeds but no type specified", field),
                    )
                })?;

            if seen_params.insert(field.to_string()) {
                let param_type = &data_spec.field_type;
                let param_with_ref = if is_pubkey_type(param_type) {
                    quote! { #field: &#param_type }
                } else {
                    quote! { #field: #param_type }
                };
                parameters.push(param_with_ref);
            }

            let expr = match method {
                Some(m) => quote! { #field.#m().as_ref() },
                None => quote! { #field.as_ref() },
            };
            expressions.push(expr);
        }

        ClientSeedInfo::Identifier(ident) => {
            if seen_params.insert(ident.to_string()) {
                parameters.push(quote! { #ident: &#pubkey_param });
            }
            expressions.push(quote! { #ident.as_ref() });
        }

        ClientSeedInfo::FunctionCall(call_expr) => {
            let mut mapped_args: Vec<TokenStream> = Vec::new();
            for arg in &call_expr.args {
                let mapped = map_call_arg(arg, instruction_data, seen_params, parameters, is_pinocchio)?;
                mapped_args.push(mapped);
            }
            let func = &call_expr.func;
            expressions.push(quote! { (#func)(#(#mapped_args),*).as_ref() });
        }

        ClientSeedInfo::RawExpr(expr) => {
            expressions.push(quote! { { let __seed: &[u8] = (#expr).as_ref(); __seed } });
        }
    }
    Ok(())
}
