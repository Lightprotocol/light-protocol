//! Unit tests for AST visitor patterns (FieldExtractor).
//!
//! Extracted from `light_pdas/program/visitors.rs`.

use syn::Expr;

use crate::light_pdas::program::visitors::FieldExtractor;

#[test]
fn test_extract_ctx_accounts_field() {
    let expr: Expr = syn::parse_quote!(ctx.accounts.user);
    let fields = FieldExtractor::ctx_fields(&[]).extract(&expr);
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].to_string(), "user");
}

#[test]
fn test_extract_ctx_direct_field() {
    let expr: Expr = syn::parse_quote!(ctx.program_id);
    let fields = FieldExtractor::ctx_fields(&[]).extract(&expr);
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].to_string(), "program_id");
}

#[test]
fn test_extract_data_field() {
    let expr: Expr = syn::parse_quote!(data.owner);
    let fields = FieldExtractor::data_fields().extract(&expr);
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].to_string(), "owner");
}

#[test]
fn test_extract_nested_in_method_call() {
    let expr: Expr = syn::parse_quote!(ctx.accounts.user.key());
    let fields = FieldExtractor::ctx_fields(&[]).extract(&expr);
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].to_string(), "user");
}

#[test]
fn test_extract_nested_in_reference() {
    let expr: Expr = syn::parse_quote!(&ctx.accounts.user.key());
    let fields = FieldExtractor::ctx_fields(&[]).extract(&expr);
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].to_string(), "user");
}

#[test]
fn test_excludes_fields() {
    let expr: Expr = syn::parse_quote!(ctx.accounts.fee_payer);
    let fields = FieldExtractor::ctx_fields(&["fee_payer"]).extract(&expr);
    assert!(fields.is_empty());
}

#[test]
fn test_deduplicates_fields() {
    let expr: Expr = syn::parse_quote!({
        ctx.accounts.user.key();
        ctx.accounts.user.owner();
    });
    let fields = FieldExtractor::ctx_fields(&[]).extract(&expr);
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].to_string(), "user");
}

#[test]
fn test_extract_from_call_args() {
    let expr: Expr = syn::parse_quote!(some_fn(&ctx.accounts.user, data.amount));
    let ctx_fields = FieldExtractor::ctx_fields(&[]).extract(&expr);
    let data_fields = FieldExtractor::data_fields().extract(&expr);
    assert_eq!(ctx_fields.len(), 1);
    assert_eq!(ctx_fields[0].to_string(), "user");
    assert_eq!(data_fields.len(), 1);
    assert_eq!(data_fields[0].to_string(), "amount");
}

#[test]
fn test_separate_extractors_for_ctx_and_data() {
    let expr: Expr = syn::parse_quote!((ctx.accounts.user, data.amount));
    let ctx_fields = FieldExtractor::ctx_fields(&[]).extract(&expr);
    let data_fields = FieldExtractor::data_fields().extract(&expr);
    assert_eq!(ctx_fields.len(), 1);
    assert_eq!(ctx_fields[0].to_string(), "user");
    assert_eq!(data_fields.len(), 1);
    assert_eq!(data_fields[0].to_string(), "amount");
}
