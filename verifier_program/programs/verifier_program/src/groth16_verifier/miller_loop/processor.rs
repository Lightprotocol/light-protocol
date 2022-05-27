use ark_ec;
use ark_ff::Field;

use ark_ec::bn::BnParameters;
use solana_program::log::sol_log_compute_units;

use crate::groth16_verifier::miller_loop::{
    instructions::*,
};
use crate::groth16_verifier::prepare_inputs::state::*;
use std::cell::RefMut;
use anchor_lang::prelude::*;
use crate::MillerLoopStateCompute;

pub fn miller_loop_process_instruction(
    tmp_account: &mut RefMut<'_, VerifierState>,
) {
    // if current_instruction_index == 0 {
    //     // initializing
    //     let mut tmp_account = VerifierState::new(
    //         ix_data[224..288].try_into().unwrap(),
    //         ix_data[288..416].try_into().unwrap(),
    //         ix_data[416..480].try_into().unwrap(),
    //         prepared_inputs_bytes.try_into().unwrap(),
    //         1_250_000);
    // }
    sol_log_compute_units();

    msg!("initing MillerLoopStateCompute");
    let mut tmp_account_compute = MillerLoopStateCompute::new(&tmp_account);
    // msg!("tmp_account_compute: {:?}", tmp_account_compute);

    sol_log_compute_units();
    msg!("computing");
    // needs 46 calls with 1_250_000 compute units per instruction
    miller_loop_onchain(
        tmp_account,
        &mut tmp_account_compute
    );
    // msg!("tmp_account_compute.f: {:?}", tmp_account_compute.f);
    // msg!("tmp_account_compute.r: {:?}", tmp_account_compute.r);
    sol_log_compute_units();
    msg!("packing");
    tmp_account_compute.pack(tmp_account);
    sol_log_compute_units();

    // msg!("tmp_account.f_bytes: {:?}", tmp_account.f_bytes);
    // msg!("tmp_account.r_bytes: {:?}", tmp_account.r_bytes);

    tmp_account.current_instruction_index +=1;
    msg!("tmp_account.current_instruction_index: {}", tmp_account.current_instruction_index);

}

