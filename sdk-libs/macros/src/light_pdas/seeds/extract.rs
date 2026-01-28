//! Seed extraction from Anchor account attributes.
//!
//! This module handles parsing `#[account(seeds = [...], bump)]` attributes
//! and extracting field information from Accounts structs.

use syn::ItemStruct;

use super::anchor_extraction::extract_anchor_seeds;
use super::instruction_args::parse_instruction_arg_names;
use super::types::SeedSpec;
use crate::light_pdas::account::validation::AccountTypeError;

/// Extract inner type from `Account<'info, T>`, `Box<Account<'info, T>>`,
/// `AccountLoader<'info, T>`, or `InterfaceAccount<'info, T>`.
///
/// Returns `(is_boxed, inner_type)` preserving the full type path.
///
/// # Errors
/// - `AccountTypeError::WrongType` if the type is not a recognized account wrapper
/// - `AccountTypeError::NestedBox` if nested Box<Box<...>> is detected
/// - `AccountTypeError::ExtractionFailed` if generic arguments couldn't be extracted
pub fn extract_account_inner_type(
    ty: &syn::Type,
) -> Result<(bool, syn::Type), AccountTypeError> {
    crate::light_pdas::account::seed_extraction::extract_account_inner_type(ty)
}

/// Check if a field has `#[light_account(init)]` attribute (PDA type).
///
/// Returns `(is_pda, is_zero_copy)`.
pub fn check_light_account_init(attrs: &[syn::Attribute]) -> (bool, bool) {
    for attr in attrs {
        if attr.path().is_ident("light_account") {
            let tokens = match &attr.meta {
                syn::Meta::List(list) => list.tokens.clone(),
                _ => continue,
            };

            let token_vec: Vec<_> = tokens.into_iter().collect();

            // Check for namespace prefixes (mint::, token::, associated_token::)
            let has_namespace_prefix = |namespace: &str| {
                token_vec.windows(2).any(|window| {
                    matches!(
                        (&window[0], &window[1]),
                        (
                            proc_macro2::TokenTree::Ident(ident),
                            proc_macro2::TokenTree::Punct(punct)
                        ) if ident == namespace && punct.as_char() == ':'
                    )
                })
            };

            let has_mint = has_namespace_prefix("mint");
            let has_token = has_namespace_prefix("token");
            let has_ata = has_namespace_prefix("associated_token");

            // Check for init keyword
            let has_init = token_vec
                .iter()
                .any(|t| matches!(t, proc_macro2::TokenTree::Ident(ident) if ident == "init"));

            // Check for zero_copy keyword
            let has_zero_copy = token_vec
                .iter()
                .any(|t| matches!(t, proc_macro2::TokenTree::Ident(ident) if ident == "zero_copy"));

            // Only return true for plain init (no namespace prefix)
            if has_init && !has_mint && !has_token && !has_ata {
                return (true, has_zero_copy);
            }
        }
    }
    (false, false)
}

/// Extract all PDA seed specs from an Accounts struct.
///
/// Returns a vector of `SeedSpec` for each field with `#[light_account(init)]`.
pub fn extract_seed_specs(item: &ItemStruct) -> syn::Result<Vec<SeedSpec>> {
    let fields = match &item.fields {
        syn::Fields::Named(named) => &named.named,
        _ => return Ok(Vec::new()),
    };

    // Parse instruction args from struct attributes
    let instruction_args = parse_instruction_arg_names(&item.attrs)?;

    let mut specs = Vec::new();

    for field in fields {
        let field_ident = match &field.ident {
            Some(id) => id.clone(),
            None => continue,
        };

        // Check for #[light_account(init)]
        let (is_pda, is_zero_copy) = check_light_account_init(&field.attrs);
        if !is_pda {
            continue;
        }

        // Extract inner type
        let (_, inner_type) = extract_account_inner_type(&field.ty)
            .map_err(|e| e.into_syn_error(&field.ty))?;

        // Extract seeds using the anchor extraction
        let seeds = extract_anchor_seeds(&field.attrs, &instruction_args)?;

        specs.push(SeedSpec::new(field_ident, inner_type, seeds, is_zero_copy));
    }

    Ok(specs)
}

