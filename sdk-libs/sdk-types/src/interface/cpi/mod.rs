//! Generic CPI module for Light system program invocation.
//!
//! Uses v2 `CpiAccounts<'a, T: AccountInfoTrait>` from light-sdk-types.
//! All CPI calls go through `AI::invoke_cpi()` for framework independence.

pub mod account;
pub mod impls;
mod instruction;
pub mod invoke;

pub use account::CpiAccountsTrait;
pub use instruction::LightCpi;
pub use invoke::{invoke_light_system_program, InvokeLightSystemProgram};
#[cfg(feature = "cpi-context")]
pub use invoke::invoke_write_pdas_to_cpi_context;
pub use light_compressed_account::instruction_data::traits::LightInstructionData;
pub use crate::{cpi_accounts::CpiAccountsConfig, CpiSigner};
// TODO: move all of this to light-sdk-types
