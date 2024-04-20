#![cfg(not(target_os = "solana"))]

use std::collections::HashMap;

use account_compression::{Pubkey, NOOP_PROGRAM_ID};
use anchor_lang::{InstructionData, ToAccountMetas};
use psp_compressed_pda::{
    compressed_account::CompressedAccountWithMerkleContext, compressed_cpi::CompressedCpiContext,
    pack_new_address_params, utils::CompressedProof, NewAddressParams,
};
use psp_compressed_token::transfer_sdk::to_account_metas;
use solana_sdk::instruction::Instruction;

use crate::CreatePdaMode;

#[derive(Debug, Clone)]
pub struct CreateCompressedPdaInstructionInputs<'a> {
    pub data: [u8; 31],
    pub signer: &'a Pubkey,
    pub output_compressed_account_merkle_tree_pubkey: &'a Pubkey,
    pub proof: &'a CompressedProof,
    pub new_address_params: NewAddressParams,
    pub cpi_signature_account: &'a Pubkey,
    pub owner_program: &'a Pubkey,
    pub signer_is_program: CreatePdaMode,
}

pub fn create_pda_instruction(input_params: CreateCompressedPdaInstructionInputs) -> Instruction {
    let (cpi_signer, bump) =
        Pubkey::find_program_address(&[b"cpi_signer".as_slice()], &crate::id());
    let mut remaining_accounts = HashMap::new();
    remaining_accounts.insert(
        *input_params.output_compressed_account_merkle_tree_pubkey,
        0,
    );
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
    let instruction_data = crate::instruction::CreateCompressedPda {
        data: input_params.data,
        proof: Some(input_params.proof.clone()),
        new_address_parameters: new_address_params[0].clone(),
        owner_program: *input_params.owner_program,
        cpi_context,
        bump,
        signer_is_program: input_params.signer_is_program,
    };

    let registered_program_pda = Pubkey::find_program_address(
        &[psp_compressed_pda::ID.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0;
    let compressed_token_cpi_authority_pda = psp_compressed_token::get_cpi_authority_pda().0;
    let account_compression_authority =
        psp_compressed_pda::utils::get_cpi_authority_pda(&psp_compressed_pda::ID);

    let accounts = crate::accounts::CreateCompressedPda {
        signer: *input_params.signer,
        noop_program: NOOP_PROGRAM_ID,
        compressed_pda_program: psp_compressed_pda::ID,
        account_compression_program: account_compression::ID,
        registered_program_pda,
        compressed_token_cpi_authority_pda,
        account_compression_authority,
        self_program: crate::ID,
        cpi_signer,
    };
    let remaining_accounts = to_account_metas(remaining_accounts);

    Instruction {
        program_id: crate::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),

        data: instruction_data.data(),
    }
}

#[derive(Debug, Clone)]
pub struct InvalidateNotOwnedCompressedAccountInstructionInputs<'a> {
    pub signer: &'a Pubkey,
    pub root_indices: &'a [u16],
    pub proof: &'a CompressedProof,
    pub input_merkle_tree_pubkey: &'a Pubkey,
    pub compressed_account: &'a CompressedAccountWithMerkleContext,
}
pub fn create_invalidate_not_owned_account_instruction(
    input_params: InvalidateNotOwnedCompressedAccountInstructionInputs,
) -> Instruction {
    let (cpi_signer, bump) =
        Pubkey::find_program_address(&[b"cpi_signer".as_slice()], &crate::id());
    let mut remaining_accounts = HashMap::new();
    remaining_accounts.insert(*input_params.input_merkle_tree_pubkey, 0);

    let instruction_data = crate::instruction::InvalidateNotOwnedAccount {
        proof: Some(input_params.proof.clone()),
        compressed_account: input_params.compressed_account.clone(),
        bump,
        root_indices: input_params.root_indices.to_vec(),
    };

    let registered_program_pda = Pubkey::find_program_address(
        &[psp_compressed_pda::ID.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0;
    let compressed_token_cpi_authority_pda = psp_compressed_token::get_cpi_authority_pda().0;
    let account_compression_authority =
        psp_compressed_pda::utils::get_cpi_authority_pda(&psp_compressed_pda::ID);

    let accounts = crate::accounts::InvalidateNotOwnedCompressedAccount {
        signer: *input_params.signer,
        noop_program: NOOP_PROGRAM_ID,
        compressed_pda_program: psp_compressed_pda::ID,
        account_compression_program: account_compression::ID,
        registered_program_pda,
        compressed_token_cpi_authority_pda,
        account_compression_authority,
        self_program: crate::ID,
        cpi_signer,
    };
    let remaining_accounts = to_account_metas(remaining_accounts);

    Instruction {
        program_id: crate::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),

        data: instruction_data.data(),
    }
}
