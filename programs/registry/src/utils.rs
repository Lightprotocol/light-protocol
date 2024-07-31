use account_compression::utils::constants::GROUP_AUTHORITY_SEED;
use anchor_lang::solana_program::pubkey::Pubkey;

use crate::{
    constants::{EPOCH_SEED, FORESTER_EPOCH_SEED, FORESTER_SEED, FORESTER_TOKEN_POOL_SEED},
    PROTOCOL_CONFIG_PDA_SEED,
};

pub fn get_group_pda(seed: Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[GROUP_AUTHORITY_SEED, seed.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0
}

pub fn get_protocol_config_pda_address() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[PROTOCOL_CONFIG_PDA_SEED], &crate::ID)
}

pub fn get_cpi_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[crate::CPI_AUTHORITY_PDA_SEED], &crate::ID)
}

pub fn get_forester_epoch_pda_address(forester_pda_address: &Pubkey, epoch: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            FORESTER_EPOCH_SEED,
            forester_pda_address.to_bytes().as_slice(),
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
    Pubkey::find_program_address(&[EPOCH_SEED, epoch.to_le_bytes().as_slice()], &crate::ID).0
}

pub fn get_forester_token_pool_pda(forester_pda_address: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[FORESTER_TOKEN_POOL_SEED, forester_pda_address.as_ref()],
        &crate::ID,
    )
    .0
}

pub struct ForesterAccounts {
    pub forester_pda: Pubkey,
    pub forester_token_pool: Pubkey,
}

pub fn get_forester_accounts(authority: &Pubkey) -> ForesterAccounts {
    let forester_pda = get_forester_pda_address(authority);
    let forester_token_pool = get_forester_token_pool_pda(&forester_pda.0);
    ForesterAccounts {
        forester_pda: forester_pda.0,
        forester_token_pool,
    }
}
pub struct ForesterAccountsEpoch {
    pub forester_pda: Pubkey,
    pub forester_token_pool: Pubkey,
    pub forester_epoch_pda: Pubkey,
}

pub fn get_forester_accounts_epoch(authority: &Pubkey, epoch: u64) -> ForesterAccountsEpoch {
    let forester_accounts = get_forester_accounts(authority);
    let forester_epoch_pda = get_forester_epoch_pda_address(&forester_accounts.forester_pda, epoch);
    ForesterAccountsEpoch {
        forester_pda: forester_accounts.forester_pda,
        forester_token_pool: forester_accounts.forester_token_pool,
        forester_epoch_pda: forester_epoch_pda.0,
    }
}
