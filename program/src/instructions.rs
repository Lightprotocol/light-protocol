use ark_ff::fields::{Field,PrimeField,SquareRootField };
use crate::parsers::*;
// use crate::constraints::{custom_mul_by_034,custom_mul_by_014};
use std::ops::{AddAssign, SubAssign};
use ark_ff::fp12_2over3over2::{Fp12, Fp12Parameters};
use ark_ff::fields::models::quadratic_extension::QuadExtField;
use ark_ff::fields::models::quadratic_extension::QuadExtParameters;
use ark_ff::fields::models::fp2::*;
use ark_ff::fields::models::fp6_3over2::{Fp6Parameters, Fp6};
use crate:: hard_coded_verifying_key_pvk_new_ciruit::*;

// init f instruction. Executed once before the miller loop:
use solana_program::{
    msg,
    log::sol_log_compute_units,
};
// 0
pub fn init_f_instruction(
    f_range: &mut Vec<u8>,
    f : &mut <ark_ec::models::bls12::Bls12::<ark_bls12_381::Parameters> as ark_ec::PairingEngine>::Fqk
){

    parse_f_to_bytes(*f, f_range); // 31000

    let f = parse_f_from_bytes(f_range);
}

// -------------------> split #3 test:
pub fn custom_square_in_place_instruction_else_1(
    f_range:  &Vec<u8>,
    cubic_v0_range: &mut Vec<u8>,
    cubic_v3_range: &mut Vec<u8>
){
    let mut f = parse_f_from_bytes(f_range);  // 58284

    let mut v0 = f.c0 - &f.c1; // cost 768
    let v3 = <ark_ff::fields::models::fp12_2over3over2::Fp12ParamsWrapper::<ark_bls12_381::Fq12Parameters>  as QuadExtParameters>::sub_and_mul_base_field_by_nonresidue(&f.c0, &f.c1); // cost 1108

    parse_cubic_to_bytes(v0, cubic_v0_range); // 15198
    // assert_eq!(v0, parse_cubic_from_bytes(cubic_v0_range));
    parse_cubic_to_bytes(v3, cubic_v3_range); // 15198
    // assert_eq!(v3, parse_cubic_from_bytes(cubic_v3_range));
}

// # 3.b => id: 17
pub fn custom_square_in_place_instruction_else_1_b(
    f_range:  &Vec<u8>,
    cubic_v2_range: &mut Vec<u8>,
){
    let mut f = parse_f_from_bytes(f_range);  // 58284
    let v2 = f.c0 * &f.c1; // cost 86525
    parse_cubic_to_bytes(v2, cubic_v2_range);
}




//4
pub fn custom_square_in_place_instruction_else_2(
    cubic_v0_range: &mut Vec<u8>,
    cubic_v3_range: &Vec<u8>
){

    let mut v0 = parse_cubic_from_bytes(cubic_v0_range);  // 29478
    let mut v3 = parse_cubic_from_bytes(cubic_v3_range);  // 29478


    v0 *= &v3; // cost 86105

    // println!("#4 changing v0: {:?}", v0);

    parse_cubic_to_bytes(v0, cubic_v0_range); // 15198


 // total cost to ~160259
}

// 5
pub fn custom_square_in_place_instruction_else_3(
    cubic_v0_range: &Vec<u8>,
    cubic_v2_range: &Vec<u8>,
    f_range: &mut Vec<u8>
){

    let v0 = parse_cubic_from_bytes(cubic_v0_range);  // 29478
    let v2 = parse_cubic_from_bytes(cubic_v2_range);  // 29478

    let c1 = v2.double(); // cost 378
    let c0 = <ark_ff::fields::models::fp12_2over3over2::Fp12ParamsWrapper::<ark_bls12_381::Fq12Parameters>  as QuadExtParameters>::add_and_mul_base_field_by_nonresidue_plus_one(&v0, &v2); // cost 1550

    // write f:
    // println!("#5 changing c0c1/f: {:?}   :{:?}    ", c0,c1);
    let c0_sub : [usize;2] = [0,288];
    let c1_sub : [usize;2] = [288,576];

    parse_cubic_to_bytes_sub(c0, f_range, c0_sub); // 15198
    parse_cubic_to_bytes_sub(c1, f_range, c1_sub); // 15198
     // total cost shorted to 91280
}



