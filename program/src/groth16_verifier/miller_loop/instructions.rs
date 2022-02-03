use crate::groth16_verifier::parsers::*;
use crate::utils::prepared_verifying_key::*;
use ark_ec;
use ark_ec::{
    models::bn::{BnParameters, TwistType},
    SWModelParameters,
};
use ark_ff::{
    fields::{
        models::{
            fp6_3over2::Fp6, quadratic_extension::QuadExtField,
            quadratic_extension::QuadExtParameters,
        },
        Field, Fp2,
    },
    fp12_2over3over2::Fp12Parameters,
    One, Zero,
};
use solana_program::msg;
use solana_program::program_error::ProgramError;

const C0_SUB_RANGE: [usize; 2] = [0, 192];
const C1_SUB_RANGE: [usize; 2] = [192, 384];
use ark_ff::fields::models::fp2::Fp2Parameters;
// All instructions are as per the bn254 implemenation of arkworks
// https://docs.rs/ark-ec/0.3.0/src/ark_ec/models/bn/g2.rs.html#139-166
pub fn doubling_step(
    r_bytes: &mut Vec<u8>,
    coeff_0_range: &mut Vec<u8>,
    coeff_1_range: &mut Vec<u8>,
    coeff_2_range: &mut Vec<u8>,
) -> Result<(), ProgramError> {
    // step 0
    let mut r = parse_r_from_bytes(r_bytes);
    let two_inv = <ark_bn254::Fq2Parameters as Fp2Parameters>::Fp::one()
        .double()
        .inverse()
        .unwrap();
    let mut a = r.x * r.y;
    a.mul_assign_by_fp(&two_inv);
    let b = r.y.square();
    let c = r.z.square();
    let e = <ark_bn254::Parameters as ark_ec::models::bn::BnParameters>::G2Parameters::COEFF_B
        * (c.double() + c);
    let f = e.double() + e;
    let j = r.x.square();
    r.x = a * (b - f);

    // step 1
    let mut g = b + f;
    g.mul_assign_by_fp(&two_inv);
    let h = (r.y + r.z).square() - (b + c);
    let i = e - b;
    let e_square = e.square();
    r.y = g.square() - (e_square.double() + e_square);
    r.z = b * h;

    parse_r_to_bytes(r, r_bytes);

    // step 2
    let j_d = j.double() + j;
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
    Ok(())
}

// https://docs.rs/ark-ec/0.3.0/src/ark_ec/models/bn/g2.rs.html#168-191
pub fn addition_step<B: BnParameters>(
    coeff_0_range: &mut Vec<u8>,
    coeff_1_range: &mut Vec<u8>,
    coeff_2_range: &mut Vec<u8>,
    r_bytes: &mut Vec<u8>,
    proof_bytes: &Vec<u8>,
    computation_flag: &str,
) -> Result<(), ProgramError> {
    let mut q = parse_proof_b_from_bytes(proof_bytes);

    let twist_mul_by_q_x = ark_bn254::Parameters::TWIST_MUL_BY_Q_X;

    let twist_mul_by_q_y = ark_bn254::Parameters::TWIST_MUL_BY_Q_Y;

    if computation_flag == "normal" {
    } else if computation_flag == "negq" {
        q = -q;
    } else if computation_flag == "q1" {
        q.x.frobenius_map(1);
        q.x *= &twist_mul_by_q_x;
        q.y.frobenius_map(1);
        q.y *= &twist_mul_by_q_y;
    } else if computation_flag == "q2" {
        q.x.frobenius_map(1);
        q.x *= &twist_mul_by_q_x;
        q.y.frobenius_map(1);
        q.y *= &twist_mul_by_q_y;
        q.x.frobenius_map(1);
        q.x *= &twist_mul_by_q_x;
        q.y.frobenius_map(1);
        q.y *= &twist_mul_by_q_y;
        q.y = -q.y;
    }

    // step 0
    // Formula for line function when working with
    // homogeneous projective coordinates.
    let mut r = parse_r_from_bytes(r_bytes);

    let theta = r.y - (q.y * r.z);
    let lambda = r.x - (q.x * r.z);
    let c = theta.square();
    let d = lambda.square();
    let e = lambda * d;
    let f = r.z * c;
    let g = r.x * d;
    let h = e + f - g.double();

    // step 1
    r.x = lambda * h;
    r.y = theta * (g - h) - (e * r.y);
    r.z *= &e;
    parse_r_to_bytes(r, r_bytes);

    // step 2
    let j = theta * q.x - (lambda * q.y);

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
    Ok(())
}

pub fn init_coeffs1(r_range: &mut Vec<u8>, proof_range: &mut Vec<u8>) -> Result<(), ProgramError> {
    let proof_b = parse_proof_b_from_bytes(proof_range);
    let r: ark_ec::models::bn::g2::G2HomProjective<ark_bn254::Parameters> =
        ark_ec::models::bn::g2::G2HomProjective {
            x: proof_b.x,
            y: proof_b.y,
            z: Fp2::one(),
        };
    parse_r_to_bytes(r, r_range);
    Ok(())
}

pub fn square_in_place_instruction(f_range: &mut Vec<u8>) -> Result<(), ProgramError> {
    let f = parse_f_from_bytes(f_range);

    let mut v0 = f.c0 - f.c1;
    let v3 = <ark_ff::fields::models::fp12_2over3over2::Fp12ParamsWrapper<
        ark_bn254::Fq12Parameters,
        > as QuadExtParameters>::sub_and_mul_base_field_by_nonresidue(&f.c0, &f.c1);
    let v2 = f.c0 * f.c1;
    v0 *= &v3;
    let c1 = v2.double();
    let c0 = <ark_ff::fields::models::fp12_2over3over2::Fp12ParamsWrapper<
        ark_bn254::Fq12Parameters,
    > as QuadExtParameters>::add_and_mul_base_field_by_nonresidue_plus_one(&v0, &v2);
    parse_cubic_to_bytes_sub(c0, f_range, C0_SUB_RANGE);
    parse_cubic_to_bytes_sub(c1, f_range, C1_SUB_RANGE);
    Ok(())
}

