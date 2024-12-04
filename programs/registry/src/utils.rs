use anchor_lang::solana_program::pubkey::Pubkey;

use crate::constants::{FORESTER_EPOCH_SEED, FORESTER_SEED, PROTOCOL_CONFIG_PDA_SEED};

pub fn get_protocol_config_pda_address() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[PROTOCOL_CONFIG_PDA_SEED], &crate::ID)
}

pub fn get_cpi_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[crate::CPI_AUTHORITY_PDA_SEED], &crate::ID)
}

pub fn get_forester_epoch_pda_from_authority(authority: &Pubkey, epoch: u64) -> (Pubkey, u8) {
    let forester_pda = get_forester_pda(authority);
    get_forester_epoch_pda(&forester_pda.0, epoch)
}

pub fn get_forester_epoch_pda_from_derivation(derivation: &Pubkey, epoch: u64) -> (Pubkey, u8) {
    let forester_pda = get_forester_pda(derivation);
    get_forester_epoch_pda(&forester_pda.0, epoch)
}

pub fn get_forester_epoch_pda(forester_pda: &Pubkey, epoch: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            FORESTER_EPOCH_SEED,
            forester_pda.as_ref(),
            epoch.to_le_bytes().as_slice(),
        ],
        &crate::ID,
    )
}

pub fn get_forester_pda(authority: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[FORESTER_SEED, authority.as_ref()], &crate::ID)
}

pub fn get_epoch_pda_address(epoch: u64) -> Pubkey {
    Pubkey::find_program_address(&[&epoch.to_le_bytes()], &crate::ID).0
}
