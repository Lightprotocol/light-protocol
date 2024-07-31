use account_compression::utils::constants::GROUP_AUTHORITY_SEED;
use anchor_lang::solana_program::pubkey::Pubkey;

use crate::{AUTHORITY_PDA_SEED, FORESTER_EPOCH_SEED, FORESTER_SEED};

pub fn get_group_pda(seed: Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[GROUP_AUTHORITY_SEED, seed.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0
}

pub fn get_protocol_config_pda_address() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[AUTHORITY_PDA_SEED], &crate::ID)
}

pub fn get_cpi_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[crate::CPI_AUTHORITY_PDA_SEED], &crate::ID)
}

pub fn get_forester_epoch_pda_address(authority: &Pubkey, epoch: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            FORESTER_EPOCH_SEED,
            authority.to_bytes().as_slice(),
            epoch.to_le_bytes().as_slice(),
        ],
        &crate::ID,
    )
}

pub fn get_forester_pda_address(authority: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[FORESTER_SEED, authority.to_bytes().as_slice()],
        &crate::ID,
    )
}

pub fn get_epoch_pda_address(epoch: u64) -> Pubkey {
    Pubkey::find_program_address(&[&epoch.to_le_bytes()], &crate::ID).0
}
