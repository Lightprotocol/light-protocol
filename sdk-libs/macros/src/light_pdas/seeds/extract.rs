//! Seed extraction from Anchor account attributes.
//!
//! This module handles parsing `#[account(seeds = [...], bump)]` attributes
//! and extracting field information from Accounts structs.

use std::collections::HashSet;

use syn::{Expr, Ident, ItemStruct};

use super::types::SeedSpec;
use crate::light_pdas::account::seed_extraction::{
    classify_seed_expr, ClassifiedSeed, InstructionArgSet,
};

/// Parse `#[instruction(...)]` attribute and return instruction argument names.
///
/// Supports two formats:
/// - Format 1: `#[instruction(params: CreateParams)]` -> returns `{"params"}`
/// - Format 2: `#[instruction(owner: Pubkey, amount: u64)]` -> returns `{"owner", "amount"}`
pub fn parse_instruction_args(attrs: &[syn::Attribute]) -> syn::Result<HashSet<String>> {
    let arg_set = crate::light_pdas::account::seed_extraction::parse_instruction_arg_names(attrs)?;
    Ok(arg_set.names)
}

/// Extract account field names from an Accounts struct.
///
/// Returns a set of field names that can be used as account references in seeds.
pub fn extract_account_fields(item: &ItemStruct) -> HashSet<String> {
    let mut fields = HashSet::new();

    if let syn::Fields::Named(named) = &item.fields {
        for field in &named.named {
            if let Some(ident) = &field.ident {
                fields.insert(ident.to_string());
            }
        }
    }

    fields
}

/// Extract seeds from `#[account(seeds = [...], bump)]` attribute.
///
/// Returns a vector of classified seeds, or an empty vector if no seeds found.
pub fn extract_seeds_from_attribute(
    attrs: &[syn::Attribute],
    instruction_args: &HashSet<String>,
    _account_fields: &HashSet<String>,
) -> syn::Result<Vec<ClassifiedSeed>> {
    // Convert HashSet<String> to InstructionArgSet for the unified classifier
    let arg_set = InstructionArgSet::from_names(instruction_args.iter().cloned());

    for attr in attrs {
        if !attr.path().is_ident("account") {
            continue;
        }

        let tokens = match &attr.meta {
            syn::Meta::List(list) => list.tokens.clone(),
            _ => continue,
        };

        // Parse as comma-separated key-value pairs
        let parsed: syn::Result<syn::punctuated::Punctuated<AccountAttrItem, syn::Token![,]>> =
            syn::parse::Parser::parse2(
                syn::punctuated::Punctuated::parse_terminated,
                tokens.clone(),
            );

        if let Ok(items) = &parsed {
            for item in items {
                if item.key == "seeds" {
                    return classify_seeds_array(&item.value, &arg_set);
                }
            }
        }
    }

    Ok(Vec::new())
}

/// Helper struct for parsing account attribute items.
struct AccountAttrItem {
    key: Ident,
    value: Expr,
}

impl syn::parse::Parse for AccountAttrItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // Handle keywords like `mut` as well as identifiers
        let key: Ident = if input.peek(syn::Token![mut]) {
            input.parse::<syn::Token![mut]>()?;
            Ident::new("mut", proc_macro2::Span::call_site())
        } else {
            input.parse()?
        };

        // Handle bare identifiers like `mut`, `init`, `bump`
        if !input.peek(syn::Token![=]) {
            return Ok(AccountAttrItem {
                key: key.clone(),
                value: syn::parse_quote!(true),
            });
        }

        input.parse::<syn::Token![=]>()?;
        let value: Expr = input.parse()?;

        Ok(AccountAttrItem { key, value })
    }
}

