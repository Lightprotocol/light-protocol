use ark_crypto_primitives::crh::TwoToOneCRH;
use ark_crypto_primitives::merkle_tree::{Config, MerkleTree, Path};

pub mod hard_coded_verifying_key_pvk_new_ciruit;
pub mod instructions_prepare_inputs;
pub mod parse_verifyingkey_254;
pub mod parsers_prepare_inputs;
pub mod pi_254_implementation;
pub mod pi_254_instructions;
pub mod pi_254_test;
// pub mod pi_381_test;
pub mod pi_254_parsers;
pub mod poseidon_instructions;
pub mod poseidon_parsers;
pub mod poseidon_processor;
pub mod pre_processor_prep_inputs;
pub mod prepare_inputs;
pub mod processor_prepare_inputs;
pub mod ranges_prepare_inputs;
pub mod state_merkle_tree_roots;
pub mod state_prep_inputs;
// pub mod verifyingkey_254_bytes;
pub mod pi_254_ranges;
pub mod verifyingkey_254_hc;

use crate::pi_254_implementation::*;
use crate::pi_254_instructions::*;

use crate::pre_processor_prep_inputs::_pre_process_instruction_prep_inputs;
use crate::state_prep_inputs::PrepareInputsBytes;

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    log::sol_log_compute_units,
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

use ark_ec;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use std::convert::TryInto;

use byteorder::ByteOrder;
use byteorder::LittleEndian;

// prepare inputs/p
use crate::parsers_prepare_inputs::*;
use crate::prepare_inputs::*;
use crate::processor_prepare_inputs::*;

entrypoint!(process_instruction);

use crate::ranges_prepare_inputs::*;
use ark_ec::{AffineCurve, PairingEngine, ProjectiveCurve};
use ark_ff::biginteger::{BigInteger256, BigInteger384};
use ark_ff::bytes::{FromBytes, ToBytes};
use ark_ff::{Fp256, Fp384};

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    let account = &mut accounts.iter();
    let account1 = next_account_info(account)?; // this one isn't needed
                                                // the remaining storage accounts are being pulled inside each loop
    if _instruction_data[1] == 3 {
        _pre_process_instruction_prep_inputs(_instruction_data, accounts);
    }

    Ok(())
}
