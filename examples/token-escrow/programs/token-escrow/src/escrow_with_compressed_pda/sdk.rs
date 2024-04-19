#![cfg(not(target_os = "solana"))]

use crate::escrow_with_compressed_pda::escrow::PackedInputCompressedPda;
use account_compression::{Pubkey, NOOP_PROGRAM_ID};
use anchor_lang::{InstructionData, ToAccountMetas};
use psp_compressed_pda::{
    compressed_account::{pack_merkle_context, MerkleContext},
    compressed_cpi::CompressedCpiContext,
    pack_new_address_params,
    utils::CompressedProof,
    NewAddressParams,
};
use psp_compressed_token::{
    transfer_sdk::{create_inputs_and_remaining_accounts_checked, to_account_metas},
    TokenTransferOutputData,
};
use solana_sdk::instruction::Instruction;

#[derive(Debug, Clone)]
pub struct CreateCompressedPdaEscrowInstructionInputs<'a> {
    pub lock_up_time: u64,
    pub signer: &'a Pubkey,
    pub input_compressed_account_merkle_tree_pubkeys: &'a [Pubkey],
    pub nullifier_array_pubkeys: &'a [Pubkey],
    pub output_compressed_account_merkle_tree_pubkeys: &'a [Pubkey],
    pub output_compressed_accounts: &'a [TokenTransferOutputData],
    pub root_indices: &'a [u16],
    pub leaf_indices: &'a [u32],
    pub proof: &'a CompressedProof,
    pub input_token_data: &'a [psp_compressed_token::TokenData],
    pub mint: &'a Pubkey,
    pub new_address_params: NewAddressParams,
    pub cpi_signature_account: &'a Pubkey,
}

pub fn create_escrow_instruction(
    input_params: CreateCompressedPdaEscrowInstructionInputs,
    escrow_amount: u64,
) -> Instruction {
    let token_owner_pda = get_token_owner_pda(&input_params.signer);
    let (mut remaining_accounts, inputs) = create_inputs_and_remaining_accounts_checked(
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
    let new_address_params =
        pack_new_address_params(&[input_params.new_address_params], &mut remaining_accounts);
    let cpi_signature_account_index: u8 =
        match remaining_accounts.get(input_params.cpi_signature_account) {
            Some(entry) => (*entry).try_into().unwrap(),
            None => {
                remaining_accounts.insert(
                    *input_params.cpi_signature_account,
                    remaining_accounts.len(),
                );
                (remaining_accounts.len() - 1) as u8
            }
        };

    let cpi_context = CompressedCpiContext {
        execute: true,
        cpi_signature_account_index,
    };
    let instruction_data = crate::instruction::EscrowCompressedTokensWithCompressedPda {
        lock_up_time: input_params.lock_up_time,
        escrow_amount,
        proof: Some(input_params.proof.clone()),
        root_indices: input_params.root_indices.to_vec(),
        mint: *input_params.mint,
        signer_is_delegate: false,
        input_token_data_with_context: inputs.input_token_data_with_context,
        output_state_merkle_tree_account_indices: inputs.output_state_merkle_tree_account_indices,
        pubkey_array: inputs.pubkey_array,
        new_address_params: new_address_params[0].clone(),
        cpi_context,
    };

    let registered_program_pda = Pubkey::find_program_address(
        &[psp_compressed_pda::ID.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0;
    let compressed_token_cpi_authority_pda = psp_compressed_token::get_cpi_authority_pda().0;
    let account_compression_authority =
        psp_compressed_pda::utils::get_cpi_authority_pda(&psp_compressed_pda::ID);

    let accounts = crate::accounts::EscrowCompressedTokensWithCompressedPda {
        signer: *input_params.signer,
        noop_program: NOOP_PROGRAM_ID,
        compressed_token_program: psp_compressed_token::ID,
        compressed_pda_program: psp_compressed_pda::ID,
        account_compression_program: account_compression::ID,
        registered_program_pda,
        compressed_token_cpi_authority_pda,
        account_compression_authority,
        self_program: crate::ID,
        token_owner_pda: token_owner_pda.0,
    };
    let remaining_accounts = to_account_metas(remaining_accounts);

    Instruction {
        program_id: crate::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),

        data: instruction_data.data(),
    }
}

pub fn get_token_owner_pda(signer: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"escrow".as_ref(), signer.to_bytes().as_ref()],
        &crate::id(),
    )
}

