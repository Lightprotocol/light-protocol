//! Unit tests for context and params extraction from function signatures.
//!
//! Extracted from `light_pdas/program/parsing.rs`.

use syn::punctuated::Punctuated;

use crate::light_pdas::program::parsing::{
    call_has_ctx_arg, extract_context_and_params, ExtractResult,
};

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