// -----------------------------------------------> ell ::M (tested)

// 6
pub fn custom_ell_instruction_M_1(
    // coeff_0: ark_ff::QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bls12_381::Fq2Parameters>>,
    // coeff_1: &mut ark_ff::QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bls12_381::Fq2Parameters>>,
    // coeff_2: &mut ark_ff::QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bls12_381::Fq2Parameters>>,
    // p: &ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bls12_381::g1::Parameters>,

    // account: &mut [u8],
    coeff_2_range: &mut Vec<u8>,
    coeff_1_range: &mut Vec<u8>,
    coeff_0_range: &mut Vec<u8>,
    p_y_range: &Vec<u8>,
    p_x_range: &Vec<u8>,
    coeff_2_bytes: &Vec<u8>,
    coeff_1_bytes: &Vec<u8>,
    coeff_0_bytes: &Vec<u8>,
){
    let mut coeff_2;
    let mut coeff_1;
    let mut coeff_0;
    if coeff_0_bytes.len() >0{

        coeff_2 = parse_quad_from_bytes(&coeff_2_bytes); // 10000
        coeff_1 = parse_quad_from_bytes(&coeff_1_bytes); // 10000
        coeff_0 = parse_quad_from_bytes(&coeff_0_bytes); // 10000
    }
    else { // means coeffs were filled (coeff1)
        coeff_2 = parse_quad_from_bytes(&coeff_2_range); // 10000
        coeff_1 = parse_quad_from_bytes(&coeff_1_range); // 10000
        coeff_0 = parse_quad_from_bytes(&coeff_0_range); // 10000
    }

    let p_y = parse_fp384_from_bytes(p_y_range); // 4913
    let p_x = parse_fp384_from_bytes(p_x_range); // 4913
    // println!("#ELL1 PxPy: {:?}, {:?}", p_x, p_y);
    // println!("MONDAY: COEFF 0: {:?}", coeff_0);
    // println!("MONDAY: COEFF 1: {:?}", coeff_1);
    // println!("MONDAY: COEFF 2: {:?}", coeff_2);

    coeff_2.mul_assign_by_fp(&p_y); // 300
    coeff_1.mul_assign_by_fp(&p_x); // 8691
    // println!("#ELL1 changing coeff0: {:?}   coeff1:{:?}    coeff2:{:?}", coeff_0, coeff_1, coeff_2);

    parse_quad_to_bytes(coeff_2, coeff_2_range); // 5066
    parse_quad_to_bytes(coeff_1, coeff_1_range); // 5066
    parse_quad_to_bytes(coeff_0, coeff_0_range); // 5066
    // total cost ~64000
}

// 7
pub fn custom_ell_instruction_M_2(
    f_range: &Vec<u8>,
    coeff_1_range: &mut Vec<u8>,
    coeff_0_range: &mut Vec<u8>,
    aa_range: &mut Vec<u8>,

){
    let sub_c0 : [usize;2] = [0,288];
    let c0 = parse_cubic_from_bytes_sub(f_range, sub_c0); // 29478 // C0
    let coeff_1 = parse_quad_from_bytes(coeff_1_range); // 9826
    let coeff_0 = parse_quad_from_bytes(coeff_0_range); // 9826

    let mut aa = c0;

    aa.mul_by_01(&coeff_0, &coeff_1); // cost 71063

    // println!("#ELL2 changing coeff0   coeff1 coeff2, aa");


    parse_cubic_to_bytes(aa, aa_range); // 15198
    parse_quad_to_bytes(coeff_1, coeff_1_range); // 5066
    parse_quad_to_bytes(coeff_0, coeff_0_range); // 5066
    // supposed total cost: 135697
}



// 8
// custom mul B
pub fn custom_ell_instruction_M_3(
    bb_range: &mut Vec<u8>,
    f_range: &Vec<u8>,
    coeff_2_range: &Vec<u8>,
){
    let sub_c1 : [usize;2] = [288,576];

    let mut c1 = parse_cubic_from_bytes_sub(f_range, sub_c1);  // 29478
    let mut coeff_2 = parse_quad_from_bytes(coeff_2_range); // 9826

    let mut bb = c1;
    bb.mul_by_1(&mut coeff_2);//     NEED MUT?? w     cost 42503
    // println!("#ELL3 changing bb");

    parse_cubic_to_bytes(bb, bb_range); // 15198
}


