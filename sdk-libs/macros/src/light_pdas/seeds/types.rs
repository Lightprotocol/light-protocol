//! Core types for the seed classification system.
//!
//! This module defines `SeedSpec` which wraps `ClassifiedSeed` (from `account/seed_extraction`)
//! with field-level metadata needed for variant code generation.

use syn::{Ident, Type};

use crate::light_pdas::account::seed_extraction::ClassifiedSeed;

/// Collection of seeds for a single PDA field.
///
/// This represents all the seeds needed to derive a PDA for a specific
/// account field in an Accounts struct.
#[derive(Clone, Debug)]
pub struct SeedSpec {
    /// Field name this seed spec belongs to (e.g., `user_record`).
    pub field_name: Ident,

    /// Inner type of the account (e.g., `UserRecord` from `Account<'info, UserRecord>`).
    /// Preserves the full type path for code generation.
    pub inner_type: Type,

    /// Classified seeds from `#[account(seeds = [...])]`.
    pub seeds: Vec<ClassifiedSeed>,

    /// True if the field uses zero-copy serialization (AccountLoader).
    pub is_zero_copy: bool,
}

impl SeedSpec {
    /// Create a new SeedSpec.
    pub fn new(
        field_name: Ident,
        inner_type: Type,
        seeds: Vec<ClassifiedSeed>,
        is_zero_copy: bool,
    ) -> Self {
        Self {
            field_name,
            inner_type,
            seeds,
            is_zero_copy,
        }
    }
}

#[cfg(test)]
impl SeedSpec {
    /// Get all account fields referenced in seeds.
    pub fn account_fields(&self) -> impl Iterator<Item = &Ident> {
        self.seeds.iter().filter_map(|s| match s {
            ClassifiedSeed::CtxRooted { account, .. } => Some(account),
            ClassifiedSeed::FunctionCall { args, .. } => args
                .iter()
                .find(|a| {
                    a.kind == crate::light_pdas::account::seed_extraction::FnArgKind::CtxAccount
                })
                .map(|a| &a.field_name),
            _ => None,
        })
    }

    /// Get all data fields referenced in seeds.
    pub fn data_fields(&self) -> impl Iterator<Item = &Ident> {
        self.seeds.iter().filter_map(|s| match s {
            ClassifiedSeed::DataRooted { root, .. } => Some(root),
            ClassifiedSeed::FunctionCall { args, .. } => args
                .iter()
                .find(|a| {
                    a.kind == crate::light_pdas::account::seed_extraction::FnArgKind::DataField
                })
                .map(|a| &a.field_name),
            _ => None,
        })
    }

    /// Get the number of seeds (for const generic array sizing).
    pub fn seed_count(&self) -> usize {
        self.seeds.len()
    }

    /// Check if any seeds reference instruction data.
    pub fn has_data_seeds(&self) -> bool {
        self.seeds.iter().any(|s| {
            matches!(s, ClassifiedSeed::DataRooted { .. })
                || matches!(s, ClassifiedSeed::FunctionCall { args, .. }
                    if args.iter().any(|a| a.kind == crate::light_pdas::account::seed_extraction::FnArgKind::DataField))
        })
    }

    /// Check if any seeds reference accounts.
    pub fn has_account_seeds(&self) -> bool {
        self.seeds.iter().any(|s| {
            matches!(s, ClassifiedSeed::CtxRooted { .. })
                || matches!(s, ClassifiedSeed::FunctionCall { args, .. }
                    if args.iter().any(|a| a.kind == crate::light_pdas::account::seed_extraction::FnArgKind::CtxAccount))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::light_pdas::account::seed_extraction::{ClassifiedFnArg, FnArgKind};

    fn make_ident(s: &str) -> Ident {
        Ident::new(s, proc_macro2::Span::call_site())
    }

    #[test]
    fn test_seed_spec_queries() {
        let inner_type: syn::Type = syn::parse_quote!(UserRecord);
        let seeds = vec![
            ClassifiedSeed::Literal(b"seed".to_vec()),
            ClassifiedSeed::CtxRooted {
                account: make_ident("authority"),
            },
            ClassifiedSeed::DataRooted {
                root: make_ident("owner"),
                expr: Box::new(syn::parse_quote!(owner.as_ref())),
            },
        ];

        let spec = SeedSpec::new(make_ident("user_record"), inner_type, seeds, false);

        assert_eq!(spec.seed_count(), 3);
        assert!(spec.has_account_seeds());
        assert!(spec.has_data_seeds());

        let account_fields: Vec<_> = spec.account_fields().collect();
        assert_eq!(account_fields.len(), 1);
        assert_eq!(account_fields[0].to_string(), "authority");

        let data_fields: Vec<_> = spec.data_fields().collect();
        assert_eq!(data_fields.len(), 1);
        assert_eq!(data_fields[0].to_string(), "owner");
    }

    #[test]
    fn test_seed_spec_with_function_call() {
        let inner_type: syn::Type = syn::parse_quote!(PoolAccount);
        let seeds = vec![
            ClassifiedSeed::Literal(b"pool".to_vec()),
            ClassifiedSeed::FunctionCall {
                func_expr: Box::new(syn::parse_quote!(crate::max_key(
                    &params.key_a,
                    &params.key_b
                ))),
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
            },
        ];

        let spec = SeedSpec::new(make_ident("pool"), inner_type, seeds, false);

        assert_eq!(spec.seed_count(), 2);
        assert!(spec.has_data_seeds());
        // FunctionCall with DataField args shows up in data_fields()
        let data_fields: Vec<_> = spec.data_fields().collect();
        assert_eq!(data_fields.len(), 1); // first match from iterator
    }
}
