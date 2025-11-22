use anchor_lang::prelude::Pubkey;

pub fn get_user_record_seeds(owner: &Pubkey) -> (Vec<Vec<u8>>, Pubkey) {
    let seeds: &[&[u8]] = &[b"user_record", owner.as_ref()];
    let (pda, bump) = Pubkey::find_program_address(seeds, &crate::ID);
    let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
    for seed in seeds {
        seeds_vec.push(seed.to_vec());
    }
    seeds_vec.push(vec![bump]);
    (seeds_vec, pda)
}

pub fn get_game_session_seeds(session_id: u64) -> (Vec<Vec<u8>>, Pubkey) {
    let session_id_bytes = session_id.to_le_bytes();
    let seeds: &[&[u8]] = &[b"game_session", session_id_bytes.as_ref()];
    let (pda, bump) = Pubkey::find_program_address(seeds, &crate::ID);
    let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
    for seed in seeds {
        seeds_vec.push(seed.to_vec());
    }
    seeds_vec.push(vec![bump]);
    (seeds_vec, pda)
}

pub fn get_placeholder_record_seeds(placeholder_id: u64) -> (Vec<Vec<u8>>, Pubkey) {
    let placeholder_id_bytes = placeholder_id.to_le_bytes();
    let seeds: &[&[u8]] = &[b"placeholder_record", placeholder_id_bytes.as_ref()];
    let (pda, bump) = Pubkey::find_program_address(seeds, &crate::ID);
    let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
    for seed in seeds {
        seeds_vec.push(seed.to_vec());
    }
    seeds_vec.push(vec![bump]);
    (seeds_vec, pda)
}

pub fn get_ctoken_signer_seeds(user: &Pubkey, mint: &Pubkey) -> (Vec<Vec<u8>>, Pubkey) {
    let seeds: &[&[u8]] = &[b"ctoken_signer", user.as_ref(), mint.as_ref()];
    let (pda, bump) = Pubkey::find_program_address(seeds, &crate::ID);
    let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
    for seed in seeds {
        seeds_vec.push(seed.to_vec());
    }
    seeds_vec.push(vec![bump]);
    (seeds_vec, pda)
}
