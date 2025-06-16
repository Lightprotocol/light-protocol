use light_compressed_token_types::constants::{POOL_SEED, PROGRAM_ID};
use solana_pubkey::Pubkey;

pub fn get_token_pool_pda(mint: &Pubkey) -> Pubkey {
    get_token_pool_pda_with_index(mint, 0)
}

pub fn find_token_pool_pda_with_index(mint: &Pubkey, token_pool_index: u8) -> (Pubkey, u8) {
    let seeds = &[POOL_SEED, mint.as_ref(), &[token_pool_index]];
    let seeds = if token_pool_index == 0 {
        &seeds[..2]
    } else {
        &seeds[..]
    };
    Pubkey::find_program_address(seeds, &Pubkey::from(PROGRAM_ID))
}

pub fn get_token_pool_pda_with_index(mint: &Pubkey, token_pool_index: u8) -> Pubkey {
    find_token_pool_pda_with_index(mint, token_pool_index).0
}
