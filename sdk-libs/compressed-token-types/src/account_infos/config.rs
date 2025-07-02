use crate::{AnchorDeserialize, AnchorSerialize};

#[derive(Debug, Default, Copy, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct AccountInfosConfig {
    pub cpi_context: bool,
}

impl AccountInfosConfig {
    pub const fn new() -> Self {
        Self { cpi_context: false }
    }

    pub const fn new_with_cpi_context() -> Self {
        Self { cpi_context: true }
    }
}