// 9 (M4.a):
pub fn custom_ell_instruction_M_4(
    f_range: &mut Vec<u8>,
){
    let mut f = parse_f_from_bytes(f_range);  // 58284
    f.c1.add_assign(&f.c0);//       cost 459
    // println!("#ELL4 changing f {:?}", f);
    parse_f_to_bytes(f, f_range);  // 31000
    // total cost  ~90000
}



// 18 (M4.b):
pub fn custom_ell_instruction_M_4_b(
    f_range: &mut Vec<u8>,
    coeff_2_range: &Vec<u8>,
    coeff_1_range: &Vec<u8>,
    coeff_0_range: &Vec<u8>,
){
    let sub_c1 : [usize;2] = [288,576];
    let mut c1 = parse_cubic_from_bytes_sub(f_range, sub_c1);  // 58284 // 30k

    let mut coeffs_2 = parse_quad_from_bytes(coeff_2_range); // 9826
    let mut coeffs_1 = parse_quad_from_bytes(coeff_1_range); // 9826
    let mut coeffs_0 = parse_quad_from_bytes(coeff_0_range); // 9826

    let mut o = coeffs_1;
    o.add_assign(coeffs_2);//   cost 176
    c1.mul_by_01(&coeffs_0, &o);//   cost 71285

    // println!("#ELL4 changing f {:?}", f);
    parse_cubic_to_bytes_sub(c1, f_range, sub_c1); // 16k
    // total cost  ~151047


}


// 10
pub fn custom_ell_instruction_M_5(
    f_range: &mut Vec<u8>,
    aa_range: &Vec<u8>,
    bb_range: &Vec<u8>,
){
    let mut f = parse_f_from_bytes(f_range);  // 58284
    let mut aa = parse_cubic_from_bytes(aa_range); // 29478
    let mut bb = parse_cubic_from_bytes(bb_range); //  29478

    f.c1.sub_assign(&aa);//     cost 625
    f.c1.sub_assign(&bb);//     cost 623
    f.c0 = bb;//                cost 77
    f.c0 = <ark_bls12_381::fq12::Fq12Parameters as Fp12Parameters>::mul_fp6_by_nonresidue(&f.c0);// cost 495
    f.c0.add_assign(&aa);//     cost 460

    // println!("#ELL5 changing f {:?}", f);

    parse_f_to_bytes(f, f_range);  // 31000

    // total cost ~151240
}

// --------------------------> conjugate

// 16
pub fn custom_conjugate_instruction(f_range: &mut Vec<u8>) {
    let mut f = parse_f_from_bytes(f_range);
    f.conjugate();
    // println!("f aftr conj.:{:?}", f);
    // println!("#16 changing f (end) {:?}", f);

    parse_f_to_bytes(f, f_range);
}



use ark_ff::biginteger::BigInteger384;


// Instructions.rs

