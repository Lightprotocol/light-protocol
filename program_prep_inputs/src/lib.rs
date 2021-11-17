use ark_crypto_primitives::crh::TwoToOneCRH;
use ark_crypto_primitives::merkle_tree::{Config, MerkleTree, Path};

pub mod ranges_prepare_inputs;
pub mod  hard_coded_verifying_key_pvk_new_ciruit;
pub mod processor_prepare_inputs;
pub mod parsers_prepare_inputs;
pub mod instructions_prepare_inputs;
pub mod prepare_inputs;
pub mod state_prep_inputs;
pub mod pre_processor_prep_inputs;
pub mod state_merkle_tree_roots;
pub mod poseidon_processor;
pub mod poseidon_instructions;
pub mod poseidon_parsers;

use crate::pre_processor_prep_inputs::_pre_process_instruction_prep_inputs;
use crate::state_prep_inputs::PrepareInputsBytes;
// pub mod constraints;
// mod constraints_test;


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




use std::convert::TryInto;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use ark_ec;

use byteorder::LittleEndian;
use byteorder::ByteOrder;


// prepare inputs/p
use crate::prepare_inputs::*;
use crate::parsers_prepare_inputs::*;
use crate::processor_prepare_inputs::*;



entrypoint!(process_instruction);

use ark_ff::biginteger::{BigInteger256,BigInteger384};
use ark_ff::{Fp256, Fp384};
use ark_ec::{AffineCurve, PairingEngine, ProjectiveCurve};
use ark_ff::bytes::{ToBytes, FromBytes};
use crate::ranges_prepare_inputs::*;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {

    let account = &mut accounts.iter();
    let account1 = next_account_info(account)?;
    // the remaining storage accounts are being pulled inside each loop


    // prepare inputs and store p2 in acc

    if _instruction_data[1] == 3 {
        _pre_process_instruction_prep_inputs(_instruction_data, accounts);

    }


    Ok(())
}
