//! AMM test cases based on cp-swap-reference patterns.
//!
//! Tests:
//! - Multiple #[light_account(init)] fields
//! - #[light_account(token)] with authority seeds
//! - #[light_mint] for LP token creation
//! - CreateTokenAccountCpi.rent_free()
//! - CreateTokenAtaCpi.rent_free()
//! - MintToCpi / BurnCpi

mod deposit;
mod initialize;
mod states;
mod withdraw;

pub use deposit::*;
pub use initialize::*;
pub use states::*;
pub use withdraw::*;
