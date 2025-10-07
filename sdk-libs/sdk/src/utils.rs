use solana_pubkey::Pubkey;

#[allow(unused_imports)]
use crate::constants::CPI_AUTHORITY_PDA_SEED;
#[macro_export]
macro_rules! find_cpi_signer_macro {
    ($program_id:expr) => {
        Pubkey::find_program_address([CPI_AUTHORITY_PDA_SEED].as_slice(), $program_id)
    };
}

pub fn get_light_cpi_signer_seeds(program_id: &Pubkey) -> ([Vec<u8>; 2], Pubkey) {
    let seeds = &[CPI_AUTHORITY_PDA_SEED];

    let (pda, bump) = solana_pubkey::Pubkey::find_program_address(seeds, program_id);

    let signer_seeds_bump = bump;

    let signer_seeds: [Vec<u8>; 2] = [CPI_AUTHORITY_PDA_SEED.to_vec(), vec![signer_seeds_bump]];
    (signer_seeds, pda)
}
