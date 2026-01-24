use light_sdk_types::{RentSponsor, RentSponsors};
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

/// Derives a single rent sponsor PDA for a given program and version.
///
/// Seeds: ["rent_sponsor", <u16 version little-endian>]
#[inline]
pub fn derive_rent_sponsor_pda(program_id: &Pubkey, version: u16) -> (Pubkey, u8) {
    let version_bytes = version.to_le_bytes();
    let seeds = &[RENT_SPONSOR_SEED, &version_bytes[..]];
    Pubkey::find_program_address(seeds, program_id)
}

/// Derives all 4 rent sponsor PDAs (versions 1-4) for a given program at runtime.
///
/// Seeds: ["rent_sponsor", <u16 version little-endian>]
pub fn derive_rent_sponsors(program_id: &Pubkey) -> RentSponsors {
    let program_id_bytes = program_id.to_bytes();
    let mut sponsors = [RentSponsor {
        program_id: program_id_bytes,
        rent_sponsor: [0u8; 32],
        bump: 0,
        version: 0,
    }; 4];

    for (i, version) in (1u16..=4u16).enumerate() {
        let (pda, bump) = derive_rent_sponsor_pda(program_id, version);
        sponsors[i] = RentSponsor {
            program_id: program_id_bytes,
            rent_sponsor: pda.to_bytes(),
            bump,
            version,
        };
    }

    RentSponsors { sponsors }
}
