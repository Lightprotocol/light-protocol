use crate::utils::prepared_verifying_key::*;
use crate::Groth16_verifier::parsers::*;

use ark_ff::{
    One,
    fp12_2over3over2::{
        Fp12Parameters
    },
    biginteger::BigInteger256,
    fields::{
        models::{
            fp6_3over2::{
                Fp6,
            },
            quadratic_extension::QuadExtField,
            quadratic_extension::QuadExtParameters,
        },
        Fp2,
        Field,

    }

};
use solana_program::{
    //log::sol_log_compute_units,
    msg
};

const C0_SUB_RANGE: [usize; 2] = [0, 192];
const C1_SUB_RANGE: [usize; 2] = [192, 384];

pub fn init_coeffs1(r_range: &mut Vec<u8>, proof_range: &mut Vec<u8>/*, proof_b_bytes: &Vec<u8>*/) {
    // pass in proof.b manually
    //change below to get bytes from account and not init from hardcoded bytes
    // let q = get_proof_b();
    let proof_b = parse_proof_b_from_bytes(proof_range);
    // //comment below for the change
    //parse_proof_b_to_bytes(proof_b, proof_range);

    let mut r: ark_ec::models::bn::g2::G2HomProjective<ark_bn254::Parameters> =
        ark_ec::models::bn::g2::G2HomProjective {
            x: proof_b.x,
            y: proof_b.y,
            z: Fp2::one(),
        };
    parse_r_to_bytes(r, r_range);
}

pub fn initialize_p1_p3_f_instruction(
    p_1_bytes: &Vec<u8>,
    p_3_bytes: &Vec<u8>,
    p_1_x_range: &mut Vec<u8>,
    p_1_y_range: &mut Vec<u8>,
    p_3_x_range: &mut Vec<u8>,
    p_3_y_range: &mut Vec<u8>,
    f_range: &mut Vec<u8>,
) {

    // init f
    let mut f_arr: Vec<u8> = vec![0; 384];
    f_arr[0] = 1;
    let f = parse_f_from_bytes(&mut f_arr);
    parse_f_to_bytes(f, f_range);
}

pub fn square_in_place_instruction(f_range: &mut Vec<u8>) {
    let f = parse_f_from_bytes(f_range); // cost: 30k

    let mut v0 = f.c0 - &f.c1; // cost: <1k
    let v3 = <ark_ff::fields::models::fp12_2over3over2::Fp12ParamsWrapper<
        ark_bn254::Fq12Parameters,
        > as QuadExtParameters>::sub_and_mul_base_field_by_nonresidue(&f.c0, &f.c1); // cost: 1k

    let v2 = f.c0 * &f.c1; // cost: 70k

    // else_2
    v0 *= &v3; // cost: 86k
    let c1 = v2.double(); // cost: <1k
    let c0 = <ark_ff::fields::models::fp12_2over3over2::Fp12ParamsWrapper<
        ark_bn254::Fq12Parameters,
    > as QuadExtParameters>::add_and_mul_base_field_by_nonresidue_plus_one(&v0, &v2);
    // cost: 2k
    parse_cubic_to_bytes_sub(c0, f_range, C0_SUB_RANGE); // cost: 8k
    parse_cubic_to_bytes_sub(c1, f_range, C1_SUB_RANGE); // cost: 8k
}

pub fn ell_instruction_d(
    // used for coeff1 but can be general as well (unmerged)
    f_range: &mut Vec<u8>,
    coeff_0_range: &Vec<u8>,
    coeff_1_range: &Vec<u8>,
    coeff_2_range: &Vec<u8>,
    p_y_range: &Vec<u8>,
    p_x_range: &Vec<u8>,
) {
    let coeff_2 = parse_quad_from_bytes(&coeff_2_range); //
    let mut coeff_1 = parse_quad_from_bytes(&coeff_1_range); // this the same
    let mut coeff_0 = parse_quad_from_bytes(&coeff_0_range); //
    let p_y = parse_fp256_from_bytes(p_y_range); // this adds like 10k
    let p_x = parse_fp256_from_bytes(p_x_range); //

    coeff_0.mul_assign_by_fp(&p_y); // 4k
    coeff_1.mul_assign_by_fp(&p_x); // 4k

    // D2
    let c0 = parse_cubic_from_bytes_sub(f_range, C0_SUB_RANGE); // cost: 15k
                                                                // let coeff_0 = parse_quad_from_bytes(coeff_0_range); // cost: 5k
    let a0 = c0.c0 * coeff_0;
    let a1 = c0.c1 * coeff_0;
    let a2 = c0.c2 * coeff_0;
    let a = Fp6::new(a0, a1, a2);
    // D3
    let c1 = parse_cubic_from_bytes_sub(f_range, C1_SUB_RANGE); // cost: 15k
                                                                // let coeff_1 = parse_quad_from_bytes(coeff_1_range); // cost: 5k
                                                                // let coeff_2 = parse_quad_from_bytes(coeff_2_range); // cost: 5k
    let mut b = c1;
    b.mul_by_01(&coeff_1, &coeff_2); // cost: 33k

    // D4
    let c00 = coeff_0 + coeff_1; //c0 = *c0 + c3
    let mut e = c0 + &c1;
    e.mul_by_01(&c00, &coeff_2); // cost: 36k
                                 // D5

    let mut f =
        <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::one();

    f.c1 = e - &(a + &b); // cost: -
    f.c0 = a + <ark_bn254::fq12::Fq12Parameters as Fp12Parameters>::mul_fp6_by_nonresidue(&b); // cost: 3k
    parse_f_to_bytes(f, f_range); // cost: 15k
}

