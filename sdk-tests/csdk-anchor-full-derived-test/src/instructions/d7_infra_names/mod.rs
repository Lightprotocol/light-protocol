//! D7: Infrastructure field naming
//!
//! Tests macro handling of different field naming conventions:
//! - payer instead of fee_payer
//! - creator instead of fee_payer
//! - ctoken_config variants

mod all;
mod creator;
mod ctoken_config;
mod payer;

pub use all::*;
pub use creator::*;
pub use ctoken_config::*;
pub use payer::*;
