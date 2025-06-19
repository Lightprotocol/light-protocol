use crate::account_infos::generic_struct::AccountInfoIndexGetter;

#[repr(usize)]
pub enum FreezeAccountInfosIndex {
    FeePayer,
    Authority,
    CpiAuthorityPda,
    LightSystemProgram,
    RegisteredProgramPda,
    NoopProgram,
    AccountCompressionAuthority,
    AccountCompressionProgram,
    SelfProgram,
    SystemProgram,
    Mint,
}

impl AccountInfoIndexGetter for FreezeAccountInfosIndex {
    const SYSTEM_ACCOUNTS_LEN: usize = 11;
    
    fn cpi_authority_index() -> usize {
        FreezeAccountInfosIndex::CpiAuthorityPda as usize
    }

    fn light_system_program_index() -> usize {
        FreezeAccountInfosIndex::LightSystemProgram as usize
    }

    fn registered_program_pda_index() -> usize {
        FreezeAccountInfosIndex::RegisteredProgramPda as usize
    }

    fn noop_program_index() -> usize {
        FreezeAccountInfosIndex::NoopProgram as usize
    }

    fn account_compression_authority_index() -> usize {
        FreezeAccountInfosIndex::AccountCompressionAuthority as usize
    }

    fn account_compression_program_index() -> usize {
        FreezeAccountInfosIndex::AccountCompressionProgram as usize
    }

    fn ctoken_program_index() -> usize {
        FreezeAccountInfosIndex::SelfProgram as usize
    }

    fn token_pool_pda_index() -> usize {
        // Freeze instruction doesn't use token pool pda
        0
    }

    fn decompression_recipient_index() -> usize {
        // Freeze instruction doesn't use decompression recipient
        0
    }

    fn spl_token_program_index() -> usize {
        // Freeze instruction doesn't use spl token program
        0
    }

    fn system_program_index() -> usize {
        FreezeAccountInfosIndex::SystemProgram as usize
    }

    fn cpi_context_index() -> usize {
        // Freeze instruction doesn't use cpi context
        0
    }
}

