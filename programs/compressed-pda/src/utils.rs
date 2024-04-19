use anchor_lang::{
    prelude::borsh, solana_program::pubkey::Pubkey, AnchorDeserialize, AnchorSerialize,
};

pub fn get_registered_program_pda(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[program_id.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0
}

pub fn get_cpi_authority_pda(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"cpi_authority"], program_id).0
}

#[derive(Debug, Clone, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct CompressedProof {
    pub a: [u8; 32],
    pub b: [u8; 64],
    pub c: [u8; 32],
}
