pub mod instructions_transform_g2_affine_to_g2_prepared;
pub mod ml_254_instructions;
pub mod ml_254_instructions_transform;
pub mod ml_254_parsers;
pub mod ml_254_pre_processor;
pub mod ml_254_processor;
pub mod ml_254_ranges;
pub mod ml_254_state;
pub mod pi_254_state_COPY;
pub mod state_prep_inputs;

use crate::state_prep_inputs::*;

pub mod hard_coded_verifying_key_pvk_new_ciruit;
pub mod instructions;
pub mod inverse;
pub mod mul_assign;
pub mod parsers;
pub mod parsers_part_2;
pub mod parsers_prepare_inputs;
pub mod pre_processor_miller_loop;
pub mod processor;
pub mod processor_part_2;
pub mod proof;
pub mod ranges;
pub mod ranges_part_2;
pub mod state_check_nullifier;
pub mod state_final_exp;
pub mod utils;
pub mod verifyingkey;

pub mod instructions_poseidon;
pub mod poseidon_params;
pub mod poseidon_round_constants_split;

pub mod state_miller_loop_transfer;

pub mod hard_coded_verifying_key_pvk_254;

pub mod init_bytes11;
pub mod instructions_merkle_tree;
pub mod parsers_merkle_tree;
pub mod processor_merkle_tree;
pub mod state_merkle_tree;

// use crate::pre_processor_miller_loop::_pre_process_instruction_miller_loop;
use crate::processor_merkle_tree::_pre_process_instruction_merkle_tree;
use crate::processor_part_2::_pre_process_instruction_final_exp;
use crate::state_merkle_tree::{HashBytes, MerkleTree};

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

// verify 1

// use crate::parsers::*;

// verify 2
use crate::parsers_merkle_tree::*;
use crate::poseidon_params::get_params;
use crate::state_final_exp::FinalExpBytes;
use ark_ec;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use byteorder::ByteOrder;
use byteorder::LittleEndian;
use std::convert::TryInto;

// Account struct for verify Part 1:

// use crate::ml_254_parsers::*;
use crate::ml_254_pre_processor::*;

pub mod state_miller_loop;

entrypoint!(process_instruction);

use crate::instructions_transform_g2_affine_to_g2_prepared::*;
use ark_ec::{AffineCurve, PairingEngine, ProjectiveCurve};
use ark_ff::biginteger::{BigInteger256, BigInteger384};
use ark_ff::bytes::{FromBytes, ToBytes};
use ark_ff::{Fp256, Fp384};

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    // MerkleTree:
    println!("ix_data_len(): {:?}", _instruction_data.len());
    println!("ix_data: {:?}", _instruction_data);
    println!("ix_data[9] shd be 1: {:?}", _instruction_data[9]);

    // for test stuff:
    // let mut _instruction_data = vec![0;130];
    // if
    // _instruction_data[8..].;

    // for testing => 1+8! => 9 instead of 1
    if _instruction_data[9] == 0 {
        _pre_process_instruction_merkle_tree(&_instruction_data, accounts);
    }
    // verify part 1:
    else if _instruction_data[9] == 1 {
        _pre_process_instruction_miller_loop(&_instruction_data, accounts);

        // testing:
        // testing all parsers
        // log_parser_compute_costs(&_instruction_data, accounts);
    }
    // verify part 2:
    else if _instruction_data[9] == 2 {
        _pre_process_instruction_final_exp(&_instruction_data, accounts);
    }
    // prepare inputs moved to separate program for size
    else if _instruction_data[9] == 3 {
        //_pre_process_instruction_prep_inputs(_instruction_data, accounts);
    }

    Ok(())
}
use crate::ml_254_parsers::*;
use crate::ml_254_state::*;
use ark_ff::fields::Fp2;
use num_traits::One;

