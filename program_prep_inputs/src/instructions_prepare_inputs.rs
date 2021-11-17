use ark_ff::fields::{Field,PrimeField,SquareRootField };
// use crate::constraints::{custom_mul_by_034,custom_mul_by_014};
use std::ops::{AddAssign, SubAssign};
use ark_ff::fp12_2over3over2::{Fp12, Fp12Parameters};
use ark_ff::fields::models::quadratic_extension::QuadExtField;
use ark_ff::fields::models::quadratic_extension::QuadExtParameters;
use ark_ff::fields::models::fp2::*;
use ark_ff::fields::models::fp6_3over2::{Fp6Parameters, Fp6};

// --> here

use ark_ec::{AffineCurve, PairingEngine, ProjectiveCurve};
// use super::{PreparedVerifyingKey, Proof, VerifyingKey};
use ark_relations::r1cs::{Result as R1CSResult, SynthesisError};
// use core::ops::{AddAssign, Neg};
use ark_ff::biginteger::{BigInteger256,BigInteger384};
use ark_ff::{Fp384, Fp256};
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

use core::ops::{Add, MulAssign, Neg, Sub};
use ark_ec::{models::SWModelParameters as Parameters};
use ark_ff::{
    // bytes::{FromBytes, ToBytes},
    fields::{BitIteratorBE},
    // ToConstraintField, UniformRand,
};


// @ark_groth16::prepare_inputs
use crate::parsers_prepare_inputs::*;
use crate:: hard_coded_verifying_key_pvk_new_ciruit::*;

// 0 (i,x)
pub fn init_pairs_instruction(
    public_inputs: &[ark_ff::Fp256<ark_ed_on_bls12_381::FqParameters>], // from bytes
    i_1_range: &mut Vec<u8>,
    x_1_range: &mut Vec<u8>,
    i_2_range: &mut Vec<u8>,
    x_2_range: &mut Vec<u8>,
    i_3_range: &mut Vec<u8>,
    x_3_range: &mut Vec<u8>,
    i_4_range: &mut Vec<u8>,
    x_4_range: &mut Vec<u8>,
    g_ic_x_range: &mut Vec<u8>,
    g_ic_y_range: &mut Vec<u8>,
    g_ic_z_range: &mut Vec<u8>
){
    // parse vk_gamma_abc_g1 from file
    let mut pvk_vk_gamma_abc_g1 = vec![get_gamma_abc_g1_0(), get_gamma_abc_g1_1(), get_gamma_abc_g1_2(), get_gamma_abc_g1_3(), get_gamma_abc_g1_4()];
    // inputs from bytes // 20k
    

    if (public_inputs.len() + 1) != pvk_vk_gamma_abc_g1.len() { // 693
        // Err(SynthesisError::MalformedVerifyingKey);
    }
    
    let g_ic = pvk_vk_gamma_abc_g1[0].into_projective(); // 80
    // parse g_ic 
    parse_group_projective_to_bytes(g_ic, g_ic_x_range,g_ic_y_range,g_ic_z_range); // 10k

    let mut i_vec : Vec<ark_ff::Fp256<ark_bls12_381::FrParameters>> = vec![];
    let mut x_vec : Vec<ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bls12_381::g1::Parameters>> = vec![];

    for (i,x) in public_inputs.iter().zip(pvk_vk_gamma_abc_g1.iter().skip(1)) {
        i_vec.push(*i);
        x_vec.push(*x);
    }
    parse_fp256_to_bytes(i_vec[0], i_1_range, [0,32]); // 3k
    parse_fp256_to_bytes(i_vec[1], i_2_range, [0,32]); // 3k
    parse_fp256_to_bytes(i_vec[2], i_3_range, [0,32]); // 3k
    parse_fp256_to_bytes(i_vec[3], i_4_range, [0,32]); // 3k
    parse_x_group_affine_to_bytes(x_vec[0],x_1_range); // 96b 10kr, 3kwr => 6k
    parse_x_group_affine_to_bytes(x_vec[1],x_2_range); // 6k
    parse_x_group_affine_to_bytes(x_vec[2],x_3_range); // 6k
    parse_x_group_affine_to_bytes(x_vec[3],x_4_range); // 6k


    let a = parse_fp256_from_bytes(i_1_range, [0,32]);
    assert_eq!(a,i_vec[0]);

    let b = parse_x_group_affine_from_bytes(x_1_range);
    assert_eq!(b,x_vec[0]);

    assert_eq!(b,pvk_vk_gamma_abc_g1[1]); // groupAffine vs G1Affine


    // assert_eq!(true, false);
}


