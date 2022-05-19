use crate::groth16_verifier::parsers::*;
use crate::utils::prepared_verifying_key::*;
use ark_ec::{AffineCurve, ProjectiveCurve};
use ark_ff::{
    fields::{Field, PrimeField},
    BitIteratorBE, Fp256, One,
    bytes::{FromBytes, ToBytes}

};
use ark_std::Zero;
use solana_program::{msg, program_error::ProgramError};
use anchor_lang::prelude::*;
use std::cell::RefMut;
use crate::state::*;
use crate::ErrorCode;

use ark_ff::to_bytes;
use ark_ff::BigInteger;

// Initializes all i,x pairs. 7 pairs for 7 public inputs.
// Creates all i,x pairs once, then stores them in specified ranges.
// Other ix can then parse the i,x pair they need. Storing all pairs allows us to replicate
// the loop behavior inside the library's implementation:
// https://docs.rs/ark-groth16/0.3.0/src/ark_groth16/verifier.rs.html#31-33
pub fn init_pairs_instruction<'info>(
    tmp_account: &mut RefMut<'_, PrepareInputsState>
) -> Result<()> {
    // Parses vk_gamma_abc_g1 from hard-coded file.
    // Should have 8 items if 7 public inputs are passed in since [0] will be used to initialize g_ic.
    // Called once.
    let pvk_vk_gamma_abc_g1 = vec![
        get_gamma_abc_g1_0(),
        get_gamma_abc_g1_1(),
        get_gamma_abc_g1_2(),
        get_gamma_abc_g1_3(),
        get_gamma_abc_g1_4(),
        get_gamma_abc_g1_5(),
        get_gamma_abc_g1_6(),
        get_gamma_abc_g1_7(),
    ];

    let public_inputs: [Fp256<ark_ed_on_bn254::FqParameters>; 7] = [
    <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&tmp_account.root_hash[..]).unwrap(),
    <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&tmp_account.amount[..]).unwrap(),
    <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&tmp_account.tx_integrity_hash[..]).unwrap(),
    <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&tmp_account.nullifier0[..]).unwrap(),
    <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&tmp_account.nullifier1[..]).unwrap(),
    <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&tmp_account.leaf_right[..]).unwrap(),
    <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&tmp_account.leaf_left[..]).unwrap(),
    ];

    if (public_inputs.len() + 1) != pvk_vk_gamma_abc_g1.len() {
        msg!("Incompatible Verifying Key");
        return err!(ErrorCode::IncompatibleVerifyingKey);
    }

    // inits g_ic into range.
    let g_ic = pvk_vk_gamma_abc_g1[0].into_projective();


    // parse_group_projective_to_bytes_254(g_ic, g_ic_x_range, g_ic_y_range, g_ic_z_range);
    tmp_account.g_ic_x_range = g_ic.x.into_repr().to_bytes_le().try_into().unwrap();
    tmp_account.g_ic_y_range = g_ic.y.into_repr().to_bytes_le().try_into().unwrap();
    tmp_account.g_ic_z_range = g_ic.z.into_repr().to_bytes_le().try_into().unwrap();

    // Creates and parses i,x pairs into ranges.
    let mut i_vec: Vec<ark_ff::Fp256<ark_ed_on_bn254::FqParameters>> = vec![];
    let mut x_vec: Vec<ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>> =
        vec![];

    for (i, x) in public_inputs.iter().zip(pvk_vk_gamma_abc_g1.iter().skip(1)) {
        i_vec.push(*i);
        x_vec.push(*x);
    }
    tmp_account.i_1_range = i_vec[0].into_repr().to_bytes_le().try_into().unwrap();
    msg!("i_1_range");
    tmp_account.i_2_range = i_vec[1].into_repr().to_bytes_le().try_into().unwrap();
    msg!("i_2_range");
    tmp_account.i_3_range = i_vec[2].into_repr().to_bytes_le().try_into().unwrap();
    msg!("i_3_range");
    tmp_account.i_4_range = i_vec[3].into_repr().to_bytes_le().try_into().unwrap();
    msg!("i_4_range");
    tmp_account.i_5_range = i_vec[4].into_repr().to_bytes_le().try_into().unwrap();
    msg!("i_5_range");
    tmp_account.i_6_range = i_vec[5].into_repr().to_bytes_le().try_into().unwrap();
    msg!("i_6_range");
    tmp_account.i_7_range = i_vec[6].into_repr().to_bytes_le().try_into().unwrap();

    // outsourced to this function to avoid stacklimit
    fill_x_ranges(x_vec, tmp_account);
    Ok(())
}




