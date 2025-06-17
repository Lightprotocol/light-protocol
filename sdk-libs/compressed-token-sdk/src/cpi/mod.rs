//! CPI helpers for compressed token operations
//!
//! This module provides utilities to invoke the compressed token program via cross program invocation (CPI).
//! It follows the same patterns as light-sdk's CPI module.
//!
//! ```ignore
//! let token_account = CTokenAccount::new_empty(recipient, output_tree_index);
//! let cpi_inputs = CpiInputs::new_compress(vec![token_account]);
//!
//! cpi_inputs
//!     .invoke_compressed_token_program(light_cpi_accounts)
//!     .map_err(ProgramError::from)?;
//! ```

pub mod accounts;
mod invoke;

pub use accounts::*;
pub use invoke::*;
pub use light_compressed_token_types::cpi_accounts::*;
