// use crate::parsers::*;
use crate::ml_254_parsers::*;
// use crate::get_proof_b;

use solana_program::{log::sol_log_compute_units, msg};

use ark_ff::Fp256;

use ark_ec;
use ark_ec::models::bn::BnParameters;
use ark_ec::models::bn::TwistType;
use ark_ec::SWModelParameters;
use ark_ff::biginteger::BigInteger256;
use ark_ff::Field;
use num_traits::One;

pub fn doubling_step_custom_0(
    r_bytes: &mut Vec<u8>,
    h_bytes: &mut Vec<u8>,
    g_bytes: &mut Vec<u8>,
    e_bytes: &mut Vec<u8>,
    lambda_bytes: &mut Vec<u8>,
    theta_bytes: &mut Vec<u8>,
) {
    // r was inited in init function.
    let mut r = parse_r_from_bytes(&r_bytes);
    //doubling_step::<ark_bls12_381::Parameters>(&mut r, &two_inv);
    let two_inv = Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
        9781510331150239090,
        15059239858463337189,
        10331104244869713732,
        2249375503248834476,
    ]));
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

    let e = <ark_bn254::Parameters as ark_ec::models::bn::BnParameters>::G2Parameters::COEFF_B
        * &(c.double() + &c);
    //sol_log_compute_units();
    //msg!("doubling_step 6");

    let f = e.double() + &e;

    //sol_log_compute_units();
    //msg!("doubling_step 11");

    let j = r.x.square();

    //sol_log_compute_units();

    //msg!("doubling_step 13");
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
    e_bytes: &Vec<u8>,
    lambda_bytes: &Vec<u8>,
) {
    let b = parse_quad_from_bytes(h_bytes); //5066
    let c = parse_quad_from_bytes(g_bytes);
    let f = parse_quad_from_bytes(lambda_bytes);
    let e = parse_quad_from_bytes(e_bytes);
    //sol_log_compute_units();
    let two_inv = Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
        9781510331150239090,
        15059239858463337189,
        10331104244869713732,
        2249375503248834476,
    ]));

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
    let j_d = j.double() + &j;
    let h = -h;

    match ark_bn254::Parameters::TWIST_TYPE {
        TwistType::M => (
            parse_quad_to_bytes(i, coeff_0_range),
            parse_quad_to_bytes(j_d, coeff_1_range),
            parse_quad_to_bytes(h, coeff_2_range),
        ),
        TwistType::D => (
            parse_quad_to_bytes(h, coeff_0_range),
            parse_quad_to_bytes(j_d, coeff_1_range),
            parse_quad_to_bytes(i, coeff_2_range),
        ),
    };
}

use ark_ff::fields::models::quadratic_extension::QuadExtField;