pub fn ell_instruction_d_c2(
    // ix: 69
    f_range: &mut Vec<u8>,
    p_y_range: &Vec<u8>,
    p_x_range: &Vec<u8>,
    current_coeff_2_range: &mut Vec<u8>,
) {
    let id = current_coeff_2_range[0];

    let mut coeff: (
        QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
        QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
        QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
    ) = (
        QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
            ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([0, 0, 0, 0])),
            ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([0, 0, 0, 0])),
        ),
        QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
            ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([0, 0, 0, 0])),
            ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([0, 0, 0, 0])),
        ),
        QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
            ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([0, 0, 0, 0])),
            ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([0, 0, 0, 0])),
        ),
    );
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

    let mut coeff_2 = coeff.2;
    let mut coeff_1 = coeff.1;
    let mut coeff_0 = coeff.0;
    let p_y = parse_fp256_from_bytes(p_y_range); // this adds like 10k
    let p_x = parse_fp256_from_bytes(p_x_range); //

    coeff_0.mul_assign_by_fp(&p_y); // 4k
    coeff_1.mul_assign_by_fp(&p_x); // 4k

    // D2
    let c0 = parse_cubic_from_bytes_sub(f_range, C0_SUB_RANGE); // cost: 15k
                                                                // let coeff_0 = parse_quad_from_bytes(coeff_0_range); // cost: 5k
    let a0 = c0.c0 * coeff_0;
    let a1 = c0.c1 * coeff_0;
    let a2 = c0.c2 * coeff_0;
    let a = Fp6::new(a0, a1, a2);
    // D3
    let c1 = parse_cubic_from_bytes_sub(f_range, C1_SUB_RANGE); // cost: 15k
                                                                // let coeff_1 = parse_quad_from_bytes(coeff_1_range); // cost: 5k
                                                                // let coeff_2 = parse_quad_from_bytes(coeff_2_range); // cost: 5k
    let mut b = c1;
    b.mul_by_01(&coeff_1, &coeff_2); // cost: 33k

    // D4
    let c00 = coeff_0 + coeff_1; //c0 = *c0 + c3
    let mut e = c0 + &c1;
    e.mul_by_01(&c00, &coeff_2); // cost: 36k
                                 // D5

    let mut f =
        <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::one();

    f.c1 = e - &(a + &b); // cost: -
    f.c0 = a + <ark_bn254::fq12::Fq12Parameters as Fp12Parameters>::mul_fp6_by_nonresidue(&b); // cost: 3k

    parse_f_to_bytes(f, f_range); // cost: 15k
}

