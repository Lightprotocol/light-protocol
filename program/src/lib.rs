pub mod hard_coded_verifying_key_pvk_new_ciruit;
pub mod instructions;
pub mod instructions_transform_g2_affine_to_g2_prepared;
pub mod ml_254_instructions;
pub mod ml_254_instructions_transform;
pub mod ml_254_parsers;
pub mod ml_254_pre_processor;
pub mod ml_254_processor;
pub mod ml_254_ranges;
pub mod ml_254_state;
pub mod parsers;
pub mod parsers_prepare_inputs;
pub mod pi_254_state_COPY;
pub mod pre_processor_final_exp;
pub mod ranges_part_2;
pub mod state_check_nullifier;
pub mod state_final_exp;

pub mod instructions_poseidon;
pub mod poseidon_round_constants_split;

pub mod state_miller_loop_transfer;

pub mod hard_coded_verifying_key_pvk_254;

pub mod init_bytes11;
pub mod init_bytes18;
pub mod instructions_merkle_tree;
pub mod processor_merkle_tree;
pub mod state_merkle_tree;

pub mod instructions_final_exponentiation;
pub mod parsers_part_2_254;
pub mod processor_final_exp;
use crate::pre_processor_final_exp::_pre_process_instruction_final_exp;
use crate::processor_merkle_tree::_pre_process_instruction_merkle_tree;

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    log::sol_log_compute_units,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

pub mod state_miller_loop;
use crate::ml_254_pre_processor::*;

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    //msg!("instruction data {:?}", _instruction_data);
    // MerkleTree:
    if _instruction_data[9] == 0 {
        _pre_process_instruction_merkle_tree(&_instruction_data, accounts);
    }
    // //  verify part 1:
    else if _instruction_data[9] == 1 {
        _pre_process_instruction_miller_loop(&_instruction_data, accounts);

        // testing:
        // testing all parsers
        // log_parser_compute_costs(&_instruction_data, accounts);
    }
    // verify part 2:
    else if _instruction_data[9] == 2 {
        _pre_process_instruction_final_exp(program_id, accounts, &_instruction_data);
    }
    // prepare inputs moved to separate program for size
    else if _instruction_data[9] == 3 {
        //_pre_process_instruction_prep_inputs(_instruction_data, accounts);
        sol_log_compute_units();

        msg!("state_check_nullifier");
        sol_log_compute_units();
    }
    Ok(())
}
