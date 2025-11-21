use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

/// Creates a `CloseAccount` instruction for non-compressible accounts (3 accounts).
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
        AccountMeta::new(*owner_pubkey, true), // signer, mutable to receive write_top_up
    ];

    Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    }
}

/// Creates a `CloseAccount` instruction for compressible accounts (4 accounts).
/// For compressible accounts, a rent_sponsor account is required.
pub fn close_compressible_account(
    token_program_id: &Pubkey,
    account_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
    rent_sponsor_pubkey: &Pubkey,
) -> Instruction {
    // TODO: do manual serialization
    let data = spl_token_2022::instruction::TokenInstruction::CloseAccount.pack();

    let accounts = vec![
        AccountMeta::new(*account_pubkey, false),
        AccountMeta::new(*destination_pubkey, false),
        AccountMeta::new(*owner_pubkey, true),         // signer
        AccountMeta::new(*rent_sponsor_pubkey, false), // rent_sponsor for compressible accounts
    ];

    Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    }
}