pub fn ell_instruction_d_c3(
    // ix: 69
    f_range: &mut Vec<u8>,
    p_y_range: &Vec<u8>,
    p_x_range: &Vec<u8>,
    current_coeff_3_range: &mut Vec<u8>,
) {
    let id = current_coeff_3_range[0];
    let mut coeff: (
        QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
        QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
        QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
    ) = (
        QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
            ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([0, 0, 0, 0])),
            ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([0, 0, 0, 0])),
        ),
        QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
            ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([0, 0, 0, 0])),
            ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([0, 0, 0, 0])),
        ),
        QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
            ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([0, 0, 0, 0])),
            ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([0, 0, 0, 0])),
        ),
    );
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
        // set to 0
        current_coeff_3_range[0] = 0;
    } else {
        // +=1
        current_coeff_3_range[0] += 1;
    }
    let mut coeff_2 = coeff.2;
    let mut coeff_1 = coeff.1;
    let mut coeff_0 = coeff.0;
    let p_y = parse_fp256_from_bytes(p_y_range); // this adds like 10k
    let p_x = parse_fp256_from_bytes(p_x_range); //

    coeff_0.mul_assign_by_fp(&p_y); // 4k
    coeff_1.mul_assign_by_fp(&p_x); // 4k

    // D2
    let c0 = parse_cubic_from_bytes_sub(f_range, C0_SUB_RANGE); // cost: 15k
                                                                // let coeff_0 = parse_quad_from_bytes(coeff_0_range); // cost: 5k
    let a0 = c0.c0 * coeff_0;
    let a1 = c0.c1 * coeff_0;
    let a2 = c0.c2 * coeff_0;
    let a = Fp6::new(a0, a1, a2);
    // D3
    let c1 = parse_cubic_from_bytes_sub(f_range, C1_SUB_RANGE); // cost: 15k
                                                                // let coeff_1 = parse_quad_from_bytes(coeff_1_range); // cost: 5k
                                                                // let coeff_2 = parse_quad_from_bytes(coeff_2_range); // cost: 5k
    let mut b = c1;
    b.mul_by_01(&coeff_1, &coeff_2); // cost: 33k

    // D4
    let c00 = coeff_0 + coeff_1; //c0 = *c0 + c3
    let mut e = c0 + &c1;
    e.mul_by_01(&c00, &coeff_2); // cost: 36k
                                 // D5

    let mut f =
        <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::one();

    f.c1 = e - &(a + &b); // cost: -
    f.c0 = a + <ark_bn254::fq12::Fq12Parameters as Fp12Parameters>::mul_fp6_by_nonresidue(&b); // cost: 3k

    parse_f_to_bytes(f, f_range); // cost: 15k
}

// Note that we're not implementing a conjugate ix here
// (as compared to the bls ix implementations) since
// a) f.conjugate is never called with current curve parameters.
// b) that can change if parameters change.

#[cfg(test)]
mod tests {
    use crate::Groth16_verifier::miller_loop::ml_instructions::{
        ell_instruction_d, ell_instruction_d_c2, ell_instruction_d_c3, square_in_place_instruction,
    };

    use crate::utils::prepared_verifying_key::{get_delta_g2_neg_pc_0, get_gamma_g2_neg_pc_0};
    use crate::Groth16_verifier::parsers::{
        parse_cubic_from_bytes_sub, parse_cubic_to_bytes_sub, parse_f_from_bytes, parse_f_to_bytes,
        parse_fp256_from_bytes, parse_fp256_to_bytes, parse_proof_b_from_bytes,
        parse_proof_b_to_bytes, parse_quad_from_bytes, parse_quad_to_bytes,
    };
    use crate::Groth16_verifier::miller_loop::ml_state::ML254Bytes;

    use crate::Groth16_verifier::miller_loop::ml_ranges::{COEFF_0_RANGE, COEFF_1_RANGE, COEFF_2_RANGE, F_RANGE};

    use ark_ec::bn::BnParameters;
    use ark_ff::{Field, Fp12};
    use ark_std::{test_rng, One, UniformRand, Zero};

