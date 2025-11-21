use light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

/// Derives the pool PDA and bump seed for a given rent authority
///
/// # Arguments
/// * `compression_authority` - The rent authority pubkey
///
/// # Returns
/// Tuple of (pool_pda, bump_seed)
#[deprecated] // TODO: remove
pub fn derive_pool_pda(compression_authority: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"pool", compression_authority.as_ref()],
        &Pubkey::from(COMPRESSED_TOKEN_PROGRAM_ID),
    )
}

/// Creates a claim instruction to claim rent from compressible token accounts
///
/// # Arguments
/// * `pool_pda` - The pool PDA that will receive the claimed rent
/// * `pool_pda_bump` - The bump seed for the pool PDA
/// * `compression_authority` - The rent authority (must be a signer)
/// * `token_accounts` - List of token accounts to claim from
///
/// # Returns
/// The claim instruction
pub fn claim(
    pool_pda: Pubkey,
    pool_pda_bump: u8,
    compression_authority: Pubkey,
    token_accounts: &[Pubkey],
) -> Instruction {
    let mut instruction_data = vec![104u8]; // Claim instruction discriminator
    instruction_data.push(pool_pda_bump);

    let mut accounts = vec![
        // Pool PDA (receives claimed rent) - must be writable to receive lamports
        AccountMeta::new(pool_pda, false),
        // Rent authority (signer only, not mutable)
        AccountMeta::new_readonly(compression_authority, true),
    ];

    // Add all token accounts to claim from
    for token_account in token_accounts {
        accounts.push(AccountMeta::new(*token_account, false));
    }

    Instruction {
        program_id: Pubkey::from(COMPRESSED_TOKEN_PROGRAM_ID),
        accounts,
        data: instruction_data,
    }
}
