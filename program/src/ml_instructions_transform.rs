// use crate::parsers::*;
use crate::ml_parsers::*;
// use crate::get_proof_b;

use solana_program::{log::sol_log_compute_units, msg};

use ark_ff::Fp256;

use ark_ec;
use ark_ec::models::bn::BnParameters;
use ark_ec::models::bn::TwistType;
use ark_ec::SWModelParameters;
use ark_ff::biginteger::BigInteger256;
use ark_ff::fields::models::quadratic_extension::QuadExtField;
use ark_ff::Field;
use ark_ff::One;

pub fn doubling_step(
    r_bytes: &mut Vec<u8>,
    coeff_0_range: &mut Vec<u8>,
    coeff_1_range: &mut Vec<u8>,
    coeff_2_range: &mut Vec<u8>,
) {
    // step 0
    let mut r = parse_r_from_bytes(&r_bytes);
    let two_inv = Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
        9781510331150239090,
        15059239858463337189,
        10331104244869713732,
        2249375503248834476,
    ]));
    let mut a = r.x * &r.y;
    a.mul_assign_by_fp(&two_inv);
    let b = r.y.square();
    let c = r.z.square();
    let e = <ark_bn254::Parameters as ark_ec::models::bn::BnParameters>::G2Parameters::COEFF_B
        * &(c.double() + &c);
    let f = e.double() + &e;
    let j = r.x.square();
    r.x = a * &(b - &f);

    // step 1
    let mut g = b + &f;
    g.mul_assign_by_fp(&two_inv);
    let h = (r.y + &r.z).square() - &(b + &c);
    let i = e - &b;
    let e_square = e.square();
    r.y = g.square() - &(e_square.double() + &e_square);
    r.z = b * &h;
    parse_r_to_bytes(r, r_bytes);

    // step 2
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

pub fn addition_step<B: BnParameters>(
    coeff_0_range: &mut Vec<u8>,
    coeff_1_range: &mut Vec<u8>,
    coeff_2_range: &mut Vec<u8>,
    r_bytes: &mut Vec<u8>,
    proof_bytes: &Vec<u8>,
    computation_flag: &str,
) {
    let mut q = parse_proof_b_from_bytes(proof_bytes);

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
    } else if computation_flag == "negq" {
        q = -q;
    } else if computation_flag == "q1" {
        q.x.frobenius_map(1);
        q.x *= &TWIST_MUL_BY_Q_X;
        q.y.frobenius_map(1);
        q.y *= &TWIST_MUL_BY_Q_Y;
    } else if computation_flag == "q2" {
        q.x.frobenius_map(1);
        q.x *= &TWIST_MUL_BY_Q_X;
        q.y.frobenius_map(1);
        q.y *= &TWIST_MUL_BY_Q_Y;
        q.x.frobenius_map(1);
        q.x *= &TWIST_MUL_BY_Q_X;
        q.y.frobenius_map(1);
        q.y *= &TWIST_MUL_BY_Q_Y;
        q.y = -q.y;
    }

    // step 0
    // Formula for line function when working with
    // homogeneous projective coordinates.
    let mut r = parse_r_from_bytes(r_bytes);

    let theta = r.y - &(q.y * &r.z);
    let lambda = r.x - &(q.x * &r.z);
    let c = theta.square();
    let d = lambda.square();
    let e = lambda * &d;
    let f = r.z * &c;
    let g = r.x * &d;
    let h = e + &f - &g.double();

    // step 1
    r.x = lambda * &h;
    r.y = theta * &(g - &h) - &(e * &r.y);
    r.z *= &e;
    parse_r_to_bytes(r, r_bytes);

    // step 2
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
