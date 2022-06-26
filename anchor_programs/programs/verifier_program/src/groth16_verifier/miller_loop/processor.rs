use ark_ec;
use ark_ff::Field;

use crate::groth16_verifier::miller_loop::instructions::*;
use crate::groth16_verifier::VerifierState;
use crate::groth16_verifier::miller_loop::MillerLoopStateCompute;
use anchor_lang::prelude::*;
use ark_ec::bn::BnParameters;
use std::cell::RefMut;

pub fn miller_loop_process_instruction(tmp_account: &mut RefMut<'_, VerifierState>) {
    msg!("initing MillerLoopStateCompute");
    let mut tmp_account_compute = MillerLoopStateCompute::new(&tmp_account);

    // needs 46 calls with 1_250_000 compute units per instruction
    miller_loop_onchain(tmp_account, &mut tmp_account_compute);

    tmp_account_compute.pack(tmp_account);

    tmp_account.current_instruction_index += 1;
    msg!(
        "tmp_account.current_instruction_index: {}",
        tmp_account.current_instruction_index
    );
}

pub fn miller_loop_onchain(
    tmp_account: &mut RefMut<'_, VerifierState>,
    tmp_account_compute: &mut MillerLoopStateCompute,
) -> u64 {
    let mut total_compute: u64 = 0;

    for i in (1..ark_bn254::Parameters::ATE_LOOP_COUNT.len()
        - (tmp_account.outer_first_loop as usize))
        .rev()
    {
        if i != ark_bn254::Parameters::ATE_LOOP_COUNT.len() - 1
            && tmp_account.square_in_place_executed == 0
        {
            tmp_account_compute.f.square_in_place();

            total_compute += 80_000;
            tmp_account.square_in_place_executed = 1;
            if total_compute >= tmp_account.ml_max_compute {
                return total_compute;
            }
        }

        // first_inner_loop_index
        for pair_index in tmp_account.first_inner_loop_index..3 {
            let current_pair = tmp_account_compute.pairs_0[pair_index as usize];

            let current_coeff = match tmp_account_compute.current_coeff {
                Some(coeff) => Some(coeff),
                None => get_coeff(
                    pair_index,
                    tmp_account,
                    &mut total_compute,
                    tmp_account_compute,
                ),
            };

            if current_coeff.is_none() {
                return total_compute;
            }
            total_compute += 120_000;
            if total_compute >= tmp_account.ml_max_compute {
                tmp_account_compute.current_coeff = current_coeff;
                return total_compute;
            }
            tmp_account_compute.current_coeff = None;
            ark_ec::models::bn::Bn::<ark_bn254::Parameters>::ell(
                &mut tmp_account_compute.f,
                &current_coeff.unwrap(),
                &current_pair,
            );
            tmp_account.first_inner_loop_index += 1;
        }

        let bit = ark_bn254::Parameters::ATE_LOOP_COUNT[i as usize - 1];

        match bit {
            1 => {
                for pair_index in tmp_account.second_inner_loop_index..3 {
                    let current_pair = tmp_account_compute.pairs_0[pair_index as usize];
                    let current_coeff = match tmp_account_compute.current_coeff {
                        Some(coeff) => Some(coeff),
                        None => get_coeff(
                            pair_index,
                            tmp_account,
                            &mut total_compute,
                            tmp_account_compute,
                        ),
                    };
                    if current_coeff.is_none() {
                        return total_compute;
                    }
                    total_compute += 120_000;
                    if total_compute >= tmp_account.ml_max_compute {
                        tmp_account_compute.current_coeff = current_coeff;
                        return total_compute;
                    }
                    tmp_account_compute.current_coeff = None;
                    ark_ec::models::bn::Bn::<ark_bn254::Parameters>::ell(
                        &mut tmp_account_compute.f,
                        &current_coeff.unwrap(),
                        &current_pair,
                    );
                    tmp_account.second_inner_loop_index += 1;
                }
                tmp_account.first_inner_loop_index = 0;
                tmp_account.second_inner_loop_index = 0;
                tmp_account.square_in_place_executed = 0;
                tmp_account.outer_first_loop += 1;
            }
            -1 => {
                for pair_index in tmp_account.second_inner_loop_index..3 {
                    let current_pair = tmp_account_compute.pairs_0[pair_index as usize];
                    let current_coeff = match tmp_account_compute.current_coeff {
                        Some(coeff) => Some(coeff),
                        None => get_coeff(
                            pair_index,
                            tmp_account,
                            &mut total_compute,
                            tmp_account_compute,
                        ),
                    };

                    if current_coeff.is_none() {
                        return total_compute;
                    }
                    total_compute += 120_000;
                    if total_compute >= tmp_account.ml_max_compute {
                        tmp_account_compute.current_coeff = current_coeff;
                        return total_compute;
                    }
                    tmp_account_compute.current_coeff = None;
                    ark_ec::models::bn::Bn::<ark_bn254::Parameters>::ell(
                        &mut tmp_account_compute.f,
                        &current_coeff.unwrap(),
                        &current_pair,
                    );
                    tmp_account.second_inner_loop_index += 1;
                }

                tmp_account.first_inner_loop_index = 0;
                tmp_account.second_inner_loop_index = 0;
                tmp_account.square_in_place_executed = 0;
                tmp_account.outer_first_loop += 1;
            }
            _ => {
                tmp_account.first_inner_loop_index = 0;
                tmp_account.second_inner_loop_index = 0;
                tmp_account.square_in_place_executed = 0;
                tmp_account.outer_first_loop += 1;
                continue;
            }
        }
    }

    if ark_bn254::Parameters::X_IS_NEGATIVE {
        tmp_account_compute.f.conjugate();
    }

    for pair_index in tmp_account.outer_second_loop..3 {
        let current_pair = tmp_account_compute.pairs_0[pair_index as usize];

        let current_coeff = match tmp_account_compute.current_coeff {
            Some(coeff) => Some(coeff),
            None => get_coeff(
                pair_index,
                tmp_account,
                &mut total_compute,
                tmp_account_compute,
            ),
        };

        if current_coeff.is_none() {
            return total_compute;
        }
        total_compute += 120_000;
        if total_compute >= tmp_account.ml_max_compute {
            tmp_account_compute.current_coeff = current_coeff;
            return total_compute;
        }
        tmp_account_compute.current_coeff = None;
        ark_ec::models::bn::Bn::<ark_bn254::Parameters>::ell(
            &mut tmp_account_compute.f,
            &current_coeff.unwrap(),
            &current_pair,
        );

        tmp_account.outer_second_loop += 1;
    }

    for pair_index in tmp_account.outer_third_loop..3 {
        let current_pair = tmp_account_compute.pairs_0[pair_index as usize];
        let current_coeff = match tmp_account_compute.current_coeff {
            Some(coeff) => Some(coeff),
            None => get_coeff(
                pair_index,
                tmp_account,
                &mut total_compute,
                tmp_account_compute,
            ),
        };

        if current_coeff.is_none() {
            return total_compute;
        }
        total_compute += 120_000;
        if total_compute >= tmp_account.ml_max_compute {
            msg!("saving coeff for next instruction: ",);
            tmp_account_compute.current_coeff = current_coeff;
            return total_compute;
        }
        tmp_account_compute.current_coeff = None;
        ark_ec::models::bn::Bn::<ark_bn254::Parameters>::ell(
            &mut tmp_account_compute.f,
            &current_coeff.unwrap(),
            &current_pair,
        );

        tmp_account.outer_third_loop += 1;
    }
    tmp_account.computing_miller_loop = false;
    total_compute
}
