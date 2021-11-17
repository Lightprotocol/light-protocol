pub mod instructions_transform_g2_affine_to_g2_prepared;
pub mod state_prep_inputs;
use crate::state_prep_inputs::*;
pub mod  hard_coded_verifying_key_pvk_new_ciruit;
pub mod inverse;
pub mod parsers_part_2;
pub mod processor_part_2;
pub mod utils;
pub mod mul_assign;
pub mod ranges_part_2;
pub mod processor;
pub mod ranges;
pub mod proof;
pub mod instructions;
pub mod parsers;
pub mod verifyingkey;
pub mod pre_processor_miller_loop;
pub mod parsers_prepare_inputs;
pub mod state_final_exp;
pub mod state_check_nullifier;


pub mod poseidon_params;
pub mod state_miller_loop_transfer;

pub mod instructions_merkle_tree;
pub mod processor_merkle_tree;
pub mod parsers_merkle_tree;
pub mod state_merkle_tree;
use crate::processor_merkle_tree::{
    _pre_process_instruction_merkle_tree,
};
use crate::pre_processor_miller_loop::_pre_process_instruction_miller_loop;
use crate::state_merkle_tree::{MerkleTree, HashBytes};
use crate::processor_part_2::_pre_process_instruction_final_exp;

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    log::sol_log_compute_units,
    program_pack::{IsInitialized, Pack, Sealed},
};

// verify 1
use crate::processor::{
    _process_instruction,
};

use crate::parsers::*;
use crate::ranges::*;


// verify 2
use crate::processor_part_2::{
    _process_instruction_part_2,
};
use crate::parsers_part_2::*;
use crate::ranges_part_2::*;

use crate::parsers_merkle_tree::*;
use crate::poseidon_params::get_params;
use std::convert::TryInto;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use byteorder::LittleEndian;
use byteorder::ByteOrder;
use ark_ec;
use crate::state_final_exp::FinalExpBytes;

// Account struct for verify Part 1:

pub mod state_miller_loop;
use crate::state_miller_loop::*;


entrypoint!(process_instruction);

use ark_ff::biginteger::{BigInteger256,BigInteger384};
use ark_ff::{Fp256, Fp384};
use ark_ec::{AffineCurve, PairingEngine, ProjectiveCurve};
use ark_ff::bytes::{ToBytes, FromBytes};
use crate::instructions_transform_g2_affine_to_g2_prepared::*;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {

    // MerkleTree:
    if _instruction_data[1] == 0 {
        _pre_process_instruction_merkle_tree(&_instruction_data, accounts);
    }

    // //  verify part 1:
    else if _instruction_data[1] == 1 {
        _pre_process_instruction_miller_loop(&_instruction_data, accounts);
    }

    // verify part 2:
    else if _instruction_data[1] == 2 {
         _pre_process_instruction_final_exp(&_instruction_data, accounts);
    }
    
    // prepare inputs moved to separate program for size
    else if _instruction_data[1] == 3 {
        //_pre_process_instruction_prep_inputs(_instruction_data, accounts);
    }

    Ok(())
}
