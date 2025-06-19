use crate::account_infos::generic_struct::AccountInfoIndexGetter;

#[repr(usize)]
pub enum GenericAccountInfosIndex {
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
}

impl AccountInfoIndexGetter for GenericAccountInfosIndex {
    const SYSTEM_ACCOUNTS_LEN: usize = 10;
    
    fn cpi_authority_index() -> usize {
        GenericAccountInfosIndex::CpiAuthorityPda as usize
    }

    fn light_system_program_index() -> usize {
        GenericAccountInfosIndex::LightSystemProgram as usize
    }

    fn registered_program_pda_index() -> usize {
        GenericAccountInfosIndex::RegisteredProgramPda as usize
    }

    fn noop_program_index() -> usize {
        GenericAccountInfosIndex::NoopProgram as usize
    }

    fn account_compression_authority_index() -> usize {
        GenericAccountInfosIndex::AccountCompressionAuthority as usize
    }

    fn account_compression_program_index() -> usize {
        GenericAccountInfosIndex::AccountCompressionProgram as usize
    }

    fn ctoken_program_index() -> usize {
        GenericAccountInfosIndex::SelfProgram as usize
    }

    fn token_pool_pda_index() -> usize {
        // Generic instruction doesn't use token pool pda
        0
    }

    fn decompression_recipient_index() -> usize {
        // Generic instruction doesn't use decompression recipient
        0
    }

    fn spl_token_program_index() -> usize {
        // Generic instruction doesn't use spl token program
        0
    }

    fn system_program_index() -> usize {
        GenericAccountInfosIndex::SystemProgram as usize
    }

    fn cpi_context_index() -> usize {
        // Generic instruction doesn't use cpi context
        0
    }
}

