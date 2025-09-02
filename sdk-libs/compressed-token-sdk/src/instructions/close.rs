use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

/// Creates a `CloseAccount` instruction.
pub fn close_account(
    token_program_id: &Pubkey,
    account_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
) -> Instruction {
    // TODO: do manual serialization
    let data = spl_token_2022::instruction::TokenInstruction::CloseAccount.pack();

    let accounts = vec![
        AccountMeta::new(*account_pubkey, false),
        AccountMeta::new(*destination_pubkey, false),
        AccountMeta::new_readonly(*owner_pubkey, true), // signer
    ];

    Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    }
}
