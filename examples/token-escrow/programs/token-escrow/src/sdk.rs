#![cfg(not(target_os = "solana"))]

use account_compression::{Pubkey, NOOP_PROGRAM_ID};
use anchor_lang::{InstructionData, ToAccountMetas};
use light_compressed_pda::utils::CompressedProof;
use light_compressed_token::{
    transfer_sdk::create_inputs_and_remaining_accounts_checked, TokenTransferOutputData,
};
use solana_sdk::instruction::Instruction;

#[derive(Debug, Clone, Copy)]
pub struct CreateEscrowInstructionInputs<'a> {
    pub lock_up_time: u64,
    pub signer: &'a Pubkey,
    pub input_compressed_account_merkle_tree_pubkeys: &'a [Pubkey],
    pub nullifier_array_pubkeys: &'a [Pubkey],
    pub output_compressed_account_merkle_tree_pubkeys: &'a [Pubkey],
    pub output_compressed_accounts: &'a [TokenTransferOutputData],
    pub root_indices: &'a [u16],
    pub leaf_indices: &'a [u32],
    pub proof: &'a CompressedProof,
    pub input_token_data: &'a [light_compressed_token::TokenData],
    pub mint: &'a Pubkey,
}

pub fn create_escrow_instruction(
    input_params: CreateEscrowInstructionInputs,
    escrow_amount: u64,
) -> Instruction {
    let cpi_signer = Pubkey::find_program_address(
        &[b"escrow".as_ref(), input_params.signer.as_ref()],
        &crate::id(),
    );
    let timelock_pda = Pubkey::find_program_address(
        &[b"timelock".as_ref(), input_params.signer.as_ref()],
        &crate::id(),
    )
    .0;
    let (remaining_accounts, inputs) = create_inputs_and_remaining_accounts_checked(
        input_params.input_compressed_account_merkle_tree_pubkeys,
        input_params.leaf_indices,
        input_params.input_token_data,
        input_params.nullifier_array_pubkeys,
        input_params.output_compressed_account_merkle_tree_pubkeys,
        None,
        input_params.output_compressed_accounts,
        input_params.root_indices,
        input_params.proof,
        *input_params.mint,
        input_params.signer,
        false,
        None,
    )
    .unwrap();

    let instruction_data = crate::instruction::EscrowCompressedTokensWithPda {
        lock_up_time: input_params.lock_up_time,
        escrow_amount,
        proof: Some(input_params.proof.clone()),
        root_indices: input_params.root_indices.to_vec(),
        mint: *input_params.mint,
        signer_is_delegate: false,
        input_token_data_with_context: inputs.input_token_data_with_context,
        output_state_merkle_tree_account_indices: inputs.output_state_merkle_tree_account_indices,
    };

    let registered_program_pda = Pubkey::find_program_address(
        &[light_compressed_pda::ID.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0;
    let compressed_token_cpi_authority_pda = light_compressed_token::get_cpi_authority_pda().0;
    let account_compression_authority =
        light_compressed_pda::utils::get_cpi_authority_pda(&light_compressed_pda::ID);
    let accounts = crate::accounts::EscrowCompressedTokensWithPda {
        signer: *input_params.signer,
        cpi_signer: cpi_signer.0,
        noop_program: NOOP_PROGRAM_ID,
        compressed_token_program: light_compressed_token::ID,
        compressed_pda_program: light_compressed_pda::ID,
        account_compression_program: account_compression::ID,
        registered_program_pda,
        compressed_token_cpi_authority_pda,
        account_compression_authority,
        timelock_pda,
        system_program: solana_sdk::system_program::ID,
    };

    Instruction {
        program_id: crate::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),

        data: instruction_data.data(),
    }
}

pub fn create_withdrawal_escrow_instruction(
    input_params: CreateEscrowInstructionInputs,
    withdrawal_amount: u64,
) -> Instruction {
    let cpi_signer = Pubkey::find_program_address(
        &[b"escrow".as_ref(), input_params.signer.as_ref()],
        &crate::id(),
    );
    let timelock_pda = Pubkey::find_program_address(
        &[b"timelock".as_ref(), input_params.signer.as_ref()],
        &crate::id(),
    )
    .0;
    let (remaining_accounts, inputs) = create_inputs_and_remaining_accounts_checked(
        input_params.input_compressed_account_merkle_tree_pubkeys,
        input_params.leaf_indices,
        input_params.input_token_data,
        input_params.nullifier_array_pubkeys,
        input_params.output_compressed_account_merkle_tree_pubkeys,
        None,
        input_params.output_compressed_accounts,
        input_params.root_indices,
        input_params.proof,
        *input_params.mint,
        &cpi_signer.0,
        false,
        None,
    )
    .unwrap();

    let instruction_data = crate::instruction::WithdrawCompressedEscrowTokensWithPda {
        bump: cpi_signer.1,
        withdrawal_amount,
        proof: Some(input_params.proof.clone()),
        root_indices: input_params.root_indices.to_vec(),
        mint: *input_params.mint,
        signer_is_delegate: false,
        input_token_data_with_context: inputs.input_token_data_with_context,
        output_state_merkle_tree_account_indices: inputs.output_state_merkle_tree_account_indices,
    };

    let registered_program_pda = Pubkey::find_program_address(
        &[light_compressed_pda::ID.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0;
    let compressed_token_cpi_authority_pda = light_compressed_token::get_cpi_authority_pda().0;
    let account_compression_authority =
        light_compressed_pda::utils::get_cpi_authority_pda(&light_compressed_pda::ID);
    let accounts = crate::accounts::EscrowCompressedTokensWithPda {
        signer: *input_params.signer,
        cpi_signer: cpi_signer.0,
        noop_program: NOOP_PROGRAM_ID,
        compressed_token_program: light_compressed_token::ID,
        compressed_pda_program: light_compressed_pda::ID,
        account_compression_program: account_compression::ID,
        registered_program_pda,
        compressed_token_cpi_authority_pda,
        account_compression_authority,
        timelock_pda,
        system_program: solana_sdk::system_program::ID,
    };

    Instruction {
        program_id: crate::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),

        data: instruction_data.data(),
    }
}