fn log_parser_compute_costs(
    _instruction_data: &[u8],
    accounts: &[AccountInfo],
) -> Result<(), ProgramError> {
    let account = &mut accounts.iter();
    let signing_account = next_account_info(account)?;
    let account_main = next_account_info(account)?;
    msg!(
        "new ix -- IX_DATA ARRIVED: {:?}",
        _instruction_data[..].to_vec()
    );
    let mut account_main_data = ML254Bytes::unpack(&account_main.data.borrow())?;
    msg!("start count:");
    sol_log_compute_units();
    let proof_b_bytes = [
        32, 255, 161, 204, 195, 74, 249, 196, 139, 193, 49, 109, 241, 230, 145, 100, 91, 134, 188,
        102, 83, 190, 140, 12, 84, 21, 107, 182, 225, 139, 23, 16, 64, 152, 20, 230, 245, 127, 35,
        113, 194, 4, 161, 242, 179, 131, 135, 66, 70, 179, 115, 118, 237, 158, 246, 97, 35, 85, 25,
        13, 30, 21, 183, 18, 254, 194, 12, 96, 211, 37, 160, 170, 7, 173, 208, 52, 22, 169, 113,
        149, 235, 85, 90, 20, 14, 171, 22, 22, 247, 254, 71, 236, 207, 18, 90, 29, 236, 211, 193,
        206, 15, 107, 89, 218, 207, 62, 76, 75, 88, 71, 9, 45, 114, 212, 43, 127, 163, 183, 245,
        213, 117, 216, 64, 56, 26, 102, 15,
        37,
        //1, 0, 0, 0, 0, 0, 0, 0,
        // 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    let proof_b = parse_proof_b_from_bytes(&proof_b_bytes.to_vec());
    msg!("cost -- parse_proof_b_from_bytes (affine):");
    sol_log_compute_units();

    let mut r: ark_ec::models::bn::g2::G2HomProjective<ark_bn254::Parameters> =
        ark_ec::models::bn::g2::G2HomProjective {
            x: proof_b.x,
            y: proof_b.y,
            z: Fp2::one(),
        };
    msg!("cost --compute r:");
    sol_log_compute_units();
    parse_proof_b_to_bytes(proof_b, &mut account_main_data.proof_b);
    msg!("cost --parse_proof_b_to_bytes:");
    sol_log_compute_units();

    parse_r_to_bytes(r, &mut account_main_data.r);
    msg!("cost --parse_r_to_bytes: (3 quads)");
    sol_log_compute_units();

    // let fp256 = parse_fp256_from_bytes();
    // msg!("cost --parse_r_to_bytes: (3 quads)");
    // sol_log_compute_units();

    // parse_fp256_to_bytes();
    // msg!("cost --parse_r_to_bytes: (3 quads)");
    // sol_log_compute_units();

    let reference_f = [
        41, 164, 125, 219, 237, 181, 202, 195, 98, 55, 97, 232, 35, 147, 153, 23, 164, 70, 211,
        144, 151, 9, 219, 197, 234, 13, 164, 242, 67, 59, 148, 5, 132, 108, 82, 161, 228, 167, 20,
        24, 207, 201, 203, 25, 249, 125, 54, 96, 182, 231, 150, 215, 149, 43, 216, 0, 36, 166, 232,
        13, 126, 3, 53, 0, 174, 209, 16, 242, 177, 143, 60, 247, 181, 65, 132, 142, 14, 231, 170,
        52, 3, 34, 70, 49, 210, 158, 211, 173, 165, 155, 219, 80, 225, 32, 64, 8, 65, 139, 16, 138,
        240, 218, 36, 220, 8, 100, 236, 141, 1, 223, 60, 59, 24, 38, 90, 254, 47, 91, 205, 228,
        169, 103, 178, 30, 124, 141, 43, 9, 83, 155, 75, 140, 209, 26, 2, 250, 250, 20, 185, 78,
        53, 54, 68, 178, 88, 78, 246, 132, 97, 167, 124, 253, 96, 26, 213, 99, 157, 155, 40, 9, 60,
        139, 112, 126, 230, 195, 217, 125, 68, 169, 208, 149, 175, 33, 226, 17, 47, 132, 8, 154,
        237, 156, 34, 97, 55, 129, 155, 64, 202, 54, 161, 19, 24, 1, 208, 104, 140, 149, 25, 229,
        96, 239, 202, 24, 235, 221, 133, 137, 30, 226, 62, 112, 26, 58, 1, 85, 207, 182, 41, 213,
        42, 72, 139, 41, 108, 152, 252, 164, 121, 76, 17, 62, 147, 226, 220, 79, 236, 132, 109,
        130, 163, 209, 203, 14, 144, 180, 25, 216, 234, 198, 199, 74, 48, 62, 57, 0, 206, 138, 12,
        130, 25, 12, 187, 216, 86, 208, 84, 198, 58, 204, 6, 161, 93, 63, 68, 121, 173, 129, 255,
        249, 47, 42, 218, 214, 129, 29, 136, 7, 213, 160, 139, 148, 58, 6, 191, 11, 161, 114, 56,
        174, 224, 86, 243, 103, 166, 151, 107, 36, 205, 170, 206, 196, 248, 251, 147, 91, 3, 136,
        208, 36, 3, 51, 84, 102, 139, 252, 193, 9, 172, 113, 116, 50, 242, 70, 26, 115, 166, 252,
        204, 163, 149, 78, 13, 255, 235, 222, 174, 120, 182, 178, 186, 22, 169, 153, 73, 48, 242,
        139, 120, 98, 33, 101, 204, 204, 169, 57, 249, 168, 45, 197, 126, 105, 54, 187, 35, 241,
        253, 4, 33, 70, 246, 206, 32, 17,
    ];
    let f = parse_f_from_bytes(&reference_f.to_vec());
    msg!("cost --parse_f_from_bytes: (3 quads)");
    sol_log_compute_units();

    parse_f_to_bytes(f, &mut account_main_data.f_range);
    msg!("cost --parse_f_to_bytes: (3 quads)");
    sol_log_compute_units();

    parse_cubic_to_bytes(f.c1, &mut account_main_data.cubic_v0_range);
    msg!("cost --parse_cubic_to_bytes: (3 quads)");
    sol_log_compute_units();
    let cubic = parse_cubic_from_bytes(&account_main_data.cubic_v0_range);
    msg!("cost --parse_cubic_from_bytes: (3 quads)");
    sol_log_compute_units();
    Ok(())
}
