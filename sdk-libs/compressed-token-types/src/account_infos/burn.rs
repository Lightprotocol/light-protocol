use crate::account_infos::generic_struct::AccountInfoIndexGetter;

#[repr(usize)]
pub enum BurnAccountInfosIndex {
    FeePayer,
    Authority,
    CpiAuthorityPda,
    Mint,
    TokenPoolPda,
    TokenProgram,
    LightSystemProgram,
    RegisteredProgramPda,
    NoopProgram,
    AccountCompressionAuthority,
    AccountCompressionProgram,
    SelfProgram,
    SystemProgram,
}

impl AccountInfoIndexGetter for BurnAccountInfosIndex {
    const SYSTEM_ACCOUNTS_LEN: usize = 13;
    
    fn cpi_authority_index() -> usize {
        BurnAccountInfosIndex::CpiAuthorityPda as usize
    }

    fn light_system_program_index() -> usize {
        BurnAccountInfosIndex::LightSystemProgram as usize
    }

    fn registered_program_pda_index() -> usize {
        BurnAccountInfosIndex::RegisteredProgramPda as usize
    }

    fn noop_program_index() -> usize {
        BurnAccountInfosIndex::NoopProgram as usize
    }

    fn account_compression_authority_index() -> usize {
        BurnAccountInfosIndex::AccountCompressionAuthority as usize
    }

    fn account_compression_program_index() -> usize {
        BurnAccountInfosIndex::AccountCompressionProgram as usize
    }

    fn ctoken_program_index() -> usize {
        BurnAccountInfosIndex::SelfProgram as usize
    }

    fn token_pool_pda_index() -> usize {
        BurnAccountInfosIndex::TokenPoolPda as usize
    }

    fn decompression_recipient_index() -> usize {
        // Burn instruction doesn't use decompression recipient
        0
    }

    fn spl_token_program_index() -> usize {
        BurnAccountInfosIndex::TokenProgram as usize
    }

    fn system_program_index() -> usize {
        BurnAccountInfosIndex::SystemProgram as usize
    }

    fn cpi_context_index() -> usize {
        // Burn instruction doesn't use cpi context
        0
    }
}

