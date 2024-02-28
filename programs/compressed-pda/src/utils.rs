use anchor_lang::solana_program::pubkey::Pubkey;

pub fn get_registered_program_pda(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[program_id.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0
}

pub fn get_cpi_authority_pda(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[
            b"cpi_authority",
            account_compression::ID.to_bytes().as_slice(),
        ],
        program_id,
    )
    .0
}
