//! Seed classification logic.
//!
//! This module provides the core `classify_seed_expr()` function and its helper functions
//! for classifying individual seed expressions into categories.

use syn::{Expr, Ident};

use super::instruction_args::InstructionArgSet;
use super::types::{ClassifiedFnArg, ClassifiedSeed, FnArgKind};
use crate::light_pdas::shared_utils::is_constant_identifier;

/// Classify a single seed expression using prefix detection + passthrough.
///
/// Strategy:
/// 1. Byte literals -> Literal
/// 2. Uppercase paths -> Constant
/// 3. Check if rooted in instruction arg -> DataRooted (pass through full expr)
/// 4. Check if rooted in ctx account -> CtxRooted (pass through full expr)
/// 5. Function calls with dynamic args -> FunctionCall
/// 6. Everything else -> Passthrough
///
/// # Errors
///
/// Returns an error if a bare identifier in a seed matches both an instruction arg
/// and could be a ctx account (name collision). Use explicit field access like
/// `params.field` to disambiguate.
pub fn classify_seed_expr(
    expr: &Expr,
    instruction_args: &InstructionArgSet,
) -> syn::Result<ClassifiedSeed> {
    // Handle byte string literals
    if let Some(bytes) = extract_byte_literal(expr) {
        return Ok(ClassifiedSeed::Literal(bytes));
    }

    // Handle constants (uppercase paths)
    if let Some(path) = extract_constant_path(expr) {
        return Ok(ClassifiedSeed::Constant {
            path,
            expr: Box::new(expr.clone()),
        });
    }

    // Check if rooted in instruction arg
    if let Some(root) = get_instruction_arg_root(expr, instruction_args) {
        // Error on name collision: bare identifier that matches instruction arg
        // could also be a ctx account with the same name
        if is_bare_identifier_collision(expr, &root) {
            return Err(syn::Error::new_spanned(
                expr,
                format!(
                    "Ambiguous seed: '{}' matches both an instruction argument and could be a \
                     context account. Use explicit field access (e.g., `params.{}`) for instruction \
                     data, or rename the instruction argument to avoid collision.",
                    root, root
                ),
            ));
        }
        return Ok(ClassifiedSeed::DataRooted {
            root,
            expr: Box::new(expr.clone()),
        });
    }

    // Check if rooted in ctx account
    if let Some(account) = get_ctx_account_root(expr) {
        return Ok(ClassifiedSeed::CtxRooted { account });
    }

    // Check for function calls with dynamic arguments
    if let Some(fc) = classify_function_call(expr, instruction_args) {
        return Ok(fc);
    }

    // Everything else: passthrough
    Ok(ClassifiedSeed::Passthrough(Box::new(expr.clone())))
}

/// Attempt to classify an expression as a FunctionCall seed.
///
/// Detects patterns like:
/// - `func(arg1, arg2)` -> Expr::Call
/// - `func(arg1, arg2).as_ref()` -> Expr::MethodCall(receiver=Expr::Call)
///
/// Returns `Some(ClassifiedSeed::FunctionCall{...})` if:
/// - The expression contains an `Expr::Call` (at top-level or as receiver of `.as_ref()`)
/// - At least one argument is rooted in instruction data or ctx accounts
///
/// Returns `None` if:
/// - Not a function call pattern
/// - No dynamic arguments (falls through to Passthrough)
fn classify_function_call(
    expr: &Expr,
    instruction_args: &InstructionArgSet,
) -> Option<ClassifiedSeed> {
    // Strip trailing .as_ref() / .as_bytes() to find the call expression
    let (call_expr, has_as_ref) = strip_trailing_as_ref(expr);

    // Check if the (possibly stripped) expression is a function call
    let call = match call_expr {
        Expr::Call(c) => c,
        _ => return None,
    };

    // Classify each argument
    let mut classified_args = Vec::new();
    let mut has_dynamic = false;

    for arg in &call.args {
        // Unwrap references for classification
        let inner = unwrap_references(arg);

        // Check if rooted in instruction arg
        if let Some(root) = get_instruction_arg_root(inner, instruction_args) {
            // Extract terminal field name (e.g., key_a from params.key_a)
            let field_name = extract_terminal_field_name(inner).unwrap_or(root);
            classified_args.push(ClassifiedFnArg {
                field_name,
                kind: FnArgKind::DataField,
            });
            has_dynamic = true;
            continue;
        }

        // Check if rooted in ctx account
        if let Some(account) = get_ctx_account_root(inner) {
            classified_args.push(ClassifiedFnArg {
                field_name: account,
                kind: FnArgKind::CtxAccount,
            });
            has_dynamic = true;
            continue;
        }

        // Not dynamic -- skip this arg (will be inlined as-is in codegen)
    }

    if !has_dynamic {
        return None;
    }

    Some(ClassifiedSeed::FunctionCall {
        func_expr: Box::new(Expr::Call(call.clone())),
        args: classified_args,
        has_as_ref,
    })
}

