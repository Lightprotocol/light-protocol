use crate::{Pubkey, CPI_AUTHORITY_PDA_SEED, PROGRAM_ID_ACCOUNT_COMPRESSION};

pub fn get_registered_program_pda(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[program_id.to_bytes().as_slice()],
        &PROGRAM_ID_ACCOUNT_COMPRESSION,
    )
    .0
}

pub fn find_cpi_signer(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address([CPI_AUTHORITY_PDA_SEED].as_slice(), program_id).0
}

pub fn get_cpi_authority_pda(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[CPI_AUTHORITY_PDA_SEED], program_id).0
}

#[macro_export]
macro_rules! find_cpi_signer_macro {
    ($program_id:expr) => {
        Pubkey::find_program_address([CPI_AUTHORITY_PDA_SEED].as_slice(), $program_id)
    };
}
