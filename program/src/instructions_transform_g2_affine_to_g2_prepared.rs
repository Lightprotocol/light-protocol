use crate::parsers::*;
// use crate::get_proof_b;

use solana_program::{
    msg,
    log::sol_log_compute_units,
};

use ark_ff::{Fp384};

use ark_ff::biginteger::{BigInteger384};
use ark_ec;
use num_traits::{One};
use ark_ec::models::bls12::Bls12Parameters;
use ark_ff::Field;
use ark_ec::SWModelParameters;
use ark_ec::models::bls12::TwistType;

pub fn doubling_step_custom_0(
    r_bytes: &mut Vec<u8>,
    h_bytes: &mut Vec<u8>,
    g_bytes: &mut Vec<u8>,
    e_bytes: &mut Vec<u8>,
    lambda_bytes: &mut Vec<u8>,
    theta_bytes: &mut Vec<u8>,
    ) {
    let mut r = parse_r_from_bytes(&r_bytes);
    //doubling_step::<ark_bls12_381::Parameters>(&mut r, &two_inv);
    let two_inv = Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([1730508156817200468, 9606178027640717313, 7150789853162776431, 7936136305760253186, 15245073033536294050, 1728177566264616342]));
    //sol_log_compute_units();
    //msg!("doubling_step 1");

    let mut a = r.x * &r.y;
    //sol_log_compute_units();
    //msg!("doubling_step 2");

    a.mul_assign_by_fp(&two_inv);
    //sol_log_compute_units();
    //msg!("doubling_step 3");

    let b = r.y.square();
    //sol_log_compute_units();
    //msg!("doubling_step 4");

    let c = r.z.square();
    //sol_log_compute_units();
    //msg!("doubling_step 5");

    let e = <ark_bls12_381::Parameters as ark_ec::models::bls12::Bls12Parameters>::G2Parameters::COEFF_B * &(c.double() + &c);
    //sol_log_compute_units();
    //msg!("doubling_step 6");

    let f = e.double() + &e;

    //sol_log_compute_units();
    //msg!("doubling_step 11");

    let j = r.x.square();

    //sol_log_compute_units();

    //msg!("doubling_step 13");
    //a b f e_square, g , b, h
    r.x = a * &(b - &f);

    //------------------------ 80000
    //saving b f c e
    parse_quad_to_bytes(b, h_bytes); //5066
    parse_quad_to_bytes(c, g_bytes);
    parse_quad_to_bytes(f, lambda_bytes);
    parse_quad_to_bytes(e, e_bytes);
    parse_quad_to_bytes(j, theta_bytes);
    parse_r_to_bytes(r, r_bytes);
}

pub fn doubling_step_custom_1(
        r_bytes: &mut Vec<u8>,
        h_bytes: &mut Vec<u8>,
        g_bytes: &mut Vec<u8>,
        e_bytes: &mut Vec<u8>,
        lambda_bytes: &mut Vec<u8>,
    ) {

    let b = parse_quad_from_bytes(h_bytes); //5066
    let c = parse_quad_from_bytes(g_bytes);
    let f = parse_quad_from_bytes(lambda_bytes);
    let e = parse_quad_from_bytes(e_bytes);
    //sol_log_compute_units();
    let two_inv = Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([1730508156817200468, 9606178027640717313, 7150789853162776431, 7936136305760253186, 15245073033536294050, 1728177566264616342]));

    let mut r = parse_r_from_bytes(&r_bytes);

    //sol_log_compute_units();
    //msg!("doubling_step 7");


    let mut g = b + &f;
    //sol_log_compute_units();
    //msg!("doubling_step 8");

    g.mul_assign_by_fp(&two_inv);
    //sol_log_compute_units();
    //msg!("doubling_step 9");

    let h = (r.y + &r.z).square() - &(b + &c);
    //sol_log_compute_units();
    //msg!("doubling_step 10");

    let i = e - &b;

    //sol_log_compute_units();
    //msg!("doubling_step 12");

    let e_square = e.square();
    //sol_log_compute_units();

    //sol_log_compute_units();
    //msg!("doubling_step 14");

    r.y = g.square() - &(e_square.double() + &e_square);
    //sol_log_compute_units();
    //msg!("doubling_step 15");

    r.z = b * &h;
    //sol_log_compute_units();
    //msg!("parsing quads");
    parse_quad_to_bytes(h, h_bytes); //5066
    parse_quad_to_bytes(i, g_bytes);
    //sol_log_compute_units();

    parse_r_to_bytes(r, r_bytes);
}

pub fn doubling_step_custom_2(
    coeff_0_range: &mut Vec<u8>,
    coeff_1_range: &mut Vec<u8>,
    coeff_2_range: &mut Vec<u8>,
    h_bytes: &Vec<u8>,
    g_bytes: &Vec<u8>,
    theta_bytes: &Vec<u8>,
    ) {

    let h = parse_quad_from_bytes(h_bytes); //5066
    let i = parse_quad_from_bytes(g_bytes);
    let j = parse_quad_from_bytes(theta_bytes);
    //println!("j: {:?}", j);
    //let j_d = j.double();
    let j_d = j.double() + &j;
    //println!("j_d: {:?}", j_d);
    let h = -h;


    match ark_bls12_381::Parameters::TWIST_TYPE {
        TwistType::M => (parse_quad_to_bytes(i, coeff_0_range), parse_quad_to_bytes(j_d, coeff_1_range), parse_quad_to_bytes(h, coeff_2_range)),
        TwistType::D => (parse_quad_to_bytes(h, coeff_0_range), parse_quad_to_bytes(j_d, coeff_1_range), parse_quad_to_bytes(i, coeff_2_range)),
    };
}


