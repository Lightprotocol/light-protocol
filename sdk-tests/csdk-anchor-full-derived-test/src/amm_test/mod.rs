//! AMM test cases based on cp-swap-reference patterns.
//!
//! Tests:
//! - Multiple #[light_account(init)] fields
//! - #[light_account(token)] with authority seeds
//! - #[light_account(init)] for LP token creation
//! - CreateTokenAccountCpi.rent_free()
//! - CreateTokenAtaCpi.rent_free()
//! - MintToCpi / BurnCpi
//! - Divergent naming: input_vault/output_vault aliases for token_0_vault/token_1_vault

mod deposit;
mod initialize;
mod states;
mod swap;
mod withdraw;

pub use deposit::*;
pub use initialize::*;
pub use states::*;
pub use swap::*;
pub use withdraw::*;