// Initializes fresh res range. Called once for each bit at the beginning of each loop (256x).
// Part of the mul() implementation: https://docs.rs/snarkvm-curves/0.5.0/src/snarkvm_curves/templates/short_weierstrass/short_weierstrass_jacobian.rs.html#161-164
// Refer to maths_instruction for details.
pub fn init_res_instruction(
    res_x_range: &mut [u8;32],
    res_y_range: &mut [u8;32],
    res_z_range: &mut [u8;32],
) -> Result<()> {
    let res: ark_ec::short_weierstrass_jacobian::GroupProjective<ark_bn254::g1::Parameters> =
        ark_ec::short_weierstrass_jacobian::GroupProjective::zero(); // 88

    parse_group_projective_to_bytes_254(res, res_x_range, res_y_range, res_z_range);
    Ok(())
}

// Represents: https://docs.rs/snarkvm-curves/0.5.0/src/snarkvm_curves/templates/short_weierstrass/short_weierstrass_jacobian.rs.html#161-164
// In order to fully match ^ implementation we need to execute init_res_instruction once + maths_instruction 256times. Also refer to the tests.
// Computes new res values. Gets the current i,x pair.
// The current i,x pair is already chosen by the processor based on ix_id.
// Called 256 times for each i,x pair - so 256*7x (7 inputs).
// Current_index (0..256) is parsed in because we need to
// replicate the stripping of leading zeroes (which are random becuase they're based on the public inputs).
pub fn maths_instruction(
    res_x_range: &mut [u8;32],
    res_y_range: &mut [u8;32],
    res_z_range: &mut [u8;32],
    i_range: &[u8;32],
    x_range: &[u8;64],
    current_index: usize,
    rounds: usize,
) -> Result<()> {
    // Parses res,x,i from range.
    let mut res = parse_group_projective_from_bytes_254(res_x_range, res_y_range, res_z_range);
    let x = parse_x_group_affine_from_bytes(x_range);
    let i = parse_fp256_ed_from_bytes(i_range);

    // create bit: (current i,x * current index).
    // First constructs all bits of current i,x pair.
    // Must skip leading zeroes. those are random based on the inputs (i).
    let a = i.into_repr();

    let bits: ark_ff::BitIteratorBE<ark_ff::BigInteger256> = BitIteratorBE::new(a);
    let bits_without_leading_zeroes: Vec<bool> = bits.skip_while(|b| !b).collect();
    let skipped = 256 - bits_without_leading_zeroes.len();

    // The current processor merges 4 rounds into one ix to efficiently use the compute budget.
    let mut index_in = current_index;
    for m in 0..rounds {
        // If i.e. two leading zeroes exists (skipped == 2), 2 ix will be skipped (0,1).
        if index_in < skipped {
            // parse_group_projective_to_bytes_254(res, res_x_range, res_y_range, res_z_range);
            // Only needed for if m==0 goes into else, which doesnt store the res value, then goes into if at m==1
            if m == rounds - 1 {
                parse_group_projective_to_bytes_254(res, res_x_range, res_y_range, res_z_range);
            }
        }
        else if (index_in - skipped) >= bits_without_leading_zeroes.len() {

        }
        else {
            // Get the current bit but account for removed zeroes.
            let current_bit = bits_without_leading_zeroes[index_in - skipped];
            // Info: when refering to the library's implementation keep in mind that here:
            // res == self
            // x == other
            res.double_in_place();

            if current_bit {
                // For reference to the native implementation: res.add_assign_mixed(&x) ==> same as ->
                if x.is_zero() {
                } else if res.is_zero() {
                    let p_basefield_one = Fp256::<ark_bn254::FqParameters>::one();
                    res.x = x.x;
                    res.y = x.y;
                    res.z = p_basefield_one;
                } else {
                    // Z1Z1 = Z1^2
                    let z1z1 = res.z.square();
                    // U2 = X2*Z1Z1
                    let u2 = x.x * z1z1;
                    // S2 = Y2*Z1*Z1Z1
                    let s2 = (x.y * res.z) * z1z1;
                    if res.x == u2 && res.y == s2 {
                        // The two points are equal, so we double.
                        res.double_in_place();
                    } else {
                        // If we're adding -a and a together, self.z becomes zero as H becomes zero.
                        // H = U2-X1
                        let h = u2 - res.x;
                        // HH = H^2
                        let hh = h.square();
                        // I = 4*HH
                        let mut i = hh;
                        i.double_in_place().double_in_place();
                        // J = H*I
                        let mut j = h * i;
                        // r = 2*(S2-Y1)
                        let r = (s2 - res.y).double();
                        // V = X1*I
                        let v = res.x * i;
                        // X3 = r^2 - J - 2*V
                        res.x = r.square();
                        res.x -= &j;
                        res.x -= &v;
                        res.x -= &v;
                        // Y3 = r*(V-X3)-2*Y1*J
                        j *= &res.y; // J = 2*Y1*J
                        j.double_in_place();
                        res.y = v - res.x;
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
            // if m == max
            if m == rounds - 1 {
                parse_group_projective_to_bytes_254(res, res_x_range, res_y_range, res_z_range);
            }
        }
        index_in += 1;
    }
    Ok(())
}

// Implements: https://docs.rs/snarkvm-curves/0.5.0/src/snarkvm_curves/templates/short_weierstrass/short_weierstrass_jacobian.rs.html#634-695
pub fn maths_g_ic_instruction(
    g_ic_x_range: &mut [u8;32],
    g_ic_y_range: &mut [u8;32],
    g_ic_z_range: &mut [u8;32],
    res_x_range: &[u8;32],
    res_y_range: &[u8;32],
    res_z_range: &[u8;32],
) -> Result<()> {
    let mut g_ic = parse_group_projective_from_bytes_254(g_ic_x_range, g_ic_y_range, g_ic_z_range); // 15k
    let res = parse_group_projective_from_bytes_254(res_x_range, res_y_range, res_z_range); // 15k

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
        let u1 = g_ic.x * z2z2;

        // U2 = X2*Z1Z1
        let u2 = res.x * z1z1;

        // S1 = Y1*Z2*Z2Z2
        let s1 = g_ic.y * res.z * z2z2;

        // S2 = Y2*Z1*Z1Z1
        let s2 = res.y * g_ic.z * z1z1;

        if u1 == u2 && s1 == s2 {
            // The two points are equal, so we double.
            g_ic.double_in_place();
        } else {
            // If we're adding -a and a together, self.z becomes zero as H becomes zero.

            // H = U2-U1
            let h = u2 - u1;

            // I = (2*H)^2
            let i = (h.double()).square();

            // J = H*I
            let j = h * i;

            // r = 2*(S2-S1)
            let r = (s2 - s1).double();

            // V = U1*I
            let v = u1 * i;

            // X3 = r^2 - J - 2*V
            g_ic.x = r.square() - j - (v.double());

            // Y3 = r*(V - X3) - 2*S1*J
            g_ic.y = r * (v - g_ic.x) - *(s1 * j).double_in_place();

            // Z3 = ((Z1+Z2)^2 - Z1Z1 - Z2Z2)*H
            g_ic.z = ((g_ic.z + res.z).square() - z1z1 - z2z2) * h;
        }
    }
    // res will be created anew with new loop, + new i,x will be used with index.
    parse_group_projective_to_bytes_254(g_ic, g_ic_x_range, g_ic_y_range, g_ic_z_range);
    Ok(())
}

// There are two ix in total to turn the g_ic from projective into affine.
// In the end the affine's stored in the x_1_range (overwrite).
// The verifier then reads the x_1_range to use the g_ic value as P2 for the millerloop.
// Split up into two ix because of compute budget limits.
pub fn g_ic_into_affine_1(
    g_ic_x_range: &mut [u8;32],
    g_ic_y_range: &mut [u8;32],
    g_ic_z_range: &mut [u8;32],
) -> Result<()> {
    let g_ic: ark_ec::short_weierstrass_jacobian::GroupProjective<ark_bn254::g1::Parameters> =
        parse_group_projective_from_bytes_254(g_ic_x_range, g_ic_y_range, g_ic_z_range); // 15k
    let zinv = ark_ff::Field::inverse(&g_ic.z).unwrap();
    let g_ic_with_zinv: ark_ec::short_weierstrass_jacobian::GroupProjective<
        ark_bn254::g1::Parameters,
    > = ark_ec::short_weierstrass_jacobian::GroupProjective::new(g_ic.x, g_ic.y, zinv);
    parse_group_projective_to_bytes_254(g_ic_with_zinv, g_ic_x_range, g_ic_y_range, g_ic_z_range);
    Ok(())
}

pub fn g_ic_into_affine_2(
    g_ic_x_range: &[u8;32],
    g_ic_y_range: &[u8;32],
    g_ic_z_range: &[u8;32],
    x_1_range: &mut [u8;64],
) -> Result<()> {
    let g_ic: ark_ec::short_weierstrass_jacobian::GroupProjective<ark_bn254::g1::Parameters> =
        parse_group_projective_from_bytes_254(g_ic_x_range, g_ic_y_range, g_ic_z_range); // 15k

    let zinv_squared = ark_ff::Field::square(&g_ic.z);
    let x = g_ic.x * zinv_squared;
    let y = g_ic.y * (zinv_squared * g_ic.z);

    let g_ic_affine: ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters> =
        ark_ec::short_weierstrass_jacobian::GroupAffine::new(x, y, false);

    parse_x_group_affine_to_bytes(g_ic_affine, x_1_range.try_into().unwrap());
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::groth16_verifier::parsers::{
        parse_fp256_from_bytes, parse_fp256_to_bytes, parse_x_group_affine_from_bytes,
        parse_x_group_affine_to_bytes,
    };
    use crate::groth16_verifier::prepare_inputs::instructions::{
        g_ic_into_affine_1, g_ic_into_affine_2, maths_g_ic_instruction, maths_instruction,
    };
    use ark_ec::AffineCurve;
    use ark_ff::PrimeField;
    use ark_std::{test_rng, UniformRand, Zero};
    use std::ops::AddAssign;

    #[test]
    fn g_ic_into_affine_should_succeed() {
        let mut rng = test_rng();
        for _i in 0..10 {
            let reference_g_ic: ark_ec::short_weierstrass_jacobian::GroupProjective<
                ark_bn254::g1::Parameters,
            > = ark_ec::short_weierstrass_jacobian::GroupProjective::rand(&mut rng);
            let test_g_ic_x = reference_g_ic.x.clone();
            let test_g_ic_y = reference_g_ic.y.clone();
            let test_g_ic_z = reference_g_ic.z.clone();
            let mut account_g_ic_x_range = vec![0u8; 32];
            let mut account_g_ic_y_range = vec![0u8; 32];
            let mut account_g_ic_z_range = vec![0u8; 32];
            let mut account_x_range = vec![0u8; 64];
            parse_fp256_to_bytes(test_g_ic_x, &mut account_g_ic_x_range);
            parse_fp256_to_bytes(test_g_ic_y, &mut account_g_ic_y_range);
            parse_fp256_to_bytes(test_g_ic_z, &mut account_g_ic_z_range);

            g_ic_into_affine_1(
                &mut account_g_ic_x_range,
                &mut account_g_ic_y_range,
                &mut account_g_ic_z_range,
            )
            .unwrap();
            g_ic_into_affine_2(
                &account_g_ic_x_range,
                &account_g_ic_y_range,
                &account_g_ic_z_range,
                &mut account_x_range,
            )
            .unwrap();
            let affine_ref: ark_ec::short_weierstrass_jacobian::GroupAffine<
                ark_bn254::g1::Parameters,
            > = reference_g_ic.into();
            assert_eq!(
                affine_ref,
                parse_x_group_affine_from_bytes(&account_x_range)
            );
        }
    }

    #[test]
    fn maths_g_ic_instruction_should_succeed() {
        let mut rng = test_rng();
        for _i in 0..10 {
            let reference_res: ark_ec::short_weierstrass_jacobian::GroupProjective<
                ark_bn254::g1::Parameters,
            > = ark_ec::short_weierstrass_jacobian::GroupProjective::rand(&mut rng);
            let mut reference_g_ic: ark_ec::short_weierstrass_jacobian::GroupProjective<
                ark_bn254::g1::Parameters,
            > = ark_ec::short_weierstrass_jacobian::GroupProjective::rand(&mut rng);

            let test_res_x = reference_res.x.clone();
            let test_res_y = reference_res.y.clone();
            let test_res_z = reference_res.z.clone();
            let test_g_ic_x = reference_g_ic.x.clone();
            let test_g_ic_y = reference_g_ic.y.clone();
            let test_g_ic_z = reference_g_ic.z.clone();

            //simulating the onchain account
            let mut account_res_x_range = vec![0u8; 32];
            let mut account_res_y_range = vec![0u8; 32];
            let mut account_res_z_range = vec![0u8; 32];
            let mut account_g_ic_x_range = vec![0u8; 32];
            let mut account_g_ic_y_range = vec![0u8; 32];
            let mut account_g_ic_z_range = vec![0u8; 32];
            parse_fp256_to_bytes(test_res_x, &mut account_res_x_range);
            parse_fp256_to_bytes(test_res_y, &mut account_res_y_range);
            parse_fp256_to_bytes(test_res_z, &mut account_res_z_range);
            parse_fp256_to_bytes(test_g_ic_x, &mut account_g_ic_x_range);
            parse_fp256_to_bytes(test_g_ic_y, &mut account_g_ic_y_range);
            parse_fp256_to_bytes(test_g_ic_z, &mut account_g_ic_z_range);

            // test instruction, mut accs
            maths_g_ic_instruction(
                &mut account_g_ic_x_range,
                &mut account_g_ic_y_range,
                &mut account_g_ic_z_range,
                &account_res_x_range,
                &account_res_y_range,
                &account_res_z_range,
            )
            .unwrap();
            // reference value
            reference_g_ic.add_assign(&reference_res);
            // ref gic..
            assert_eq!(
                reference_g_ic.x,
                parse_fp256_from_bytes(&account_g_ic_x_range)
            );
            assert_eq!(
                reference_g_ic.y,
                parse_fp256_from_bytes(&account_g_ic_y_range)
            );
            assert_eq!(
                reference_g_ic.z,
                parse_fp256_from_bytes(&account_g_ic_z_range)
            );
        }
    }

    #[test]
    fn maths_g_ic_instruction_should_fail() {
        let mut rng = test_rng();
        for _i in 0..10 {
            let reference_res: ark_ec::short_weierstrass_jacobian::GroupProjective<
                ark_bn254::g1::Parameters,
            > = ark_ec::short_weierstrass_jacobian::GroupProjective::rand(&mut rng);
            let mut reference_g_ic: ark_ec::short_weierstrass_jacobian::GroupProjective<
                ark_bn254::g1::Parameters,
            > = ark_ec::short_weierstrass_jacobian::GroupProjective::rand(&mut rng);

            let test_res_x = reference_res.x.clone();
            let test_res_y = reference_res.y.clone();
            let test_res_z = reference_res.z.clone();

            // fails here
            let test_g_ic: ark_ec::short_weierstrass_jacobian::GroupProjective<
                ark_bn254::g1::Parameters,
            > = ark_ec::short_weierstrass_jacobian::GroupProjective::rand(&mut rng);
            let test_g_ic_x = test_g_ic.x.clone();
            let test_g_ic_y = test_g_ic.y.clone();
            let test_g_ic_z = test_g_ic.z.clone();

            //simulating the onchain account
            let mut account_res_x_range = vec![0u8; 32];
            let mut account_res_y_range = vec![0u8; 32];
            let mut account_res_z_range = vec![0u8; 32];
            let mut account_g_ic_x_range = vec![0u8; 32];
            let mut account_g_ic_y_range = vec![0u8; 32];
            let mut account_g_ic_z_range = vec![0u8; 32];
            parse_fp256_to_bytes(test_res_x, &mut account_res_x_range);
            parse_fp256_to_bytes(test_res_y, &mut account_res_y_range);
            parse_fp256_to_bytes(test_res_z, &mut account_res_z_range);
            parse_fp256_to_bytes(test_g_ic_x, &mut account_g_ic_x_range);
            parse_fp256_to_bytes(test_g_ic_y, &mut account_g_ic_y_range);
            parse_fp256_to_bytes(test_g_ic_z, &mut account_g_ic_z_range);

            // test instruction, mut accs
            maths_g_ic_instruction(
                &mut account_g_ic_x_range,
                &mut account_g_ic_y_range,
                &mut account_g_ic_z_range,
                &account_res_x_range,
                &account_res_y_range,
                &account_res_z_range,
            )
            .unwrap();
            // reference value
            reference_g_ic.add_assign(&reference_res);
            assert!(reference_g_ic.x != parse_fp256_from_bytes(&account_g_ic_x_range));
            assert!(reference_g_ic.y != parse_fp256_from_bytes(&account_g_ic_y_range));
            assert!(reference_g_ic.z != parse_fp256_from_bytes(&account_g_ic_z_range));
        }
    }
    #[test]
    fn maths_instruction_should_succeed() {
        let mut rng = test_rng();
        for _i in 0..10 {
            let reference_i_range =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                );

            let reference_x_range = ark_ec::short_weierstrass_jacobian::GroupAffine::<
                ark_bn254::g1::Parameters,
            >::new(
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                ),
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                ),
                false,
            );
            // init res as 0 like in mul_bits
            let reference_res: ark_ec::short_weierstrass_jacobian::GroupProjective<
                ark_bn254::g1::Parameters,
            > = ark_ec::short_weierstrass_jacobian::GroupProjective::zero();

            let test_res_x = reference_res.x.clone();
            let test_res_y = reference_res.y.clone();
            let test_res_z = reference_res.z.clone();
            let test_i_range = reference_i_range.clone();
            let test_x_range = reference_x_range.clone();

            //simulating the onchain account
            let mut account_res_x_range = vec![0u8; 32];
            let mut account_res_y_range = vec![0u8; 32];
            let mut account_res_z_range = vec![0u8; 32];
            let mut account_i_range = vec![0u8; 32];
            let mut account_x_range = vec![0u8; 64];

            parse_fp256_to_bytes(test_res_x, &mut account_res_x_range);
            parse_fp256_to_bytes(test_res_y, &mut account_res_y_range);
            parse_fp256_to_bytes(test_res_z, &mut account_res_z_range);
            parse_fp256_to_bytes(test_i_range, &mut account_i_range);
            parse_x_group_affine_to_bytes(test_x_range, &mut account_x_range);

            // test instruction, mut accs
            for current_index in 0..256 {
                maths_instruction(
                    &mut account_res_x_range,
                    &mut account_res_y_range,
                    &mut account_res_z_range,
                    &account_i_range,
                    &account_x_range,
                    current_index,
                    1,
                )
                .unwrap();
            }
            // reference value
            let repr = reference_i_range.into_repr();
            let res_ref = &reference_x_range.mul(repr);
            assert_eq!(res_ref.x, parse_fp256_from_bytes(&account_res_x_range));
            assert_eq!(res_ref.y, parse_fp256_from_bytes(&account_res_y_range));
            assert_eq!(res_ref.z, parse_fp256_from_bytes(&account_res_z_range));
        }
    }
    #[test]
    fn maths_instruction_should_fail() {
        let mut rng = test_rng();
        for _i in 0..10 {
            let reference_i_range =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                );

            let reference_x_range = ark_ec::short_weierstrass_jacobian::GroupAffine::<
                ark_bn254::g1::Parameters,
            >::new(
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                ),
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                ),
                false,
            );
            // init res as 0 like in mul_bits
            let reference_res: ark_ec::short_weierstrass_jacobian::GroupProjective<
                ark_bn254::g1::Parameters,
            > = ark_ec::short_weierstrass_jacobian::GroupProjective::zero();

            let test_res_x = reference_res.x.clone();
            let test_res_y = reference_res.y.clone();
            let test_res_z = reference_res.z.clone();
            // failing here:
            let test_i_range =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                );
            let test_x_range = reference_x_range.clone();

            //simulating the onchain account
            let mut account_res_x_range = vec![0u8; 32];
            let mut account_res_y_range = vec![0u8; 32];
            let mut account_res_z_range = vec![0u8; 32];
            let mut account_i_range = vec![0u8; 32];
            let mut account_x_range = vec![0u8; 64];

            parse_fp256_to_bytes(test_res_x, &mut account_res_x_range);
            parse_fp256_to_bytes(test_res_y, &mut account_res_y_range);
            parse_fp256_to_bytes(test_res_z, &mut account_res_z_range);
            parse_fp256_to_bytes(test_i_range, &mut account_i_range);
            parse_x_group_affine_to_bytes(test_x_range, &mut account_x_range);

            // test instruction
            for current_index in 0..256 {
                maths_instruction(
                    &mut account_res_x_range,
                    &mut account_res_y_range,
                    &mut account_res_z_range,
                    &account_i_range,
                    &account_x_range,
                    current_index,
                    1,
                )
                .unwrap();
            }
            let res_ref = &reference_x_range.mul(reference_i_range.into_repr());
            assert!(res_ref.x != parse_fp256_from_bytes(&account_res_x_range));
            assert!(res_ref.y != parse_fp256_from_bytes(&account_res_y_range));
            assert!(res_ref.z != parse_fp256_from_bytes(&account_res_z_range));
        }
    }
}
