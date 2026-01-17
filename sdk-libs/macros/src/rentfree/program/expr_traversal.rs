//! AST expression transformation utilities.
//!
//! This module provides expression transformation for converting field access patterns
//! used in seed derivation code generation.

use std::collections::HashSet;
use syn::Expr;

use crate::rentfree::shared_utils::is_base_path;

// =============================================================================
// EXPRESSION TRANSFORMER
// =============================================================================

/// Transform expressions by replacing field access patterns.
///
/// Used for converting:
/// - `data.field` -> `self.field`
/// - `ctx.field` -> `ctx_seeds.field` (if field is in ctx_field_names)
/// - `ctx.accounts.field` -> `ctx_seeds.field`
pub fn transform_expr_for_ctx_seeds(expr: &Expr, ctx_field_names: &HashSet<String>) -> Expr {
    match expr {
        Expr::Field(field_expr) => {
            let Some(syn::Member::Named(field_name)) = Some(&field_expr.member) else {
                return expr.clone();
            };

            // Check for ctx.accounts.field -> ctx_seeds.field
            if let Expr::Field(nested_field) = &*field_expr.base {
                if let syn::Member::Named(base_name) = &nested_field.member {
                    if base_name == "accounts" && is_base_path(&nested_field.base, "ctx") {
                        return syn::parse_quote! { ctx_seeds.#field_name };
                    }
                }
            }

            // Check for data.field -> self.field or ctx.field -> ctx_seeds.field
            if is_base_path(&field_expr.base, "data") {
                return syn::parse_quote! { self.#field_name };
            }
            if is_base_path(&field_expr.base, "ctx")
                && ctx_field_names.contains(&field_name.to_string())
            {
                return syn::parse_quote! { ctx_seeds.#field_name };
            }

            expr.clone()
        }
        Expr::MethodCall(method_call) => {
            let mut new_call = method_call.clone();
            new_call.receiver =
                Box::new(transform_expr_for_ctx_seeds(&method_call.receiver, ctx_field_names));
            new_call.args = method_call
                .args
                .iter()
                .map(|a| transform_expr_for_ctx_seeds(a, ctx_field_names))
                .collect();
            Expr::MethodCall(new_call)
        }
        Expr::Call(call_expr) => {
            let mut new_call = call_expr.clone();
            new_call.args = call_expr
                .args
                .iter()
                .map(|a| transform_expr_for_ctx_seeds(a, ctx_field_names))
                .collect();
            Expr::Call(new_call)
        }
        Expr::Reference(ref_expr) => {
            let mut new_ref = ref_expr.clone();
            new_ref.expr = Box::new(transform_expr_for_ctx_seeds(&ref_expr.expr, ctx_field_names));
            Expr::Reference(new_ref)
        }
        _ => expr.clone(),
    }
}