pub fn addition_step_helper<B: BnParameters>(
    proof_bytes_range: &mut Vec<u8>,
    proof_b_tmp_range: &Vec<u8>,
    computation_flag: &str,
) {
    // Manipulates q in respective manner. always called before
    // addition_step_custom_0.
    // Based on library implementation for bn254.
    let mut q = parse_proof_b_from_bytes(proof_b_tmp_range); // gets the unaffected proof.b (p) value as in the library implementation

    let TWIST_MUL_BY_Q_X = QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
        ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
            13075984984163199792,
            3782902503040509012,
            8791150885551868305,
            1825854335138010348,
        ])),
        ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
            7963664994991228759,
            12257807996192067905,
            13179524609921305146,
            2767831111890561987,
        ])),
    );
    // let x = <ark_bn254::Parameters>::B::TWIST_MUL_BY_Q_X;
    let TWIST_MUL_BY_Q_Y = QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
        ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
            16482010305593259561,
            13488546290961988299,
            3578621962720924518,
            2681173117283399901,
        ])),
        ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
            11661927080404088775,
            553939530661941723,
            7860678177968807019,
            3208568454732775116,
        ])),
    );
    if computation_flag == "normal" {
        parse_proof_b_to_bytes(q, proof_bytes_range);
    } else if computation_flag == "negq" {
        msg!("negq flag");
        let negq = -q;
        parse_proof_b_to_bytes(negq, proof_bytes_range);
    } else if computation_flag == "q1" {
        // let q1 = mul_by_char::<B>(q);
        msg!("q1 flag");

        q.x.frobenius_map(1);
        q.x *= &TWIST_MUL_BY_Q_X;

        q.y.frobenius_map(1);
        q.y *= &TWIST_MUL_BY_Q_Y;
        parse_proof_b_to_bytes(q, proof_bytes_range);
    } else if computation_flag == "q2" {
        // Note that rn q is alwas overwriting the proof_b_range.
        // Means future addition_step_helpers reuse a mutated value
        msg!("q2 flag");

        // let s = q.clone();
        // let q1 = ark_ec::models::bn::g2::mul_by_char::<ark_bn254::Parameters>(s);
        // let mut q2 = ark_ec::models::bn::g2::mul_by_char::<ark_bn254::Parameters>(q1);

        q.x.frobenius_map(1);
        q.x *= &TWIST_MUL_BY_Q_X;
        q.y.frobenius_map(1);
        q.y *= &TWIST_MUL_BY_Q_Y;

        // q2 part
        q.x.frobenius_map(1);
        q.x *= &TWIST_MUL_BY_Q_X;
        q.y.frobenius_map(1);
        q.y *= &TWIST_MUL_BY_Q_Y;
        // lastly
        q.y = -q.y;
        // q2.y = -q2.y
        // assert_eq!(q, q2, "halts top!");
        parse_proof_b_to_bytes(q, proof_bytes_range);
    }
}

pub fn addition_step_custom_0<B: BnParameters>(
    r_bytes: &Vec<u8>,
    h_bytes: &mut Vec<u8>,
    g_bytes: &mut Vec<u8>,
    e_bytes: &mut Vec<u8>,
    lambda_bytes: &mut Vec<u8>,
    theta_bytes: &mut Vec<u8>,
    proof_bytes: &Vec<u8>,
) {
    // Formula for line function when working with
    // homogeneous projective coordinates.
    let r = parse_r_from_bytes(r_bytes);
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
    // parse_r_to_bytes(r, r_bytes);
    //16000

    //12000 left complete
}

pub fn addition_step_custom_1<B: BnParameters>(
    r_bytes: &mut Vec<u8>,
    h_bytes: &Vec<u8>,
    g_bytes: &Vec<u8>,
    e_bytes: &Vec<u8>,
    lambda_bytes: &Vec<u8>,
    theta_bytes: &Vec<u8>,
) {
    let mut r = parse_r_from_bytes(r_bytes);
    let h = parse_quad_from_bytes(h_bytes); //5066
    let g = parse_quad_from_bytes(g_bytes);
    let e = parse_quad_from_bytes(e_bytes);
    let lambda = parse_quad_from_bytes(lambda_bytes);
    let theta = parse_quad_from_bytes(theta_bytes);

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

pub fn addition_step_custom_2<B: BnParameters>(
    coeff_0_range: &mut Vec<u8>,
    coeff_1_range: &mut Vec<u8>,
    coeff_2_range: &mut Vec<u8>,
    lambda_bytes: &Vec<u8>,
    theta_bytes: &Vec<u8>,
    proof_bytes: &Vec<u8>,
) {
    let q = parse_proof_b_from_bytes(proof_bytes);
    let lambda = parse_quad_from_bytes(lambda_bytes);
    let theta = parse_quad_from_bytes(theta_bytes);

    let j = theta * &q.x - &(lambda * &q.y);

    match B::TWIST_TYPE {
        TwistType::M => (
            parse_quad_to_bytes(j, coeff_0_range),
            parse_quad_to_bytes(-theta, coeff_1_range),
            parse_quad_to_bytes(lambda, coeff_2_range),
        ),
        TwistType::D => (
            parse_quad_to_bytes(lambda, coeff_0_range),
            parse_quad_to_bytes(-theta, coeff_1_range),
            parse_quad_to_bytes(j, coeff_2_range),
        ),
    };
}