use ark_std::Zero;




// 1
pub fn init_res_instruction(
    res_x_range: &mut Vec<u8>,
    res_y_range: &mut Vec<u8>,
    res_z_range: &mut Vec<u8>,
){

    let mut res : ark_ec::short_weierstrass_jacobian::GroupProjective<ark_bls12_381::g1::Parameters> = ark_ec::short_weierstrass_jacobian::GroupProjective::zero(); // 88
    
    // parse res
    parse_group_projective_to_bytes(res, res_x_range, res_y_range, res_z_range); //10k
}




// 2
pub fn maths_instruction(
    // i_range: &Vec<u8>, // current pair range based on I_data
    // x_range: &Vec<u8>,
    res_x_range: &mut Vec<u8>,
    res_y_range: &mut Vec<u8>,
    res_z_range: &mut Vec<u8>,

    i_range: &Vec<u8>, // current i based on instruction
    x_range: &Vec<u8>,

    current_index: usize, // based on index 
){
    // parse res from range
    let mut res = parse_group_projective_from_bytes(res_x_range,res_y_range,res_z_range); //15k
    // parse x from range
    let mut x = parse_x_group_affine_from_bytes(x_range); // 10k
    // parse i from range
    let mut i = parse_fp256_from_bytes(i_range, [0,32]); // 5k

    // create bit: (current i,x * current index)
    let a = i.into_repr(); // 1037
    let mut bits : ark_ff::BitIteratorBE<ark_ff::BigInteger256> = BitIteratorBE::new(a.into()); // 58

    let mut bits_without_leading_zeroes : Vec<bool> = bits.skip_while(|b| !b).collect();
    let skipped = 256 - bits_without_leading_zeroes.len(); // 1 => skipped 1
    // let new_len = skipped;

    if current_index < skipped{ // 0,1
        msg!("skipping leading zeroe instruction...");
        return
    } else {
        sol_log_compute_units();

        msg!("current index: {:?}", current_index);
        msg!("skipped: {:?}", skipped);
        sol_log_compute_units();

        // get the current bit but accounting for removed zeroes!
        // print bits_without_leading_zeroes
        let current_bit = bits_without_leading_zeroes[current_index-skipped];
        // let current_bit = bits_without_leading_zeroes.nth(current_index-new_len).unwrap(); // repl with index that shall be passed // nth consumes other bits

        // maths:
        res.double_in_place(); // 252 // 28145 // 28469 // 28411 // 28522 // 28306
        sol_log_compute_units();
    
        if current_bit {
            // res.add_assign_mixed(&x) ==> same as > (other=> <P>)
            if x.is_zero() { // 0
                msg!("if if");
                // return
            } 
            else if res.is_zero() { // 162
                msg!("if if else if");
                let p_basefield_one : Fp384<ark_bls12_381::FqParameters> = Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([8505329371266088957, 17002214543764226050, 6865905132761471162, 8632934651105793861, 6631298214892334189, 1582556514881692819])); // 31
                sol_log_compute_units();
            
                res.x = x.x;
                res.y = x.y;
                // res.z = res_t;
                // res.z = 
                res.z = p_basefield_one; // HARDCODED!&P:BASEFIELD::ONE();
                // return
            } else { 
                msg!("if if else if else"); 
                // Z1Z1 = Z1^2                    
                let z1z1 = res.z.square();
                // U2 = X2*Z1Z1
                let u2 = x.x * &z1z1;
                // S2 = Y2*Z1*Z1Z1
                let s2 = (x.y * &res.z) * &z1z1;
                sol_log_compute_units(); // 16709 COST 


                if res.x == u2 && res.y == s2 { // 30k cost?
                    msg!("if if else if else if");

                    // The two points are equal, so we double.
                    res.double_in_place();
                    sol_log_compute_units();

                } else {  // 29894 COST

                    // If we're adding -a and a together, self.z becomes zero as H becomes zero.
                    msg!("if if else if else if else");
                    // H = U2-X1
                    let h = u2 - &res.x;
        
                    // HH = H^2
                    let hh = h.square();
        
                    // I = 4*HH
                    let mut i = hh;
                    i.double_in_place().double_in_place();
        
                    // J = H*I
                    let mut j = h * &i;
        
                    // r = 2*(S2-Y1)
                    let r = (s2 - &res.y).double();
        
                    // V = X1*I
                    let v = res.x * &i;
        
                    // X3 = r^2 - J - 2*V
                    res.x = r.square();
                    res.x -= &j;
                    res.x -= &v;
                    res.x -= &v;
        
                    // Y3 = r*(V-X3)-2*Y1*J
                    j *= &res.y; // J = 2*Y1*J
                    j.double_in_place();
                    res.y = v - &res.x;
                    res.y *= &r;
                    res.y -= &j;
        
                    // Z3 = (Z1+H)^2-Z1Z1-HH
                    res.z += &h;
                    res.z.square_in_place();
                    res.z -= &z1z1;
                    res.z -= &hh;
                }
            }
        }
        sol_log_compute_units();


        // parse res
        parse_group_projective_to_bytes(res, res_x_range, res_y_range, res_z_range);

    }
}


