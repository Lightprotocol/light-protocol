//! Simplified seed extraction and classification module.
//!
//! This module provides a 3-category classification system for PDA seeds based on
//! what the client needs to send:
//!
//! | Category | Client Action | Example |
//! |----------|---------------|---------|
//! | **Constant** | Nothing - known at compile time | `b"user"`, `SEED_PREFIX` |
//! | **Account** | Pass account pubkey | `authority.key().as_ref()` |
//! | **Data** | Include in instruction data | `params.owner.as_ref()` |
//!
//! # Usage
//!
//! ```ignore
//! use light_sdk_macros::light_pdas::seeds::{
//!     classify_seed, extract_seed_specs, Seed, SeedKind, SeedSpec,
//! };
//!
//! // Classify a single seed expression
//! let expr: syn::Expr = syn::parse_quote!(authority.key().as_ref());
//! let mut account_fields = HashSet::new();
//! account_fields.insert("authority".to_string());
//!
//! let seed = classify_seed(&expr, &HashSet::new(), &account_fields)?;
//! assert_eq!(seed.kind, SeedKind::Account);
//! assert_eq!(seed.field.unwrap().to_string(), "authority");
//!
//! // Extract all seed specs from an Accounts struct
//! let specs = extract_seed_specs(&accounts_struct)?;
//! for spec in specs {
//!     println!("Field: {}", spec.field_name);
//!     for seed in &spec.seeds {
//!         match seed.kind {
//!             SeedKind::Constant => println!("  Constant seed"),
//!             SeedKind::Account => println!("  Account: {:?}", seed.field),
//!             SeedKind::Data => println!("  Data: {:?}", seed.field),
//!         }
//!     }
//! }
//! ```
//!
//! # Module Structure
//!
//! - [`types`]: Core types (`SeedKind`, `Seed`, `SeedSpec`)
//! - [`classify`]: Classification logic (`classify_seed`)
//! - [`extract`]: Extraction from Anchor attributes (`extract_seed_specs`, `parse_instruction_args`)

mod classify;
mod derive;
mod extract;
mod types;

// Re-export core types
pub use types::{Seed, SeedKind, SeedSpec};

// Re-export classification function
pub use classify::classify_seed;

// Re-export extraction functions
pub use extract::{
    check_light_account_init, extract_account_fields, extract_account_inner_type,
    extract_seed_specs, extract_seeds_from_attribute, parse_instruction_args,
};

// Re-export derive function
pub use derive::derive_seed;
