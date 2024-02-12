#[cfg(feature = "solana")]
use num_traits::ToPrimitive;
use thiserror::Error;

#[derive(Debug, Error)]
#[cfg_attr(feature = "solana", derive(num_derive::ToPrimitive))]
pub enum EventError {
    #[error("Integer overflow")]
    IntegerOverflow = 4001,
    #[error("Emitting an event requires at least one changelog entry")]
    EventNoChangelogEntry,
}

#[cfg(feature = "solana")]
impl From<EventError> for solana_program::program_error::ProgramError {
    fn from(e: EventError) -> Self {
        solana_program::program_error::ProgramError::Custom(e.to_u32().unwrap_or(4001))
    }
}