pub fn instruction_onchain_coeffs_2(current_coeff_2_range: &mut Vec<u8>,coeff_2_range: &mut Vec<u8>,coeff_1_range: &mut Vec<u8>,coeff_0_range: &mut Vec<u8>){
    let id = current_coeff_2_range[0];

    let mut coeff : (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>) =
    (
        QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
            ark_ff::Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([0,0, 0, 0, 0, 0])),
            ark_ff::Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([0,0, 0, 0, 0, 0]))
        ),
        QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
            ark_ff::Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([0,0, 0, 0, 0, 0])),
            ark_ff::Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([0,0, 0, 0, 0, 0]))
        ),
        QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
            ark_ff::Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([0,0, 0, 0, 0, 0])),
            ark_ff::Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([0,0, 0, 0, 0, 0]))
        )
    );
    if id == 0 { coeff = get_gamma_g2_neg_pc_0();
    } else if id == 1 { coeff = get_gamma_g2_neg_pc_1();
    } else if id == 2 { coeff = get_gamma_g2_neg_pc_2();
    } else if id == 3 { coeff = get_gamma_g2_neg_pc_3();
    } else if id == 4 { coeff = get_gamma_g2_neg_pc_4();
    } else if id == 5 { coeff = get_gamma_g2_neg_pc_5();
    } else if id == 6 { coeff = get_gamma_g2_neg_pc_6();
    } else if id == 7 { coeff = get_gamma_g2_neg_pc_7();
    } else if id == 8 { coeff = get_gamma_g2_neg_pc_8();
    } else if id == 9 { coeff = get_gamma_g2_neg_pc_9();
    } else if id == 10 { coeff = get_gamma_g2_neg_pc_10();
    } else if id == 11 { coeff = get_gamma_g2_neg_pc_11();
    } else if id == 12 { coeff = get_gamma_g2_neg_pc_12();
    } else if id == 13 { coeff = get_gamma_g2_neg_pc_13();
    } else if id == 14 { coeff = get_gamma_g2_neg_pc_14();
    } else if id == 15 { coeff = get_gamma_g2_neg_pc_15();
    } else if id == 16 { coeff = get_gamma_g2_neg_pc_16();
    } else if id == 17 { coeff = get_gamma_g2_neg_pc_17();
    } else if id == 18 { coeff = get_gamma_g2_neg_pc_18();
    } else if id == 19 { coeff = get_gamma_g2_neg_pc_19();
    } else if id == 20 { coeff = get_gamma_g2_neg_pc_20();
    } else if id == 21 { coeff = get_gamma_g2_neg_pc_21();
    } else if id == 22 { coeff = get_gamma_g2_neg_pc_22();
    } else if id == 23 { coeff = get_gamma_g2_neg_pc_23();
    } else if id == 24 { coeff = get_gamma_g2_neg_pc_24();
    } else if id == 25 { coeff = get_gamma_g2_neg_pc_25();
    } else if id == 26 { coeff = get_gamma_g2_neg_pc_26();
    } else if id == 27 { coeff = get_gamma_g2_neg_pc_27();
    } else if id == 28 { coeff = get_gamma_g2_neg_pc_28();
    } else if id == 29 { coeff = get_gamma_g2_neg_pc_29();
    } else if id == 30 { coeff = get_gamma_g2_neg_pc_30();
    } else if id == 31 { coeff = get_gamma_g2_neg_pc_31();
    } else if id == 32 { coeff = get_gamma_g2_neg_pc_32();
    } else if id == 33 { coeff = get_gamma_g2_neg_pc_33();
    } else if id == 34 { coeff = get_gamma_g2_neg_pc_34();
    } else if id == 35 { coeff = get_gamma_g2_neg_pc_35();
    } else if id == 36 { coeff = get_gamma_g2_neg_pc_36();
    } else if id == 37 { coeff = get_gamma_g2_neg_pc_37();
    } else if id == 38 { coeff = get_gamma_g2_neg_pc_38();
    } else if id == 39 { coeff = get_gamma_g2_neg_pc_39();
    } else if id == 40 { coeff = get_gamma_g2_neg_pc_40();
    } else if id == 41 { coeff = get_gamma_g2_neg_pc_41();
    } else if id == 42 { coeff = get_gamma_g2_neg_pc_42();
    } else if id == 43 { coeff = get_gamma_g2_neg_pc_43();
    } else if id == 44 { coeff = get_gamma_g2_neg_pc_44();
    } else if id == 45 { coeff = get_gamma_g2_neg_pc_45();
    } else if id == 46 { coeff = get_gamma_g2_neg_pc_46();
    } else if id == 47 { coeff = get_gamma_g2_neg_pc_47();
    } else if id == 48 { coeff = get_gamma_g2_neg_pc_48();
    } else if id == 49 { coeff = get_gamma_g2_neg_pc_49();
    } else if id == 50 { coeff = get_gamma_g2_neg_pc_50();
    } else if id == 51 { coeff = get_gamma_g2_neg_pc_51();
    } else if id == 52 { coeff = get_gamma_g2_neg_pc_52();
    } else if id == 53 { coeff = get_gamma_g2_neg_pc_53();
    } else if id == 54 { coeff = get_gamma_g2_neg_pc_54();
    } else if id == 55 { coeff = get_gamma_g2_neg_pc_55();
    } else if id == 56 { coeff = get_gamma_g2_neg_pc_56();
    } else if id == 57 { coeff = get_gamma_g2_neg_pc_57();
    } else if id == 58 { coeff = get_gamma_g2_neg_pc_58();
    } else if id == 59 { coeff = get_gamma_g2_neg_pc_59();
    } else if id == 60 { coeff = get_gamma_g2_neg_pc_60();
    } else if id == 61 { coeff = get_gamma_g2_neg_pc_61();
    } else if id == 62 { coeff = get_gamma_g2_neg_pc_62();
    } else if id == 63 { coeff = get_gamma_g2_neg_pc_63();
    } else if id == 64 { coeff = get_gamma_g2_neg_pc_64();
    } else if id == 65 { coeff = get_gamma_g2_neg_pc_65();
    } else if id == 66 { coeff = get_gamma_g2_neg_pc_66();
    } else if id == 67 { coeff = get_gamma_g2_neg_pc_67();
    } else {
        msg!("ERR: coeff uninitialized value");
    }

    // parse coeff to acc
    parse_quad_to_bytes(coeff.2, coeff_2_range); // 5066
    parse_quad_to_bytes(coeff.1, coeff_1_range); // 5066
    parse_quad_to_bytes(coeff.0, coeff_0_range); // 5066

    if id == 67{
        // set to 0
        current_coeff_2_range[0] = 0;
    } else {
        // +=1
        current_coeff_2_range[0] += 1;
    }

}

