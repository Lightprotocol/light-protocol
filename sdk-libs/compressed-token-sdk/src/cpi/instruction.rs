#[cfg(feature = "anchor")]
use anchor_lang::AnchorSerialize;
#[cfg(not(feature = "anchor"))]
use borsh::BorshSerialize as AnchorSerialize;
use light_compressed_account::instruction_data::{
    compressed_proof::CompressedProof, cpi_context::CompressedCpiContext,
};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    cpi::accounts::CompressedTokenDecompressCpiAccounts,
    state::{CompressedTokenInstructionDataTransfer, InputTokenDataWithContext},
};

/// Return Instruction to decompress compressed token accounts.
/// Proof can be None if prove_by_index is used.
pub fn decompress(
    mint: &Pubkey,
    compressed_token_accounts: Vec<InputTokenDataWithContext>,
    proof: &Option<CompressedProof>,
    light_cpi_accounts: &CompressedTokenDecompressCpiAccounts,
    cpi_context: Option<&CompressedCpiContext>,
) -> Result<Instruction, ProgramError> {
    let data =
        decompress_token_instruction_data(mint, proof, compressed_token_accounts, cpi_context);

    let accounts = vec![
        AccountMeta::new(*light_cpi_accounts.fee_payer.key, true),
        AccountMeta::new_readonly(*light_cpi_accounts.authority.key, true),
        AccountMeta::new_readonly(*light_cpi_accounts.cpi_authority_pda.key, true),
        AccountMeta::new_readonly(*light_cpi_accounts.light_system_program.key, false),
        AccountMeta::new_readonly(*light_cpi_accounts.registered_program_pda.key, false),
        AccountMeta::new_readonly(*light_cpi_accounts.noop_program.key, false),
        AccountMeta::new_readonly(*light_cpi_accounts.account_compression_authority.key, false),
        AccountMeta::new_readonly(*light_cpi_accounts.account_compression_program.key, false),
        AccountMeta::new_readonly(*light_cpi_accounts.self_program.key, false),
        AccountMeta::new(*light_cpi_accounts.token_pool_pda.key, false),
        AccountMeta::new(*light_cpi_accounts.decompress_destination.key, false),
        AccountMeta::new_readonly(*light_cpi_accounts.token_program.key, false),
        AccountMeta::new_readonly(*light_cpi_accounts.system_program.key, false),
        AccountMeta::new(*light_cpi_accounts.state_merkle_tree.key, false),
        AccountMeta::new(*light_cpi_accounts.queue.key, false),
    ];

    Ok(Instruction {
        program_id: *light_cpi_accounts.token_program.key,
        accounts,
        data,
    })
}

/// Return Instruction Data to decompress compressed token accounts.
pub fn decompress_token_instruction_data(
    mint: &Pubkey,
    proof: &Option<CompressedProof>,
    compressed_token_accounts: Vec<InputTokenDataWithContext>,
    cpi_context: Option<&CompressedCpiContext>,
) -> Vec<u8> {
    let amount = compressed_token_accounts
        .iter()
        .map(|data| data.amount)
        .sum();

    let compressed_token_instruction_data_transfer = CompressedTokenInstructionDataTransfer {
        proof: *proof,
        mint: *mint,
        delegated_transfer: None,
        input_token_data_with_context: compressed_token_accounts,
        output_compressed_accounts: Vec::new(),
        is_compress: false,
        compress_or_decompress_amount: Some(amount),
        cpi_context: cpi_context.copied(),
        lamports_change_account_merkle_tree_index: None,
    };

    let mut inputs = Vec::new();

    compressed_token_instruction_data_transfer
        .serialize(&mut inputs)
        .unwrap();
    inputs
}