pub fn ell_instruction_d(
    // used for coeff1 but can be general as well
    f_range: &mut Vec<u8>,
    coeff_0_range: &Vec<u8>,
    coeff_1_range: &Vec<u8>,
    coeff_2_range: &Vec<u8>,
    p_y_range: &Vec<u8>,
    p_x_range: &Vec<u8>,
) -> Result<(), ProgramError> {
    let coeff_2 = parse_quad_from_bytes(coeff_2_range);
    let mut coeff_1 = parse_quad_from_bytes(coeff_1_range);
    let mut coeff_0 = parse_quad_from_bytes(coeff_0_range);
    let p_y = parse_fp256_from_bytes(p_y_range);
    let p_x = parse_fp256_from_bytes(p_x_range);

    coeff_0.mul_assign_by_fp(&p_y);
    coeff_1.mul_assign_by_fp(&p_x);

    let c0 = parse_cubic_from_bytes_sub(f_range, C0_SUB_RANGE);
    let a0 = c0.c0 * coeff_0;
    let a1 = c0.c1 * coeff_0;
    let a2 = c0.c2 * coeff_0;
    let a = Fp6::new(a0, a1, a2);
    let c1 = parse_cubic_from_bytes_sub(f_range, C1_SUB_RANGE);
    let mut b = c1;
    b.mul_by_01(&coeff_1, &coeff_2);

    let c00 = coeff_0 + coeff_1; //c0 = *c0 + c3
    let mut e = c0 + c1;
    e.mul_by_01(&c00, &coeff_2);

    let mut f =
        <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::one();
    f.c1 = e - (a + b);
    f.c0 = a + <ark_bn254::fq12::Fq12Parameters as Fp12Parameters>::mul_fp6_by_nonresidue(&b);
    parse_f_to_bytes(f, f_range);
    Ok(())
}

