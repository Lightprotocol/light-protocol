pub mod account_compression_state;
pub mod accounts;
pub mod constants;
pub mod context;
pub mod cpi_context;
pub mod errors;
pub mod invoke;
pub mod invoke_cpi;
pub mod processor;
pub mod utils;

use pinocchio::program_error::ProgramError;
pub use processor::Processor;

pub type Result<T> = std::result::Result<T, ProgramError>;
