use crate::account_infos::generic_struct::AccountInfoIndexGetter;

#[repr(usize)]
pub enum BatchCompressAccountInfosIndex {
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
    MerkleTree,
    SelfProgram,
    SystemProgram,
}

impl AccountInfoIndexGetter for BatchCompressAccountInfosIndex {
    const SYSTEM_ACCOUNTS_LEN: usize = 14;
    
    fn cpi_authority_index() -> usize {
        BatchCompressAccountInfosIndex::CpiAuthorityPda as usize
    }

    fn light_system_program_index() -> usize {
        BatchCompressAccountInfosIndex::LightSystemProgram as usize
    }

    fn registered_program_pda_index() -> usize {
        BatchCompressAccountInfosIndex::RegisteredProgramPda as usize
    }

    fn noop_program_index() -> usize {
        BatchCompressAccountInfosIndex::NoopProgram as usize
    }

    fn account_compression_authority_index() -> usize {
        BatchCompressAccountInfosIndex::AccountCompressionAuthority as usize
    }

    fn account_compression_program_index() -> usize {
        BatchCompressAccountInfosIndex::AccountCompressionProgram as usize
    }

    fn ctoken_program_index() -> usize {
        BatchCompressAccountInfosIndex::SelfProgram as usize
    }

    fn token_pool_pda_index() -> usize {
        BatchCompressAccountInfosIndex::TokenPoolPda as usize
    }

    fn decompression_recipient_index() -> usize {
        // BatchCompress doesn't use decompression recipient
        0
    }

    fn spl_token_program_index() -> usize {
        BatchCompressAccountInfosIndex::TokenProgram as usize
    }

    fn system_program_index() -> usize {
        BatchCompressAccountInfosIndex::SystemProgram as usize
    }

    fn cpi_context_index() -> usize {
        // BatchCompress doesn't use cpi context
        0
    }
}