#[cfg(test)]
mod tests {
    use super::super::instruction_args::InstructionArgSet;
    use super::super::types::ClassifiedSeed;
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_parse_instruction_arg_names_format1() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[instruction(params: CreateParams)])];
        let arg_set = parse_instruction_arg_names(&attrs).expect("should parse");
        assert!(arg_set.names.contains("params"));
        assert_eq!(arg_set.names.len(), 1);
    }

    #[test]
    fn test_parse_instruction_arg_names_format2() {
        let attrs: Vec<syn::Attribute> =
            vec![parse_quote!(#[instruction(owner: Pubkey, amount: u64)])];
        let arg_set = parse_instruction_arg_names(&attrs).expect("should parse");
        assert!(arg_set.names.contains("owner"));
        assert!(arg_set.names.contains("amount"));
        assert_eq!(arg_set.names.len(), 2);
    }

    #[test]
    fn test_parse_instruction_arg_names_empty() {
        let attrs: Vec<syn::Attribute> = vec![];
        let arg_set = parse_instruction_arg_names(&attrs).expect("should parse");
        assert!(arg_set.names.is_empty());
    }

    #[test]
    fn test_extract_anchor_seeds() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(
            #[account(
                init,
                payer = fee_payer,
                space = 100,
                seeds = [b"seed", authority.key().as_ref()],
                bump
            )]
        )];

        let arg_set = InstructionArgSet::empty();

        let seeds = extract_anchor_seeds(&attrs, &arg_set).expect("should extract");

        assert_eq!(seeds.len(), 2);
        assert!(matches!(seeds[0], ClassifiedSeed::Literal(_)));
        assert!(matches!(seeds[1], ClassifiedSeed::CtxRooted { .. }));
    }

    #[test]
    fn test_extract_account_inner_type() {
        let ty: syn::Type = parse_quote!(Account<'info, UserRecord>);
        let result = extract_account_inner_type(&ty);
        assert!(result.is_ok(), "Should extract Account inner type");
        let (is_boxed, inner) = result.unwrap();
        assert!(!is_boxed);

        if let syn::Type::Path(path) = inner {
            assert_eq!(
                path.path.segments.last().unwrap().ident.to_string(),
                "UserRecord"
            );
        } else {
            panic!("Expected path type");
        }
    }

    #[test]
    fn test_extract_account_inner_type_boxed() {
        let ty: syn::Type = parse_quote!(Box<Account<'info, UserRecord>>);
        let result = extract_account_inner_type(&ty);
        assert!(result.is_ok(), "Should extract Box<Account> inner type");
        let (is_boxed, inner) = result.unwrap();
        assert!(is_boxed);

        if let syn::Type::Path(path) = inner {
            assert_eq!(
                path.path.segments.last().unwrap().ident.to_string(),
                "UserRecord"
            );
        } else {
            panic!("Expected path type");
        }
    }

    #[test]
    fn test_extract_account_inner_type_nested_box_fails() {
        use crate::light_pdas::account::validation::AccountTypeError;
        let ty: syn::Type = parse_quote!(Box<Box<Account<'info, UserRecord>>>);
        let result = extract_account_inner_type(&ty);
        assert!(
            matches!(result, Err(AccountTypeError::NestedBox)),
            "Nested Box should return NestedBox error"
        );
    }

    #[test]
    fn test_extract_account_inner_type_wrong_type_fails() {
        use crate::light_pdas::account::validation::AccountTypeError;
        let ty: syn::Type = parse_quote!(String);
        let result = extract_account_inner_type(&ty);
        assert!(
            matches!(result, Err(AccountTypeError::WrongType { .. })),
            "Wrong type should return WrongType error"
        );
    }

    #[test]
    fn test_check_light_account_init() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[light_account(init)])];
        let (is_pda, is_zero_copy) = check_light_account_init(&attrs);
        assert!(is_pda);
        assert!(!is_zero_copy);
    }

    #[test]
    fn test_check_light_account_init_zero_copy() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[light_account(init, zero_copy)])];
        let (is_pda, is_zero_copy) = check_light_account_init(&attrs);
        assert!(is_pda);
        assert!(is_zero_copy);
    }

    #[test]
    fn test_check_light_account_init_mint_namespace() {
        // mint:: namespace should NOT be detected as PDA
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(
            #[light_account(init, mint::authority = authority)]
        )];
        let (is_pda, _) = check_light_account_init(&attrs);
        assert!(!is_pda);
    }

    #[test]
    fn test_full_extraction_create_example() {
        // Full pipeline test with the example from issue
        let item: syn::ItemStruct = parse_quote!(
            #[derive(Accounts, LightAccounts)]
            #[instruction(params: CreateParams)]
            pub struct Create<'info> {
                #[account(mut)]
                pub fee_payer: Signer<'info>,

                #[account(
                    init,
                    payer = fee_payer,
                    space = 100,
                    seeds = [b"user", SEED_PREFIX, authority.key().as_ref(), params.owner.as_ref()],
                    bump
                )]
                #[light_account(init)]
                pub user_record: Account<'info, UserRecord>,
            }
        );

        // Step 1: Parse instruction args from struct attributes
        let instruction_args = parse_instruction_arg_names(&item.attrs)
            .expect("should parse instruction args");
        assert!(instruction_args.contains("params"));

        // Step 2: Use full extraction
        let specs = extract_seed_specs(&item).expect("should extract seed specs");
        assert_eq!(specs.len(), 1, "Should have one PDA field");

        let spec = &specs[0];
        assert_eq!(spec.field_name.to_string(), "user_record");
        assert!(!spec.is_zero_copy);
        assert_eq!(spec.seeds.len(), 4, "Should have 4 seeds");

        // Verify seed classification
        assert!(matches!(spec.seeds[0], ClassifiedSeed::Literal(_)), "Seed 0: Literal b\"user\"");
        assert!(matches!(spec.seeds[1], ClassifiedSeed::Constant { .. }), "Seed 1: Constant SEED_PREFIX");
        assert!(matches!(spec.seeds[2], ClassifiedSeed::CtxRooted { .. }), "Seed 2: CtxRooted authority");
        assert!(matches!(spec.seeds[3], ClassifiedSeed::DataRooted { .. }), "Seed 3: DataRooted params.owner");
    }

    #[test]
    fn test_edge_case_empty_seeds() {
        // Empty seeds array should return no seeds
        let item: syn::ItemStruct = parse_quote!(
            #[derive(Accounts)]
            pub struct Test<'info> {
                #[account(seeds = [], bump)]
                #[light_account(init)]
                pub account: Account<'info, MyType>,
            }
        );

        let specs = extract_seed_specs(&item).expect("should extract");
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].seeds.len(), 0);
    }

    #[test]
    fn test_edge_case_no_instruction_attribute() {
        // No #[instruction] attribute - instruction_args should be empty
        // When no instruction args present, the classification follows this path:
        // - params (base) is NOT in instruction_args (empty set) -> falls through
        // - params is checked as ctx account root -> returns Some("params")
        // - But for nested access like params.owner, get_ctx_account_root extracts "owner"
        //   as the terminal field name
        let item: syn::ItemStruct = parse_quote!(
            #[derive(Accounts)]
            pub struct Test<'info> {
                #[account(
                    init,
                    seeds = [b"seed", params.owner.as_ref()],
                    bump
                )]
                #[light_account(init)]
                pub account: Account<'info, MyType>,
            }
        );

        let specs = extract_seed_specs(&item).expect("should extract");
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].seeds.len(), 2);

        // With no instruction args, params.owner should be classified as CtxRooted
        // The account root is extracted as the terminal field "owner" from params.owner
        match &specs[0].seeds[1] {
            ClassifiedSeed::CtxRooted { account } => {
                assert_eq!(account.to_string(), "owner");
            }
            other => panic!("Expected CtxRooted, got {:?}", other),
        }
    }

    #[test]
    fn test_edge_case_field_without_light_account_init() {
        // Field without #[light_account(init)] should be skipped
        let item: syn::ItemStruct = parse_quote!(
            #[derive(Accounts)]
            pub struct Test<'info> {
                #[account(
                    init,
                    seeds = [b"seed"],
                    bump
                )]
                pub regular_field: Account<'info, MyType>,

                #[account(
                    init,
                    seeds = [b"pda"],
                    bump
                )]
                #[light_account(init)]
                pub pda_field: Account<'info, MyType>,
            }
        );

        let specs = extract_seed_specs(&item).expect("should extract");
        assert_eq!(specs.len(), 1, "Should only extract PDA field");
        assert_eq!(specs[0].field_name.to_string(), "pda_field");
    }

    #[test]
    fn test_edge_case_multiple_light_account_fields() {
        // Multiple #[light_account(init)] fields should all be extracted
        let item: syn::ItemStruct = parse_quote!(
            #[derive(Accounts)]
            pub struct Test<'info> {
                #[account(init, seeds = [b"pda1"], bump)]
                #[light_account(init)]
                pub pda_field1: Account<'info, Type1>,

                #[account(init, seeds = [b"pda2"], bump)]
                #[light_account(init)]
                pub pda_field2: Account<'info, Type2>,
            }
        );

        let specs = extract_seed_specs(&item).expect("should extract");
        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].field_name.to_string(), "pda_field1");
        assert_eq!(specs[1].field_name.to_string(), "pda_field2");
    }

    #[test]
    fn test_edge_case_zero_copy_field() {
        // #[light_account(init, zero_copy)] should be detected
        let item: syn::ItemStruct = parse_quote!(
            #[derive(Accounts)]
            pub struct Test<'info> {
                #[account(init, seeds = [b"pda"], bump)]
                #[light_account(init, zero_copy)]
                pub account: Account<'info, MyType>,
            }
        );

        let specs = extract_seed_specs(&item).expect("should extract");
        assert_eq!(specs.len(), 1);
        assert!(specs[0].is_zero_copy);
    }
}
