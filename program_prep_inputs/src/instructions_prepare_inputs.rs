use ark_ff::fields::{Field, PrimeField, SquareRootField};
// use crate::constraints::{custom_mul_by_034,custom_mul_by_014};
use ark_ff::fields::models::fp2::*;
use ark_ff::fields::models::fp6_3over2::{Fp6, Fp6Parameters};
use ark_ff::fields::models::quadratic_extension::QuadExtField;
use ark_ff::fields::models::quadratic_extension::QuadExtParameters;
use ark_ff::fp12_2over3over2::{Fp12, Fp12Parameters};
use std::ops::{AddAssign, SubAssign};

// --> here

use ark_ec::{AffineCurve, PairingEngine, ProjectiveCurve};
// use super::{PreparedVerifyingKey, Proof, VerifyingKey};
use ark_relations::r1cs::{Result as R1CSResult, SynthesisError};
// use core::ops::{AddAssign, Neg};
use ark_ff::biginteger::{BigInteger256, BigInteger384};
use ark_ff::{Fp256, Fp384};
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

use ark_ec::models::SWModelParameters as Parameters;
use ark_ff::fields::BitIteratorBE;
use core::ops::{Add, MulAssign, Neg, Sub};

use crate::hard_coded_verifying_key_pvk_new_ciruit::*;
use crate::parsers_prepare_inputs::*;
use ark_std::Zero;

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
    g_ic_z_range: &mut Vec<u8>,
) {
    // parse vk_gamma_abc_g1 from file
    // inputs from bytes // 20k
    let mut pvk_vk_gamma_abc_g1 = vec![
        get_gamma_abc_g1_0(),
        get_gamma_abc_g1_1(),
        get_gamma_abc_g1_2(),
        get_gamma_abc_g1_3(),
        get_gamma_abc_g1_4(),
    ];

    if (public_inputs.len() + 1) != pvk_vk_gamma_abc_g1.len() { // 693
         // Err(SynthesisError::MalformedVerifyingKey);
    }

    // init g_ic into range
    let g_ic = pvk_vk_gamma_abc_g1[0].into_projective(); // 80
    parse_group_projective_to_bytes(g_ic, g_ic_x_range, g_ic_y_range, g_ic_z_range); // 10k

    // store i,x pairs into ranges
    let mut i_vec: Vec<ark_ff::Fp256<ark_bls12_381::FrParameters>> = vec![];
    let mut x_vec: Vec<
        ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bls12_381::g1::Parameters>,
    > = vec![];

    for (i, x) in public_inputs.iter().zip(pvk_vk_gamma_abc_g1.iter().skip(1)) {
        i_vec.push(*i);
        x_vec.push(*x);
    }
    parse_fp256_to_bytes(i_vec[0], i_1_range, [0, 32]); // 3k
    parse_fp256_to_bytes(i_vec[1], i_2_range, [0, 32]); // 3k
    parse_fp256_to_bytes(i_vec[2], i_3_range, [0, 32]); // 3k
    parse_fp256_to_bytes(i_vec[3], i_4_range, [0, 32]); // 3k
    parse_x_group_affine_to_bytes(x_vec[0], x_1_range); // 96bytes 10kr, 3kwr => 6k
    parse_x_group_affine_to_bytes(x_vec[1], x_2_range); // 6k
    parse_x_group_affine_to_bytes(x_vec[2], x_3_range); // 6k
    parse_x_group_affine_to_bytes(x_vec[3], x_4_range); // 6k

    // check proper parsing
    let a = parse_fp256_from_bytes(i_1_range, [0, 32]);
    assert_eq!(a, i_vec[0]);
    let b = parse_x_group_affine_from_bytes(x_1_range);
    assert_eq!(b, x_vec[0]);
    assert_eq!(b, pvk_vk_gamma_abc_g1[1]); // groupAffine vs G1Affine
}

