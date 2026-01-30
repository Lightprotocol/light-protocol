use light_sdk_types::RentSponsor;
use solana_pubkey::Pubkey;

#[allow(unused_imports)]
use crate::constants::{CPI_AUTHORITY_PDA_SEED, RENT_SPONSOR_SEED};
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

/// Derives the rent sponsor PDA for a given program (version 1, hardcoded).
///
/// Seeds: ["rent_sponsor", <1u16 little-endian>]
#[inline]
pub fn derive_rent_sponsor(program_id: &Pubkey) -> RentSponsor {
    const VERSION: u16 = 1;
    let version_bytes = VERSION.to_le_bytes();
    let seeds = &[RENT_SPONSOR_SEED, &version_bytes[..]];
    let (pda, bump) = Pubkey::find_program_address(seeds, program_id);
    RentSponsor {
        program_id: program_id.to_bytes(),
        rent_sponsor: pda.to_bytes(),
        bump,
    }
}
