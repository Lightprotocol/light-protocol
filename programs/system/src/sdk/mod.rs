pub mod accounts;
pub mod address;
pub mod compressed_account;
pub mod event;
pub mod invoke;
pub mod invoke_cpi;

use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompressedCpiContext {
    pub set_context: bool,
    pub cpi_context_account_index: u8,
}
