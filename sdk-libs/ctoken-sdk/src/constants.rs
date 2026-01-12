use solana_pubkey::Pubkey;

pub const SPL_TOKEN_PROGRAM_ID: Pubkey =
    Pubkey::new_from_array(light_token_types::SPL_TOKEN_PROGRAM_ID);

pub const SPL_TOKEN_2022_PROGRAM_ID: Pubkey =
    Pubkey::new_from_array(light_token_types::SPL_TOKEN_2022_PROGRAM_ID);

pub const LIGHT_SYSTEM_PROGRAM_ID: Pubkey =
    Pubkey::new_from_array(light_token_types::LIGHT_SYSTEM_PROGRAM_ID);

pub const ACCOUNT_COMPRESSION_PROGRAM_ID: Pubkey =
    Pubkey::new_from_array(light_token_types::ACCOUNT_COMPRESSION_PROGRAM_ID);

pub const ACCOUNT_COMPRESSION_AUTHORITY_PDA: Pubkey =
    Pubkey::new_from_array(light_token_types::ACCOUNT_COMPRESSION_AUTHORITY_PDA);

pub const NOOP_PROGRAM_ID: Pubkey = Pubkey::new_from_array(light_token_types::NOOP_PROGRAM_ID);

pub const CPI_AUTHORITY_PDA: Pubkey = Pubkey::new_from_array(light_token_types::CPI_AUTHORITY_PDA);

pub const CTOKEN_PROGRAM_ID: Pubkey = Pubkey::new_from_array(light_token_types::PROGRAM_ID);
