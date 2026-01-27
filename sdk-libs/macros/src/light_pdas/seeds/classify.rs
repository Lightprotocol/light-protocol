//! Seed classification - now delegated to account/seed_extraction.rs
//!
//! This module is kept for backward compatibility during transition.
//! All classification logic is now in `account::seed_extraction::classify_seed_expr`.
//!
//! The old `classify_seed` function that took `(expr, instruction_args, account_fields)`
//! is replaced by `classify_seed_expr(expr, instruction_args)` which treats
//! unknown identifiers as CtxRooted (producing the same codegen result).