/// Classify seeds from an array expression `[seed1, seed2, ...]`.
fn classify_seeds_array(
    expr: &Expr,
    instruction_args: &InstructionArgSet,
) -> syn::Result<Vec<ClassifiedSeed>> {
    let array = match expr {
        Expr::Array(arr) => arr,
        Expr::Reference(r) => {
            if let Expr::Array(arr) = &*r.expr {
                arr
            } else {
                return Err(syn::Error::new_spanned(expr, "Expected seeds array"));
            }
        }
        _ => return Err(syn::Error::new_spanned(expr, "Expected seeds array")),
    };

    let mut seeds = Vec::new();
    for elem in &array.elems {
        seeds.push(classify_seed_expr(elem, instruction_args)?);
    }

    Ok(seeds)
}

/// Extract inner type from `Account<'info, T>`, `Box<Account<'info, T>>`,
/// `AccountLoader<'info, T>`, or `InterfaceAccount<'info, T>`.
///
/// Returns `(is_boxed, inner_type)` preserving the full type path.
pub fn extract_account_inner_type(ty: &syn::Type) -> Option<(bool, syn::Type)> {
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
    let instruction_args = parse_instruction_args(&item.attrs)?;

    // Get all account field names (used as context for classify_seed, but
    // classify_seed_expr treats unknown idents as CtxRooted, so this
    // gives the same result)
    let account_fields = extract_account_fields(item);

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
        let (_, inner_type) = match extract_account_inner_type(&field.ty) {
            Some(result) => result,
            None => {
                return Err(syn::Error::new_spanned(
                    &field.ty,
                    "#[light_account(init)] requires Account<'info, T> or Box<Account<'info, T>>",
                ));
            }
        };

        // Extract seeds using the unified classifier
        let seeds = extract_seeds_from_attribute(&field.attrs, &instruction_args, &account_fields)?;

        specs.push(SeedSpec::new(field_ident, inner_type, seeds, is_zero_copy));
    }

    Ok(specs)
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_parse_instruction_args_format1() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(#[instruction(params: CreateParams)])];
        let args = parse_instruction_args(&attrs).expect("should parse");
        assert!(args.contains("params"));
        assert_eq!(args.len(), 1);
    }

    #[test]
    fn test_parse_instruction_args_format2() {
        let attrs: Vec<syn::Attribute> =
            vec![parse_quote!(#[instruction(owner: Pubkey, amount: u64)])];
        let args = parse_instruction_args(&attrs).expect("should parse");
        assert!(args.contains("owner"));
        assert!(args.contains("amount"));
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn test_parse_instruction_args_empty() {
        let attrs: Vec<syn::Attribute> = vec![];
        let args = parse_instruction_args(&attrs).expect("should parse");
        assert!(args.is_empty());
    }

    #[test]
    fn test_extract_account_fields() {
        let item: ItemStruct = parse_quote! {
            pub struct MyAccounts<'info> {
                pub fee_payer: Signer<'info>,
                pub authority: Signer<'info>,
                pub record: Account<'info, Record>,
            }
        };

        let fields = extract_account_fields(&item);
        assert!(fields.contains("fee_payer"));
        assert!(fields.contains("authority"));
        assert!(fields.contains("record"));
        assert_eq!(fields.len(), 3);
    }

    #[test]
    fn test_extract_seeds_from_attribute() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(
            #[account(
                init,
                payer = fee_payer,
                space = 100,
                seeds = [b"seed", authority.key().as_ref()],
                bump
            )]
        )];

        let instruction_args = HashSet::new();
        let account_fields: HashSet<String> = ["authority".to_string()].into();

        let seeds = extract_seeds_from_attribute(&attrs, &instruction_args, &account_fields)
            .expect("should extract");

        assert_eq!(seeds.len(), 2);
        assert!(matches!(seeds[0], ClassifiedSeed::Literal(_)));
        assert!(matches!(seeds[1], ClassifiedSeed::CtxRooted { .. }));
    }

    #[test]
    fn test_extract_account_inner_type() {
        let ty: syn::Type = parse_quote!(Account<'info, UserRecord>);
        let (is_boxed, inner) = extract_account_inner_type(&ty).expect("should extract");
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
        let (is_boxed, inner) = extract_account_inner_type(&ty).expect("should extract");
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
}
