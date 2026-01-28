//! Instruction argument parsing from `#[instruction(...)]` attributes.
//!
//! This module extracts instruction argument names from Anchor's attribute syntax
//! to enable proper seed classification.

use std::collections::HashSet;

/// Set of instruction argument names for Format 2 detection.
///
/// Anchor supports two formats for `#[instruction(...)]`:
/// - Format 1: `#[instruction(params: SomeStruct)]` - users write `params.field`
/// - Format 2: `#[instruction(owner: Pubkey, amount: u64)]` - users write bare `owner`
///
/// This struct holds the names from Format 2 so we can recognize them in seed expressions.
#[derive(Clone, Debug, Default)]
pub struct InstructionArgSet {
    /// Names of instruction args (e.g., {"owner", "amount", "bump"})
    pub names: HashSet<String>,
}

impl InstructionArgSet {
    /// Create an empty arg set (used when no #[instruction] attribute present)
    pub fn empty() -> Self {
        Self {
            names: HashSet::new(),
        }
    }

    /// Create from a list of argument names
    pub fn from_names(names: impl IntoIterator<Item = String>) -> Self {
        Self {
            names: names.into_iter().collect(),
        }
    }

    /// Check if a name is a known instruction argument
    pub fn contains(&self, name: &str) -> bool {
        self.names.contains(name)
    }
}

/// Parse #[instruction(...)] attribute from a struct's attributes and return InstructionArgSet
pub fn parse_instruction_arg_names(attrs: &[syn::Attribute]) -> syn::Result<InstructionArgSet> {
    for attr in attrs {
        if attr.path().is_ident("instruction") {
            let content = attr.parse_args_with(|input: syn::parse::ParseStream| {
                let args: syn::punctuated::Punctuated<InstructionArg, syn::Token![,]> =
                    syn::punctuated::Punctuated::parse_terminated(input)?;
                Ok(args
                    .into_iter()
                    .map(|a| a.name.to_string())
                    .collect::<Vec<_>>())
            })?;
            return Ok(InstructionArgSet::from_names(content));
        }
    }
    Ok(InstructionArgSet::empty())
}

/// Helper struct for parsing instruction args
struct InstructionArg {
    name: syn::Ident,
}

impl syn::parse::Parse for InstructionArg {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        input.parse::<syn::Token![:]>()?;
        input.parse::<syn::Type>()?;
        Ok(Self { name })
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

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
}