/// Strip trailing `.as_ref()` or `.as_bytes()` method calls from an expression.
/// Returns the inner expression and a flag indicating whether stripping occurred.
fn strip_trailing_as_ref(expr: &Expr) -> (&Expr, bool) {
    if let Expr::MethodCall(mc) = expr {
        let method = mc.method.to_string();
        if (method == "as_ref" || method == "as_bytes") && mc.args.is_empty() {
            return (&mc.receiver, true);
        }
    }
    (expr, false)
}

/// Unwrap reference expressions (&expr, &mut expr) to get the inner expression.
fn unwrap_references(expr: &Expr) -> &Expr {
    match expr {
        Expr::Reference(r) => unwrap_references(&r.expr),
        _ => expr,
    }
}

/// Extract the terminal (deepest) field name from an expression.
/// For `params.key_a.as_ref()` returns `key_a`.
/// For `params.key_a` returns `key_a`.
/// For bare `owner` returns `owner`.
fn extract_terminal_field_name(expr: &Expr) -> Option<Ident> {
    match expr {
        Expr::Field(field) => {
            if let syn::Member::Named(name) = &field.member {
                Some(name.clone())
            } else {
                None
            }
        }
        Expr::MethodCall(mc) => extract_terminal_field_name(&mc.receiver),
        Expr::Reference(r) => extract_terminal_field_name(&r.expr),
        Expr::Path(path) => path.path.get_ident().cloned(),
        _ => None,
    }
}

/// Extract byte literal from expression.
/// Handles: b"literal", "string", b"literal"[..]
fn extract_byte_literal(expr: &Expr) -> Option<Vec<u8>> {
    match expr {
        Expr::Lit(lit) => {
            if let syn::Lit::ByteStr(bs) = &lit.lit {
                return Some(bs.value());
            }
            if let syn::Lit::Str(s) = &lit.lit {
                return Some(s.value().into_bytes());
            }
            None
        }
        // Handle b"literal"[..] - full range slice
        Expr::Index(idx) => {
            if let Expr::Range(range) = &*idx.index {
                if range.start.is_none() && range.end.is_none() {
                    if let Expr::Lit(lit) = &*idx.expr {
                        if let syn::Lit::ByteStr(bs) = &lit.lit {
                            return Some(bs.value());
                        }
                    }
                }
            }
            None
        }
        // Unwrap references
        Expr::Reference(r) => extract_byte_literal(&r.expr),
        _ => None,
    }
}

/// Extract constant path from expression.
/// Handles: CONSTANT, path::CONSTANT, CONSTANT.as_bytes(), CONSTANT.as_ref()
/// Does NOT handle type-qualified paths like <T as Trait>::CONST (returns None for passthrough)
fn extract_constant_path(expr: &Expr) -> Option<syn::Path> {
    match expr {
        Expr::Path(path) => {
            // Type-qualified paths go to passthrough
            if path.qself.is_some() {
                return None;
            }

            if let Some(ident) = path.path.get_ident() {
                // Single-segment uppercase path
                if is_constant_identifier(&ident.to_string()) {
                    return Some(path.path.clone());
                }
            } else if let Some(last_seg) = path.path.segments.last() {
                // Multi-segment path - check if last segment is uppercase
                if is_constant_identifier(&last_seg.ident.to_string()) {
                    return Some(path.path.clone());
                }
            }
            None
        }
        // Unwrap references
        Expr::Reference(r) => extract_constant_path(&r.expr),
        // Handle method calls on constants: CONSTANT.as_bytes(), CONSTANT.as_ref()
        Expr::MethodCall(mc) => extract_constant_path(&mc.receiver),
        _ => None,
    }
}