// 1
pub fn init_res_instruction(
    res_x_range: &mut Vec<u8>,
    res_y_range: &mut Vec<u8>,
    res_z_range: &mut Vec<u8>,
) {
    // init fresh res ranges
    let mut res: ark_ec::short_weierstrass_jacobian::GroupProjective<
        ark_bls12_381::g1::Parameters,
    > = ark_ec::short_weierstrass_jacobian::GroupProjective::zero(); // 88

    parse_group_projective_to_bytes(res, res_x_range, res_y_range, res_z_range);
    //10k
}

// 2
pub fn maths_instruction(
    res_x_range: &mut Vec<u8>,
    res_y_range: &mut Vec<u8>,
    res_z_range: &mut Vec<u8>,
    i_range: &Vec<u8>, // current i (0..4) based on instruction
    x_range: &Vec<u8>,
    current_index: usize, // based on index (0..256)
) {
    // parse res,x,i from range
    let mut res = parse_group_projective_from_bytes(res_x_range, res_y_range, res_z_range); //15k
    let x = parse_x_group_affine_from_bytes(x_range); // 10k
    let i = parse_fp256_from_bytes(i_range, [0, 32]); // 5k

    // create bit: (current i,x * current index).
    // first constructs all bits of current i,x pair.
    // must skip leading zeroes. those are random based on the inputs (i).
    let a = i.into_repr(); // 1037
    let bits: ark_ff::BitIteratorBE<ark_ff::BigInteger256> = BitIteratorBE::new(a.into()); // 58
    let bits_without_leading_zeroes: Vec<bool> = bits.skip_while(|b| !b).collect();
    let skipped = 256 - bits_without_leading_zeroes.len();

    // if i.e. two leading zeroes exists (skipped == 2), 2 ix will be skipped (0,1)
    if current_index < skipped {
        msg!("skipping leading zero instruction...");
        return;
    } else {
        sol_log_compute_units();
        msg!("current index: {:?}", current_index);
        msg!("skipped: {:?}", skipped);
        // get the current bit but account for removed zeroes.
        let current_bit = bits_without_leading_zeroes[current_index - skipped];

        // this is where the actual maths start
        res.double_in_place(); // 252 // 28145 // 28469 // 28411 // 28522 // 28306
        sol_log_compute_units();
        if current_bit {
            // res.add_assign_mixed(&x) ==> same as >
            if x.is_zero() {
                // cost: 0
                msg!("if if");
            } else if res.is_zero() {
                // cost: 162
                msg!("if if else if");
                let p_basefield_one: Fp384<ark_bls12_381::FqParameters> =
                    Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([
                        8505329371266088957,
                        17002214543764226050,
                        6865905132761471162,
                        8632934651105793861,
                        6631298214892334189,
                        1582556514881692819,
                    ])); // cost: 31
                sol_log_compute_units();
                res.x = x.x;
                res.y = x.y;
                // res.z = res_t;
                // res.z =
                res.z = p_basefield_one; // HARDCODED!&P:BASEFIELD::ONE();
            } else {
                msg!("if if else if else");
                // Z1Z1 = Z1^2
                let z1z1 = res.z.square();
                // U2 = X2*Z1Z1
                let u2 = x.x * &z1z1;
                // S2 = Y2*Z1*Z1Z1
                let s2 = (x.y * &res.z) * &z1z1;
                sol_log_compute_units(); // cost: 16709

                if res.x == u2 && res.y == s2 {
                    // cost: 30k
                    msg!("if if else if else if");

                    // The two points are equal, so we double.
                    res.double_in_place();
                    sol_log_compute_units();
                } else {
                    // cost: 29894

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
) {
    // parse g_ic
    let mut g_ic = parse_group_projective_from_bytes(g_ic_x_range, g_ic_y_range, g_ic_z_range); // 15k
    let res = parse_group_projective_from_bytes(res_x_range, res_y_range, res_z_range); // 15k

    if g_ic.is_zero() {
        g_ic = res;
    } else if res.is_zero() {
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
    // res will be created anew with new loop, + new i,x will be used with index
    // cost: 15k
    parse_group_projective_to_bytes(g_ic, g_ic_x_range, g_ic_y_range, g_ic_z_range)
}
