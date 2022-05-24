#![allow(clippy::type_complexity, clippy::ptr_arg, clippy::too_many_arguments)]

pub mod instructions;
pub mod poseidon_merkle_tree;
pub mod processor;
pub mod state;
pub mod utils;

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::rent::Rent,
    sysvar::Sysvar,
};
use solana_security_txt::security_txt;

security_txt! {
    name: "light_protocol_merkle_tree",
    project_url: "lightprotocol.com",
    contacts: "email:security@lightprotocol.com",
    policy: "https://github.com/Lightprotocol/light-protocol-program/blob/main/SECURITY.md",
    source_code: "https://github.com/Lightprotocol/light-protocol-program/program_merkle_tree"
}

use crate::config::{ENCRYPTED_UTXOS_LENGTH, MERKLE_TREE_INIT_AUTHORITY};
use crate::poseidon_merkle_tree::processor::MerkleTreeProcessor;
use crate::state::InstructionIndex;
use crate::utils::config;
use crate::instructions::
{
    create_and_try_initialize_tmp_storage_pda,
    create_authority_config_pda,
    update_authority_config_pda
};
use crate::state::MerkleTreeTmpPda;

#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

#[allow(clippy::clone_double_ref)]
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {

    msg!("entered");
    let accounts_mut = accounts.clone();
    let account = &mut accounts_mut.iter();
    // 0. `[]` signer
    let signer_account = next_account_info(account)?;
    if !signer_account.is_signer {
        msg!("signer account needs to be passed in first place");
        return Err(ProgramError::IllegalOwner);
    }
    // Initialize new merkle tree account.
    msg!("Instruction data {:?}", instruction_data);
    if instruction_data.len() >= 9 && instruction_data[8] == 240 {
        let merkle_tree_storage_acc = next_account_info(account)?;
        // Check whether signer is merkle_tree_init_authority.
        if *signer_account.key != Pubkey::new(&MERKLE_TREE_INIT_AUTHORITY) {
            msg!("Signer is not merkle tree init authority.");
            return Err(ProgramError::IllegalOwner);
        }
        let rent_sysvar_info = next_account_info(account)?;
        let rent = &Rent::from_account_info(rent_sysvar_info)?;
        if !rent.is_exempt(
            **merkle_tree_storage_acc.lamports.borrow(),
            merkle_tree_storage_acc.data.borrow().len(),
        ) {
            msg!("Account is not rent exempt.");
            return Err(ProgramError::AccountNotRentExempt);
        }
        let mut merkle_tree_processor =
            MerkleTreeProcessor::new(None, Some(merkle_tree_storage_acc), *program_id)?;
        merkle_tree_processor
            .initialize_new_merkle_tree_from_bytes(&config::INIT_BYTES_MERKLE_TREE_18[..])
    }
    // Create merkle tree tmp account.
    else if instruction_data.len() >= 9 && instruction_data[8] == 1 {
        // let rent_sysvar_info = next_account_info(account)?;
        // let rent = &Rent::from_account_info(rent_sysvar_info)?;
        msg!("\nprior create_and_try_initialize_tmp_storage_pda\n");

        create_and_try_initialize_tmp_storage_pda(
            &program_id,
            accounts,
            &instruction_data[9..]
        )
    }
    // Update merkle tree.
    else if instruction_data.len() >= 9 && instruction_data[8] == 2 {
        let tmp_storage_pda = next_account_info(account)?;

        let mut tmp_storage_pda_data =
            MerkleTreeTmpPda::unpack(&tmp_storage_pda.data.borrow())?;

        processor::process_instruction(&program_id, &accounts, &mut tmp_storage_pda_data, &instruction_data[9..])

    }
    // transfer sol
    else if instruction_data.len() >= 9 && instruction_data[8] == 3 {
        let tmp_storage_pda = next_account_info(account)?;
        // let mut tmp_storage_pda_data =
        //     MerkleTreeTmpPda::unpack(&tmp_storage_pda.data.borrow())?;
        //TODO: Security checks

        processor::process_sol_transfer(
            &program_id,
            &accounts,
            &instruction_data[9..]
        )

    }
    // transfer tokens
    // else if instruction_data.len() >= 9 && instruction_data[8] == 4 {
    //         let tmp_storage_pda = next_account_info(account)?;
    //
    //         let mut tmp_storage_pda_data =
    //             MerkleTreeTmpPda::unpack(&tmp_storage_pda.data.borrow())?;
    //         //TODO: add security check
    //         processor::transfer_tokens(&program_id, &accounts, &mut tmp_storage_pda_data)
    //
    // }
    // create & update authority config account
    else if instruction_data.len() >= 9 && instruction_data[8] == 5 {
        if instruction_data.len() > 10 && instruction_data[9] == 0 {
            msg!("\n create_authority_config_pda\n");
            create_authority_config_pda(
                &program_id,
                accounts,
                &instruction_data[10..]
            )
        } else if instruction_data.len() > 10 && instruction_data[9] == 1 {
            msg!("\n update_authority_config_pda\n");
            update_authority_config_pda(
                &program_id,
                accounts,
                &instruction_data[10..]
            )
        } else {
            return Err(ProgramError::InvalidInstructionData);
        }
    }
    else if instruction_data.len() >= 9 && instruction_data[8] == 6 {
        msg!("\n create_verifier_config_pda\n");
        return Err(ProgramError::InvalidInstructionData);
    }
    else {
        panic!("");
        Ok(())
    }
}

const ROOT_CHECK: u8 = 15;
const INSERT_LEAVES_NULLIFIER_AND_TRANSFER: usize = 1501;
const VERIFICATION_END_INDEX: usize = 1266;
pub const NULLIFIER_0_START: usize = 320;
pub const NULLIFIER_0_END: usize = 352;
pub const NULLIFIER_1_START: usize = 352;
pub const NULLIFIER_1_END: usize = 384;
pub const TWO_LEAVES_PDA_SIZE: u64 = 106 + ENCRYPTED_UTXOS_LENGTH as u64;
//instruction order
pub const IX_ORDER: [u8; 76] = [
    ROOT_CHECK, 34, 14, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2, 25, 0, 1, 2,
    16,
    //perform last checks and transfer requested amount
    241,
];
