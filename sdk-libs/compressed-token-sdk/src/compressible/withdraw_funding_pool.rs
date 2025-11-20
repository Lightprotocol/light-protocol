use light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

/// Creates a withdraw funding pool instruction to withdraw SOL from the pool PDA
///
/// # Arguments
/// * `pool_pda` - The pool PDA that holds the funds
/// * `pool_pda_bump` - The bump seed for the pool PDA
/// * `authority` - The authority (must be a signer and match pool PDA derivation)
/// * `destination` - The destination account to receive the withdrawn funds
/// * `amount` - The amount of lamports to withdraw
///
/// # Returns
/// The withdraw funding pool instruction
pub fn withdraw_funding_pool(
    pool_pda: Pubkey,
    pool_pda_bump: u8,
    authority: Pubkey,
    destination: Pubkey,
    amount: u64,
) -> Instruction {
    // Build instruction data: [discriminator: u8][bump: u8][amount: u64]
    let mut instruction_data = vec![105u8]; // WithdrawFundingPool instruction discriminator
    instruction_data.push(pool_pda_bump);
    instruction_data.extend_from_slice(&amount.to_le_bytes());

    let accounts = vec![
        // Pool PDA (source of funds) - must be writable
        AccountMeta::new(pool_pda, false),
        // Authority (signer) - must match pool PDA derivation
        AccountMeta::new_readonly(authority, true),
        // Destination (receives funds) - must be writable
        AccountMeta::new(destination, false),
        // System program
        AccountMeta::new_readonly(Pubkey::default(), false),
    ];

    Instruction {
        program_id: Pubkey::from(COMPRESSED_TOKEN_PROGRAM_ID),
        accounts,
        data: instruction_data,
    }
}
