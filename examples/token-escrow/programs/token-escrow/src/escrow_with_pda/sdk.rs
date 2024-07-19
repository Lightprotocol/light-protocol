#![cfg(not(target_os = "solana"))]

use anchor_lang::{InstructionData, ToAccountMetas};
use light_compressed_token::{
    process_transfer::get_cpi_authority_pda,
    process_transfer::{
        transfer_sdk::{
            create_inputs_and_remaining_accounts, create_inputs_and_remaining_accounts_checked,
            to_account_metas,
        },
        TokenTransferOutputData,
    },
};
use light_system_program::{
    invoke::processor::CompressedProof,
    sdk::{
        address::add_and_get_remaining_account_indices,
        compressed_account::{CompressedAccount, MerkleContext},
    },
};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

use crate::escrow_with_compressed_pda::sdk::get_token_owner_pda;

#[derive(Debug, Clone, Copy)]
pub struct CreateEscrowInstructionInputs<'a> {
    pub lock_up_time: u64,
    pub signer: &'a Pubkey,
    pub input_merkle_context: &'a [MerkleContext],
    pub output_compressed_account_merkle_tree_pubkeys: &'a [Pubkey],
    pub output_compressed_accounts: &'a [TokenTransferOutputData],
    pub root_indices: &'a [u16],
    pub proof: &'a Option<CompressedProof>,
    pub input_token_data: &'a [light_compressed_token::token_data::TokenData],
    pub input_compressed_accounts: &'a [CompressedAccount],
    pub mint: &'a Pubkey,
}

pub fn get_timelock_pda(signer: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"timelock".as_ref(), signer.as_ref()], &crate::id()).0
}

pub fn create_escrow_instruction(
    input_params: CreateEscrowInstructionInputs,
    escrow_amount: u64,
) -> Instruction {
    let token_owner_pda = get_token_owner_pda(input_params.signer);
    let timelock_pda = get_timelock_pda(input_params.signer);
    // TODO: separate the creation of inputs and remaining accounts
    let (mut remaining_accounts, inputs) = create_inputs_and_remaining_accounts_checked(
        input_params.input_token_data,
        input_params.input_compressed_accounts,
        input_params.input_merkle_context,
        None,
        input_params.output_compressed_accounts,
        input_params.root_indices,
        input_params.proof,
        *input_params.mint,
        input_params.signer,
        false,
        None,
        None,
        None,
    )
    .unwrap();

    let merkle_tree_indices = add_and_get_remaining_account_indices(
        input_params.output_compressed_account_merkle_tree_pubkeys,
        &mut remaining_accounts,
    );

    let instruction_data = crate::instruction::EscrowCompressedTokensWithPda {
        lock_up_time: input_params.lock_up_time,
        escrow_amount,
        proof: input_params.proof.clone().unwrap(),
        mint: *input_params.mint,
        signer_is_delegate: false,
        input_token_data_with_context: inputs.input_token_data_with_context,
        output_state_merkle_tree_account_indices: merkle_tree_indices,
    };

    let registered_program_pda = Pubkey::find_program_address(
        &[light_system_program::ID.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0;
    let compressed_token_cpi_authority_pda = get_cpi_authority_pda().0;
    let account_compression_authority =
        light_system_program::utils::get_cpi_authority_pda(&light_system_program::ID);
    let accounts = crate::accounts::EscrowCompressedTokensWithPda {
        signer: *input_params.signer,
        noop_program: Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
        compressed_token_program: light_compressed_token::ID,
        light_system_program: light_system_program::ID,
        account_compression_program: account_compression::ID,
        registered_program_pda,
        compressed_token_cpi_authority_pda,
        account_compression_authority,
        timelock_pda,
        system_program: solana_sdk::system_program::ID,
        token_owner_pda: token_owner_pda.0,
    };
    let remaining_accounts = to_account_metas(remaining_accounts);

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
    let token_owner_pda = get_token_owner_pda(input_params.signer);
    let timelock_pda = get_timelock_pda(input_params.signer);
    // Token transactions with an invalid signer will just fail with invalid proof verification.
    // Thus, it's recommented to use create_inputs_and_remaining_accounts_checked, which returns a descriptive error in case of a wrong signer.
    // We use unchecked here to perform a failing test with an invalid signer.
    let (mut remaining_accounts, inputs) = create_inputs_and_remaining_accounts(
        input_params.input_token_data,
        input_params.input_compressed_accounts,
        input_params.input_merkle_context,
        None,
        input_params.output_compressed_accounts,
        input_params.root_indices,
        input_params.proof,
        *input_params.mint,
        false,
        None,
        None,
        None,
    );

    let merkle_tree_indices = add_and_get_remaining_account_indices(
        input_params.output_compressed_account_merkle_tree_pubkeys,
        &mut remaining_accounts,
    );

    let instruction_data = crate::instruction::WithdrawCompressedEscrowTokensWithPda {
        bump: token_owner_pda.1,
        withdrawal_amount,
        proof: input_params.proof.clone().unwrap(),
        mint: *input_params.mint,
        signer_is_delegate: false,
        input_token_data_with_context: inputs.input_token_data_with_context,
        output_state_merkle_tree_account_indices: merkle_tree_indices,
    };

    let registered_program_pda = Pubkey::find_program_address(
        &[light_system_program::ID.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0;
    let compressed_token_cpi_authority_pda = get_cpi_authority_pda().0;
    let account_compression_authority =
        light_system_program::utils::get_cpi_authority_pda(&light_system_program::ID);
    let accounts = crate::accounts::EscrowCompressedTokensWithPda {
        signer: *input_params.signer,
        token_owner_pda: token_owner_pda.0,
        noop_program: Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
        compressed_token_program: light_compressed_token::ID,
        light_system_program: light_system_program::ID,
        account_compression_program: account_compression::ID,
        registered_program_pda,
        compressed_token_cpi_authority_pda,
        account_compression_authority,
        timelock_pda,
        system_program: solana_sdk::system_program::ID,
    };
    let remaining_accounts = to_account_metas(remaining_accounts);

    Instruction {
        program_id: crate::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),

        data: instruction_data.data(),
    }
}