#[derive(Debug, Clone)]
pub struct CreateCompressedPdaWithdrawalInstructionInputs<'a> {
    pub signer: &'a Pubkey,
    pub input_compressed_account_merkle_tree_pubkeys: &'a [Pubkey],
    pub nullifier_array_pubkeys: &'a [Pubkey],
    pub output_compressed_account_merkle_tree_pubkeys: &'a [Pubkey],
    pub output_compressed_accounts: &'a [TokenTransferOutputData],
    pub root_indices: &'a [u16],
    pub leaf_indices: &'a [u32],
    pub proof: &'a CompressedProof,
    pub input_token_data: &'a [psp_compressed_token::TokenData],
    pub mint: &'a Pubkey,
    pub old_lock_up_time: u64,
    pub new_lock_up_time: u64,
    pub address: [u8; 32],
    pub merkle_context: MerkleContext,
    pub cpi_signature_account: &'a Pubkey,
}

pub fn create_withdrawal_instruction(
    input_params: CreateCompressedPdaWithdrawalInstructionInputs,
    withdrawal_amount: u64,
) -> Instruction {
    let (token_owner_pda, bump) = get_token_owner_pda(&input_params.signer);
    let (mut remaining_accounts, inputs) = create_inputs_and_remaining_accounts_checked(
        input_params.input_compressed_account_merkle_tree_pubkeys,
        &input_params.leaf_indices[1..],
        input_params.input_token_data,
        input_params.nullifier_array_pubkeys,
        input_params.output_compressed_account_merkle_tree_pubkeys,
        None,
        input_params.output_compressed_accounts,
        input_params.root_indices,
        input_params.proof,
        *input_params.mint,
        &token_owner_pda,
        false,
        None,
    )
    .unwrap();
    let merkle_context_packed =
        pack_merkle_context(&[input_params.merkle_context], &mut remaining_accounts);
    let cpi_signature_account_index: u8 =
        match remaining_accounts.get(input_params.cpi_signature_account) {
            Some(entry) => (*entry).try_into().unwrap(),
            None => {
                remaining_accounts.insert(
                    *input_params.cpi_signature_account,
                    remaining_accounts.len(),
                );
                (remaining_accounts.len() - 1) as u8
            }
        };

    let cpi_context = CompressedCpiContext {
        execute: true,
        cpi_signature_account_index,
    };

    let input_compressed_pda = PackedInputCompressedPda {
        old_lock_up_time: input_params.old_lock_up_time,
        new_lock_up_time: input_params.new_lock_up_time,
        address: input_params.address,
        merkle_context: merkle_context_packed[0].clone(),
    };
    let instruction_data = crate::instruction::WithdrawCompressedTokensWithCompressedPda {
        proof: Some(input_params.proof.clone()),
        root_indices: input_params.root_indices.to_vec(),
        mint: *input_params.mint,
        signer_is_delegate: false,
        input_token_data_with_context: inputs.input_token_data_with_context,
        output_state_merkle_tree_account_indices: inputs.output_state_merkle_tree_account_indices,
        pubkey_array: inputs.pubkey_array,
        cpi_context,
        input_compressed_pda,
        withdrawal_amount,
        bump,
    };

    let registered_program_pda = Pubkey::find_program_address(
        &[psp_compressed_pda::ID.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0;
    let compressed_token_cpi_authority_pda = psp_compressed_token::get_cpi_authority_pda().0;
    let account_compression_authority =
        psp_compressed_pda::utils::get_cpi_authority_pda(&psp_compressed_pda::ID);

    let accounts = crate::accounts::EscrowCompressedTokensWithCompressedPda {
        signer: *input_params.signer,
        noop_program: NOOP_PROGRAM_ID,
        compressed_token_program: psp_compressed_token::ID,
        compressed_pda_program: psp_compressed_pda::ID,
        account_compression_program: account_compression::ID,
        registered_program_pda,
        compressed_token_cpi_authority_pda,
        account_compression_authority,
        self_program: crate::ID,
        token_owner_pda,
    };
    let remaining_accounts = to_account_metas(remaining_accounts);

    Instruction {
        program_id: crate::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),

        data: instruction_data.data(),
    }
}