//3
pub fn maths_g_ic_instruction(
    g_ic_x_range: &mut Vec<u8>,
    g_ic_y_range: &mut Vec<u8>,
    g_ic_z_range: &mut Vec<u8>,
    res_x_range: &Vec<u8>,
    res_y_range: &Vec<u8>,
    res_z_range: &Vec<u8>,

){

    // parse g_ic
    let mut g_ic = parse_group_projective_from_bytes(g_ic_x_range,g_ic_y_range,g_ic_z_range); // 15k
    // parse res
    let res = parse_group_projective_from_bytes(res_x_range,res_y_range,res_z_range); // 15k

    if g_ic.is_zero() {
        g_ic = res; // ? cost
        // return;
    } else 
    if res.is_zero() {
        // return;
    } else {
        // http://www.hyperelliptic.org/EFD/g1p/auto-shortw-jacobian-0.html#addition-add-2007-bl
        // Works for all curves.

        // Z1Z1 = Z1^2
        let z1z1 = g_ic.z.square();

        // Z2Z2 = Z2^2
        let z2z2 = res.z.square();

        // U1 = X1*Z2Z2
        let u1 = g_ic.x * &z2z2;

        // U2 = X2*Z1Z1
        let u2 = res.x * &z1z1;

        // S1 = Y1*Z2*Z2Z2
        let s1 = g_ic.y * &res.z * &z2z2;

        // S2 = Y2*Z1*Z1Z1
        let s2 = res.y * &g_ic.z * &z1z1;

        if u1 == u2 && s1 == s2 {
            // The two points are equal, so we double.
            g_ic.double_in_place();
        } else {
            // If we're adding -a and a together, self.z becomes zero as H becomes zero.

            // H = U2-U1
            let h = u2 - &u1;

            // I = (2*H)^2
            let i = (h.double()).square();

            // J = H*I
            let j = h * &i;

            // r = 2*(S2-S1)
            let r = (s2 - &s1).double();

            // V = U1*I
            let v = u1 * &i;

            // X3 = r^2 - J - 2*V
            g_ic.x = r.square() - &j - &(v.double());

            // Y3 = r*(V - X3) - 2*S1*J
            g_ic.y = r * &(v - &g_ic.x) - &*(s1 * &j).double_in_place();

            // Z3 = ((Z1+Z2)^2 - Z1Z1 - Z2Z2)*H
            g_ic.z = ((g_ic.z + &res.z).square() - &z1z1 - &z2z2) * &h;
        }
    }
    // pass g_ic => store in acc, pass on
    parse_group_projective_to_bytes(g_ic, g_ic_x_range,g_ic_y_range,g_ic_z_range) // 15k
    // res will be created anew with new loop, + new i,x will be used with index 
}



// do this inside lib i guess!
// pub fn pass_prepared_inputs_as_p(

// ){
// // parse g_ic 
// // turn into p:
// let p2 =  ark_ec::bls12::g1::G1Prepared::from((g_ic).into_affine());
// // parse p2 as p2xy into account_main

// } // pass from acc_prp to acc_main => or : turn into the p2 and then pass as p into acc