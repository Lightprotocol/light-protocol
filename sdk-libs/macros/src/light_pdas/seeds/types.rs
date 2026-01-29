//! Core types for the seed classification system.
//!
//! This module defines the primary types for seed classification:
//! - `ClassifiedSeed` - Individual seed classification (Literal, Constant, CtxRooted, DataRooted, FunctionCall, Passthrough)
//! - `ClassifiedFnArg` - Classified argument to a function call seed
//! - `FnArgKind` - Classification of a function call argument (CtxAccount or DataField)
//! - `SeedSpec` - Collection of seeds for a single PDA field with metadata

use syn::{Ident, Type};

// =============================================================================
// CLASSIFIED SEED TYPES
// =============================================================================

/// Classified seed element from Anchor's seeds array.
///
/// Uses prefix detection + passthrough strategy:
/// - Identifies the root (ctx/data/constant/literal) to determine which namespace
/// - Passes through the full expression unchanged for code generation
/// - Complex expressions like `identity_seed::<12>(b"seed")` become Passthrough
#[derive(Clone, Debug)]
pub enum ClassifiedSeed {
    /// b"literal" or "string" - hardcoded bytes
    Literal(Vec<u8>),
    /// CONSTANT or path::CONSTANT - uppercase identifier.
    /// `path` is the extracted constant path (for crate:: qualification).
    /// `expr` is the full original expression (e.g., `SEED.as_bytes()`) for codegen.
    Constant {
        path: syn::Path,
        expr: Box<syn::Expr>,
    },
    /// Expression rooted in ctx account (e.g., authority.key().as_ref())
    /// `account` is the root identifier
    CtxRooted { account: Ident },
    /// Expression rooted in instruction arg (e.g., params.owner.as_ref())
    /// `root` is the instruction arg name, `expr` is the full expression for codegen
    DataRooted { root: Ident, expr: Box<syn::Expr> },
    /// Function call with dynamic arguments (e.g., crate::max_key(&params.key_a, &params.key_b).as_ref())
    /// Detected when `Expr::Call` or `Expr::MethodCall(receiver=Expr::Call)` has args
    /// rooted in instruction data or ctx accounts.
    FunctionCall {
        /// The full function call expression (without trailing .as_ref()/.as_bytes())
        func_expr: Box<syn::Expr>,
        /// Classified arguments to the function
        args: Vec<ClassifiedFnArg>,
        /// Whether the original expression had trailing .as_ref() or .as_bytes()
        has_as_ref: bool,
    },
    /// Everything else - pass through unchanged
    Passthrough(Box<syn::Expr>),
}

/// A classified argument to a function call seed.
#[derive(Clone, Debug)]
pub struct ClassifiedFnArg {
    /// The field name extracted from the argument (e.g., `key_a` from `&params.key_a`)
    pub field_name: Ident,
    /// Whether this is a ctx account or instruction data field
    pub kind: FnArgKind,
}

/// Classification of a function call argument.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FnArgKind {
    /// Argument is rooted in a ctx account field
    CtxAccount,
    /// Argument is rooted in instruction data
    DataField,
}

// =============================================================================
// SEED SPEC TYPE
// =============================================================================

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
                .find(|a| a.kind == FnArgKind::CtxAccount)
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
                .find(|a| a.kind == FnArgKind::DataField)
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
                    if args.iter().any(|a| a.kind == FnArgKind::DataField))
        })
    }

    /// Check if any seeds reference accounts.
    pub fn has_account_seeds(&self) -> bool {
        self.seeds.iter().any(|s| {
            matches!(s, ClassifiedSeed::CtxRooted { .. })
                || matches!(s, ClassifiedSeed::FunctionCall { args, .. }
                    if args.iter().any(|a| a.kind == FnArgKind::CtxAccount))
        })
    }
}

// =============================================================================
// EXTRACTED SPEC TYPES (for #[light_program] macro)
// =============================================================================

/// Extracted seed specification for a light account field.
///
/// This is a richer version of `SeedSpec` with additional metadata needed
/// for code generation in the `#[light_program]` macro.
#[derive(Clone, Debug)]
pub struct ExtractedSeedSpec {
    /// The variant name derived from field_name (snake_case -> CamelCase)
    pub variant_name: Ident,
    /// The inner type (e.g., crate::state::UserRecord from Account<'info, UserRecord>)
    /// Preserves the full type path for code generation.
    pub inner_type: Type,
    /// Classified seeds from #[account(seeds = [...])]
    pub seeds: Vec<ClassifiedSeed>,
    /// True if the field uses zero-copy serialization (AccountLoader)
    pub is_zero_copy: bool,
    /// The instruction struct name this field was extracted from (for error messages)
    pub struct_name: String,
    /// The full module path where this struct was defined (e.g., "crate::instructions::create")
    /// Used to qualify bare constant/function names in seed expressions.
    pub module_path: String,
}

/// Extracted token specification for a #[light_account(token, ...)] field
#[derive(Clone, Debug)]
pub struct ExtractedTokenSpec {
    /// The field name in the Accounts struct
    pub field_name: Ident,
    /// The variant name derived from field name
    pub variant_name: Ident,
    /// Seeds from #[account(seeds = [...])]
    pub seeds: Vec<ClassifiedSeed>,
    /// Owner PDA seeds - used when the token owner is a PDA that needs to sign.
    /// Must contain only constant values (byte literals, const references).
    pub owner_seeds: Option<Vec<ClassifiedSeed>>,
    /// The full module path where this struct was defined (e.g., "crate::instructions::create")
    /// Used to qualify bare constant/function names in seed expressions.
    pub module_path: String,
}

/// All extracted info from an Accounts struct
#[derive(Clone, Debug)]
pub struct ExtractedAccountsInfo {
    pub struct_name: Ident,
    pub pda_fields: Vec<ExtractedSeedSpec>,
    pub token_fields: Vec<ExtractedTokenSpec>,
    /// True if struct has any #[light_account(init, mint::...)] fields
    pub has_light_mint_fields: bool,
    /// True if struct has any #[light_account(init, associated_token::...)] fields
    pub has_light_ata_fields: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

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
