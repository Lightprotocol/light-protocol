pub const CPI_AUTHORITY_PDA_BUMP: u8 = 255;

pub const CPI_AUTHORITY_PDA_SEED: &[u8] = b"cpi_authority";

pub const MAX_OUTPUT_ACCOUNTS: usize = 30;

// discriminator of CpiContextAccount2
pub const CPI_CONTEXT_ACCOUNT_2_DISCRIMINATOR: [u8; 8] = [34, 184, 183, 14, 100, 80, 183, 124];
pub const CPI_CONTEXT_ACCOUNT_1_DISCRIMINATOR: [u8; 8] = [22, 20, 149, 218, 74, 204, 128, 166];

#[cfg(test)]
mod test {
    use light_compressed_account::constants::ACCOUNT_COMPRESSION_PROGRAM_ID;
    use solana_pubkey::Pubkey;

    use super::*;

    fn check_hardcoded_bump(program_id: Pubkey, seeds: &[&[u8]], bump: u8) -> bool {
        let (_, found_bump) = Pubkey::find_program_address(seeds, &program_id);
        found_bump == bump
    }

    #[test]
    fn test_account_compression_cpi_authority_bump() {
        assert!(check_hardcoded_bump(
            ACCOUNT_COMPRESSION_PROGRAM_ID.into(),
            &[CPI_AUTHORITY_PDA_SEED],
            CPI_AUTHORITY_PDA_BUMP
        ));
    }
}