    #[test]
    fn ell_instruction_d_test_should_succeed() {
        //generating input
        let mut rng = test_rng();
        for i in 0..10 {
            let mut reference_f =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                    &mut rng,
                );
            let mut reference_coeff_2 =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let mut reference_coeff_1 =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let mut reference_coeff_0 =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let mut reference_p_y =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                );
            let mut reference_p_x =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                );

            let mut test_f = reference_f.clone();
            let mut test_coeff_2 = reference_coeff_2.clone();
            let mut test_coeff_1 = reference_coeff_1.clone();
            let mut test_coeff_0 = reference_coeff_0.clone();
            let mut test_p_y = reference_p_y.clone();
            let mut test_p_x = reference_p_x.clone();

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
            );
            // reference value for comparison
            <ark_ec::models::bn::Bn<ark_bn254::Parameters>>::ell(
                &mut reference_f,
                &(reference_coeff_0, reference_coeff_1, reference_coeff_2), // coeffs: &g2::EllCoeff<Fp2<P::Fp2Params>>,
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
        for i in 0..10 {
            let mut reference_f =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                    &mut rng,
                );
            let mut reference_coeff_2 =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let mut reference_coeff_1 =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let mut reference_coeff_0 =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let mut reference_p_y =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                );
            let mut reference_p_x =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                );

            let mut test_f = reference_f.clone();
            let mut test_coeff_2 = reference_coeff_2.clone();
            let mut test_coeff_1 = reference_coeff_1.clone();
            // failing here:
            let mut test_coeff_0 =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqe::rand(
                    &mut rng,
                );
            let mut test_p_y = reference_p_y.clone();
            let mut test_p_x = reference_p_x.clone();

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
            );
            // reference value for comparison
            <ark_ec::models::bn::Bn<ark_bn254::Parameters>>::ell(
                &mut reference_f,
                &(reference_coeff_0, reference_coeff_1, reference_coeff_2), // coeffs: &g2::EllCoeff<Fp2<P::Fp2Params>>,
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
            let mut current_coeff_2 = i.clone();
            let mut reference_coeff_0 = get_gamma_g2_neg_pc_0().0;
            let mut reference_coeff_1 = get_gamma_g2_neg_pc_0().1;
            let mut reference_coeff_2 = get_gamma_g2_neg_pc_0().2;

            let mut reference_p_y =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                );
            let mut reference_p_x =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                );

            let mut test_f = reference_f.clone();
            let mut test_p_y = reference_p_y.clone();
            let mut test_p_x = reference_p_x.clone();

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
            );
            // reference value for comparison
            <ark_ec::models::bn::Bn<ark_bn254::Parameters>>::ell(
                &mut reference_f,
                &(reference_coeff_0, reference_coeff_1, reference_coeff_2), // coeffs: &g2::EllCoeff<Fp2<P::Fp2Params>>,
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
            let mut current_coeff_2 = i.clone();
            let mut reference_coeff_0 = get_gamma_g2_neg_pc_0().0;
            let mut reference_coeff_1 = get_gamma_g2_neg_pc_0().1;
            let mut reference_coeff_2 = get_gamma_g2_neg_pc_0().2;

            let mut reference_p_y =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                );
            let mut reference_p_x =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                );

            let mut test_f =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                    &mut rng,
                );
            let mut test_p_y = reference_p_y.clone();
            let mut test_p_x = reference_p_x.clone();

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
            );
            // reference value for comparison
            <ark_ec::models::bn::Bn<ark_bn254::Parameters>>::ell(
                &mut reference_f,
                &(reference_coeff_0, reference_coeff_1, reference_coeff_2), // coeffs: &g2::EllCoeff<Fp2<P::Fp2Params>>,
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
            let mut current_coeff_3 = i.clone();
            let mut reference_coeff_0 = get_delta_g2_neg_pc_0().0;
            let mut reference_coeff_1 = get_delta_g2_neg_pc_0().1;
            let mut reference_coeff_2 = get_delta_g2_neg_pc_0().2;

            let mut reference_p_y =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                );
            let mut reference_p_x =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                );

            let mut test_f = reference_f.clone();
            let mut test_p_y = reference_p_y.clone();
            let mut test_p_x = reference_p_x.clone();

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
            );
            // reference value for comparison
            <ark_ec::models::bn::Bn<ark_bn254::Parameters>>::ell(
                &mut reference_f,
                &(reference_coeff_0, reference_coeff_1, reference_coeff_2), // coeffs: &g2::EllCoeff<Fp2<P::Fp2Params>>,
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
            let mut current_coeff_2 = i.clone();
            let mut reference_coeff_0 = get_delta_g2_neg_pc_0().0;
            let mut reference_coeff_1 = get_delta_g2_neg_pc_0().1;
            let mut reference_coeff_2 = get_delta_g2_neg_pc_0().2;

            let mut reference_p_y =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                );
            let mut reference_p_x =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fq::rand(
                    &mut rng,
                );

            // fails because of this:
            let mut test_f =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                    &mut rng,
                );
            let mut test_p_y = reference_p_y.clone();
            let mut test_p_x = reference_p_x.clone();

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
            );
            // reference value for comparison
            <ark_ec::models::bn::Bn<ark_bn254::Parameters>>::ell(
                &mut reference_f,
                &(reference_coeff_0, reference_coeff_1, reference_coeff_2), // coeffs: &g2::EllCoeff<Fp2<P::Fp2Params>>,
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
        for i in 0..10 {
            let mut reference_f =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                    &mut rng,
                );
            let mut test_f = reference_f.clone();
            //simulating the onchain account
            let mut account_f_range = vec![0u8; 384];

            parse_f_to_bytes(test_f, &mut account_f_range);
            // test instruction, mut acc
            square_in_place_instruction(&mut account_f_range);
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
        for i in 0..10 {
            let mut reference_f =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                    &mut rng,
                );
            // should fail because of this:
            let mut test_f =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                    &mut rng,
                );
            //simulating the onchain account
            let mut account_f_range = vec![0u8; 384];

            parse_f_to_bytes(test_f, &mut account_f_range);
            // test instruction, mut acc
            square_in_place_instruction(&mut account_f_range);
            // reference value for comparison
            reference_f.square_in_place();

            // ell mutates f in the end. So we can just check f.
            assert!(reference_f != parse_f_from_bytes(&account_f_range));
        }
    }
}