pub fn ell_instruction_d_c2(
    f_range: &mut Vec<u8>,
    p_y_range: &Vec<u8>,
    p_x_range: &Vec<u8>,
    current_coeff_2_range: &mut Vec<u8>,
) -> Result<(), ProgramError> {
    let id = current_coeff_2_range[0];

    let mut coeff: (
        QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
        QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
        QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
    ) = (
        QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::zero(),
        QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::zero(),
        QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::zero(),
    );
    // Reads from hardcoded verifying key.
    if id == 0 {
        coeff = get_gamma_g2_neg_pc_0();
    } else if id == 1 {
        coeff = get_gamma_g2_neg_pc_1();
    } else if id == 2 {
        coeff = get_gamma_g2_neg_pc_2();
    } else if id == 3 {
        coeff = get_gamma_g2_neg_pc_3();
    } else if id == 4 {
        coeff = get_gamma_g2_neg_pc_4();
    } else if id == 5 {
        coeff = get_gamma_g2_neg_pc_5();
    } else if id == 6 {
        coeff = get_gamma_g2_neg_pc_6();
    } else if id == 7 {
        coeff = get_gamma_g2_neg_pc_7();
    } else if id == 8 {
        coeff = get_gamma_g2_neg_pc_8();
    } else if id == 9 {
        coeff = get_gamma_g2_neg_pc_9();
    } else if id == 10 {
        coeff = get_gamma_g2_neg_pc_10();
    } else if id == 11 {
        coeff = get_gamma_g2_neg_pc_11();
    } else if id == 12 {
        coeff = get_gamma_g2_neg_pc_12();
    } else if id == 13 {
        coeff = get_gamma_g2_neg_pc_13();
    } else if id == 14 {
        coeff = get_gamma_g2_neg_pc_14();
    } else if id == 15 {
        coeff = get_gamma_g2_neg_pc_15();
    } else if id == 16 {
        coeff = get_gamma_g2_neg_pc_16();
    } else if id == 17 {
        coeff = get_gamma_g2_neg_pc_17();
    } else if id == 18 {
        coeff = get_gamma_g2_neg_pc_18();
    } else if id == 19 {
        coeff = get_gamma_g2_neg_pc_19();
    } else if id == 20 {
        coeff = get_gamma_g2_neg_pc_20();
    } else if id == 21 {
        coeff = get_gamma_g2_neg_pc_21();
    } else if id == 22 {
        coeff = get_gamma_g2_neg_pc_22();
    } else if id == 23 {
        coeff = get_gamma_g2_neg_pc_23();
    } else if id == 24 {
        coeff = get_gamma_g2_neg_pc_24();
    } else if id == 25 {
        coeff = get_gamma_g2_neg_pc_25();
    } else if id == 26 {
        coeff = get_gamma_g2_neg_pc_26();
    } else if id == 27 {
        coeff = get_gamma_g2_neg_pc_27();
    } else if id == 28 {
        coeff = get_gamma_g2_neg_pc_28();
    } else if id == 29 {
        coeff = get_gamma_g2_neg_pc_29();
    } else if id == 30 {
        coeff = get_gamma_g2_neg_pc_30();
    } else if id == 31 {
        coeff = get_gamma_g2_neg_pc_31();
    } else if id == 32 {
        coeff = get_gamma_g2_neg_pc_32();
    } else if id == 33 {
        coeff = get_gamma_g2_neg_pc_33();
    } else if id == 34 {
        coeff = get_gamma_g2_neg_pc_34();
    } else if id == 35 {
        coeff = get_gamma_g2_neg_pc_35();
    } else if id == 36 {
        coeff = get_gamma_g2_neg_pc_36();
    } else if id == 37 {
        coeff = get_gamma_g2_neg_pc_37();
    } else if id == 38 {
        coeff = get_gamma_g2_neg_pc_38();
    } else if id == 39 {
        coeff = get_gamma_g2_neg_pc_39();
    } else if id == 40 {
        coeff = get_gamma_g2_neg_pc_40();
    } else if id == 41 {
        coeff = get_gamma_g2_neg_pc_41();
    } else if id == 42 {
        coeff = get_gamma_g2_neg_pc_42();
    } else if id == 43 {
        coeff = get_gamma_g2_neg_pc_43();
    } else if id == 44 {
        coeff = get_gamma_g2_neg_pc_44();
    } else if id == 45 {
        coeff = get_gamma_g2_neg_pc_45();
    } else if id == 46 {
        coeff = get_gamma_g2_neg_pc_46();
    } else if id == 47 {
        coeff = get_gamma_g2_neg_pc_47();
    } else if id == 48 {
        coeff = get_gamma_g2_neg_pc_48();
    } else if id == 49 {
        coeff = get_gamma_g2_neg_pc_49();
    } else if id == 50 {
        coeff = get_gamma_g2_neg_pc_50();
    } else if id == 51 {
        coeff = get_gamma_g2_neg_pc_51();
    } else if id == 52 {
        coeff = get_gamma_g2_neg_pc_52();
    } else if id == 53 {
        coeff = get_gamma_g2_neg_pc_53();
    } else if id == 54 {
        coeff = get_gamma_g2_neg_pc_54();
    } else if id == 55 {
        coeff = get_gamma_g2_neg_pc_55();
    } else if id == 56 {
        coeff = get_gamma_g2_neg_pc_56();
    } else if id == 57 {
        coeff = get_gamma_g2_neg_pc_57();
    } else if id == 58 {
        coeff = get_gamma_g2_neg_pc_58();
    } else if id == 59 {
        coeff = get_gamma_g2_neg_pc_59();
    } else if id == 60 {
        coeff = get_gamma_g2_neg_pc_60();
    } else if id == 61 {
        coeff = get_gamma_g2_neg_pc_61();
    } else if id == 62 {
        coeff = get_gamma_g2_neg_pc_62();
    } else if id == 63 {
        coeff = get_gamma_g2_neg_pc_63();
    } else if id == 64 {
        coeff = get_gamma_g2_neg_pc_64();
    } else if id == 65 {
        coeff = get_gamma_g2_neg_pc_65();
    } else if id == 66 {
        coeff = get_gamma_g2_neg_pc_66();
    } else if id == 67 {
        coeff = get_gamma_g2_neg_pc_67();
    } else if id == 68 {
        coeff = get_gamma_g2_neg_pc_68();
    } else if id == 69 {
        coeff = get_gamma_g2_neg_pc_69();
    } else if id == 70 {
        coeff = get_gamma_g2_neg_pc_70();
    } else if id == 71 {
        coeff = get_gamma_g2_neg_pc_71();
    } else if id == 72 {
        coeff = get_gamma_g2_neg_pc_72();
    } else if id == 73 {
        coeff = get_gamma_g2_neg_pc_73();
    } else if id == 74 {
        coeff = get_gamma_g2_neg_pc_74();
    } else if id == 75 {
        coeff = get_gamma_g2_neg_pc_75();
    } else if id == 76 {
        coeff = get_gamma_g2_neg_pc_76();
    } else if id == 77 {
        coeff = get_gamma_g2_neg_pc_77();
    } else if id == 78 {
        coeff = get_gamma_g2_neg_pc_78();
    } else if id == 79 {
        coeff = get_gamma_g2_neg_pc_79();
    } else if id == 80 {
        coeff = get_gamma_g2_neg_pc_80();
    } else if id == 81 {
        coeff = get_gamma_g2_neg_pc_81();
    } else if id == 82 {
        coeff = get_gamma_g2_neg_pc_82();
    } else if id == 83 {
        coeff = get_gamma_g2_neg_pc_83();
    } else if id == 84 {
        coeff = get_gamma_g2_neg_pc_84();
    } else if id == 85 {
        coeff = get_gamma_g2_neg_pc_85();
    } else if id == 86 {
        coeff = get_gamma_g2_neg_pc_86();
    } else if id == 87 {
        coeff = get_gamma_g2_neg_pc_87();
    } else if id == 88 {
        coeff = get_gamma_g2_neg_pc_88();
    } else if id == 89 {
        coeff = get_gamma_g2_neg_pc_89();
    } else if id == 90 {
        coeff = get_gamma_g2_neg_pc_90();
    } else {
        msg!("ERR: coeff uninitialized value");
    }
    if id == 90 {
        // set to 0
        current_coeff_2_range[0] = 0;
    } else {
        // +=1
        current_coeff_2_range[0] += 1;
    }

    let coeff_2 = coeff.2;
    let mut coeff_1 = coeff.1;
    let mut coeff_0 = coeff.0;
    let p_y = parse_fp256_from_bytes(p_y_range);
    let p_x = parse_fp256_from_bytes(p_x_range);

    coeff_0.mul_assign_by_fp(&p_y);
    coeff_1.mul_assign_by_fp(&p_x);

    let c0 = parse_cubic_from_bytes_sub(f_range, C0_SUB_RANGE);
    let a0 = c0.c0 * coeff_0;
    let a1 = c0.c1 * coeff_0;
    let a2 = c0.c2 * coeff_0;
    let a = Fp6::new(a0, a1, a2);

    let c1 = parse_cubic_from_bytes_sub(f_range, C1_SUB_RANGE);
    let mut b = c1;
    b.mul_by_01(&coeff_1, &coeff_2);

    let c00 = coeff_0 + coeff_1; //c0 = *c0 + c3
    let mut e = c0 + c1;
    e.mul_by_01(&c00, &coeff_2);

    let mut f =
        <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::one();

    f.c1 = e - (a + b);
    f.c0 = a + <ark_bn254::fq12::Fq12Parameters as Fp12Parameters>::mul_fp6_by_nonresidue(&b);

    parse_f_to_bytes(f, f_range);
    Ok(())
}

