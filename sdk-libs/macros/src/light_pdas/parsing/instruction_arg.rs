//! Instruction argument parsing from `#[instruction(...)]` attributes.
//!
//! This module provides:
//! - `InstructionArg` - Full argument with name and type (for code generation)
//! - Re-exports `InstructionArgSet` from seeds/ (for seed classification)
//!
//! Anchor supports two formats for `#[instruction(...)]`:
//! - Format 1: `#[instruction(params: SomeStruct)]` - users write `params.field`
//! - Format 2: `#[instruction(owner: Pubkey, amount: u64)]` - users write bare `owner`

use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Attribute, Ident, Token, Type,
};

// Re-export InstructionArgSet from seeds (canonical location for seed classification)
pub use crate::light_pdas::seeds::InstructionArgSet;

// ============================================================================
// InstructionArg - Full argument with type
// ============================================================================

/// Full instruction argument with name and type.
///
/// Used by `#[derive(LightAccounts)]` for generating complete function signatures.
#[derive(Debug, Clone)]
pub struct InstructionArg {
    pub name: Ident,
    pub ty: Type,
}

impl Parse for InstructionArg {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty: Type = input.parse()?;
        Ok(Self { name, ty })
    }
}

/// Convert a slice of InstructionArg to InstructionArgSet
pub fn args_to_set(args: &[InstructionArg]) -> InstructionArgSet {
    InstructionArgSet::from_names(args.iter().map(|a| a.name.to_string()))
}

// ============================================================================
// Parsing Functions
// ============================================================================

/// Parse #[instruction(...)] attribute from struct.
///
/// Returns `Ok(None)` if no instruction attribute is present,
/// `Ok(Some(args))` if successfully parsed, or `Err` on malformed syntax.
///
/// This returns the full `InstructionArg` with types for code generation.
pub fn parse_instruction_attr(
    attrs: &[Attribute],
) -> Result<Option<Vec<InstructionArg>>, syn::Error> {
    for attr in attrs {
        if attr.path().is_ident("instruction") {
            let args = attr.parse_args_with(|input: ParseStream| {
                let content: Punctuated<InstructionArg, Token![,]> =
                    Punctuated::parse_terminated(input)?;
                Ok(content.into_iter().collect::<Vec<_>>())
            })?;
            return Ok(Some(args));
        }
    }
    Ok(None)
}

/// Parse #[instruction(...)] and return just the names (for seed classification).
///
/// This is a convenience wrapper that parses instruction arguments and converts
/// them to an `InstructionArgSet` for seed classification. Use this when you only
/// need the argument names, not their types.
pub fn parse_instruction_arg_names(attrs: &[Attribute]) -> Result<InstructionArgSet, syn::Error> {
    match parse_instruction_attr(attrs)? {
        Some(args) => Ok(args_to_set(&args)),
        None => Ok(InstructionArgSet::empty()),
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_parse_instruction_attr() {
        let attrs: Vec<Attribute> = vec![parse_quote!(#[instruction(params: CreateParams)])];
        let args = parse_instruction_attr(&attrs).unwrap();
        assert!(args.is_some());
        let args = args.unwrap();
        assert_eq!(args.len(), 1);
        assert_eq!(args[0].name.to_string(), "params");
    }

    #[test]
    fn test_parse_instruction_attr_multiple() {
        let attrs: Vec<Attribute> =
            vec![parse_quote!(#[instruction(owner: Pubkey, amount: u64, flag: bool)])];
        let args = parse_instruction_attr(&attrs).unwrap();
        assert!(args.is_some());
        let args = args.unwrap();
        assert_eq!(args.len(), 3);
        assert_eq!(args[0].name.to_string(), "owner");
        assert_eq!(args[1].name.to_string(), "amount");
        assert_eq!(args[2].name.to_string(), "flag");
    }

    #[test]
    fn test_parse_instruction_attr_none() {
        let attrs: Vec<Attribute> = vec![parse_quote!(#[derive(Debug)])];
        let args = parse_instruction_attr(&attrs).unwrap();
        assert!(args.is_none());
    }

    #[test]
    fn test_instruction_arg_set_empty() {
        let args = InstructionArgSet::empty();
        assert!(!args.contains("owner"));
        assert!(args.names.is_empty());
    }

    #[test]
    fn test_instruction_arg_set_from_names() {
        let args = InstructionArgSet::from_names(vec!["owner".to_string(), "amount".to_string()]);
        assert!(args.contains("owner"));
        assert!(args.contains("amount"));
        assert!(!args.contains("other"));
    }

    #[test]
    fn test_args_to_set() {
        let args = vec![
            InstructionArg {
                name: parse_quote!(owner),
                ty: parse_quote!(Pubkey),
            },
            InstructionArg {
                name: parse_quote!(amount),
                ty: parse_quote!(u64),
            },
        ];
        let set = args_to_set(&args);
        assert!(set.contains("owner"));
        assert!(set.contains("amount"));
        assert!(!set.contains("other"));
    }
}
