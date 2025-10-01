use solana_pubkey::Pubkey;

#[allow(unused_imports)]
use crate::constants::CPI_AUTHORITY_PDA_SEED;
#[macro_export]
macro_rules! find_cpi_signer_macro {
    ($program_id:expr) => {
        Pubkey::find_program_address([CPI_AUTHORITY_PDA_SEED].as_slice(), $program_id)
    };
}

pub fn get_light_cpi_signer_seeds(program_id: &Pubkey) -> (Vec<Vec<u8>>, Pubkey) {
    let seeds = &[b"cpi_authority".as_slice()];

    // Compute the PDA at compile time
    let (pda, bump) = solana_pubkey::Pubkey::find_program_address(seeds, program_id);

    let token_signer_seeds_bump = bump;

    let token_signer_seeds: Vec<Vec<u8>> = vec![
        CPI_AUTHORITY_PDA_SEED.to_vec(),
        vec![token_signer_seeds_bump],
    ];
    (token_signer_seeds, pda)
}
