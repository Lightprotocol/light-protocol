use ark_ff::fields::models::quadratic_extension::QuadExtField;
use ark_ff::fields::models::quadratic_extension::QuadExtParameters;
use ark_ff::fields::models::cubic_extension::CubicExtParameters;
use ark_ff::Field;
use ark_ec;

use crate::parsers_part_2::*;
use crate::ranges_part_2::*;
use solana_program::{
    msg,
    log::sol_log_compute_units,
};


pub fn custom_f_inverse_1(_f_f2_range: &Vec<u8>,_cubic_range_1: &mut  Vec<u8>) {
    //first part of calculating the inverse to f

    let f = parse_f_from_bytes_new(_f_f2_range);
    let v1 = f.c1.square();
    parse_cubic_to_bytes_new(v1, _cubic_range_1, solo_cubic_0_range);
}


pub fn custom_f_inverse_2(_f_f2_range: &Vec<u8>,_cubic_range_0: &mut  Vec<u8>,_cubic_range_1: & Vec<u8>) {

    let f = parse_f_from_bytes_new(_f_f2_range);
    let v1 = parse_cubic_from_bytes_new(_cubic_range_1, solo_cubic_0_range);
    // cost 58976
    let v0 = <ark_ff::fields::models::fp12_2over3over2::Fp12ParamsWrapper::<ark_bls12_381::Fq12Parameters>  as QuadExtParameters>::sub_and_mul_base_field_by_nonresidue(&f.c0.square(), &v1);
    parse_cubic_to_bytes_new(v0, _cubic_range_0, solo_cubic_0_range);
}


pub fn custom_f_inverse_3(
        _cubic_range_1: &mut Vec<u8>,
        _cubic_range_0: &Vec<u8>,
        _f_f2_range: &Vec<u8>,

    ) {
    let v1 = parse_cubic_from_bytes_new(_cubic_range_0, solo_cubic_0_range);
    let f_c0 = parse_cubic_from_bytes_new(_f_f2_range, f_cubic_0_range);

    let c0 = f_c0 * &v1;//      cost 87545

    parse_cubic_to_bytes_new(c0, _cubic_range_1, solo_cubic_0_range);
    //assert_eq!(c0, parse_cubic_from_bytes(account, cubic_range_1));
}


pub fn custom_f_inverse_4(
    _cubic: &mut Vec<u8>,
    _f_f2_range: &Vec<u8>,
    ) {

    let v1 = parse_cubic_from_bytes_new(_cubic, solo_cubic_0_range);//30
    let f_c1 = parse_cubic_from_bytes_new(_f_f2_range, f_cubic_1_range);//30
    let c1 = -(f_c1 * &v1);//   cost 86867
    parse_cubic_to_bytes_new(c1, _cubic, solo_cubic_0_range);
}


pub fn custom_f_inverse_5(
    _cubic_0: &Vec<u8>,
    _cubic_1: &Vec<u8>,
    _f_f2_range: &mut Vec<u8>,
    ) {

    let c0 = parse_cubic_from_bytes_new(_cubic_1, solo_cubic_0_range);//30
    let c1 = parse_cubic_from_bytes_new(_cubic_0, solo_cubic_0_range);//30
    parse_f_to_bytes_new(<ark_ec::models::bls12::Bls12::<ark_bls12_381::Parameters> as ark_ec::PairingEngine>::Fqk::new(c0, c1), _f_f2_range);//30

}
pub fn custom_cubic_inverse_1(
        _cubic_range_0: &Vec<u8>,
        _quad_range_0: &mut Vec<u8>,
        _quad_range_1: &mut Vec<u8>,
        _quad_range_2: &mut Vec<u8>,
        _quad_range_3: &mut Vec<u8>,
    ) {


        let f = parse_cubic_from_bytes_new(_cubic_range_0, solo_cubic_0_range);
        // From "High-Speed Software Implementation of the Optimal Ate AbstractPairing
        // over
        // Barreto-Naehrig Curves"; Algorithm 17
        ////msg!"cubic inverse ");
        ////sol_log_compute_units();
        ////msg!"t0 ");
        let t0 = f.c0.square(); //  cost 9188
        ////sol_log_compute_units();
        ////msg!"t1 ");
        let t1 = f.c1.square();//  cost 9255 cumulative 18443
        ////sol_log_compute_units();
        ////msg!"t2 ");
        let t2 = f.c2.square();//   cost 9255 cumulative 27698
        ////sol_log_compute_units();
        ////msg!"t3 ");
        let t3 = f.c0 * &f.c1;//    cost 13790 cumulative 41488
        ////sol_log_compute_units();
        ////msg!"t4 ");
        let t4 = f.c0 * &f.c2;//    cost 13789 cumulative 55277
        ////sol_log_compute_units();
        ////msg!"t5 ");
        let t5 = f.c1 * &f.c2;//    cost 13791 cumulative 69068
        ////sol_log_compute_units();
        ////msg!"n5 ");
        //    cost 270 cumulative 69350
        let n5 = <ark_ff::fields::models::fp6_3over2::Fp6ParamsWrapper::<ark_bls12_381::Fq6Parameters>  as CubicExtParameters>::mul_base_field_by_nonresidue(&t5);
        ////sol_log_compute_units();
        ////msg!"s0");
        let s0 = t0 - &n5;  //      cost 255 cumulative 69593
        ////sol_log_compute_units();
        ////msg!"s1 ");
        //    cost 505 cumulative 70098
        let s1 = <ark_ff::fields::models::fp6_3over2::Fp6ParamsWrapper::<ark_bls12_381::Fq6Parameters>  as CubicExtParameters>::mul_base_field_by_nonresidue(&t2) - &t3;
        ////sol_log_compute_units();
        ////msg!"s2 ");        //      cost 265 cumulative 70363
        let s2 = t1 - &t4; // typo in paper referenced above. should be "-" as per Scott, but is "*"

        ////sol_log_compute_units();
        ////msg!"a1 ");
        let a1 = f.c2 * &s1; //     cost 13791 cumulative 84154
        ////sol_log_compute_units();
        ////msg!"a2 ");
        let a2 = f.c1 * &s2;//      cost 13791 cumulative 97945
        ////sol_log_compute_units();
        ////msg!"a3 ");
        let mut a3 = a1 + &a2;//    cost 182 cumulative 98127
        ////sol_log_compute_units();
        ////msg!"a3.1 ");
        // cost 297 cumulative 98424
        a3 = <ark_ff::fields::models::fp6_3over2::Fp6ParamsWrapper::<ark_bls12_381::Fq6Parameters>  as CubicExtParameters>::mul_base_field_by_nonresidue(&a3);

        //13895
        let t6_input = f.c0 * &s0 + &a3;

        parse_quad_to_bytes_new(s0, _quad_range_0);
        parse_quad_to_bytes_new(s1, _quad_range_1);
        parse_quad_to_bytes_new(s2, _quad_range_2);
        parse_quad_to_bytes_new(t6_input, _quad_range_3);

    //}
}


