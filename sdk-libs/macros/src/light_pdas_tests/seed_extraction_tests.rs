//! Unit tests for seed extraction and classification.
//!
//! Extracted from `light_pdas/account/seed_extraction.rs`.

use syn::parse_quote;

use crate::light_pdas::account::seed_extraction::{
    check_light_account_type, classify_seed_expr, parse_instruction_arg_names, ClassifiedSeed,
    InstructionArgSet,
};

fn make_instruction_args(names: &[&str]) -> InstructionArgSet {
    InstructionArgSet::from_names(names.iter().map(|s| s.to_string()))
}

#[test]
fn test_bare_pubkey_instruction_arg() {
    let args = make_instruction_args(&["owner", "amount"]);
    let expr: syn::Expr = parse_quote!(owner);
    let result = classify_seed_expr(&expr, &args).unwrap();
    assert!(
        matches!(result, ClassifiedSeed::DataField { field_name, .. } if field_name == "owner")
    );
}

#[test]
fn test_bare_primitive_with_to_le_bytes() {
    let args = make_instruction_args(&["amount"]);
    let expr: syn::Expr = parse_quote!(amount.to_le_bytes().as_ref());
    let result = classify_seed_expr(&expr, &args).unwrap();
    assert!(matches!(
        result,
        ClassifiedSeed::DataField {
            field_name,
            conversion: Some(conv)
        } if field_name == "amount" && conv == "to_le_bytes"
    ));
}

#[test]
fn test_custom_struct_param_name() {
    let args = make_instruction_args(&["input"]);
    let expr: syn::Expr = parse_quote!(input.owner.as_ref());
    let result = classify_seed_expr(&expr, &args).unwrap();
    assert!(
        matches!(result, ClassifiedSeed::DataField { field_name, .. } if field_name == "owner")
    );
}

#[test]
fn test_nested_field_access() {
    let args = make_instruction_args(&["data"]);
    let expr: syn::Expr = parse_quote!(data.inner.key.as_ref());
    let result = classify_seed_expr(&expr, &args).unwrap();
    assert!(matches!(result, ClassifiedSeed::DataField { field_name, .. } if field_name == "key"));
}

#[test]
fn test_context_account_not_confused_with_arg() {
    let args = make_instruction_args(&["owner"]); // "authority" is NOT an arg
    let expr: syn::Expr = parse_quote!(authority.key().as_ref());
    let result = classify_seed_expr(&expr, &args).unwrap();
    assert!(matches!(
        result,
        ClassifiedSeed::CtxAccount(ident) if ident == "authority"
    ));
}

#[test]
fn test_empty_instruction_args() {
    let args = InstructionArgSet::empty();
    let expr: syn::Expr = parse_quote!(owner);
    let result = classify_seed_expr(&expr, &args).unwrap();
    // Without instruction args, bare ident treated as ctx account
    assert!(matches!(result, ClassifiedSeed::CtxAccount(_)));
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
    assert!(matches!(result, ClassifiedSeed::Constant(_)));
}

#[test]
fn test_standard_params_field_access() {
    // Traditional format: #[instruction(params: CreateParams)]
    let args = make_instruction_args(&["params"]);
    let expr: syn::Expr = parse_quote!(params.owner.as_ref());
    let result = classify_seed_expr(&expr, &args).unwrap();
    assert!(
        matches!(result, ClassifiedSeed::DataField { field_name, .. } if field_name == "owner")
    );
}

#[test]
fn test_args_naming_format() {
    // Alternative naming: #[instruction(args: MyArgs)]
    let args = make_instruction_args(&["args"]);
    let expr: syn::Expr = parse_quote!(args.key.as_ref());
    let result = classify_seed_expr(&expr, &args).unwrap();
    assert!(matches!(result, ClassifiedSeed::DataField { field_name, .. } if field_name == "key"));
}

#[test]
fn test_data_naming_format() {
    // Alternative naming: #[instruction(data: DataInput)]
    let args = make_instruction_args(&["data"]);
    let expr: syn::Expr = parse_quote!(data.value.to_le_bytes().as_ref());
    let result = classify_seed_expr(&expr, &args).unwrap();
    assert!(matches!(
        result,
        ClassifiedSeed::DataField {
            field_name,
            conversion: Some(conv)
        } if field_name == "value" && conv == "to_le_bytes"
    ));
}

