use light_compressed_account::instruction_data::zero_copy_set::CompressedCpiContextTrait;
use light_zero_copy::{ZeroCopy, ZeroCopyMut};

use crate::{AnchorDeserialize, AnchorSerialize};

#[repr(C)]
#[derive(
    AnchorSerialize,
    AnchorDeserialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    ZeroCopy,
    ZeroCopyMut,
)]
pub struct CompressedCpiContext {
    /// Is set by the program that is invoking the CPI to signal that is should
    /// set the cpi context.
    pub set_context: bool,
    /// Is set to clear the cpi context since someone could have set it before
    /// with unrelated data.
    pub first_set_context: bool,
}

impl CompressedCpiContextTrait for ZCompressedCpiContext<'_> {
    fn first_set_context(&self) -> u8 {
        if self.first_set_context() {
            1
        } else {
            0
        }
    }

    fn set_context(&self) -> u8 {
        if self.set_context() {
            1
        } else {
            0
        }
    }
}

impl CompressedCpiContextTrait for CompressedCpiContext {
    fn first_set_context(&self) -> u8 {
        if self.first_set_context {
            1
        } else {
            0
        }
    }

    fn set_context(&self) -> u8 {
        if self.set_context {
            1
        } else {
            0
        }
    }
}