pub fn custom_cubic_inverse_2(
    _cubic_range_0: &mut Vec<u8>,
    _quad_range_0: & Vec<u8>,
    _quad_range_1: & Vec<u8>,
    _quad_range_2: & Vec<u8>,
    _quad_range_3: & Vec<u8>
    ) {

    let t6 = parse_quad_from_bytes_new(&_quad_range_3);
    let s0 = parse_quad_from_bytes_new(& _quad_range_0);
    let s1 = parse_quad_from_bytes_new(& _quad_range_1);
    let s2 = parse_quad_from_bytes_new(& _quad_range_2);
    let c0 = t6 * &s0;//      cost 13698
    ////sol_log_compute_units();
    ////msg!"c1 ");
    let c1 = t6 * &s1;//      cost 13790
    ////sol_log_compute_units();
    ////msg!"c2 ");
    let c2 = t6 * &s2;//      cost 13790
    ////sol_log_compute_units();
    parse_cubic_to_bytes_new(ark_ff::fields::models::cubic_extension::CubicExtField::<ark_ff::fields::models::fp6_3over2::Fp6ParamsWrapper<ark_bls12_381::Fq6Parameters>>::new(c0, c1, c2), _cubic_range_0, solo_cubic_0_range);

}

pub fn custom_quadratic_fp384_inverse_1(
    _quad_range_3: &Vec<u8>,
    _fp384_range: &mut Vec<u8>) {
    //if account[quad_range_3[0]..quad_range_3[1]] == [0;(quad_range_3[1] - quad_range_3[0])] {
        //returns none in arkworks maybe we should add a byte to signal a crash
    //} else {
        // Guide to Pairing-based Cryptography, Algorithm 5.19.
        let f = parse_quad_from_bytes_new(_quad_range_3);
        let v1 = f.c1.square();//      cost 3659

        //   cost 184974
        let v0 = <ark_ff::fields::models::fp2::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>  as QuadExtParameters>::sub_and_mul_base_field_by_nonresidue(&f.c0.square(), &v1);
        parse_fp384_to_bytes_new(v0,_fp384_range);
    //}
}
pub fn custom_quadratic_fp384_inverse_2(
        _quad_range_3: &mut Vec<u8>,
        _fp384_range: &Vec<u8>
        ) {

    let v0 = parse_fp384_from_bytes_new(&_fp384_range);
    let f = parse_quad_from_bytes_new(&_quad_range_3);
    v0.inverse().map(|v1| { //      cost 181186

        //   cost 184974

        let c0 = f.c0 * &v1;//      cost 4367
        ////sol_log_compute_units();
        ////msg!"c1");
        let c1 = -(f.c1 * &v1);//   cost 4471
        ////sol_log_compute_units();

        let res = QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(c0, c1);
        parse_quad_to_bytes_new(res, _quad_range_3);
    });

}
