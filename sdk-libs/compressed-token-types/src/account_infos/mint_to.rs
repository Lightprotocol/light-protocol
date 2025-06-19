use crate::account_infos::generic_struct::AccountInfoIndexGetter;

#[repr(usize)]
pub enum MintToAccountInfosIndex {
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

impl AccountInfoIndexGetter for MintToAccountInfosIndex {
    const SYSTEM_ACCOUNTS_LEN: usize = 14;
    
    fn cpi_authority_index() -> usize {
        MintToAccountInfosIndex::CpiAuthorityPda as usize
    }

    fn light_system_program_index() -> usize {
        MintToAccountInfosIndex::LightSystemProgram as usize
    }

    fn registered_program_pda_index() -> usize {
        MintToAccountInfosIndex::RegisteredProgramPda as usize
    }

    fn noop_program_index() -> usize {
        MintToAccountInfosIndex::NoopProgram as usize
    }

    fn account_compression_authority_index() -> usize {
        MintToAccountInfosIndex::AccountCompressionAuthority as usize
    }

    fn account_compression_program_index() -> usize {
        MintToAccountInfosIndex::AccountCompressionProgram as usize
    }

    fn ctoken_program_index() -> usize {
        MintToAccountInfosIndex::SelfProgram as usize
    }

    fn token_pool_pda_index() -> usize {
        MintToAccountInfosIndex::TokenPoolPda as usize
    }

    fn decompression_recipient_index() -> usize {
        // MintTo doesn't use decompression recipient
        0
    }

    fn spl_token_program_index() -> usize {
        MintToAccountInfosIndex::TokenProgram as usize
    }

    fn system_program_index() -> usize {
        MintToAccountInfosIndex::SystemProgram as usize
    }

    fn cpi_context_index() -> usize {
        // MintTo doesn't use cpi context
        0
    }
}

