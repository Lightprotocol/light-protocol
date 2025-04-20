use account_compression::processor::initialize_address_merkle_tree::Pubkey;
use anchor_lang::solana_program::{instruction::Instruction, system_instruction};
use solana_sdk::signature::{Keypair, Signer};

pub fn create_account_instruction(
    payer: &Pubkey,
    size: usize,
    rent: u64,
    id: &Pubkey,
    keypair: Option<&Keypair>,
) -> Instruction {
    let keypair = match keypair {
        Some(keypair) => keypair.insecure_clone(),
        None => Keypair::new(),
    };
    system_instruction::create_account(payer, &keypair.pubkey(), rent, size as u64, id)
}
