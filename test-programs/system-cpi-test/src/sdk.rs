#![cfg(not(target_os = "solana"))]

use std::collections::HashMap;

use anchor_lang::{InstructionData, ToAccountMetas};
use light_compressed_token::transfer_sdk::to_account_metas;
use light_system_program::{
    invoke::processor::CompressedProof, sdk::address::pack_new_address_params,
    sdk::compressed_account::PackedCompressedAccountWithMerkleContext, NewAddressParams,
};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

use crate::CreatePdaMode;

#[derive(Debug, Clone)]
pub struct CreateCompressedPdaInstructionInputs<'a> {
    pub data: [u8; 31],
    pub signer: &'a Pubkey,
    pub output_compressed_account_merkle_tree_pubkey: &'a Pubkey,
    pub proof: &'a CompressedProof,
    pub new_address_params: NewAddressParams,
    pub cpi_context_account: &'a Pubkey,
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

    let instruction_data = crate::instruction::CreateCompressedPda {
        data: input_params.data,
        proof: Some(input_params.proof.clone()),
        new_address_parameters: new_address_params[0],
        owner_program: *input_params.owner_program,
        bump,
        signer_is_program: input_params.signer_is_program,
        cpi_context: None,
    };

    let registered_program_pda = Pubkey::find_program_address(
        &[light_system_program::ID.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0;
    let compressed_token_cpi_authority_pda = light_compressed_token::get_cpi_authority_pda().0;
    let account_compression_authority =
        light_system_program::utils::get_cpi_authority_pda(&light_system_program::ID);

    let accounts = crate::accounts::CreateCompressedPda {
        signer: *input_params.signer,
        noop_program: Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
        light_system_program: light_system_program::ID,
        account_compression_program: account_compression::ID,
        registered_program_pda,
        compressed_token_cpi_authority_pda,
        account_compression_authority,
        self_program: crate::ID,
        cpi_signer,
        system_program: solana_sdk::system_program::id(),
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
    pub proof: &'a CompressedProof,
    pub input_merkle_tree_pubkey: &'a Pubkey,
    pub input_nullifier_pubkey: &'a Pubkey,
    pub cpi_context_account: &'a Pubkey,
    pub compressed_account: &'a PackedCompressedAccountWithMerkleContext,
    pub token_transfer_data: Option<crate::TokenTransferData>,
    pub cpi_context: Option<crate::CompressedCpiContext>,
}
pub fn create_invalidate_not_owned_account_instruction(
    input_params: InvalidateNotOwnedCompressedAccountInstructionInputs,
    mode: crate::WithInputAccountsMode,
) -> Instruction {
    let (cpi_signer, bump) =
        Pubkey::find_program_address(&[b"cpi_signer".as_slice()], &crate::id());
    let cpi_context = input_params.cpi_context;

    let mut remaining_accounts = HashMap::new();
    remaining_accounts.insert(*input_params.input_merkle_tree_pubkey, 0);
    remaining_accounts.insert(*input_params.input_nullifier_pubkey, 1);
    remaining_accounts.insert(*input_params.cpi_context_account, 2);

    let instruction_data = crate::instruction::WithInputAccounts {
        proof: Some(input_params.proof.clone()),
        compressed_account: input_params.compressed_account.clone(),
        bump,
        mode,
        cpi_context,
        token_transfer_data: input_params.token_transfer_data.clone(),
    };

    let registered_program_pda = Pubkey::find_program_address(
        &[light_system_program::ID.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0;
    let compressed_token_cpi_authority_pda = light_compressed_token::get_cpi_authority_pda().0;
    let account_compression_authority =
        light_system_program::utils::get_cpi_authority_pda(&light_system_program::ID);

    let accounts = crate::accounts::InvalidateNotOwnedCompressedAccount {
        signer: *input_params.signer,
        noop_program: Pubkey::new_from_array(account_compression::utils::constants::NOOP_PUBKEY),
        light_system_program: light_system_program::ID,
        account_compression_program: account_compression::ID,
        registered_program_pda,
        compressed_token_cpi_authority_pda,
        account_compression_authority,
        self_program: crate::ID,
        cpi_signer,
        system_program: solana_sdk::system_program::id(),
        compressed_token_program: light_compressed_token::ID,
    };
    let remaining_accounts = to_account_metas(remaining_accounts);

    Instruction {
        program_id: crate::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    }
}