/// Check if expression is a bare identifier collision.
///
/// Returns true if the expression root is a bare identifier (not a field access like `params.field`)
/// that matches an instruction arg. This is ambiguous because the same name could refer to
/// either the instruction arg or a ctx account.
///
/// Examples:
/// - `owner.as_ref()` with instruction_args={"owner"} -> true (ambiguous)
/// - `params.owner.as_ref()` with instruction_args={"params"} -> false (clearly instruction data)
/// - `owner.key().as_ref()` with instruction_args={"owner"} -> true (ambiguous)
fn is_bare_identifier_collision(expr: &Expr, root: &Ident) -> bool {
    // Unwrap method calls and references to find the base expression
    let base = unwrap_to_base(expr);

    // Check if the base is a bare identifier (single-segment path)
    matches!(base, Expr::Path(path) if path.path.get_ident().is_some_and(|id| id == root))
}

/// Unwrap method calls, references, and other wrappers to find the base expression.
fn unwrap_to_base(expr: &Expr) -> &Expr {
    match expr {
        Expr::MethodCall(mc) => unwrap_to_base(&mc.receiver),
        Expr::Reference(r) => unwrap_to_base(&r.expr),
        Expr::Paren(p) => unwrap_to_base(&p.expr),
        _ => expr,
    }
}

/// Get the root instruction arg identifier if expression is rooted in one.
/// Returns the instruction arg name (e.g., "params", "owner", "data").
fn get_instruction_arg_root(expr: &Expr, instruction_args: &InstructionArgSet) -> Option<Ident> {
    match expr {
        // Bare identifier: owner, amount (Format 2)
        Expr::Path(path) => {
            if let Some(ident) = path.path.get_ident() {
                let name = ident.to_string();
                // Skip uppercase (constants) and check if it's an instruction arg
                if !is_constant_identifier(&name) && instruction_args.contains(&name) {
                    return Some(ident.clone());
                }
            }
            None
        }
        // Field access: params.owner, data.field.nested
        Expr::Field(field) => get_instruction_arg_root(&field.base, instruction_args),
        // Method call: params.owner.as_ref(), owner.to_le_bytes()
        Expr::MethodCall(mc) => get_instruction_arg_root(&mc.receiver, instruction_args),
        // Index: params.arrays[0]
        Expr::Index(idx) => get_instruction_arg_root(&idx.expr, instruction_args),
        // Reference: &params.owner
        Expr::Reference(r) => get_instruction_arg_root(&r.expr, instruction_args),
        _ => None,
    }
}