pub fn init (r_range: &mut Vec<u8>, proof_range: &mut Vec<u8>, proof_b_bytes: &Vec<u8> ) { // pass in proof.b manually
    //change below to get bytes from account and not init from hardcoded bytes
    // let q = get_proof_b();
    let proof_b = parse_proof_b_from_bytes(proof_b_bytes);
    // //comment below for the change
    parse_proof_b_to_bytes(&proof_b, proof_range);

    let mut r: ark_ec::models::bls12::g2::G2HomProjective::<ark_bls12_381::Parameters> = ark_ec::models::bls12::g2::G2HomProjective {
        x: proof_b.x,
        y: proof_b.y,
        z: ark_bls12_381::Fq2::one(),
    };
    parse_r_to_bytes(r, r_range);
}

pub fn addition_step_custom_0<B: Bls12Parameters>(
        r_bytes: &mut Vec<u8>,
        h_bytes: &mut Vec<u8>,
        g_bytes: &mut Vec<u8>,
        e_bytes: &mut Vec<u8>,
        lambda_bytes: &mut Vec<u8>,
        theta_bytes: &mut Vec<u8>,
        proof_bytes: & Vec<u8>,
    )  {
    // Formula for line function when working with
    // homogeneous projective coordinates.
    let mut r = parse_r_from_bytes(&r_bytes);
    let q = parse_proof_b_from_bytes(proof_bytes);
    //sol_log_compute_units();
    //msg!("addition_step 1");
    let theta = r.y - &(q.y * &r.z);
    //sol_log_compute_units();
    //msg!("addition_step 2");
    let lambda = r.x - &(q.x * &r.z);
    //sol_log_compute_units();
    //msg!("addition_step 3");

    let c = theta.square();
    //sol_log_compute_units();
    //msg!("addition_step 4");
    let d = lambda.square();
    //sol_log_compute_units();
    //msg!("addition_step 5");
    let e = lambda * &d;
    //sol_log_compute_units();
    //msg!("addition_step 6");
    let f = r.z * &c;
    //sol_log_compute_units();
    //msg!("addition_step 7");
    let g = r.x * &d;
    //sol_log_compute_units();
    //msg!("addition_step 8");
    let h = e + &f - &g.double();
    //save h, g, e, lambda, theta,
    //sol_log_compute_units();
    //msg!("addition_step 9");

    parse_quad_to_bytes(h, h_bytes); //5066
    parse_quad_to_bytes(g, g_bytes);
    parse_quad_to_bytes(e, e_bytes);
    parse_quad_to_bytes(lambda, lambda_bytes);
    parse_quad_to_bytes(theta, theta_bytes);
    //26000
    //parse_r_to_bytes(r, r_bytes);
    //16000

    //12000 left complete
}

pub fn addition_step_custom_1<B: Bls12Parameters>(
        r_bytes: &mut Vec<u8>,
        h_bytes: &mut Vec<u8>,
        g_bytes: &mut Vec<u8>,
        e_bytes: &mut Vec<u8>,
        lambda_bytes: &mut Vec<u8>,
        theta_bytes: &mut Vec<u8>,
    )  {

    let mut r = parse_r_from_bytes(&r_bytes);
    let h = parse_quad_from_bytes(h_bytes); //5066
    let g = parse_quad_from_bytes(g_bytes);
    let e = parse_quad_from_bytes(e_bytes);
    let lambda =    parse_quad_from_bytes(lambda_bytes);
    let theta =     parse_quad_from_bytes(theta_bytes);
    //sol_log_compute_units();
    //msg!("addition_step 10");
    r.x = lambda * &h;
    //sol_log_compute_units();
    //msg!("addition_step 10");
    r.y = theta * &(g - &h) - &(e * &r.y);
    //sol_log_compute_units();
    //msg!("addition_step 11");
    r.z *= &e;
    //sol_log_compute_units();
    parse_r_to_bytes(r, r_bytes);

}

pub fn addition_step_custom_2<B: Bls12Parameters>(
        coeff_0_range: &mut Vec<u8>,
        coeff_1_range: &mut Vec<u8>,
        coeff_2_range: &mut Vec<u8>,
        lambda_bytes: &mut Vec<u8>,
        theta_bytes: &mut Vec<u8>,
        proof_bytes: & Vec<u8>,
    )  {

    let q = parse_proof_b_from_bytes(proof_bytes);

    let lambda =    parse_quad_from_bytes(lambda_bytes);
    let theta =     parse_quad_from_bytes(theta_bytes);
    //sol_log_compute_units();
    //msg!("addition_step 12");
    let j = theta * &q.x - &(lambda * &q.y);
    //sol_log_compute_units();
    //msg!("addition_step 13");

    match B::TWIST_TYPE {
        TwistType::M => (parse_quad_to_bytes(j, coeff_0_range), parse_quad_to_bytes(-theta, coeff_1_range), parse_quad_to_bytes(lambda, coeff_2_range)),
        TwistType::D => (parse_quad_to_bytes(lambda, coeff_0_range), parse_quad_to_bytes(-theta, coeff_1_range), parse_quad_to_bytes(j, coeff_2_range)),
    };

}