pub fn instruction_onchain_coeffs_3(current_coeff_3_range: &mut Vec<u8>,coeff_2_range: &mut Vec<u8>,coeff_1_range: &mut Vec<u8>,coeff_0_range: &mut Vec<u8>){
    let id = current_coeff_3_range[0];

    let mut coeff : (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>) =
    (
        QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
            ark_ff::Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([0,0, 0, 0, 0, 0])),
            ark_ff::Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([0,0, 0, 0, 0, 0]))
        ),
        QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
            ark_ff::Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([0,0, 0, 0, 0, 0])),
            ark_ff::Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([0,0, 0, 0, 0, 0]))
        ),
        QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
            ark_ff::Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([0,0, 0, 0, 0, 0])),
            ark_ff::Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([0,0, 0, 0, 0, 0]))
        )
    );
    if id == 0 { coeff = get_delta_g2_neg_pc_0();
    } else if id == 1 { coeff = get_delta_g2_neg_pc_1();
    } else if id == 2 { coeff = get_delta_g2_neg_pc_2();
    } else if id == 3 { coeff = get_delta_g2_neg_pc_3();
    } else if id == 4 { coeff = get_delta_g2_neg_pc_4();
    } else if id == 5 { coeff = get_delta_g2_neg_pc_5();
    } else if id == 6 { coeff = get_delta_g2_neg_pc_6();
    } else if id == 7 { coeff = get_delta_g2_neg_pc_7();
    } else if id == 8 { coeff = get_delta_g2_neg_pc_8();
    } else if id == 9 { coeff = get_delta_g2_neg_pc_9();
    } else if id == 10 { coeff = get_delta_g2_neg_pc_10();
    } else if id == 11 { coeff = get_delta_g2_neg_pc_11();
    } else if id == 12 { coeff = get_delta_g2_neg_pc_12();
    } else if id == 13 { coeff = get_delta_g2_neg_pc_13();
    } else if id == 14 { coeff = get_delta_g2_neg_pc_14();
    } else if id == 15 { coeff = get_delta_g2_neg_pc_15();
    } else if id == 16 { coeff = get_delta_g2_neg_pc_16();
    } else if id == 17 { coeff = get_delta_g2_neg_pc_17();
    } else if id == 18 { coeff = get_delta_g2_neg_pc_18();
    } else if id == 19 { coeff = get_delta_g2_neg_pc_19();
    } else if id == 20 { coeff = get_delta_g2_neg_pc_20();
    } else if id == 21 { coeff = get_delta_g2_neg_pc_21();
    } else if id == 22 { coeff = get_delta_g2_neg_pc_22();
    } else if id == 23 { coeff = get_delta_g2_neg_pc_23();
    } else if id == 24 { coeff = get_delta_g2_neg_pc_24();
    } else if id == 25 { coeff = get_delta_g2_neg_pc_25();
    } else if id == 26 { coeff = get_delta_g2_neg_pc_26();
    } else if id == 27 { coeff = get_delta_g2_neg_pc_27();
    } else if id == 28 { coeff = get_delta_g2_neg_pc_28();
    } else if id == 29 { coeff = get_delta_g2_neg_pc_29();
    } else if id == 30 { coeff = get_delta_g2_neg_pc_30();
    } else if id == 31 { coeff = get_delta_g2_neg_pc_31();
    } else if id == 32 { coeff = get_delta_g2_neg_pc_32();
    } else if id == 33 { coeff = get_delta_g2_neg_pc_33();
    } else if id == 34 { coeff = get_delta_g2_neg_pc_34();
    } else if id == 35 { coeff = get_delta_g2_neg_pc_35();
    } else if id == 36 { coeff = get_delta_g2_neg_pc_36();
    } else if id == 37 { coeff = get_delta_g2_neg_pc_37();
    } else if id == 38 { coeff = get_delta_g2_neg_pc_38();
    } else if id == 39 { coeff = get_delta_g2_neg_pc_39();
    } else if id == 40 { coeff = get_delta_g2_neg_pc_40();
    } else if id == 41 { coeff = get_delta_g2_neg_pc_41();
    } else if id == 42 { coeff = get_delta_g2_neg_pc_42();
    } else if id == 43 { coeff = get_delta_g2_neg_pc_43();
    } else if id == 44 { coeff = get_delta_g2_neg_pc_44();
    } else if id == 45 { coeff = get_delta_g2_neg_pc_45();
    } else if id == 46 { coeff = get_delta_g2_neg_pc_46();
    } else if id == 47 { coeff = get_delta_g2_neg_pc_47();
    } else if id == 48 { coeff = get_delta_g2_neg_pc_48();
    } else if id == 49 { coeff = get_delta_g2_neg_pc_49();
    } else if id == 50 { coeff = get_delta_g2_neg_pc_50();
    } else if id == 51 { coeff = get_delta_g2_neg_pc_51();
    } else if id == 52 { coeff = get_delta_g2_neg_pc_52();
    } else if id == 53 { coeff = get_delta_g2_neg_pc_53();
    } else if id == 54 { coeff = get_delta_g2_neg_pc_54();
    } else if id == 55 { coeff = get_delta_g2_neg_pc_55();
    } else if id == 56 { coeff = get_delta_g2_neg_pc_56();
    } else if id == 57 { coeff = get_delta_g2_neg_pc_57();
    } else if id == 58 { coeff = get_delta_g2_neg_pc_58();
    } else if id == 59 { coeff = get_delta_g2_neg_pc_59();
    } else if id == 60 { coeff = get_delta_g2_neg_pc_60();
    } else if id == 61 { coeff = get_delta_g2_neg_pc_61();
    } else if id == 62 { coeff = get_delta_g2_neg_pc_62();
    } else if id == 63 { coeff = get_delta_g2_neg_pc_63();
    } else if id == 64 { coeff = get_delta_g2_neg_pc_64();
    } else if id == 65 { coeff = get_delta_g2_neg_pc_65();
    } else if id == 66 { coeff = get_delta_g2_neg_pc_66();
    } else if id == 67 { coeff = get_delta_g2_neg_pc_67();
    } else {
        msg!("ERR: no coeff initialized");
    }

    // parse coeff to acc

        parse_quad_to_bytes(coeff.2, coeff_2_range); // 5066
        parse_quad_to_bytes(coeff.1, coeff_1_range); // 5066
        parse_quad_to_bytes(coeff.0, coeff_0_range); // 5066



    if id == 67{
        // set to 0
        current_coeff_3_range[0] = 0;
    } else {
        // +=1
        current_coeff_3_range[0] += 1;
    }


}

// 0
pub fn init_f_instruction_TEST(
    account: &mut [u8],
    f : &mut <ark_ec::models::bls12::Bls12::<ark_bls12_381::Parameters> as ark_ec::PairingEngine>::Fqk
){

    parse_f_to_bytes_TEST(*f, account,[0,576]); // 31000

}