/// Get the root ctx account identifier if expression is rooted in one.
/// Returns the account name (e.g., "authority", "owner").
///
/// For field chains like `ctx.accounts.authority` or `authority.key()`, this extracts
/// the terminal field name that corresponds to an account in the Context struct.
/// This is intentional - we want the account name, not an intermediate field like "accounts".
fn get_ctx_account_root(expr: &Expr) -> Option<Ident> {
    match expr {
        // Bare identifier (not uppercase): authority, owner
        Expr::Path(path) => {
            if let Some(ident) = path.path.get_ident() {
                let name = ident.to_string();
                // Skip uppercase (constants)
                if !is_constant_identifier(&name) {
                    return Some(ident.clone());
                }
            }
            None
        }
        // Field access: authority.key, ctx.accounts.authority
        Expr::Field(field) => {
            // First check if terminal member is named
            if let syn::Member::Named(field_name) = &field.member {
                // If base is a simple path (like ctx.accounts), return the field
                // Otherwise recurse into the base
                match &*field.base {
                    Expr::Path(_) => Some(field_name.clone()),
                    Expr::Field(_) => {
                        // For ctx.accounts.authority - take terminal field "authority"
                        // This is correct: we want the account name in the Context, not "accounts"
                        Some(field_name.clone())
                    }
                    _ => get_ctx_account_root(&field.base),
                }
            } else {
                None
            }
        }
        // Method call: authority.key().as_ref()
        Expr::MethodCall(mc) => get_ctx_account_root(&mc.receiver),
        // Reference: &authority.key()
        Expr::Reference(r) => get_ctx_account_root(&r.expr),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    fn make_instruction_args(names: &[&str]) -> InstructionArgSet {
        InstructionArgSet::from_names(names.iter().map(|s| s.to_string()))
    }

    #[test]
    fn test_bare_pubkey_instruction_arg() {
        // Format 2: bare instruction arg "owner" should be DataRooted
        let args = make_instruction_args(&["owner", "amount"]);
        let expr: syn::Expr = parse_quote!(owner);
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(result, ClassifiedSeed::DataRooted { root, .. } if root == "owner"));
    }

    #[test]
    fn test_bare_primitive_with_to_le_bytes() {
        // Format 2: amount.to_le_bytes() should be DataRooted with root "amount"
        let args = make_instruction_args(&["amount"]);
        let expr: syn::Expr = parse_quote!(amount.to_le_bytes().as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(
            result,
            ClassifiedSeed::DataRooted { root, .. } if root == "amount"
        ));
    }

    #[test]
    fn test_custom_struct_param_name() {
        // Custom param name "input" - should be DataRooted with root "input"
        let args = make_instruction_args(&["input"]);
        let expr: syn::Expr = parse_quote!(input.owner.as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(result, ClassifiedSeed::DataRooted { root, .. } if root == "input"));
    }

    #[test]
    fn test_nested_field_access() {
        // data.inner.key should be DataRooted with root "data"
        let args = make_instruction_args(&["data"]);
        let expr: syn::Expr = parse_quote!(data.inner.key.as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(result, ClassifiedSeed::DataRooted { root, .. } if root == "data"));
    }

    #[test]
    fn test_context_account_not_confused_with_arg() {
        let args = make_instruction_args(&["owner"]); // "authority" is NOT an arg
        let expr: syn::Expr = parse_quote!(authority.key().as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(
            result,
            ClassifiedSeed::CtxRooted { account, .. } if account == "authority"
        ));
    }

    #[test]
    fn test_empty_instruction_args() {
        let args = InstructionArgSet::empty();
        let expr: syn::Expr = parse_quote!(owner);
        let result = classify_seed_expr(&expr, &args).unwrap();
        // Without instruction args, bare ident treated as ctx account
        assert!(matches!(result, ClassifiedSeed::CtxRooted { account, .. } if account == "owner"));
    }

    #[test]
    fn test_literal_seed() {
        let args = InstructionArgSet::empty();
        let expr: syn::Expr = parse_quote!(b"seed");
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(result, ClassifiedSeed::Literal(bytes) if bytes == b"seed"));
    }

    #[test]
    fn test_constant_seed() {
        let args = InstructionArgSet::empty();
        let expr: syn::Expr = parse_quote!(SEED_PREFIX);
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(result, ClassifiedSeed::Constant { .. }));
    }

    #[test]
    fn test_standard_params_field_access() {
        // Traditional format: #[instruction(params: CreateParams)]
        let args = make_instruction_args(&["params"]);
        let expr: syn::Expr = parse_quote!(params.owner.as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(result, ClassifiedSeed::DataRooted { root, .. } if root == "params"));
    }

    #[test]
    fn test_args_naming_format() {
        // Alternative naming: #[instruction(args: MyArgs)]
        let args = make_instruction_args(&["args"]);
        let expr: syn::Expr = parse_quote!(args.key.as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(result, ClassifiedSeed::DataRooted { root, .. } if root == "args"));
    }

    #[test]
    fn test_data_naming_format() {
        // Alternative naming: #[instruction(data: DataInput)]
        let args = make_instruction_args(&["data"]);
        let expr: syn::Expr = parse_quote!(data.value.to_le_bytes().as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(
            result,
            ClassifiedSeed::DataRooted { root, .. } if root == "data"
        ));
    }

    #[test]
    fn test_format2_multiple_params() {
        // Format 2: #[instruction(owner: Pubkey, amount: u64)]
        let args = make_instruction_args(&["owner", "amount"]);

        let expr1: syn::Expr = parse_quote!(owner.as_ref());
        let result1 = classify_seed_expr(&expr1, &args).unwrap();
        assert!(matches!(result1, ClassifiedSeed::DataRooted { root, .. } if root == "owner"));

        let expr2: syn::Expr = parse_quote!(amount.to_le_bytes().as_ref());
        let result2 = classify_seed_expr(&expr2, &args).unwrap();
        assert!(matches!(
            result2,
            ClassifiedSeed::DataRooted { root, .. } if root == "amount"
        ));
    }

    #[test]
    fn test_passthrough_for_complex_expressions() {
        // Type-qualified paths should become Passthrough
        let args = InstructionArgSet::empty();
        let expr: syn::Expr = parse_quote!(<Type as Trait>::CONST);
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(result, ClassifiedSeed::Passthrough(_)));
    }

    #[test]
    fn test_passthrough_for_generic_function_call() {
        // Complex function calls with no dynamic args should become Passthrough
        let args = InstructionArgSet::empty();
        let expr: syn::Expr = parse_quote!(identity_seed::<12>(b"seed"));
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(result, ClassifiedSeed::Passthrough(_)));
    }

    #[test]
    fn test_function_call_with_data_args() {
        // crate::max_key(&params.key_a, &params.key_b).as_ref() should be FunctionCall
        let args = make_instruction_args(&["params"]);
        let expr: syn::Expr = parse_quote!(crate::max_key(&params.key_a, &params.key_b).as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        match result {
            ClassifiedSeed::FunctionCall {
                args: fn_args,
                has_as_ref,
                ..
            } => {
                assert!(has_as_ref, "Should detect trailing .as_ref()");
                assert_eq!(fn_args.len(), 2, "Should have 2 classified args");
                assert_eq!(fn_args[0].field_name.to_string(), "key_a");
                assert_eq!(fn_args[0].kind, FnArgKind::DataField);
                assert_eq!(fn_args[1].field_name.to_string(), "key_b");
                assert_eq!(fn_args[1].kind, FnArgKind::DataField);
            }
            other => panic!("Expected FunctionCall, got {:?}", other),
        }
    }

    #[test]
    fn test_function_call_with_ctx_args() {
        // some_func(&fee_payer, &authority).as_ref() with no instruction args
        let args = InstructionArgSet::empty();
        let expr: syn::Expr = parse_quote!(some_func(&fee_payer, &authority).as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        match result {
            ClassifiedSeed::FunctionCall {
                args: fn_args,
                has_as_ref,
                ..
            } => {
                assert!(has_as_ref);
                assert_eq!(fn_args.len(), 2);
                assert_eq!(fn_args[0].kind, FnArgKind::CtxAccount);
                assert_eq!(fn_args[1].kind, FnArgKind::CtxAccount);
            }
            other => panic!("Expected FunctionCall, got {:?}", other),
        }
    }

    #[test]
    fn test_function_call_no_dynamic_args_becomes_passthrough() {
        // crate::id().as_ref() -- no dynamic args -> Passthrough
        let args = InstructionArgSet::empty();
        let expr: syn::Expr = parse_quote!(crate::id().as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(
            matches!(result, ClassifiedSeed::Passthrough(_)),
            "No-arg function call should be Passthrough, got {:?}",
            result
        );
    }

    #[test]
    fn test_constant_method_call_not_function_call() {
        // SeedHolder::NAMESPACE.as_bytes() should be Constant, not FunctionCall
        let args = InstructionArgSet::empty();
        let expr: syn::Expr = parse_quote!(SeedHolder::NAMESPACE.as_bytes());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(
            matches!(result, ClassifiedSeed::Constant { .. }),
            "Method call on constant should be Constant, got {:?}",
            result
        );
    }

    #[test]
    fn test_function_call_mixed_args() {
        // func(&params.key_a, &authority).as_ref() - mixed data + ctx args
        let args = make_instruction_args(&["params"]);
        let expr: syn::Expr = parse_quote!(func(&params.key_a, &authority).as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        match result {
            ClassifiedSeed::FunctionCall {
                args: fn_args,
                has_as_ref,
                ..
            } => {
                assert!(has_as_ref);
                assert_eq!(fn_args.len(), 2);
                assert_eq!(fn_args[0].field_name.to_string(), "key_a");
                assert_eq!(fn_args[0].kind, FnArgKind::DataField);
                assert_eq!(fn_args[1].field_name.to_string(), "authority");
                assert_eq!(fn_args[1].kind, FnArgKind::CtxAccount);
            }
            other => panic!("Expected FunctionCall, got {:?}", other),
        }
    }

    #[test]
    fn test_literal_sliced() {
        // b"literal"[..] - byte literal with full range slice
        let args = InstructionArgSet::empty();
        let expr: syn::Expr = parse_quote!(b"literal"[..]);
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(matches!(result, ClassifiedSeed::Literal(bytes) if bytes == b"literal"));
    }

    #[test]
    fn test_constant_qualified() {
        // crate::path::CONSTANT - qualified constant path
        let args = InstructionArgSet::empty();
        let expr: syn::Expr = parse_quote!(crate::state::SEED_CONSTANT);
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(
            matches!(result, ClassifiedSeed::Constant { path, .. } if path.segments.last().unwrap().ident == "SEED_CONSTANT")
        );
    }

    #[test]
    fn test_ctx_account_nested() {
        // ctx.accounts.authority.key().as_ref() - nested ctx account access
        // The macro extracts the terminal field "authority" as the account root
        let args = InstructionArgSet::empty();
        let expr: syn::Expr = parse_quote!(ctx.accounts.authority.key().as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(
            matches!(result, ClassifiedSeed::CtxRooted { account, .. } if account == "authority")
        );
    }

    #[test]
    fn test_ctx_account_root_terminal_extraction() {
        // Verifies that get_ctx_account_root() correctly extracts the terminal field name
        // which corresponds to the account name in the Context struct

        let args = InstructionArgSet::empty();

        // Case 1: ctx.accounts.authority.key().as_ref() -> "authority"
        let expr1: syn::Expr = parse_quote!(ctx.accounts.authority.key().as_ref());
        let result1 = get_ctx_account_root(&expr1);
        assert_eq!(result1.as_ref().map(|i| i.to_string()).as_deref(), Some("authority"));

        // Case 2: authority.key().as_ref() -> "authority"
        let expr2: syn::Expr = parse_quote!(authority.key().as_ref());
        let result2 = get_ctx_account_root(&expr2);
        assert_eq!(result2.as_ref().map(|i| i.to_string()).as_deref(), Some("authority"));

        // Case 3: ctx.accounts.authority -> "authority"
        let expr3: syn::Expr = parse_quote!(ctx.accounts.authority);
        let result3 = get_ctx_account_root(&expr3);
        assert_eq!(result3.as_ref().map(|i| i.to_string()).as_deref(), Some("authority"));

        // Case 4: Verify it integrates correctly with classify_seed_expr
        let expr4: syn::Expr = parse_quote!(authority.key().as_ref());
        let classified = classify_seed_expr(&expr4, &args).unwrap();
        assert!(matches!(classified, ClassifiedSeed::CtxRooted { account, .. } if account == "authority"));
    }

    #[test]
    fn test_bare_identifier_collision_error() {
        // When a bare identifier matches an instruction arg AND could be a ctx account,
        // we should get an error because the intent is ambiguous.
        //
        // Example scenario:
        //   #[instruction(authority: Pubkey)]
        //   pub struct MyAccounts<'info> {
        //       pub authority: Signer<'info>,  // Same name as instruction arg!
        //   }
        //   seeds = [authority.as_ref()]  // Which "authority"?

        let args = make_instruction_args(&["authority"]);

        // Bare identifier with method call - should error
        let expr: syn::Expr = parse_quote!(authority.as_ref());
        let result = classify_seed_expr(&expr, &args);
        assert!(result.is_err(), "Expected error for ambiguous bare identifier");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Ambiguous seed"),
            "Error should mention ambiguity: {}",
            err
        );
    }

    #[test]
    fn test_field_access_no_collision() {
        // Field access like params.authority is NOT ambiguous - clearly instruction data
        let args = make_instruction_args(&["params"]);

        let expr: syn::Expr = parse_quote!(params.authority.as_ref());
        let result = classify_seed_expr(&expr, &args).unwrap();
        assert!(
            matches!(result, ClassifiedSeed::DataRooted { root, .. } if root == "params"),
            "Field access should be DataRooted without error"
        );
    }

    #[test]
    fn test_is_bare_identifier_collision() {
        // Test the helper function directly
        let authority = syn::Ident::new("authority", proc_macro2::Span::call_site());

        // Bare identifier - is collision
        let expr1: syn::Expr = parse_quote!(authority);
        assert!(is_bare_identifier_collision(&expr1, &authority));

        // Bare identifier with method - is collision
        let expr2: syn::Expr = parse_quote!(authority.as_ref());
        assert!(is_bare_identifier_collision(&expr2, &authority));

        // Field access - not a collision (clearly instruction data)
        let params = syn::Ident::new("params", proc_macro2::Span::call_site());
        let expr3: syn::Expr = parse_quote!(params.authority);
        assert!(!is_bare_identifier_collision(&expr3, &params));

        // Nested field access - not a collision
        let expr4: syn::Expr = parse_quote!(params.inner.authority.as_ref());
        assert!(!is_bare_identifier_collision(&expr4, &params));
    }
}
