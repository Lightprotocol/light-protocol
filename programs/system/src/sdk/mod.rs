pub mod accounts;
pub mod address;
pub mod compressed_account;
pub mod event;
pub mod invoke;
use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CompressedCpiContext {
    /// Is set by the program that is invoking the CPI to signal that is should
    /// set the cpi context.
    pub set_context: bool,
    /// Is set to wipe the cpi context since someone could have set it before
    /// with unrelated data.
    pub first_set_context: bool,
    /// Index of cpi context account in remaining accounts.
    pub cpi_context_account_index: u8,
}
