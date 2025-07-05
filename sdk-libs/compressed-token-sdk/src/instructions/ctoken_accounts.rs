use light_compressed_token_types::{
    ACCOUNT_COMPRESSION_AUTHORITY_PDA, ACCOUNT_COMPRESSION_PROGRAM_ID, CPI_AUTHORITY_PDA,
    LIGHT_SYSTEM_PROGRAM_ID, NOOP_PROGRAM_ID, PROGRAM_ID as LIGHT_COMPRESSED_TOKEN_PROGRAM_ID,
};
use light_sdk::constants::{C_TOKEN_PROGRAM_ID, REGISTERED_PROGRAM_PDA};
use solana_pubkey::Pubkey;

/// Standard pubkeys for compressed token instructions
#[derive(Debug, Copy, Clone)]
pub struct CTokenDefaultAccounts {
    pub light_system_program: Pubkey,
    pub registered_program_pda: Pubkey,
    pub noop_program: Pubkey,
    pub account_compression_authority: Pubkey,
    pub account_compression_program: Pubkey,
    pub self_program: Pubkey,
    pub cpi_authority_pda: Pubkey,
    pub system_program: Pubkey,
    pub compressed_token_program: Pubkey,
}

impl Default for CTokenDefaultAccounts {
    fn default() -> Self {
        Self {
            light_system_program: Pubkey::from(LIGHT_SYSTEM_PROGRAM_ID),
            registered_program_pda: Pubkey::from(REGISTERED_PROGRAM_PDA),
            noop_program: Pubkey::from(NOOP_PROGRAM_ID),
            account_compression_authority: Pubkey::from(ACCOUNT_COMPRESSION_AUTHORITY_PDA),
            account_compression_program: Pubkey::from(ACCOUNT_COMPRESSION_PROGRAM_ID),
            self_program: Pubkey::from(LIGHT_COMPRESSED_TOKEN_PROGRAM_ID),
            cpi_authority_pda: Pubkey::from(CPI_AUTHORITY_PDA),
            system_program: Pubkey::default(),
            compressed_token_program: Pubkey::from(C_TOKEN_PROGRAM_ID),
        }
    }
}
