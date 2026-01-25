//! Core types for the simplified seed classification system.
//!
//! This module defines a 3-category classification based on what the client needs to send:
//! - **Constant**: Known at compile time (literals, constants) - client sends nothing
//! - **Account**: Account pubkey reference - client must include the account
//! - **Data**: Instruction data field - client must include in params

use syn::{Expr, Ident, Type};

/// Primary seed classification based on what the client needs to send.
///
/// This is a semantic classification, not syntax-based:
/// - `Constant`: Client doesn't send anything (compile-time known)
/// - `Account`: Client must include this account's pubkey
/// - `Data`: Client must include this value in instruction data
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SeedKind {
    /// Compile-time constant - client doesn't send anything.
    ///
    /// Includes:
    /// - Byte literals: `b"literal"`, `"string"`, `b"literal"[..]`
    /// - Uppercase constants: `SEED_PREFIX`, `crate::state::SEED_CONSTANT`
    /// - Complex expressions that don't reference accounts or params (passthrough)
    Constant,

    /// Account reference - client must include this account's pubkey.
    ///
    /// Includes:
    /// - Bare account: `authority.key().as_ref()`
    /// - Nested ctx access: `ctx.accounts.authority.key().as_ref()`
    Account,

    /// Instruction data - client must include in instruction params.
    ///
    /// Includes:
    /// - Struct field access: `params.owner.as_ref()`, `params.id.to_le_bytes()`
    /// - Bare instruction arg: `owner.as_ref()` (Format 2)
    Data,
}

/// A classified seed with its original expression and optional field name.
#[derive(Clone, Debug)]
pub struct Seed {
    /// Primary classification determining what the client needs to send
    pub kind: SeedKind,

    /// The original expression for code generation.
    ///
    /// For Account and Data kinds, this is the stripped expression
    /// (e.g., `authority.key().as_ref()` not `ctx.accounts.authority.key().as_ref()`).
    pub expr: Expr,

    /// Extracted field name (for Account and Data kinds).
    ///
    /// - For `Account`: The account field name (e.g., `authority`)
    /// - For `Data`: The data field name (e.g., `owner`, `id`)
    /// - For `Constant`: Always `None`
    pub field: Option<Ident>,
}

impl Seed {
    /// Create a new Constant seed (literals, constants, passthrough).
    pub fn constant(expr: Expr) -> Self {
        Self {
            kind: SeedKind::Constant,
            expr,
            field: None,
        }
    }

    /// Create a new Account seed.
    pub fn account(expr: Expr, field: Ident) -> Self {
        Self {
            kind: SeedKind::Account,
            expr,
            field: Some(field),
        }
    }

    /// Create a new Data seed.
    pub fn data(expr: Expr, field: Ident) -> Self {
        Self {
            kind: SeedKind::Data,
            expr,
            field: Some(field),
        }
    }

    /// Check if this is a constant seed.
    pub fn is_constant(&self) -> bool {
        self.kind == SeedKind::Constant
    }

    /// Check if this is an account seed.
    pub fn is_account(&self) -> bool {
        self.kind == SeedKind::Account
    }

    /// Check if this is a data seed.
    pub fn is_data(&self) -> bool {
        self.kind == SeedKind::Data
    }
}

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
    pub seeds: Vec<Seed>,

    /// True if the field uses zero-copy serialization (AccountLoader).
    pub is_zero_copy: bool,
}

impl SeedSpec {
    /// Create a new SeedSpec.
    pub fn new(field_name: Ident, inner_type: Type, seeds: Vec<Seed>, is_zero_copy: bool) -> Self {
        Self {
            field_name,
            inner_type,
            seeds,
            is_zero_copy,
        }
    }

    /// Get all account fields referenced in seeds.
    pub fn account_fields(&self) -> impl Iterator<Item = &Ident> {
        self.seeds
            .iter()
            .filter(|s| s.is_account())
            .filter_map(|s| s.field.as_ref())
    }

    /// Get all data fields referenced in seeds.
    pub fn data_fields(&self) -> impl Iterator<Item = &Ident> {
        self.seeds
            .iter()
            .filter(|s| s.is_data())
            .filter_map(|s| s.field.as_ref())
    }

    /// Get the number of seeds (for const generic array sizing).
    pub fn seed_count(&self) -> usize {
        self.seeds.len()
    }

    /// Check if any seeds reference instruction data.
    pub fn has_data_seeds(&self) -> bool {
        self.seeds.iter().any(|s| s.is_data())
    }

    /// Check if any seeds reference accounts.
    pub fn has_account_seeds(&self) -> bool {
        self.seeds.iter().any(|s| s.is_account())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ident(s: &str) -> Ident {
        Ident::new(s, proc_macro2::Span::call_site())
    }

    fn make_lit_expr() -> Expr {
        syn::parse_quote!(b"seed")
    }

    fn make_account_expr() -> Expr {
        syn::parse_quote!(authority.key().as_ref())
    }

    fn make_data_expr() -> Expr {
        syn::parse_quote!(owner.as_ref())
    }

    #[test]
    fn test_seed_constructors() {
        let const_seed = Seed::constant(make_lit_expr());
        assert!(const_seed.is_constant());
        assert!(const_seed.field.is_none());

        let account_seed = Seed::account(make_account_expr(), make_ident("authority"));
        assert!(account_seed.is_account());
        assert_eq!(account_seed.field.as_ref().unwrap().to_string(), "authority");

        let data_seed = Seed::data(make_data_expr(), make_ident("owner"));
        assert!(data_seed.is_data());
        assert_eq!(data_seed.field.as_ref().unwrap().to_string(), "owner");
    }

    #[test]
    fn test_seed_spec_queries() {
        let inner_type: syn::Type = syn::parse_quote!(UserRecord);
        let seeds = vec![
            Seed::constant(make_lit_expr()),
            Seed::account(make_account_expr(), make_ident("authority")),
            Seed::data(make_data_expr(), make_ident("owner")),
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
}
