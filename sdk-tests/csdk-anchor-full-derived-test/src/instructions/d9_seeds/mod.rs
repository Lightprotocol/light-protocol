//! D9: Seed expression classification
//!
//! Tests different seed expression types in ClassifiedSeed enum:
//! - Literal: b"record"
//! - Constant: SEED_CONSTANT
//! - CtxAccount: authority.key()
//! - DataField (param): params.owner.as_ref()
//! - DataField (bytes): params.id.to_le_bytes()
//! - FunctionCall: max_key(&a, &b)
//!
//! Extended tests:
//! - Qualified paths: crate::, self::, external crate paths
//! - Method chains: as_ref(), as_bytes(), to_le_bytes(), to_be_bytes()
//! - Array bumps: &[params.bump] patterns
//! - Complex mixed: 3+ seeds, function calls, program ID
//! - Edge cases: empty literals, single byte, special names
//! - External paths: light_sdk_types, light_ctoken_types

mod all;
pub mod array_bumps;
pub mod complex_mixed;
pub mod const_patterns;
mod constant;
mod ctx_account;
pub mod edge_cases;
pub mod external_paths;
mod function_call;
pub mod instruction_data;
mod literal;
pub mod method_chains;
mod mixed;
pub mod nested_seeds;
mod param;
mod param_bytes;
pub mod qualified_paths;

pub use all::*;
pub use array_bumps::*;
pub use complex_mixed::*;
pub use const_patterns::*;
pub use constant::*;
pub use ctx_account::*;
pub use edge_cases::*;
pub use external_paths::*;
pub use function_call::*;
pub use instruction_data::*;
pub use literal::*;
pub use method_chains::*;
pub use mixed::*;
pub use nested_seeds::*;
pub use param::*;
pub use param_bytes::*;
pub use qualified_paths::*;
