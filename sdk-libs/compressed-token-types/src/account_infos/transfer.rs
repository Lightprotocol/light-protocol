use anchor_lang::{AnchorDeserialize, AnchorSerialize};

use crate::account_infos::generic_struct::AccountInfoIndexGetter;

#[repr(usize)]
pub enum TransferAccountInfosIndex {
    CpiAuthority,
    LightSystemProgram,
    RegisteredProgramPda,
    NoopProgram,
    AccountCompressionAuthority,
    AccountCompressionProgram,
    CTokenProgram,
    TokenPoolPda,
    DecompressionRecipient,
    SplTokenProgram,
    SystemProgram,
    CpiContext,
}

impl AccountInfoIndexGetter for TransferAccountInfosIndex {
    const SYSTEM_ACCOUNTS_LEN: usize = 12;
    fn cpi_authority_index() -> usize {
        TransferAccountInfosIndex::CpiAuthority as usize
    }

    fn light_system_program_index() -> usize {
        TransferAccountInfosIndex::LightSystemProgram as usize
    }

    fn registered_program_pda_index() -> usize {
        TransferAccountInfosIndex::RegisteredProgramPda as usize
    }

    fn noop_program_index() -> usize {
        TransferAccountInfosIndex::NoopProgram as usize
    }

    fn account_compression_authority_index() -> usize {
        TransferAccountInfosIndex::AccountCompressionAuthority as usize
    }

    fn account_compression_program_index() -> usize {
        TransferAccountInfosIndex::AccountCompressionProgram as usize
    }

    fn ctoken_program_index() -> usize {
        TransferAccountInfosIndex::CTokenProgram as usize
    }

    fn token_pool_pda_index() -> usize {
        TransferAccountInfosIndex::TokenPoolPda as usize
    }

    fn decompression_recipient_index() -> usize {
        TransferAccountInfosIndex::DecompressionRecipient as usize
    }

    fn spl_token_program_index() -> usize {
        TransferAccountInfosIndex::SplTokenProgram as usize
    }

    fn system_program_index() -> usize {
        TransferAccountInfosIndex::SystemProgram as usize
    }

    fn cpi_context_index() -> usize {
        TransferAccountInfosIndex::CpiContext as usize
    }
}

#[derive(Debug, Default, Copy, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct TransferAccountInfosConfig {
    pub cpi_context: bool,
    pub compress: bool,
    pub decompress: bool,
}

impl TransferAccountInfosConfig {
    pub const fn new_with_cpi_context() -> Self {
        Self {
            cpi_context: true,
            compress: false,
            decompress: false,
        }
    }

    pub fn new_compress() -> Self {
        Self {
            cpi_context: false,
            compress: true,
            decompress: false,
        }
    }

    pub fn new_decompress() -> Self {
        Self {
            cpi_context: false,
            compress: false,
            decompress: true,
        }
    }

    pub fn is_compress_or_decompress(&self) -> bool {
        self.compress || self.decompress
    }
}
