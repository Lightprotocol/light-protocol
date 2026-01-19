//! D7: Infrastructure field naming
//!
//! Tests macro handling of different field naming conventions:
//! - payer instead of fee_payer
//! - creator instead of fee_payer
//! - light_token_config variants

mod all;
mod creator;
mod light_token_config;
mod payer;

pub use all::*;
pub use creator::*;
pub use light_token_config::*;
pub use payer::*;
