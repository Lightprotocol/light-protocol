//! Core types for the seed classification system.
//!
//! This module defines the primary types for seed classification:
//! - `ClassifiedSeed` - Individual seed classification (Literal, Constant, CtxRooted, DataRooted, FunctionCall, Passthrough)
//! - `ClassifiedFnArg` - Classified argument to a function call seed
//! - `FnArgKind` - Classification of a function call argument (CtxAccount or DataField)
//! - `ExtractedSeedSpec` - Collection of seeds for a single PDA field with metadata

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

// =============================================================================
// EXTRACTED SPEC TYPES (for #[light_program] macro)
// =============================================================================

/// Extracted seed specification for a light account field.
///
/// Contains seed metadata needed for code generation in the `#[light_program]` macro.
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
