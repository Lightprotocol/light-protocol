use light_client::rpc::RpcError;
use light_compressed_token_sdk::{SPL_TOKEN_2022_PROGRAM_ID, SPL_TOKEN_PROGRAM_ID};
use light_sdk::constants::CPI_AUTHORITY_PDA_SEED;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

pub const CREATE_TOKEN_POOL_DISCRIMINATOR: [u8; 8] = [23, 169, 27, 122, 147, 169, 209, 152];
pub const TOKEN_POOL_SEED: &[u8] = b"pool";

/// Creates an instruction to create a token pool PDA for a given mint
///
/// This creates a token pool account that is owned by the CPI authority PDA
/// and can hold SPL tokens for compression/decompression operations.
///
/// # Arguments
/// * `fee_payer` - Account that pays for the transaction fees
/// * `mint` - The SPL mint for which to create the token pool
/// * `is_token_22` - Whether this is a Token-2022 mint (vs regular SPL Token)
///
/// # Returns
/// `Result<Instruction, RpcError>` - The create token pool instruction
pub fn create_token_pool_instruction(
    fee_payer: &Pubkey,
    mint: &Pubkey,
    is_token_22: bool,
) -> Result<Instruction, RpcError> {
    let compressed_token_program_id = Pubkey::from(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID);

    let (token_pool_pda, _bump) = Pubkey::find_program_address(
        &[TOKEN_POOL_SEED, mint.as_ref()],
        &compressed_token_program_id,
    );

    let (cpi_authority_pda, _cpi_bump) =
        Pubkey::find_program_address(&[CPI_AUTHORITY_PDA_SEED], &compressed_token_program_id);

    let token_program = if is_token_22 {
        Pubkey::from(SPL_TOKEN_2022_PROGRAM_ID)
    } else {
        Pubkey::from(SPL_TOKEN_PROGRAM_ID)
    };

    let mut instruction_data = Vec::new();
    instruction_data.extend_from_slice(&CREATE_TOKEN_POOL_DISCRIMINATOR);

    let instruction = Instruction {
        program_id: compressed_token_program_id,
        accounts: vec![
            AccountMeta::new(*fee_payer, true), // fee_payer (signer, writable)
            AccountMeta::new(token_pool_pda, false), // token_pool_pda (writable)
            AccountMeta::new_readonly(Pubkey::from([0; 32]), false), // system_program
            AccountMeta::new(*mint, false),     // mint (writable)
            AccountMeta::new_readonly(token_program, false), // token_program
            AccountMeta::new_readonly(cpi_authority_pda, false), // cpi_authority_pda
        ],
        data: instruction_data,
    };

    Ok(instruction)
}

/// Helper function to derive token pool PDA address
pub fn get_token_pool_pda(mint: &Pubkey) -> Pubkey {
    get_token_pool_pda_with_index(mint, 0)
}

/// Helper function to derive token pool PDA address with specific index
pub fn get_token_pool_pda_with_index(mint: &Pubkey, token_pool_index: u8) -> Pubkey {
    find_token_pool_pda_with_index(mint, token_pool_index).0
}

/// Helper function to find token pool PDA address and bump with specific index
pub fn find_token_pool_pda_with_index(mint: &Pubkey, token_pool_index: u8) -> (Pubkey, u8) {
    const POOL_SEED: &[u8] = b"pool";
    let compressed_token_program_id = Pubkey::from(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID);

    let seeds = &[POOL_SEED, mint.as_ref(), &[token_pool_index]];
    let seeds = if token_pool_index == 0 {
        &seeds[..2] // For index 0, we don't include the index byte
    } else {
        &seeds[..]
    };

    Pubkey::find_program_address(seeds, &compressed_token_program_id)
}

/// Helper function to derive CPI authority PDA
pub fn get_cpi_authority_pda() -> Pubkey {
    const CPI_AUTHORITY_PDA_SEED: &[u8] = b"cpi_authority";
    let compressed_token_program_id = Pubkey::from(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID);

    Pubkey::find_program_address(&[CPI_AUTHORITY_PDA_SEED], &compressed_token_program_id).0
}
