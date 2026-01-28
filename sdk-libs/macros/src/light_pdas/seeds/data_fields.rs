//! Data field extraction from classified seeds.
//!
//! This module provides utilities for extracting field information from
//! DataRooted seeds for code generation.

use syn::{Expr, Ident};

use super::types::{ClassifiedSeed, FnArgKind};
use crate::light_pdas::shared_utils::is_base_path;

/// Get data field names from classified seeds.
/// Extracts the terminal field name from DataRooted expressions.
pub fn get_data_fields(seeds: &[ClassifiedSeed]) -> Vec<(Ident, Option<Ident>)> {
    let mut fields = Vec::new();
    for seed in seeds {
        match seed {
            ClassifiedSeed::DataRooted { expr, .. } => {
                if let Some((field_name, conversion)) = extract_data_field_info(expr) {
                    if !fields.iter().any(|(f, _): &(Ident, _)| f == &field_name) {
                        fields.push((field_name, conversion));
                    }
                }
            }
            ClassifiedSeed::FunctionCall { args, .. } => {
                // Include DataField args from function calls (e.g., max_key(&params.key_a, &params.key_b))
                for arg in args {
                    if matches!(arg.kind, FnArgKind::DataField) {
                        let field_name = arg.field_name.clone();
                        if !fields.iter().any(|(f, _): &(Ident, _)| *f == field_name) {
                            // FunctionCall data args are Pubkey by default (no conversion)
                            fields.push((field_name, None));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    fields
}

/// Extract field name and conversion method from a data-rooted expression.
/// Returns (field_name, Some(method)) for expressions like `params.field.to_le_bytes()`.
pub fn extract_data_field_info(expr: &Expr) -> Option<(Ident, Option<Ident>)> {
    match expr {
        // Bare identifier: amount (Format 2 instruction arg used directly)
        Expr::Path(path) => {
            if let Some(ident) = path.path.get_ident() {
                return Some((ident.clone(), None));
            }
            None
        }
        // Field access: params.owner, data.field
        Expr::Field(field) => {
            if let syn::Member::Named(field_name) = &field.member {
                return Some((field_name.clone(), None));
            }
            None
        }
        // Method call: params.field.to_le_bytes(), amount.as_ref()
        Expr::MethodCall(mc) => {
            let method_name = mc.method.to_string();
            // Check for conversion methods
            if method_name == "to_le_bytes" || method_name == "to_be_bytes" {
                if let Some((field_name, _)) = extract_data_field_info(&mc.receiver) {
                    return Some((field_name, Some(mc.method.clone())));
                }
            }
            // Skip .as_ref(), .as_bytes(), etc. and recurse
            if method_name == "as_ref" || method_name == "as_bytes" || method_name == "as_slice" {
                return extract_data_field_info(&mc.receiver);
            }
            None
        }
        // Index: params.arrays[0]
        Expr::Index(idx) => extract_data_field_info(&idx.expr),
        // Reference: &params.owner
        Expr::Reference(r) => extract_data_field_info(&r.expr),
        _ => None,
    }
}

/// Get params-only seed fields from a TokenSeedSpec.
/// This is a convenience wrapper that works with the SeedElement type.
pub fn get_params_only_seed_fields_from_spec(
    spec: &crate::light_pdas::program::instructions::TokenSeedSpec,
    state_field_names: &std::collections::HashSet<String>,
) -> Vec<(Ident, syn::Type, bool)> {
    use crate::light_pdas::program::instructions::SeedElement;

    let mut fields = Vec::new();
    for seed in &spec.seeds {
        if let SeedElement::Expression(expr) = seed {
            // Extract data fields from top-level expressions (e.g., data.owner.as_ref())
            if let Some((field_name, has_conversion)) = extract_data_field_from_expr(expr) {
                add_params_only_field(&field_name, has_conversion, state_field_names, &mut fields);
            }
            // Also extract data fields from function call arguments
            // (e.g., crate::max_key(&data.key_a, &data.key_b).as_ref())
            extract_data_fields_from_nested_calls(expr, state_field_names, &mut fields);
        }
    }
    fields
}

/// Add a params-only field if it's not on the state struct and not already added.
fn add_params_only_field(
    field_name: &Ident,
    has_conversion: bool,
    state_field_names: &std::collections::HashSet<String>,
    fields: &mut Vec<(Ident, syn::Type, bool)>,
) {
    let field_str = field_name.to_string();
    if !state_field_names.contains(&field_str)
        && !fields
            .iter()
            .any(|(f, _, _): &(Ident, _, _)| f == field_name)
    {
        let field_type: syn::Type = if has_conversion {
            syn::parse_quote!(u64)
        } else {
            syn::parse_quote!(Pubkey)
        };
        fields.push((field_name.clone(), field_type, has_conversion));
    }
}

/// Recursively extract data fields from function call arguments within an expression.
fn extract_data_fields_from_nested_calls(
    expr: &syn::Expr,
    state_field_names: &std::collections::HashSet<String>,
    fields: &mut Vec<(Ident, syn::Type, bool)>,
) {
    match expr {
        syn::Expr::Call(call) => {
            for arg in &call.args {
                if let Some((field_name, has_conversion)) = extract_data_field_from_expr(arg) {
                    add_params_only_field(&field_name, has_conversion, state_field_names, fields);
                }
                extract_data_fields_from_nested_calls(arg, state_field_names, fields);
            }
        }
        syn::Expr::MethodCall(mc) => {
            extract_data_fields_from_nested_calls(&mc.receiver, state_field_names, fields);
            for arg in &mc.args {
                extract_data_fields_from_nested_calls(arg, state_field_names, fields);
            }
        }
        syn::Expr::Reference(r) => {
            extract_data_fields_from_nested_calls(&r.expr, state_field_names, fields);
        }
        _ => {}
    }
}

/// Extract the terminal field name from a DataRooted seed expression.
///
/// For `params.owner.as_ref()` returns `owner`.
/// For `params.nonce.to_le_bytes()` returns `nonce`.
/// For bare `owner` returns `owner`.
pub fn extract_data_field_name_from_expr(expr: &syn::Expr) -> Option<Ident> {
    // Try extract_data_field_info first (works for most expressions)
    if let Some((field, _)) = extract_data_field_info(expr) {
        return Some(field);
    }
    // Fallback: try extract_data_field_from_expr (handles data.X pattern)
    extract_data_field_from_expr(expr).map(|(name, _)| name)
}

/// Extract data field name and conversion info from an expression.
/// Returns (field_name, has_conversion) if the expression is a data.* field.
fn extract_data_field_from_expr(expr: &syn::Expr) -> Option<(Ident, bool)> {
    match expr {
        syn::Expr::Field(field_expr) => {
            if let syn::Member::Named(field_name) = &field_expr.member {
                if is_base_path(&field_expr.base, "data") {
                    return Some((field_name.clone(), false));
                }
            }
            None
        }
        syn::Expr::MethodCall(method_call) => {
            // Handle data.field.to_le_bytes().as_ref() etc.
            let has_bytes_conversion =
                method_call.method == "to_le_bytes" || method_call.method == "to_be_bytes";
            if has_bytes_conversion {
                return extract_data_field_from_expr(&method_call.receiver)
                    .map(|(name, _)| (name, true));
            }
            // For .as_ref(), recurse without marking conversion
            if method_call.method == "as_ref" || method_call.method == "as_bytes" {
                return extract_data_field_from_expr(&method_call.receiver);
            }
            None
        }
        syn::Expr::Reference(ref_expr) => extract_data_field_from_expr(&ref_expr.expr),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;
    use crate::light_pdas::seeds::types::ClassifiedFnArg;

    fn make_ident(s: &str) -> Ident {
        Ident::new(s, proc_macro2::Span::call_site())
    }

    #[test]
    fn test_get_data_fields_simple() {
        let seeds = vec![
            ClassifiedSeed::Literal(b"seed".to_vec()),
            ClassifiedSeed::DataRooted {
                root: make_ident("params"),
                expr: Box::new(parse_quote!(params.owner.as_ref())),
            },
        ];

        let fields = get_data_fields(&seeds);
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].0.to_string(), "owner");
        assert!(fields[0].1.is_none()); // No conversion
    }

    #[test]
    fn test_get_data_fields_with_conversion() {
        let seeds = vec![ClassifiedSeed::DataRooted {
            root: make_ident("params"),
            expr: Box::new(parse_quote!(params.amount.to_le_bytes().as_ref())),
        }];

        let fields = get_data_fields(&seeds);
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].0.to_string(), "amount");
        assert!(fields[0].1.is_some()); // Has conversion
        assert_eq!(fields[0].1.as_ref().unwrap().to_string(), "to_le_bytes");
    }

    #[test]
    fn test_get_data_fields_from_function_call() {
        let seeds = vec![ClassifiedSeed::FunctionCall {
            func_expr: Box::new(parse_quote!(crate::max_key(&params.key_a, &params.key_b))),
            args: vec![
                ClassifiedFnArg {
                    field_name: make_ident("key_a"),
                    kind: FnArgKind::DataField,
                },
                ClassifiedFnArg {
                    field_name: make_ident("key_b"),
                    kind: FnArgKind::DataField,
                },
            ],
            has_as_ref: true,
        }];

        let fields = get_data_fields(&seeds);
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].0.to_string(), "key_a");
        assert_eq!(fields[1].0.to_string(), "key_b");
    }

    #[test]
    fn test_get_data_fields_deduplicates() {
        // Same field referenced twice should only appear once
        let seeds = vec![
            ClassifiedSeed::DataRooted {
                root: make_ident("params"),
                expr: Box::new(parse_quote!(params.owner.as_ref())),
            },
            ClassifiedSeed::DataRooted {
                root: make_ident("params"),
                expr: Box::new(parse_quote!(params.owner.as_ref())),
            },
        ];

        let fields = get_data_fields(&seeds);
        assert_eq!(fields.len(), 1);
    }

    #[test]
    fn test_extract_data_field_info_bare_ident() {
        let expr: syn::Expr = parse_quote!(owner);
        let result = extract_data_field_info(&expr);
        assert!(result.is_some());
        let (field, conversion) = result.unwrap();
        assert_eq!(field.to_string(), "owner");
        assert!(conversion.is_none());
    }

    #[test]
    fn test_extract_data_field_info_field_access() {
        let expr: syn::Expr = parse_quote!(params.owner);
        let result = extract_data_field_info(&expr);
        assert!(result.is_some());
        let (field, conversion) = result.unwrap();
        assert_eq!(field.to_string(), "owner");
        assert!(conversion.is_none());
    }

    #[test]
    fn test_extract_data_field_info_with_as_ref() {
        let expr: syn::Expr = parse_quote!(params.owner.as_ref());
        let result = extract_data_field_info(&expr);
        assert!(result.is_some());
        let (field, conversion) = result.unwrap();
        assert_eq!(field.to_string(), "owner");
        assert!(conversion.is_none());
    }

    #[test]
    fn test_extract_data_field_info_with_to_le_bytes() {
        let expr: syn::Expr = parse_quote!(params.amount.to_le_bytes());
        let result = extract_data_field_info(&expr);
        assert!(result.is_some());
        let (field, conversion) = result.unwrap();
        assert_eq!(field.to_string(), "amount");
        assert!(conversion.is_some());
        assert_eq!(conversion.unwrap().to_string(), "to_le_bytes");
    }

    #[test]
    fn test_extract_data_field_name_from_expr() {
        let expr: syn::Expr = parse_quote!(params.owner.as_ref());
        let result = extract_data_field_name_from_expr(&expr);
        assert!(result.is_some());
        assert_eq!(result.unwrap().to_string(), "owner");
    }
}