#[test]
fn test_format2_multiple_params() {
    // Format 2: #[instruction(owner: Pubkey, amount: u64)]
    let args = make_instruction_args(&["owner", "amount"]);

    let expr1: syn::Expr = parse_quote!(owner.as_ref());
    let result1 = classify_seed_expr(&expr1, &args).unwrap();
    assert!(
        matches!(result1, ClassifiedSeed::DataField { field_name, .. } if field_name == "owner")
    );

    let expr2: syn::Expr = parse_quote!(amount.to_le_bytes().as_ref());
    let result2 = classify_seed_expr(&expr2, &args).unwrap();
    assert!(matches!(
        result2,
        ClassifiedSeed::DataField {
            field_name,
            conversion: Some(_)
        } if field_name == "amount"
    ));
}

#[test]
fn test_parse_instruction_arg_names() {
    // Test that we can parse instruction attributes
    let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[instruction(owner: Pubkey)])];
    let args = parse_instruction_arg_names(&attrs).unwrap();
    assert!(args.contains("owner"));
}

#[test]
fn test_parse_instruction_arg_names_multiple() {
    let attrs: Vec<syn::Attribute> =
        vec![parse_quote!(#[instruction(owner: Pubkey, amount: u64, flag: bool)])];
    let args = parse_instruction_arg_names(&attrs).unwrap();
    assert!(args.contains("owner"));
    assert!(args.contains("amount"));
    assert!(args.contains("flag"));
}

#[test]
fn test_check_light_account_type_mint_namespace() {
    // Test that mint:: namespace is detected correctly
    let attrs: Vec<syn::Attribute> = vec![parse_quote!(
        #[light_account(init,
            mint::signer = mint_signer,
            mint::authority = fee_payer,
            mint::decimals = 6
        )]
    )];
    let (has_pda, has_mint, has_ata) = check_light_account_type(&attrs);
    assert!(!has_pda, "Should NOT be detected as PDA");
    assert!(has_mint, "Should be detected as mint");
    assert!(!has_ata, "Should NOT be detected as ATA");
}

#[test]
fn test_check_light_account_type_pda_only() {
    // Test that plain init (no mint::) is detected as PDA
    let attrs: Vec<syn::Attribute> = vec![parse_quote!(
        #[light_account(init)]
    )];
    let (has_pda, has_mint, has_ata) = check_light_account_type(&attrs);
    assert!(has_pda, "Should be detected as PDA");
    assert!(!has_mint, "Should NOT be detected as mint");
    assert!(!has_ata, "Should NOT be detected as ATA");
}

#[test]
fn test_check_light_account_type_token_namespace() {
    // Test that token:: namespace is not detected as mint (it's neither PDA nor mint nor ATA)
    let attrs: Vec<syn::Attribute> = vec![parse_quote!(
        #[light_account(token::authority = [b"auth"])]
    )];
    let (has_pda, has_mint, has_ata) = check_light_account_type(&attrs);
    assert!(!has_pda, "Should NOT be detected as PDA (no init)");
    assert!(!has_mint, "Should NOT be detected as mint");
    assert!(!has_ata, "Should NOT be detected as ATA");
}

#[test]
fn test_check_light_account_type_associated_token_init() {
    // Test that associated_token:: with init is detected as ATA
    let attrs: Vec<syn::Attribute> = vec![parse_quote!(
        #[light_account(init,
            associated_token::authority = owner,
            associated_token::mint = mint
        )]
    )];
    let (has_pda, has_mint, has_ata) = check_light_account_type(&attrs);
    assert!(!has_pda, "Should NOT be detected as PDA");
    assert!(!has_mint, "Should NOT be detected as mint");
    assert!(has_ata, "Should be detected as ATA");
}

#[test]
fn test_check_light_account_type_token_init() {
    // Test that token:: with init is NOT detected as PDA
    let attrs: Vec<syn::Attribute> = vec![parse_quote!(
        #[light_account(init,
            token::authority = [b"vault_auth"],
            token::mint = mint
        )]
    )];
    let (has_pda, has_mint, has_ata) = check_light_account_type(&attrs);
    assert!(!has_pda, "Should NOT be detected as PDA");
    assert!(!has_mint, "Should NOT be detected as mint");
    assert!(!has_ata, "Should NOT be detected as ATA");
}