pub fn ell_instruction_d_c3(
    f_range: &mut Vec<u8>,
    p_y_range: &Vec<u8>,
    p_x_range: &Vec<u8>,
    current_coeff_3_range: &mut Vec<u8>,
) -> Result<(), ProgramError> {
    let id = current_coeff_3_range[0];
    let mut coeff: (
        QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
        QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
        QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
    ) = (
        QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::zero(),
        QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::zero(),
        QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::zero(),
    );
    // Reads from hardcoded verifying key.
    if id == 0 {
        coeff = get_delta_g2_neg_pc_0();
    } else if id == 1 {
        coeff = get_delta_g2_neg_pc_1();
    } else if id == 2 {
        coeff = get_delta_g2_neg_pc_2();
    } else if id == 3 {
        coeff = get_delta_g2_neg_pc_3();
    } else if id == 4 {
        coeff = get_delta_g2_neg_pc_4();
    } else if id == 5 {
        coeff = get_delta_g2_neg_pc_5();
    } else if id == 6 {
        coeff = get_delta_g2_neg_pc_6();
    } else if id == 7 {
        coeff = get_delta_g2_neg_pc_7();
    } else if id == 8 {
        coeff = get_delta_g2_neg_pc_8();
    } else if id == 9 {
        coeff = get_delta_g2_neg_pc_9();
    } else if id == 10 {
        coeff = get_delta_g2_neg_pc_10();
    } else if id == 11 {
        coeff = get_delta_g2_neg_pc_11();
    } else if id == 12 {
        coeff = get_delta_g2_neg_pc_12();
    } else if id == 13 {
        coeff = get_delta_g2_neg_pc_13();
    } else if id == 14 {
        coeff = get_delta_g2_neg_pc_14();
    } else if id == 15 {
        coeff = get_delta_g2_neg_pc_15();
    } else if id == 16 {
        coeff = get_delta_g2_neg_pc_16();
    } else if id == 17 {
        coeff = get_delta_g2_neg_pc_17();
    } else if id == 18 {
        coeff = get_delta_g2_neg_pc_18();
    } else if id == 19 {
        coeff = get_delta_g2_neg_pc_19();
    } else if id == 20 {
        coeff = get_delta_g2_neg_pc_20();
    } else if id == 21 {
        coeff = get_delta_g2_neg_pc_21();
    } else if id == 22 {
        coeff = get_delta_g2_neg_pc_22();
    } else if id == 23 {
        coeff = get_delta_g2_neg_pc_23();
    } else if id == 24 {
        coeff = get_delta_g2_neg_pc_24();
    } else if id == 25 {
        coeff = get_delta_g2_neg_pc_25();
    } else if id == 26 {
        coeff = get_delta_g2_neg_pc_26();
    } else if id == 27 {
        coeff = get_delta_g2_neg_pc_27();
    } else if id == 28 {
        coeff = get_delta_g2_neg_pc_28();
    } else if id == 29 {
        coeff = get_delta_g2_neg_pc_29();
    } else if id == 30 {
        coeff = get_delta_g2_neg_pc_30();
    } else if id == 31 {
        coeff = get_delta_g2_neg_pc_31();
    } else if id == 32 {
        coeff = get_delta_g2_neg_pc_32();
    } else if id == 33 {
        coeff = get_delta_g2_neg_pc_33();
    } else if id == 34 {
        coeff = get_delta_g2_neg_pc_34();
    } else if id == 35 {
        coeff = get_delta_g2_neg_pc_35();
    } else if id == 36 {
        coeff = get_delta_g2_neg_pc_36();
    } else if id == 37 {
        coeff = get_delta_g2_neg_pc_37();
    } else if id == 38 {
        coeff = get_delta_g2_neg_pc_38();
    } else if id == 39 {
        coeff = get_delta_g2_neg_pc_39();
    } else if id == 40 {
        coeff = get_delta_g2_neg_pc_40();
    } else if id == 41 {
        coeff = get_delta_g2_neg_pc_41();
    } else if id == 42 {
        coeff = get_delta_g2_neg_pc_42();
    } else if id == 43 {
        coeff = get_delta_g2_neg_pc_43();
    } else if id == 44 {
        coeff = get_delta_g2_neg_pc_44();
    } else if id == 45 {
        coeff = get_delta_g2_neg_pc_45();
    } else if id == 46 {
        coeff = get_delta_g2_neg_pc_46();
    } else if id == 47 {
        coeff = get_delta_g2_neg_pc_47();
    } else if id == 48 {
        coeff = get_delta_g2_neg_pc_48();
    } else if id == 49 {
        coeff = get_delta_g2_neg_pc_49();
    } else if id == 50 {
        coeff = get_delta_g2_neg_pc_50();
    } else if id == 51 {
        coeff = get_delta_g2_neg_pc_51();
    } else if id == 52 {
        coeff = get_delta_g2_neg_pc_52();
    } else if id == 53 {
        coeff = get_delta_g2_neg_pc_53();
    } else if id == 54 {
        coeff = get_delta_g2_neg_pc_54();
    } else if id == 55 {
        coeff = get_delta_g2_neg_pc_55();
    } else if id == 56 {
        coeff = get_delta_g2_neg_pc_56();
    } else if id == 57 {
        coeff = get_delta_g2_neg_pc_57();
    } else if id == 58 {
        coeff = get_delta_g2_neg_pc_58();
    } else if id == 59 {
        coeff = get_delta_g2_neg_pc_59();
    } else if id == 60 {
        coeff = get_delta_g2_neg_pc_60();
    } else if id == 61 {
        coeff = get_delta_g2_neg_pc_61();
    } else if id == 62 {
        coeff = get_delta_g2_neg_pc_62();
    } else if id == 63 {
        coeff = get_delta_g2_neg_pc_63();
    } else if id == 64 {
        coeff = get_delta_g2_neg_pc_64();
    } else if id == 65 {
        coeff = get_delta_g2_neg_pc_65();
    } else if id == 66 {
        coeff = get_delta_g2_neg_pc_66();
    } else if id == 67 {
        coeff = get_delta_g2_neg_pc_67();
    } else if id == 68 {
        coeff = get_delta_g2_neg_pc_68();
    } else if id == 69 {
        coeff = get_delta_g2_neg_pc_69();
    } else if id == 70 {
        coeff = get_delta_g2_neg_pc_70();
    } else if id == 71 {
        coeff = get_delta_g2_neg_pc_71();
    } else if id == 72 {
        coeff = get_delta_g2_neg_pc_72();
    } else if id == 73 {
        coeff = get_delta_g2_neg_pc_73();
    } else if id == 74 {
        coeff = get_delta_g2_neg_pc_74();
    } else if id == 75 {
        coeff = get_delta_g2_neg_pc_75();
    } else if id == 76 {
        coeff = get_delta_g2_neg_pc_76();
    } else if id == 77 {
        coeff = get_delta_g2_neg_pc_77();
    } else if id == 78 {
        coeff = get_delta_g2_neg_pc_78();
    } else if id == 79 {
        coeff = get_delta_g2_neg_pc_79();
    } else if id == 80 {
        coeff = get_delta_g2_neg_pc_80();
    } else if id == 81 {
        coeff = get_delta_g2_neg_pc_81();
    } else if id == 82 {
        coeff = get_delta_g2_neg_pc_82();
    } else if id == 83 {
        coeff = get_delta_g2_neg_pc_83();
    } else if id == 84 {
        coeff = get_delta_g2_neg_pc_84();
    } else if id == 85 {
        coeff = get_delta_g2_neg_pc_85();
    } else if id == 86 {
        coeff = get_delta_g2_neg_pc_86();
    } else if id == 87 {
        coeff = get_delta_g2_neg_pc_87();
    } else if id == 88 {
        coeff = get_delta_g2_neg_pc_88();
    } else if id == 89 {
        coeff = get_delta_g2_neg_pc_89();
    } else if id == 90 {
        coeff = get_delta_g2_neg_pc_90();
    } else {
        msg!("ERR: coeff uninitialized value");
    }
    if id == 90 {
        current_coeff_3_range[0] = 0;
    } else {
        current_coeff_3_range[0] += 1;
    }
    let coeff_2 = coeff.2;
    let mut coeff_1 = coeff.1;
    let mut coeff_0 = coeff.0;
    let p_y = parse_fp256_from_bytes(p_y_range);
    let p_x = parse_fp256_from_bytes(p_x_range);

    coeff_0.mul_assign_by_fp(&p_y);
    coeff_1.mul_assign_by_fp(&p_x);

    let c0 = parse_cubic_from_bytes_sub(f_range, C0_SUB_RANGE);
    let a0 = c0.c0 * coeff_0;
    let a1 = c0.c1 * coeff_0;
    let a2 = c0.c2 * coeff_0;
    let a = Fp6::new(a0, a1, a2);
    let c1 = parse_cubic_from_bytes_sub(f_range, C1_SUB_RANGE);
    let mut b = c1;
    b.mul_by_01(&coeff_1, &coeff_2);

    let c00 = coeff_0 + coeff_1; //c0 = *c0 + c3
    let mut e = c0 + c1;
    e.mul_by_01(&c00, &coeff_2);

    let mut f =
        <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::one();

    f.c1 = e - (a + b);
    f.c0 = a + <ark_bn254::fq12::Fq12Parameters as Fp12Parameters>::mul_fp6_by_nonresidue(&b);

    parse_f_to_bytes(f, f_range);
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::groth16_verifier::miller_loop::instructions::{
        addition_step, doubling_step, ell_instruction_d, ell_instruction_d_c2,
        ell_instruction_d_c3, square_in_place_instruction,
    };
    use crate::groth16_verifier::parsers::{
        parse_f_from_bytes, parse_f_to_bytes, parse_fp256_to_bytes, parse_proof_b_to_bytes,
        parse_quad_to_bytes, parse_r_to_bytes,
    };
    use crate::utils::prepared_verifying_key::{get_delta_g2_neg_pc_0, get_gamma_g2_neg_pc_0};
    use ark_ff::fields::models::fp2::Fp2Parameters;
    use ark_ff::Field;
    use ark_std::{test_rng, One, UniformRand};

    #[test]
    fn doubling_step_should_succeed() {
        let mut rng = test_rng();
        for _i in 0..10 {
            let reference_coeff_2 =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let reference_coeff_1 =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let reference_coeff_0 =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let mut reference_r =
            ark_ec::models::bn::g2::G2HomProjective::<ark_bn254::Parameters> {
                x: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                ),
                y: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                ),
                z: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                ),
            };

            let test_coeff_2 = reference_coeff_2.clone();
            let test_coeff_1 = reference_coeff_1.clone();
            let test_coeff_0 = reference_coeff_0.clone();
            let test_r = reference_r.clone();

            //simulating the onchain account
            let mut account_coeff_2_range = vec![0u8; 64];
            let mut account_coeff_1_range = vec![0u8; 64];
            let mut account_coeff_0_range = vec![0u8; 64];
            let mut account_r_range = vec![0u8; 192];

            parse_quad_to_bytes(test_coeff_2, &mut account_coeff_2_range);
            parse_quad_to_bytes(test_coeff_1, &mut account_coeff_1_range);
            parse_quad_to_bytes(test_coeff_0, &mut account_coeff_0_range);
            parse_r_to_bytes(test_r, &mut account_r_range);

            // test instruction, mut accs
            doubling_step(
                &mut account_r_range,
                &mut account_coeff_0_range,
                &mut account_coeff_1_range,
                &mut account_coeff_2_range,
            )
            .unwrap();
            // reference value
            let two_inv = <ark_bn254::Fq2Parameters as Fp2Parameters>::Fp::one()
                .double()
                .inverse()
                .unwrap();
            ark_ec::models::bn::g2::doubling_step(&mut reference_r, &two_inv);

            let mut ref_r_range = vec![0u8; 192]; // ell mutates f in the end. So we can just check f.
            parse_r_to_bytes(reference_r, &mut ref_r_range);
            assert_eq!(ref_r_range, account_r_range);
        }
    }

    #[test]
    fn doubling_step_should_fail() {
        let mut rng = test_rng();
        for _i in 0..10 {
            let reference_coeff_2 =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let reference_coeff_1 =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let reference_coeff_0 =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let mut reference_r =
            ark_ec::models::bn::g2::G2HomProjective::<ark_bn254::Parameters> {
                x: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                ),
                y: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                ),
                z: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                ),
            };

            let test_coeff_2 = reference_coeff_2.clone();
            let test_coeff_1 = reference_coeff_1.clone();
            let test_coeff_0 = reference_coeff_0.clone();
            // failing here
            let test_r =  ark_ec::models::bn::g2::G2HomProjective::<ark_bn254::Parameters> {
                x: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                ),
                y: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                ),
                z: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                ),
            };

            //simulating the onchain account
            let mut account_coeff_2_range = vec![0u8; 64];
            let mut account_coeff_1_range = vec![0u8; 64];
            let mut account_coeff_0_range = vec![0u8; 64];
            let mut account_r_range = vec![0u8; 192];

            parse_quad_to_bytes(test_coeff_2, &mut account_coeff_2_range);
            parse_quad_to_bytes(test_coeff_1, &mut account_coeff_1_range);
            parse_quad_to_bytes(test_coeff_0, &mut account_coeff_0_range);
            parse_r_to_bytes(test_r, &mut account_r_range);

            // test instruction, mut accs
            doubling_step(
                &mut account_r_range,
                &mut account_coeff_0_range,
                &mut account_coeff_1_range,
                &mut account_coeff_2_range,
            )
            .unwrap();
            // reference value
            let two_inv = <ark_bn254::Fq2Parameters as Fp2Parameters>::Fp::one()
                .double()
                .inverse()
                .unwrap();
            ark_ec::models::bn::g2::doubling_step(&mut reference_r, &two_inv);

            let mut ref_r_range = vec![0u8; 192]; // ell mutates f in the end. So we can just check f.
            parse_r_to_bytes(reference_r, &mut ref_r_range);
            assert!(ref_r_range != account_r_range);
        }
    }

    #[test]
    fn addition_step_should_succeed() {
        let mut rng = test_rng();
        for _i in 0..10 {
            let x =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let y =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let reference_proof_b =
                ark_ec::bn::g2::G2Affine::<ark_bn254::Parameters>::new(x, y, false);
            let reference_coeff_2 =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let reference_coeff_1 =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let reference_coeff_0 =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let mut reference_r =
            ark_ec::models::bn::g2::G2HomProjective::<ark_bn254::Parameters> {
                x: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                ),
                y: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                ),
                z: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                ),
            };

            let test_proof_b = reference_proof_b.clone();
            let test_coeff_2 = reference_coeff_2.clone();
            let test_coeff_1 = reference_coeff_1.clone();
            let test_coeff_0 = reference_coeff_0.clone();
            let test_r = reference_r.clone();

            //simulating the onchain account
            let mut account_proof_b_range = vec![0u8; 128];
            let mut account_coeff_2_range = vec![0u8; 64];
            let mut account_coeff_1_range = vec![0u8; 64];
            let mut account_coeff_0_range = vec![0u8; 64];
            let mut account_r_range = vec![0u8; 192];

            parse_proof_b_to_bytes(test_proof_b, &mut account_proof_b_range);
            parse_quad_to_bytes(test_coeff_2, &mut account_coeff_2_range);
            parse_quad_to_bytes(test_coeff_1, &mut account_coeff_1_range);
            parse_quad_to_bytes(test_coeff_0, &mut account_coeff_0_range);
            parse_r_to_bytes(test_r, &mut account_r_range);

            // test instruction, mut accs
            addition_step::<ark_bn254::Parameters>(
                &mut account_coeff_0_range,
                &mut account_coeff_1_range,
                &mut account_coeff_2_range,
                &mut account_r_range,
                &account_proof_b_range,
                "normal",
            )
            .unwrap();

            // reference value
            ark_ec::models::bn::g2::addition_step(&mut reference_r, &reference_proof_b);

            let mut ref_r_range = vec![0u8; 192]; // ell mutates f in the end. So we can just check f.
            parse_r_to_bytes(reference_r, &mut ref_r_range);
            assert_eq!(ref_r_range, account_r_range);
        }
    }

    #[test]
    fn addition_step_should_fail() {
        let mut rng = test_rng();
        for _i in 0..10 {
            let x =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let y =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let reference_proof_b =
                ark_ec::bn::g2::G2Affine::<ark_bn254::Parameters>::new(x, y, false);

            let reference_coeff_2 =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let reference_coeff_1 =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let reference_coeff_0 =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let mut reference_r =
            ark_ec::models::bn::g2::G2HomProjective::<ark_bn254::Parameters> {
                x: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                ),
                y: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                ),
                z: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                ),
            };

            let test_proof_b = reference_proof_b.clone();
            let test_coeff_2 = reference_coeff_2.clone();
            let test_coeff_1 = reference_coeff_1.clone();
            let test_coeff_0 = reference_coeff_0.clone();
            // failing here
            let test_r =  ark_ec::models::bn::g2::G2HomProjective::<ark_bn254::Parameters> {
                x: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                ),
                y: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                ),
                z: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                ),
            };

            //simulating the onchain account
            let mut account_proof_b_range = vec![0u8; 128];
            let mut account_coeff_2_range = vec![0u8; 64];
            let mut account_coeff_1_range = vec![0u8; 64];
            let mut account_coeff_0_range = vec![0u8; 64];
            let mut account_r_range = vec![0u8; 192];

            parse_proof_b_to_bytes(test_proof_b, &mut account_proof_b_range);
            parse_quad_to_bytes(test_coeff_2, &mut account_coeff_2_range);
            parse_quad_to_bytes(test_coeff_1, &mut account_coeff_1_range);
            parse_quad_to_bytes(test_coeff_0, &mut account_coeff_0_range);
            parse_r_to_bytes(test_r, &mut account_r_range);

            // test instruction, mut accs, using wrong r
            addition_step::<ark_bn254::Parameters>(
                &mut account_coeff_0_range,
                &mut account_coeff_1_range,
                &mut account_coeff_2_range,
                &mut account_r_range,
                &account_proof_b_range,
                "normal",
            )
            .unwrap();

            // reference value
            ark_ec::models::bn::g2::addition_step(&mut reference_r, &reference_proof_b);

            let mut ref_r_range = vec![0u8; 192]; // ell mutates f in the end. So we can just check f.
            parse_r_to_bytes(reference_r, &mut ref_r_range);
            assert!(ref_r_range != account_r_range);
        }
    }
    #[test]
    fn ell_instruction_d_test_should_succeed() {
        //generating input
        let mut rng = test_rng();
        for _i in 0..10 {
            let mut reference_f =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                    &mut rng,
                );
            let reference_coeff_2 =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let reference_coeff_1 =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let reference_coeff_0 =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let reference_p_y =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                );
            let reference_p_x =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                );

            let test_f = reference_f.clone();
            let test_coeff_2 = reference_coeff_2.clone();
            let test_coeff_1 = reference_coeff_1.clone();
            let test_coeff_0 = reference_coeff_0.clone();
            let test_p_y = reference_p_y.clone();
            let test_p_x = reference_p_x.clone();

            //simulating the onchain account
            let mut account_f_range = vec![0u8; 384];
            let mut account_coeff_2_range = vec![0u8; 64];
            let mut account_coeff_1_range = vec![0u8; 64];
            let mut account_coeff_0_range = vec![0u8; 64];
            let mut account_p_y_range = vec![0u8; 32];
            let mut account_p_x_range = vec![0u8; 32];

            parse_f_to_bytes(test_f, &mut account_f_range);
            parse_quad_to_bytes(test_coeff_2, &mut account_coeff_2_range);
            parse_quad_to_bytes(test_coeff_1, &mut account_coeff_1_range);
            parse_quad_to_bytes(test_coeff_0, &mut account_coeff_0_range);
            parse_fp256_to_bytes(test_p_y, &mut account_p_y_range);
            parse_fp256_to_bytes(test_p_x, &mut account_p_x_range);

            // test instruction, mut accs
            ell_instruction_d(
                &mut account_f_range,
                &mut account_coeff_0_range,
                &mut account_coeff_1_range,
                &mut account_coeff_2_range,
                &mut account_p_y_range,
                &mut account_p_x_range,
            )
            .unwrap();
            // reference value for comparison
            <ark_ec::models::bn::Bn<ark_bn254::Parameters>>::ell(
                &mut reference_f,
                &(reference_coeff_0, reference_coeff_1, reference_coeff_2),
                &ark_ec::short_weierstrass_jacobian::GroupAffine::<ark_bn254::g1::Parameters>::new(
                    reference_p_x,
                    reference_p_y,
                    false,
                ),
            );

            // ell mutates f in the end. So we can just check f.
            assert_eq!(reference_f, parse_f_from_bytes(&account_f_range));
        }
    }

    #[test]
    fn ell_instruction_d_test_should_fail() {
        //generating input
        let mut rng = test_rng();
        for _i in 0..10 {
            let mut reference_f =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                    &mut rng,
                );
            let reference_coeff_2 =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let reference_coeff_1 =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let reference_coeff_0 =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let reference_p_y =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                );
            let reference_p_x =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                );

            let test_f = reference_f.clone();
            let test_coeff_2 = reference_coeff_2.clone();
            let test_coeff_1 = reference_coeff_1.clone();
            // failing here:
            let test_coeff_0 =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let test_p_y = reference_p_y.clone();
            let test_p_x = reference_p_x.clone();

            //simulating the onchain account
            let mut account_f_range = vec![0u8; 384];
            let mut account_coeff_2_range = vec![0u8; 64];
            let mut account_coeff_1_range = vec![0u8; 64];
            let mut account_coeff_0_range = vec![0u8; 64];
            let mut account_p_y_range = vec![0u8; 32];
            let mut account_p_x_range = vec![0u8; 32];

            parse_f_to_bytes(test_f, &mut account_f_range);
            parse_quad_to_bytes(test_coeff_2, &mut account_coeff_2_range);
            parse_quad_to_bytes(test_coeff_1, &mut account_coeff_1_range);
            parse_quad_to_bytes(test_coeff_0, &mut account_coeff_0_range);
            parse_fp256_to_bytes(test_p_y, &mut account_p_y_range);
            parse_fp256_to_bytes(test_p_x, &mut account_p_x_range);

            // test instruction, mut accs
            ell_instruction_d(
                &mut account_f_range,
                &mut account_coeff_0_range,
                &mut account_coeff_1_range,
                &mut account_coeff_2_range,
                &mut account_p_y_range,
                &mut account_p_x_range,
            )
            .unwrap();
            // reference value for comparison
            <ark_ec::models::bn::Bn<ark_bn254::Parameters>>::ell(
                &mut reference_f,
                &(reference_coeff_0, reference_coeff_1, reference_coeff_2),
                &ark_ec::short_weierstrass_jacobian::GroupAffine::<ark_bn254::g1::Parameters>::new(
                    reference_p_x,
                    reference_p_y,
                    false,
                ),
            );

            // ell mutates f in the end. So we can just check f.
            assert!(reference_f != parse_f_from_bytes(&account_f_range));
        }
    }

    #[test]
    fn ell_instruction_d_c2_test_should_succeed() {
        //generating input
        let mut rng = test_rng();
        for i in 0..1 {
            let mut reference_f =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                    &mut rng,
                );
            let current_coeff_2 = i.clone();
            let reference_coeff_0 = get_gamma_g2_neg_pc_0().0;
            let reference_coeff_1 = get_gamma_g2_neg_pc_0().1;
            let reference_coeff_2 = get_gamma_g2_neg_pc_0().2;

            let reference_p_y =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                );
            let reference_p_x =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                );

            let test_f = reference_f.clone();
            let test_p_y = reference_p_y.clone();
            let test_p_x = reference_p_x.clone();

            //simulating the onchain account
            let mut account_f_range = vec![0u8; 384];
            let mut account_current_coeff_2_range = vec![current_coeff_2 as u8; 1];
            let mut account_p_y_range = vec![0u8; 32];
            let mut account_p_x_range = vec![0u8; 32];

            // read coeffs from hardcoded vk here
            parse_f_to_bytes(test_f, &mut account_f_range);
            parse_fp256_to_bytes(test_p_y, &mut account_p_y_range);
            parse_fp256_to_bytes(test_p_x, &mut account_p_x_range);

            // test instruction, mut accs
            ell_instruction_d_c2(
                &mut account_f_range,
                &mut account_p_y_range,
                &mut account_p_x_range,
                &mut account_current_coeff_2_range,
            )
            .unwrap();
            // reference value for comparison
            <ark_ec::models::bn::Bn<ark_bn254::Parameters>>::ell(
                &mut reference_f,
                &(reference_coeff_0, reference_coeff_1, reference_coeff_2),
                &ark_ec::short_weierstrass_jacobian::GroupAffine::<ark_bn254::g1::Parameters>::new(
                    reference_p_x,
                    reference_p_y,
                    false,
                ),
            );

            // note: need to read the right coeffs indeed

            // ell mutates f in the end. So we can just check f.
            assert_eq!(reference_f, parse_f_from_bytes(&account_f_range));
        }
    }

    #[test]
    fn ell_instruction_d_c2_test_should_fail() {
        //generating input
        let mut rng = test_rng();
        for i in 0..1 {
            let mut reference_f =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                    &mut rng,
                );
            let current_coeff_2 = i.clone();
            let reference_coeff_0 = get_gamma_g2_neg_pc_0().0;
            let reference_coeff_1 = get_gamma_g2_neg_pc_0().1;
            let reference_coeff_2 = get_gamma_g2_neg_pc_0().2;

            let reference_p_y =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                );
            let reference_p_x =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                );

            let test_f =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                    &mut rng,
                );
            let test_p_y = reference_p_y.clone();
            let test_p_x = reference_p_x.clone();

            //simulating the onchain account
            let mut account_f_range = vec![0u8; 384];
            let mut account_current_coeff_2_range = vec![current_coeff_2 as u8; 1];
            let mut account_p_y_range = vec![0u8; 32];
            let mut account_p_x_range = vec![0u8; 32];

            // read coeffs from hardcoded vk here
            parse_f_to_bytes(test_f, &mut account_f_range);
            parse_fp256_to_bytes(test_p_y, &mut account_p_y_range);
            parse_fp256_to_bytes(test_p_x, &mut account_p_x_range);

            // test instruction, mut accs
            ell_instruction_d_c2(
                &mut account_f_range,
                &mut account_p_y_range,
                &mut account_p_x_range,
                &mut account_current_coeff_2_range,
            )
            .unwrap();
            // reference value for comparison
            <ark_ec::models::bn::Bn<ark_bn254::Parameters>>::ell(
                &mut reference_f,
                &(reference_coeff_0, reference_coeff_1, reference_coeff_2),
                &ark_ec::short_weierstrass_jacobian::GroupAffine::<ark_bn254::g1::Parameters>::new(
                    reference_p_x,
                    reference_p_y,
                    false,
                ),
            );
            assert!(reference_f != parse_f_from_bytes(&account_f_range));
        }
    }

    #[test]
    fn ell_instruction_d_c3_test_should_succeed() {
        //generating input
        let mut rng = test_rng();
        for i in 0..1 {
            let mut reference_f =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                    &mut rng,
                );
            let current_coeff_3 = i.clone();
            let reference_coeff_0 = get_delta_g2_neg_pc_0().0;
            let reference_coeff_1 = get_delta_g2_neg_pc_0().1;
            let reference_coeff_2 = get_delta_g2_neg_pc_0().2;

            let reference_p_y =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                );
            let reference_p_x =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                );

            let test_f = reference_f.clone();
            let test_p_y = reference_p_y.clone();
            let test_p_x = reference_p_x.clone();

            //simulating the onchain account
            let mut account_f_range = vec![0u8; 384];
            let mut account_current_coeff_3_range = vec![current_coeff_3 as u8; 1];
            let mut account_p_y_range = vec![0u8; 32];
            let mut account_p_x_range = vec![0u8; 32];

            // read coeffs from hardcoded vk here
            parse_f_to_bytes(test_f, &mut account_f_range);
            parse_fp256_to_bytes(test_p_y, &mut account_p_y_range);
            parse_fp256_to_bytes(test_p_x, &mut account_p_x_range);

            // test instruction, mut accs
            ell_instruction_d_c3(
                &mut account_f_range,
                &mut account_p_y_range,
                &mut account_p_x_range,
                &mut account_current_coeff_3_range,
            )
            .unwrap();
            // reference value for comparison
            <ark_ec::models::bn::Bn<ark_bn254::Parameters>>::ell(
                &mut reference_f,
                &(reference_coeff_0, reference_coeff_1, reference_coeff_2),
                &ark_ec::short_weierstrass_jacobian::GroupAffine::<ark_bn254::g1::Parameters>::new(
                    reference_p_x,
                    reference_p_y,
                    false,
                ),
            );
            assert_eq!(reference_f, parse_f_from_bytes(&account_f_range));
        }
    }

    #[test]
    fn ell_instruction_d_c3_test_should_fail() {
        //generating input
        let mut rng = test_rng();
        for i in 0..1 {
            let mut reference_f =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                    &mut rng,
                );
            let current_coeff_2 = i.clone();
            let reference_coeff_0 = get_delta_g2_neg_pc_0().0;
            let reference_coeff_1 = get_delta_g2_neg_pc_0().1;
            let reference_coeff_2 = get_delta_g2_neg_pc_0().2;

            let reference_p_y =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                );
            let reference_p_x =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                );

            // fails because of this:
            let test_f =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                    &mut rng,
                );
            let test_p_y = reference_p_y.clone();
            let test_p_x = reference_p_x.clone();

            //simulating the onchain account
            let mut account_f_range = vec![0u8; 384];
            let mut account_current_coeff_2_range = vec![current_coeff_2 as u8; 1];
            let mut account_p_y_range = vec![0u8; 32];
            let mut account_p_x_range = vec![0u8; 32];

            // read coeffs from hardcoded vk here
            parse_f_to_bytes(test_f, &mut account_f_range);
            parse_fp256_to_bytes(test_p_y, &mut account_p_y_range);
            parse_fp256_to_bytes(test_p_x, &mut account_p_x_range);

            // test instruction, mut accs
            ell_instruction_d_c2(
                &mut account_f_range,
                &mut account_p_y_range,
                &mut account_p_x_range,
                &mut account_current_coeff_2_range,
            )
            .unwrap();
            // reference value for comparison
            <ark_ec::models::bn::Bn<ark_bn254::Parameters>>::ell(
                &mut reference_f,
                &(reference_coeff_0, reference_coeff_1, reference_coeff_2),
                &ark_ec::short_weierstrass_jacobian::GroupAffine::<ark_bn254::g1::Parameters>::new(
                    reference_p_x,
                    reference_p_y,
                    false,
                ),
            );
            assert!(reference_f != parse_f_from_bytes(&account_f_range));
        }
    }

    #[test]
    fn square_in_place_test_should_succeed() {
        //generating input
        let mut rng = test_rng();
        for _i in 0..10 {
            let mut reference_f =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                    &mut rng,
                );
            let test_f = reference_f.clone();
            //simulating the onchain account
            let mut account_f_range = vec![0u8; 384];

            parse_f_to_bytes(test_f, &mut account_f_range);
            // test instruction, mut acc
            square_in_place_instruction(&mut account_f_range).unwrap();
            // reference value for comparison
            reference_f.square_in_place();

            // ell mutates f in the end. So we can just check f.
            assert_eq!(reference_f, parse_f_from_bytes(&account_f_range));
        }
    }

    #[test]
    fn square_in_place_test_should_fail() {
        //generating input
        let mut rng = test_rng();
        for _i in 0..10 {
            let mut reference_f =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                    &mut rng,
                );
            // should fail because of this:
            let test_f =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                    &mut rng,
                );
            //simulating the onchain account
            let mut account_f_range = vec![0u8; 384];

            parse_f_to_bytes(test_f, &mut account_f_range);
            // test instruction, mut acc
            square_in_place_instruction(&mut account_f_range).unwrap();
            // reference value for comparison
            reference_f.square_in_place();

            // ell mutates f in the end. So we can just check f.
            assert!(reference_f != parse_f_from_bytes(&account_f_range));
        }
    }
}
