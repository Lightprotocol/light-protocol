#[allow(unused_imports)]
use crate::constants::CPI_AUTHORITY_PDA_SEED;
#[macro_export]
macro_rules! find_cpi_signer_macro {
    ($program_id:expr) => {
        Pubkey::find_program_address([CPI_AUTHORITY_PDA_SEED].as_slice(), $program_id)
    };
}