pub fn miller_loop_onchain(
        tmp_account: &mut RefMut<'_, VerifierState>,
        tmp_account_compute: &mut MillerLoopStateCompute

    ) -> u64 {
    let mut total_steps: u64 = 0;


    for i in (1..ark_bn254::Parameters::ATE_LOOP_COUNT.len()-(tmp_account.outer_first_loop as usize)).rev() {

        if i != ark_bn254::Parameters::ATE_LOOP_COUNT.len() - 1  && tmp_account.square_in_place_executed == 0 {
            tmp_account_compute.f.square_in_place();

            // msg!("step: {}, f: {:?}\n",total_steps, tmp_account_compute.f);
            // msg!("coeff_index: {:?} ", tmp_account.coeff_index);
            // msg!("outer_first_loop: {} ", tmp_account.outer_first_loop);
            // msg!("outer_second_loop: {} ", tmp_account.outer_second_loop);
            // msg!("outer_third_loop: {} ", tmp_account.outer_third_loop);
            // msg!("square_in_place_executed: {} ", tmp_account.square_in_place_executed);
            // msg!("first_inner_loop_index: {} ", tmp_account.first_inner_loop_index);
            // msg!("second_inner_loop_index: {} \n", tmp_account.second_inner_loop_index);

            total_steps+=80_000;
            tmp_account.square_in_place_executed = 1;
            if total_steps >= tmp_account.compute_max_miller_loop  {
                // msg!("trying to return in square_in_place");
                return total_steps;
            }
        }

        // first_inner_loop_index
        for pair_index in tmp_account.first_inner_loop_index..3 {
            let current_pair = tmp_account_compute.pairs_0[pair_index as usize];
            // msg!("\ncurrent_pair {:?}\n", current_pair);

            let current_coeff = match tmp_account_compute.current_coeff {
                Some(coeff) => Some(coeff),
                None => get_coeff(pair_index, tmp_account, &mut total_steps, tmp_account_compute),
            };
            // msg!("\ncurrent_coeff {:?}\n", current_coeff);
            // msg!(" before f: {:?}\n", tmp_account_compute.f);
            if current_coeff.is_none() {
                return total_steps;
            }
            total_steps+=120_000;
            if total_steps >= tmp_account.compute_max_miller_loop  {
                tmp_account_compute.current_coeff = current_coeff;
                return total_steps;
            }
            tmp_account_compute.current_coeff = None;
            ark_ec::models::bn::Bn::<ark_bn254::Parameters>::ell(&mut tmp_account_compute.f, &current_coeff.unwrap(), &current_pair);
            // msg!("\ninner_first_coeff {}\n", tmp_account.inner_first_coeff);
            // msg!("\nouter_first_loop_coeff {}\n", tmp_account.outer_first_loop_coeff);
            // msg!("\nouter_second_coeff {}\n", tmp_account.outer_second_coeff);

            // msg!("step: {}, after f: {:?}\n",total_steps, tmp_account_compute.f);
            // msg!("coeff_index: {:?} ", tmp_account.coeff_index);
            // msg!("outer_first_loop: {} ", tmp_account.outer_first_loop);
            // msg!("outer_second_loop: {} ", tmp_account.outer_second_loop);
            // msg!("outer_third_loop: {} ", tmp_account.outer_third_loop);
            // msg!("square_in_place_executed: {} ", tmp_account.square_in_place_executed);
            // msg!("first_inner_loop_index: {} ", tmp_account.first_inner_loop_index);
            // msg!("second_inner_loop_index: {} \n", tmp_account.second_inner_loop_index);


            tmp_account.first_inner_loop_index+=1;



        }

        let bit = ark_bn254::Parameters::ATE_LOOP_COUNT[i as usize - 1];
        msg!("bit {}", bit);
        match bit {
            1 => {

                for pair_index in tmp_account.second_inner_loop_index..3 {
                    let current_pair = tmp_account_compute.pairs_0[pair_index as usize];
                    let current_coeff = match tmp_account_compute.current_coeff {
                        Some(coeff) => Some(coeff),
                        None => get_coeff(pair_index, tmp_account, &mut total_steps, tmp_account_compute),
                    };
                    // msg!("\ncurrent_coeff {:?}\n", current_coeff);
                    // msg!(" before f: {:?}\n", tmp_account_compute.f);
                    if current_coeff.is_none() {
                        return total_steps;
                    }
                    total_steps+=120_000;
                    if total_steps >= tmp_account.compute_max_miller_loop  {
                        tmp_account_compute.current_coeff = current_coeff;
                        return total_steps;
                    }
                    tmp_account_compute.current_coeff = None;
                    ark_ec::models::bn::Bn::<ark_bn254::Parameters>::ell(&mut tmp_account_compute.f, &current_coeff.unwrap(), &current_pair);

                    // msg!("step: {}, f: {:?}\n",total_steps, tmp_account_compute.f);
                    // msg!("coeff_index: {:?} ", tmp_account.coeff_index);
                    // msg!("outer_first_loop: {} ", tmp_account.outer_first_loop);
                    // msg!("outer_second_loop: {} ", tmp_account.outer_second_loop);
                    // msg!("outer_third_loop: {} ", tmp_account.outer_third_loop);
                    // msg!("square_in_place_executed: {} ", tmp_account.square_in_place_executed);
                    // msg!("first_inner_loop_index: {} ", tmp_account.first_inner_loop_index);
                    // msg!("second_inner_loop_index: {} \n", tmp_account.second_inner_loop_index);


                    tmp_account.second_inner_loop_index+=1;

                }
                // assert_eq!(*(pairs[0].1.next().unwrap()), coeffs_custom[0][coeff_index[0]], "coeffs wrong.");

                // assert_eq!(*f, *f_custom, "failed after second inner_loop0. {}",total_steps);

                tmp_account.first_inner_loop_index = 0;
                tmp_account.second_inner_loop_index =0;
                tmp_account.square_in_place_executed = 0;
                tmp_account.outer_first_loop+=1;
            }
            -1 => {

                for pair_index in tmp_account.second_inner_loop_index..3 {
                    let current_pair = tmp_account_compute.pairs_0[pair_index as usize];
                    let current_coeff = match tmp_account_compute.current_coeff {
                        Some(coeff) => Some(coeff),
                        None => get_coeff(pair_index, tmp_account, &mut total_steps, tmp_account_compute),
                    };
                    // msg!("\ncurrent_coeff {:?}\n", current_coeff);
                    // msg!(" before f: {:?}\n", tmp_account_compute.f);
                    if current_coeff.is_none() {
                        return total_steps;
                    }
                    total_steps+=120_000;
                    if total_steps >= tmp_account.compute_max_miller_loop  {
                        tmp_account_compute.current_coeff = current_coeff;
                        return total_steps;
                    }
                    tmp_account_compute.current_coeff = None;
                    ark_ec::models::bn::Bn::<ark_bn254::Parameters>::ell(&mut tmp_account_compute.f, &current_coeff.unwrap(), &current_pair);

                    // msg!("step: {}, f: {:?}\n",total_steps, tmp_account_compute.f);
                    // msg!("coeff_index: {:?} ", tmp_account.coeff_index);
                    // msg!("outer_first_loop: {} ", tmp_account.outer_first_loop);
                    // msg!("outer_second_loop: {} ", tmp_account.outer_second_loop);
                    // msg!("outer_third_loop: {} ", tmp_account.outer_third_loop);
                    // msg!("square_in_place_executed: {} ", tmp_account.square_in_place_executed);
                    // msg!("first_inner_loop_index: {} ", tmp_account.first_inner_loop_index);
                    // msg!("second_inner_loop_index: {} \n", tmp_account.second_inner_loop_index);


                    tmp_account.second_inner_loop_index+=1;

                }
                // assert_eq!(*f, *f_custom, "failed after second inner_loop1 {}",total_steps);

                tmp_account.first_inner_loop_index = 0;
                tmp_account.second_inner_loop_index =0;
                tmp_account.square_in_place_executed = 0;
                tmp_account.outer_first_loop+=1;
            }
            _ => {  tmp_account.first_inner_loop_index = 0;
                    tmp_account.second_inner_loop_index =0;
                    tmp_account.square_in_place_executed = 0;
                    tmp_account.outer_first_loop+=1;
                    // if total_steps == *compute_max_miller_loop  {
                    //     msg!("trying to return in match2");
                    //
                    //     return *f_custom;
                    // }
                    continue;
                },
        }


    }
    // msg!("after for loop *outer_first_loop {} < {}", *outer_first_loop, ark_bn254::Parameters::ATE_LOOP_COUNT.len());

    if ark_bn254::Parameters::X_IS_NEGATIVE {
        msg!("conjugating");
        tmp_account_compute.f.conjugate();
    }


    for pair_index in tmp_account.outer_second_loop..3 {
        msg!("outer_second_loop: pair_index: {}", pair_index);
        let current_pair = tmp_account_compute.pairs_0[pair_index as usize];
        msg!("tmp_account_compute.current_coeff is some: {}",tmp_account_compute.current_coeff.is_some());
        sol_log_compute_units();
        let current_coeff = match tmp_account_compute.current_coeff {
            Some(coeff) => Some(coeff),
            None => get_coeff(pair_index, tmp_account, &mut total_steps, tmp_account_compute),
        };
        sol_log_compute_units();
        // msg!("\ncurrent_coeff {:?}\n", current_coeff);
        // msg!(" before f: {:?}\n", tmp_account_compute.f);
        if current_coeff.is_none() {
            return total_steps;
        }
        total_steps+=120_000;
        if total_steps >= tmp_account.compute_max_miller_loop  {
            tmp_account_compute.current_coeff = current_coeff;
            return total_steps;
        }
        tmp_account_compute.current_coeff = None;
        ark_ec::models::bn::Bn::<ark_bn254::Parameters>::ell(&mut tmp_account_compute.f, &current_coeff.unwrap(), &current_pair);


        // msg!("step: {}, f: {:?}\n",total_steps, tmp_account_compute.f);
        // msg!("coeff_index: {:?} ", tmp_account.coeff_index);
        // msg!("outer_first_loop: {} ", tmp_account.outer_first_loop);
        // msg!("outer_second_loop: {} ", tmp_account.outer_second_loop);
        // msg!("outer_third_loop: {} ", tmp_account.outer_third_loop);
        // msg!("square_in_place_executed: {} ", tmp_account.square_in_place_executed);
        // msg!("first_inner_loop_index: {} ", tmp_account.first_inner_loop_index);
        // msg!("second_inner_loop_index: {} \n", tmp_account.second_inner_loop_index);


        tmp_account.outer_second_loop+=1;

    }
    // assert_eq!(*f, *f_custom, "outer_second_loop *f check failed");


    for pair_index in tmp_account.outer_third_loop..3 {
        // msg!("outer_second_loop: pair_index: {}", pair_index);

        let current_pair = tmp_account_compute.pairs_0[pair_index as usize];
        sol_log_compute_units();
        let current_coeff = match tmp_account_compute.current_coeff {
            Some(coeff) => Some(coeff),
            None => get_coeff(pair_index, tmp_account, &mut total_steps, tmp_account_compute),
        };
        sol_log_compute_units();
        // msg!("\ncurrent_coeff {:?}\n", current_coeff);
        // msg!(" before f: {:?}\n", tmp_account_compute.f);
        if current_coeff.is_none() {
            return total_steps;
        }
        total_steps+=120_000;
        if total_steps >= tmp_account.compute_max_miller_loop  {
            msg!("saving coeff for next instruction: ",);
            tmp_account_compute.current_coeff = current_coeff;
            return total_steps;
        }
        tmp_account_compute.current_coeff = None;
        ark_ec::models::bn::Bn::<ark_bn254::Parameters>::ell(&mut tmp_account_compute.f, &current_coeff.unwrap(), &current_pair);

        // let x: u8 = coeffs_custom[pair_index][coeff_index[pair_index]];
        // msg!("step: {}, f: {:?}\n",total_steps, tmp_account_compute.f);
        // msg!("coeff_index: {:?} ", tmp_account.coeff_index);
        // msg!("outer_first_loop: {} ", tmp_account.outer_first_loop);
        // msg!("outer_second_loop: {} ", tmp_account.outer_second_loop);
        // msg!("outer_third_loop: {} ", tmp_account.outer_third_loop);
        // msg!("square_in_place_executed: {} ", tmp_account.square_in_place_executed);
        // msg!("first_inner_loop_index: {} ", tmp_account.first_inner_loop_index);
        // msg!("second_inner_loop_index: {} \n", tmp_account.second_inner_loop_index);

        tmp_account.outer_third_loop+=1;

    }
    tmp_account.computing_miller_loop = false;
    total_steps
}
