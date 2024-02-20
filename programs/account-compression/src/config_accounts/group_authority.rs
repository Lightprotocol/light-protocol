use aligned_sized::aligned_sized;
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

#[account]
#[aligned_sized(anchor)]
#[derive(Debug)]
pub struct GroupAuthority {
    pub authority: Pubkey,
    pub seed: [u8; 32],
}
