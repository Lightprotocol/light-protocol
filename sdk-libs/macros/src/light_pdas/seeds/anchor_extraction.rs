//! Anchor seed extraction from `#[account(seeds = [...], bump)]` attributes.
//!
//! This module extracts PDA seeds from Anchor's attribute syntax and classifies them
//! into the categories needed for compression.

use syn::{Expr, Ident};

use super::classification::classify_seed_expr;
use super::instruction_args::InstructionArgSet;
use super::types::ClassifiedSeed;

/// Extract seeds from #[account(seeds = [...], bump)] attribute
pub fn extract_anchor_seeds(
    attrs: &[syn::Attribute],
    instruction_args: &InstructionArgSet,
) -> syn::Result<Vec<ClassifiedSeed>> {
    for attr in attrs {
        if !attr.path().is_ident("account") {
            continue;
        }

        // Parse the attribute as a token stream and look for seeds = [...]
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
                    return classify_seeds_array(&item.value, instruction_args);
                }
            }
        }
    }

    Ok(Vec::new())
}

/// Helper struct for parsing account attribute items
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

/// Classify seeds from an array expression [seed1, seed2, ...]
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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_extract_anchor_seeds_simple() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(
            #[account(
                init,
                payer = fee_payer,
                space = 100,
                seeds = [b"seed", authority.key().as_ref()],
                bump
            )]
        )];

        let instruction_args = InstructionArgSet::empty();
        let seeds = extract_anchor_seeds(&attrs, &instruction_args).expect("should extract");

        assert_eq!(seeds.len(), 2);
        assert!(matches!(seeds[0], ClassifiedSeed::Literal(_)));
        assert!(matches!(seeds[1], ClassifiedSeed::CtxRooted { .. }));
    }

    #[test]
    fn test_extract_anchor_seeds_with_data() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(
            #[account(
                seeds = [b"user", params.owner.as_ref()],
                bump
            )]
        )];

        let instruction_args = InstructionArgSet::from_names(vec!["params".to_string()]);
        let seeds = extract_anchor_seeds(&attrs, &instruction_args).expect("should extract");

        assert_eq!(seeds.len(), 2);
        assert!(matches!(seeds[0], ClassifiedSeed::Literal(_)));
        assert!(matches!(seeds[1], ClassifiedSeed::DataRooted { .. }));
    }

    #[test]
    fn test_extract_anchor_seeds_no_seeds() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(
            #[account(mut)]
        )];

        let instruction_args = InstructionArgSet::empty();
        let seeds = extract_anchor_seeds(&attrs, &instruction_args).expect("should return empty");

        assert!(seeds.is_empty());
    }

    #[test]
    fn test_extract_anchor_seeds_with_constant() {
        let attrs: Vec<syn::Attribute> = vec![parse_quote!(
            #[account(
                seeds = [SEED_PREFIX, user.key().as_ref()],
                bump
            )]
        )];

        let instruction_args = InstructionArgSet::empty();
        let seeds = extract_anchor_seeds(&attrs, &instruction_args).expect("should extract");

        assert_eq!(seeds.len(), 2);
        assert!(matches!(seeds[0], ClassifiedSeed::Constant { .. }));
        assert!(matches!(seeds[1], ClassifiedSeed::CtxRooted { .. }));
    }
}
