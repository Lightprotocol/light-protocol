use ark_ec;
use ark_ff::Field;

use crate::groth16_verifier::miller_loop::instructions::*;
use crate::groth16_verifier::VerifierState;
use crate::groth16_verifier::miller_loop::MillerLoopStateCompute;
use anchor_lang::prelude::*;
use ark_ec::bn::BnParameters;
use std::cell::RefMut;
use crate::{parse_f_from_bytes, parse_f_to_bytes};

pub fn miller_loop_process_instruction(verifier_state_data: &mut RefMut<'_, VerifierState>) {
    msg!("initing MillerLoopStateCompute");
    let mut miller_loop_compute = MillerLoopStateCompute::new(&verifier_state_data);

    // needs 46 calls with 1_250_000 compute units per instruction
    miller_loop_onchain(verifier_state_data, &mut miller_loop_compute);

    miller_loop_compute.pack(verifier_state_data);

    verifier_state_data.current_instruction_index += 1;
    msg!(
        "verifier_state_data.current_instruction_index: {}",
        verifier_state_data.current_instruction_index
    );
}

pub fn miller_loop_onchain(
    verifier_state_data: &mut RefMut<'_, VerifierState>,
    miller_loop_compute: &mut MillerLoopStateCompute,
) -> u64 {
    let mut total_compute: u64 = 0;

    for i in (1..ark_bn254::Parameters::ATE_LOOP_COUNT.len()
        - (verifier_state_data.outer_first_loop as usize))
        .rev()
    {
        if i != ark_bn254::Parameters::ATE_LOOP_COUNT.len() - 1
            && verifier_state_data.square_in_place_executed == 0
        {
            miller_loop_compute.f.square_in_place();

            total_compute += 80_000;
            verifier_state_data.square_in_place_executed = 1;
            if total_compute >= verifier_state_data.ml_max_compute {
                return total_compute;
            }
        }

        // first_inner_loop_index
        for pair_index in verifier_state_data.first_inner_loop_index..3 {
            let current_pair = miller_loop_compute.pairs_0[pair_index as usize];

            let current_coeff = match miller_loop_compute.current_coeff {
                Some(coeff) => Some(coeff),
                None => get_coeff(
                    pair_index,
                    verifier_state_data,
                    &mut total_compute,
                    miller_loop_compute,
                ),
            };

            if current_coeff.is_none() {
                return total_compute;
            }
            total_compute += 120_000;
            if total_compute >= verifier_state_data.ml_max_compute {
                miller_loop_compute.current_coeff = current_coeff;
                return total_compute;
            }
            miller_loop_compute.current_coeff = None;
            ark_ec::models::bn::Bn::<ark_bn254::Parameters>::ell(
                &mut miller_loop_compute.f,
                &current_coeff.unwrap(),
                &current_pair,
            );
            verifier_state_data.first_inner_loop_index += 1;
        }

        let bit = ark_bn254::Parameters::ATE_LOOP_COUNT[i as usize - 1];

        match bit {
            1 => {
                for pair_index in verifier_state_data.second_inner_loop_index..3 {
                    let current_pair = miller_loop_compute.pairs_0[pair_index as usize];
                    let current_coeff = match miller_loop_compute.current_coeff {
                        Some(coeff) => Some(coeff),
                        None => get_coeff(
                            pair_index,
                            verifier_state_data,
                            &mut total_compute,
                            miller_loop_compute,
                        ),
                    };
                    if current_coeff.is_none() {
                        return total_compute;
                    }
                    total_compute += 120_000;
                    if total_compute >= verifier_state_data.ml_max_compute {
                        miller_loop_compute.current_coeff = current_coeff;
                        return total_compute;
                    }
                    miller_loop_compute.current_coeff = None;
                    ark_ec::models::bn::Bn::<ark_bn254::Parameters>::ell(
                        &mut miller_loop_compute.f,
                        &current_coeff.unwrap(),
                        &current_pair,
                    );
                    verifier_state_data.second_inner_loop_index += 1;
                }
                verifier_state_data.first_inner_loop_index = 0;
                verifier_state_data.second_inner_loop_index = 0;
                verifier_state_data.square_in_place_executed = 0;
                verifier_state_data.outer_first_loop += 1;
            }
            -1 => {
                for pair_index in verifier_state_data.second_inner_loop_index..3 {
                    let current_pair = miller_loop_compute.pairs_0[pair_index as usize];
                    let current_coeff = match miller_loop_compute.current_coeff {
                        Some(coeff) => Some(coeff),
                        None => get_coeff(
                            pair_index,
                            verifier_state_data,
                            &mut total_compute,
                            miller_loop_compute,
                        ),
                    };

                    if current_coeff.is_none() {
                        return total_compute;
                    }
                    total_compute += 120_000;
                    if total_compute >= verifier_state_data.ml_max_compute {
                        miller_loop_compute.current_coeff = current_coeff;
                        return total_compute;
                    }
                    miller_loop_compute.current_coeff = None;
                    ark_ec::models::bn::Bn::<ark_bn254::Parameters>::ell(
                        &mut miller_loop_compute.f,
                        &current_coeff.unwrap(),
                        &current_pair,
                    );
                    verifier_state_data.second_inner_loop_index += 1;
                }

                verifier_state_data.first_inner_loop_index = 0;
                verifier_state_data.second_inner_loop_index = 0;
                verifier_state_data.square_in_place_executed = 0;
                verifier_state_data.outer_first_loop += 1;
            }
            _ => {
                verifier_state_data.first_inner_loop_index = 0;
                verifier_state_data.second_inner_loop_index = 0;
                verifier_state_data.square_in_place_executed = 0;
                verifier_state_data.outer_first_loop += 1;
                continue;
            }
        }
    }

    if ark_bn254::Parameters::X_IS_NEGATIVE {
        miller_loop_compute.f.conjugate();
    }

    for pair_index in verifier_state_data.outer_second_loop..3 {
        let current_pair = miller_loop_compute.pairs_0[pair_index as usize];

        let current_coeff = match miller_loop_compute.current_coeff {
            Some(coeff) => Some(coeff),
            None => get_coeff(
                pair_index,
                verifier_state_data,
                &mut total_compute,
                miller_loop_compute,
            ),
        };

        if current_coeff.is_none() {
            return total_compute;
        }
        total_compute += 120_000;
        if total_compute >= verifier_state_data.ml_max_compute {
            miller_loop_compute.current_coeff = current_coeff;
            return total_compute;
        }
        miller_loop_compute.current_coeff = None;
        ark_ec::models::bn::Bn::<ark_bn254::Parameters>::ell(
            &mut miller_loop_compute.f,
            &current_coeff.unwrap(),
            &current_pair,
        );

        verifier_state_data.outer_second_loop += 1;
    }

    for pair_index in verifier_state_data.outer_third_loop..3 {
        let current_pair = miller_loop_compute.pairs_0[pair_index as usize];
        let current_coeff = match miller_loop_compute.current_coeff {
            Some(coeff) => Some(coeff),
            None => get_coeff(
                pair_index,
                verifier_state_data,
                &mut total_compute,
                miller_loop_compute,
            ),
        };

        if current_coeff.is_none() {
            return total_compute;
        }
        total_compute += 120_000;
        if total_compute >= verifier_state_data.ml_max_compute {
            msg!("saving coeff for next instruction: ",);
            miller_loop_compute.current_coeff = current_coeff;
            return total_compute;
        }
        miller_loop_compute.current_coeff = None;
        ark_ec::models::bn::Bn::<ark_bn254::Parameters>::ell(
            &mut miller_loop_compute.f,
            &current_coeff.unwrap(),
            &current_pair,
        );

        verifier_state_data.outer_third_loop += 1;
    }
    verifier_state_data.computing_miller_loop = false;
    msg!("Initializing state for final_exponentiation.");
    verifier_state_data.computing_final_exponentiation = true;
    let mut f1 = miller_loop_compute.f;
    f1.conjugate();
    verifier_state_data.f_bytes1 = parse_f_to_bytes(f1);
    // Initializing temporary storage for final_exponentiation
    // with fqk::zero() which is equivalent to [[1], [0;383]].concat()
    verifier_state_data.f_bytes2[0] = 1;
    verifier_state_data.f_bytes3[0] = 1;
    verifier_state_data.f_bytes4[0] = 1;
    verifier_state_data.f_bytes5[0] = 1;
    verifier_state_data.i_bytes[0] = 1;
    // Skipping the first loop iteration since the naf_vec is zero.
    verifier_state_data.outer_loop = 1;
    // Adding compute costs for packing the initialized fs.
    // verifier_state_data.current_compute+=150_000;

    total_compute
}
