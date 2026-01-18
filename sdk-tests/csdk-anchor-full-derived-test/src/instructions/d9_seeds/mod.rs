//! D9: Seed expression classification
//!
//! Tests different seed expression types in ClassifiedSeed enum:
//! - Literal: b"record"
//! - Constant: SEED_CONSTANT
//! - CtxAccount: authority.key()
//! - DataField (param): params.owner.as_ref()
//! - DataField (bytes): params.id.to_le_bytes()
//! - FunctionCall: max_key(&a, &b)

mod all;
mod constant;
mod ctx_account;
mod function_call;
mod literal;
mod mixed;
mod param;
mod param_bytes;

pub use all::*;
pub use constant::*;
pub use ctx_account::*;
pub use function_call::*;
pub use literal::*;
pub use mixed::*;
pub use param::*;
pub use param_bytes::*;
