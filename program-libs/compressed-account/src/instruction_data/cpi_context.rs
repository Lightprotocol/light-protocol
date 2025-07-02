use light_zero_copy::ZeroCopyMut;

use crate::{
    instruction_data::{
        zero_copy::ZCompressedCpiContext, zero_copy_set::CompressedCpiContextTrait,
    },
    AnchorDeserialize, AnchorSerialize,
};

#[repr(C)]
#[derive(
    AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy, PartialEq, Eq, Default, ZeroCopyMut,
)]
pub struct CompressedCpiContext {
    /// Is set by the program that is invoking the CPI to signal that is should
    /// set the cpi context.
    pub set_context: bool,
    /// Is set to clear the cpi context since someone could have set it before
    /// with unrelated data.
    pub first_set_context: bool,
    /// Index of cpi context account in remaining accounts.
    pub cpi_context_account_index: u8,
}

impl CompressedCpiContextTrait for ZCompressedCpiContext {
    fn first_set_context(&self) -> u8 {
        self.first_set_context() as u8
    }

    fn set_context(&self) -> u8 {
        self.set_context() as u8
    }
}

impl CompressedCpiContextTrait for CompressedCpiContext {
    fn first_set_context(&self) -> u8 {
        self.first_set_context as u8
    }

    fn set_context(&self) -> u8 {
        self.set_context as u8
    }
}
