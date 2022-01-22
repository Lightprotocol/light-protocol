use crate::groth16_verifier::{
    final_exponentiation::{ranges::*, state::FinalExpBytes},
    parsers::{
        parse_cubic_from_bytes_sub, parse_cubic_to_bytes_sub, parse_f_from_bytes, parse_f_to_bytes,
        parse_fp256_from_bytes, parse_fp256_to_bytes, parse_quad_from_bytes, parse_quad_to_bytes,
    },
};

use ark_ec;
use ark_ff::{
    fields::models::{
        cubic_extension::CubicExtParameters,
        quadratic_extension::{QuadExtField, QuadExtParameters},
    },
    Field,
};

use solana_program::{log::sol_log_compute_units, msg, program_error::ProgramError};

pub fn verify_result(main_account_data: &FinalExpBytes) -> Result<(), ProgramError> {
    let pvk = vec![
        198, 242, 4, 28, 9, 35, 146, 101, 152, 133, 231, 128, 253, 46, 174, 170, 116, 96, 135, 45,
        77, 156, 161, 40, 238, 232, 55, 247, 15, 79, 136, 20, 73, 78, 229, 119, 48, 86, 133, 39,
        142, 172, 194, 67, 33, 2, 66, 111, 127, 20, 159, 85, 92, 82, 21, 187, 149, 99, 99, 91, 169,
        57, 127, 10, 238, 159, 54, 204, 152, 63, 242, 50, 16, 39, 141, 61, 149, 81, 36, 246, 69, 1,
        232, 157, 153, 3, 1, 25, 105, 84, 109, 205, 9, 78, 8, 26, 113, 240, 149, 249, 171, 170, 41,
        39, 144, 143, 89, 229, 207, 106, 60, 195, 236, 5, 73, 82, 126, 170, 50, 181, 192, 135, 129,
        217, 185, 227, 223, 0, 50, 203, 114, 165, 128, 252, 58, 245, 74, 48, 92, 144, 199, 108,
        126, 82, 103, 46, 23, 236, 159, 71, 113, 45, 183, 105, 200, 135, 142, 182, 196, 3, 138,
        113, 217, 236, 105, 118, 157, 226, 54, 90, 23, 215, 59, 110, 169, 133, 96, 175, 12, 86, 33,
        94, 130, 8, 57, 246, 139, 86, 246, 147, 174, 17, 57, 27, 122, 247, 174, 76, 162, 173, 26,
        134, 230, 177, 70, 148, 183, 2, 54, 46, 65, 165, 64, 15, 42, 11, 245, 15, 136, 32, 213,
        228, 4, 27, 176, 63, 169, 82, 178, 89, 227, 58, 204, 40, 159, 210, 216, 255, 223, 194, 117,
        203, 57, 49, 152, 42, 162, 80, 248, 55, 92, 240, 231, 192, 161, 14, 169, 65, 231, 215, 238,
        131, 144, 139, 153, 142, 76, 100, 40, 134, 147, 164, 89, 148, 195, 194, 117, 36, 53, 100,
        231, 61, 164, 217, 129, 190, 160, 44, 30, 94, 13, 159, 6, 83, 126, 195, 26, 86, 113, 177,
        101, 79, 110, 143, 220, 57, 110, 235, 91, 73, 189, 191, 253, 187, 76, 214, 232, 86, 132, 6,
        135, 153, 111, 175, 12, 109, 157, 73, 181, 171, 29, 118, 147, 102, 65, 153, 99, 57, 198,
        45, 85, 153, 67, 208, 177, 113, 205, 237, 210, 233, 79, 46, 231, 168, 16, 11, 21, 249, 174,
        127, 70, 3, 32, 60, 115, 188, 192, 101, 159, 85, 66, 193, 194, 157, 76, 121, 108, 222, 128,
        27, 15, 163, 156, 8,
    ];

    if pvk != main_account_data.y1_range_s {
        msg!("verification failed");
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}
//conjugate should work onyl wrapper
pub fn conjugate_wrapper(_range: &mut Vec<u8>) {
    msg!("conjugate_wrapper: ------------------------------------------");
    let mut f = parse_f_from_bytes(_range);
    f.conjugate();
    parse_f_to_bytes(f, _range);
}

//multiplication

//85282
pub fn mul_assign_1(
    _cubic: &Vec<u8>,
    _cubic_0_range: [usize; 2],
    _cubic_other: &Vec<u8>,
    _range_other_c0: [usize; 2],
    _store_cubic: &mut Vec<u8>,
    _store_cubic_range: [usize; 2],
) {
    msg!("mul_assign_1: ------------------------------------------");

    let f_c0 = parse_cubic_from_bytes_sub(&_cubic, _cubic_0_range); //30000
                                                                    //println!("str f_c0 = {:?}", f_c0);
    let other_c0 = parse_cubic_from_bytes_sub(&_cubic_other, _range_other_c0); //30000
                                                                               //println!("str other_c0 = {:?}", other_c0);

    let v0 = f_c0 * &other_c0; //   cost 86548
                               //println!("str v0 = {:?}", v0);
    parse_cubic_to_bytes_sub(v0, _store_cubic, _store_cubic_range); //15000
                                                                    //total 162000
}

//84953
pub fn mul_assign_2(
    _cubic: &Vec<u8>,
    _cubic_1_range: [usize; 2],
    _cubic_other: &Vec<u8>,
    range_other_c1: [usize; 2],
    _store_cubic: &mut Vec<u8>,
    _store_cubic_range: [usize; 2],
) {
    let f_c1 = parse_cubic_from_bytes_sub(&_cubic, _cubic_1_range); //30000
    let other_c1 = parse_cubic_from_bytes_sub(&_cubic_other, range_other_c1); //30000

    let v1 = f_c1 * &other_c1; //86827
                               //println!("v1 = {:?}", v1);
    parse_cubic_to_bytes_sub(v1, _store_cubic, _store_cubic_range); //15000
                                                                    //total 193000
}

pub fn mul_assign_1_2(
    _f_range: &Vec<u8>,
    _other: &Vec<u8>,
    _store_cubic0: &mut Vec<u8>,
    _store_cubic1: &mut Vec<u8>,
) {
    //msg!("mul_assign_1_2: ------------------------------------------");

    let f = parse_f_from_bytes(&_f_range);
    let other = parse_f_from_bytes(&_other);

    let v0 = f.c0 * &other.c0;

    //let mut f_c1 = parse_cubic_from_bytes_sub(&_cubic1, _cubic_1_range1);
    //let other_c1 = parse_cubic_from_bytes_sub(&_cubic_other1, range_other_c1);

    let v1 = f.c1 * &other.c1;

    parse_cubic_to_bytes_sub(v0, _store_cubic0, SOLO_CUBIC_0_RANGE); //15000
    parse_cubic_to_bytes_sub(v1, _store_cubic1, SOLO_CUBIC_0_RANGE);
}

//52545 - 15 = 37
pub fn mul_assign_3(_f_range: &mut Vec<u8>) {
    //msg!("mul_assign_3: ------------------------------------------");

    let mut f = parse_f_from_bytes(&_f_range); //60000
    f.c1 += &f.c0; //<1000
    parse_f_to_bytes(f, _f_range); //30000
                                   //total 93000
}

pub fn mul_assign_3_4_5(
    _f_range_other: &Vec<u8>,
    _cubic_range_0: &Vec<u8>,
    _cubic_range_1: &Vec<u8>,
    _f_range: &mut Vec<u8>,
) {
    //msg!("mul_assign_3_4_5: ------------------------------------------");

    //3
    let mut f = parse_f_from_bytes(&_f_range);
    f.c1 += &f.c0;

    //4
    let other = parse_f_from_bytes(&_f_range_other);

    f.c1 *= &(other.c0 + &other.c1);

    //5
    let v0 = parse_cubic_from_bytes_sub(&_cubic_range_0, SOLO_CUBIC_0_RANGE); //30000
    let v1 = parse_cubic_from_bytes_sub(&_cubic_range_1, SOLO_CUBIC_0_RANGE); //30000
    f.c1 -= &v0;
    f.c1 -= &v1;
    f.c0 = <ark_ff::fields::models::fp12_2over3over2::Fp12ParamsWrapper::<ark_bn254::Fq12Parameters>  as QuadExtParameters>::add_and_mul_base_field_by_nonresidue(&v0, &v1); //1000

    parse_f_to_bytes(f, _f_range);
}
//45031 - 8k = 37
pub fn mul_assign_4_1(_f_range_other: &Vec<u8>, _store_cubic_range: &mut Vec<u8>) {
    msg!("mul_assign_4_1: ------------------------------------------");

    //let mut f = parse_f_from_bytes(&account[..], range_f);
    //let mut f_c1 = parse_cubic_from_bytes_sub(&account[..], _f1_r_cubic_1_range); //30000
    let other = parse_f_from_bytes(&_f_range_other); //60000
                                                     //println!("str Other res : {:?}", (other.c0 + other.c1));

    //f.c1 += &f.c0;
    //
    //////msg!"4");
    let tmp = &(other.c0 + &other.c1); //87075
    parse_cubic_to_bytes_sub(*tmp, _store_cubic_range, SOLO_CUBIC_0_RANGE); //15000
                                                                            //195000
}

//85164 - 27 - 8  = 50
pub fn mul_assign_4_2(
    _f1_r_range: &mut Vec<u8>,
    _f1_r_cubic_1_range: [usize; 2],
    _store_cubic_range: &Vec<u8>,
) {
    msg!("mul_assign_4_2: ------------------------------------------");

    //let mut f = parse_f_from_bytes(&account[..], range_f);
    let mut f_c1 = parse_cubic_from_bytes_sub(&_f1_r_range, _f1_r_cubic_1_range); //30000
    let other_cs = parse_cubic_from_bytes_sub(&_store_cubic_range, SOLO_CUBIC_0_RANGE); //60000
                                                                                        //println!("str loaded other res : {:?}", other_cs);

    //f.c1 += &f.c0;
    //if these logs are removed a stackoverflow might occur in the overall program
    sol_log_compute_units();
    //msg!("mulassing 4_2 0");
    f_c1 *= other_cs;
    sol_log_compute_units();
    //87075
    parse_cubic_to_bytes_sub(f_c1, _f1_r_range, _f1_r_cubic_1_range);
    //msg!("mulassing 4_2 1");
    sol_log_compute_units();

    //assert_eq!(f_c1, parse_cubic_from_bytes_sub(&_f1_r_range, _f1_r_cubic_1_range));           //15000
    //195000
}

//81001 - 27
pub fn mul_assign_5(_f1_r_range: &mut Vec<u8>, _cubic_range_0: &Vec<u8>, _cubic_range_1: &Vec<u8>) {
    msg!("mul_assign_5: ------------------------------------------");

    //--------------- additional split -------------------------
    let mut f = parse_f_from_bytes(&_f1_r_range); //60000
    let v0 = parse_cubic_from_bytes_sub(&_cubic_range_0, SOLO_CUBIC_0_RANGE); //30000
    let v1 = parse_cubic_from_bytes_sub(&_cubic_range_1, SOLO_CUBIC_0_RANGE); //30000
                                                                              //////msg!"5");
    f.c1 -= &v0; //<1000
                 //
                 //////msg!"6");
    f.c1 -= &v1; //<1000
                 //
                 //println!("multi assign: {:?}", f);
                 //////msg!"7");
    f.c0 = <ark_ff::fields::models::fp12_2over3over2::Fp12ParamsWrapper::<ark_bn254::Fq12Parameters>  as QuadExtParameters>::add_and_mul_base_field_by_nonresidue(&v0, &v1); //1000

    parse_f_to_bytes(f, _f1_r_range); //30000
                                      //153000
}

//frobenius_map should work only wrapper
pub fn custom_frobenius_map_1(account: &mut Vec<u8>) {
    let mut f = parse_f_from_bytes(&account);
    f.frobenius_map(1);
    parse_f_to_bytes(f, account);
}

pub fn custom_frobenius_map_2(account: &mut Vec<u8>) {
    let mut f = parse_f_from_bytes(&account);
    f.frobenius_map(2);
    parse_f_to_bytes(f, account);
}

pub fn custom_frobenius_map_3(account: &mut Vec<u8>) {
    let mut f = parse_f_from_bytes(&account);
    f.frobenius_map(3);
    parse_f_to_bytes(f, account);
}

/*exp_by_neg_x is cyclotomic_exp plus possible conjugate
* cyclotomic_exp is:
* - conjugate
* - loop
*   - if naf non-zero -> cyclotomic_square_in_place
*       - only enters if characteristic_square_mod_6_is_one
*   - if part with mul_assign
*/

pub fn custom_cyclotomic_square_in_place(_store_f_range: &mut Vec<u8>) {
    msg!("custom_cyclotomic_square_in_place: ------------------------------------------ ");

    let mut f = parse_f_from_bytes(&_store_f_range);
    // cost 46000
    f.cyclotomic_square_in_place();

    parse_f_to_bytes(f, _store_f_range);
}
//cyclotomic_square should work only wrapper

pub fn custom_cyclotomic_square(_f_range: &Vec<u8>, _store_f_range: &mut Vec<u8>) {
    msg!("custom_cyclotomic_square: ------------------------------------------ ");
    let f = parse_f_from_bytes(&_f_range);
    // cost 90464
    let y0 = f.cyclotomic_square();
    //assert!(f != y0);

    parse_f_to_bytes(y0, _store_f_range);
}

//inverse works
pub fn custom_f_inverse_1(_f_f2_range: &Vec<u8>, _cubic_range_1: &mut Vec<u8>) {
    //first part of calculating the inverse to f
    msg!("custom_f_inverse_1: ------------------------------------------ ");

    let f = parse_f_from_bytes(_f_f2_range);
    let v1 = f.c1.square();
    parse_cubic_to_bytes_sub(v1, _cubic_range_1, SOLO_CUBIC_0_RANGE);
}

pub fn custom_f_inverse_2(
    _f_f2_range: &Vec<u8>,
    _cubic_range_0: &mut Vec<u8>,
    _cubic_range_1: &Vec<u8>,
) {
    msg!("custom_f_inverse_2: ------------------------------------------ ");

    let f = parse_f_from_bytes(_f_f2_range);
    let v1 = parse_cubic_from_bytes_sub(_cubic_range_1, SOLO_CUBIC_0_RANGE);
    // cost 58976
    let v0 = <ark_ff::fields::models::fp12_2over3over2::Fp12ParamsWrapper::<ark_bn254::Fq12Parameters>  as QuadExtParameters>::sub_and_mul_base_field_by_nonresidue(&f.c0.square(), &v1);
    parse_cubic_to_bytes_sub(v0, _cubic_range_0, SOLO_CUBIC_0_RANGE);
}

pub fn custom_f_inverse_3(
    _cubic_range_1: &mut Vec<u8>,
    _cubic_range_0: &Vec<u8>,
    _f_f2_range: &Vec<u8>,
) {
    msg!("custom_f_inverse_3: ------------------------------------------ ");

    let v1 = parse_cubic_from_bytes_sub(_cubic_range_0, SOLO_CUBIC_0_RANGE);
    let f_c0 = parse_cubic_from_bytes_sub(_f_f2_range, F_CUBIC_0_RANGE);

    let c0 = f_c0 * &v1; //      cost 87545

    parse_cubic_to_bytes_sub(c0, _cubic_range_1, SOLO_CUBIC_0_RANGE);
    //assert_eq!(c0, parse_cubic_from_bytes_sub(account, cubic_range_1));
}

pub fn custom_f_inverse_4(_cubic: &mut Vec<u8>, _f_f2_range: &Vec<u8>) {
    msg!("custom_f_inverse_4: ------------------------------------------ ");

    let v1 = parse_cubic_from_bytes_sub(_cubic, SOLO_CUBIC_0_RANGE); //30
    let f_c1 = parse_cubic_from_bytes_sub(_f_f2_range, F_CUBIC_1_RANGE); //30
    let c1 = -(f_c1 * &v1); //   cost 86867
    parse_cubic_to_bytes_sub(c1, _cubic, SOLO_CUBIC_0_RANGE);
}

pub fn custom_f_inverse_5(_cubic_0: &Vec<u8>, _cubic_1: &Vec<u8>, _f_f2_range: &mut Vec<u8>) {
    msg!("custom_f_inverse_5: ------------------------------------------ ");

    let c0 = parse_cubic_from_bytes_sub(_cubic_1, SOLO_CUBIC_0_RANGE); //30
    let c1 = parse_cubic_from_bytes_sub(_cubic_0, SOLO_CUBIC_0_RANGE); //30
    parse_f_to_bytes(
        <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::new(c0, c1),
        _f_f2_range,
    ); //30
}

pub fn custom_cubic_inverse_1(
    _cubic_range_0: &Vec<u8>,
    _quad_range_0: &mut Vec<u8>,
    _quad_range_1: &mut Vec<u8>,
    _quad_range_2: &mut Vec<u8>,
    _quad_range_3: &mut Vec<u8>,
) {
    msg!("custom_cubic_inverse_1: ------------------------------------------ ");

    let f = parse_cubic_from_bytes_sub(_cubic_range_0, SOLO_CUBIC_0_RANGE);
    // From "High-Speed Software Implementation of the Optimal Ate AbstractPairing
    // over
    // Barreto-Naehrig Curves"; Algorithm 17
    ////msg!"cubic inverse ");
    ////sol_log_compute_units();
    ////msg!"t0 ");
    let t0 = f.c0.square(); //  cost 9188
                            ////sol_log_compute_units();
                            ////msg!"t1 ");
    let t1 = f.c1.square(); //  cost 9255 cumulative 18443
                            ////sol_log_compute_units();
                            ////msg!"t2 ");
    let t2 = f.c2.square(); //   cost 9255 cumulative 27698
                            ////sol_log_compute_units();
                            ////msg!"t3 ");
    let t3 = f.c0 * &f.c1; //    cost 13790 cumulative 41488
                           ////sol_log_compute_units();
                           ////msg!"t4 ");
    let t4 = f.c0 * &f.c2; //    cost 13789 cumulative 55277
                           ////sol_log_compute_units();
                           ////msg!"t5 ");
    let t5 = f.c1 * &f.c2; //    cost 13791 cumulative 69068
                           ////sol_log_compute_units();
                           ////msg!"n5 ");
                           //    cost 270 cumulative 69350
    let n5 = <ark_ff::fields::models::fp6_3over2::Fp6ParamsWrapper::<ark_bn254::Fq6Parameters>  as CubicExtParameters>::mul_base_field_by_nonresidue(&t5);
    ////sol_log_compute_units();
    ////msg!"s0");
    let s0 = t0 - &n5; //      cost 255 cumulative 69593
                       ////sol_log_compute_units();
                       ////msg!"s1 ");
                       //    cost 505 cumulative 70098
    let s1 = <ark_ff::fields::models::fp6_3over2::Fp6ParamsWrapper::<ark_bn254::Fq6Parameters>  as CubicExtParameters>::mul_base_field_by_nonresidue(&t2) - &t3;
    ////sol_log_compute_units();
    ////msg!"s2 ");        //      cost 265 cumulative 70363
    let s2 = t1 - &t4; // typo in paper referenced above. should be "-" as per Scott, but is "*"

    ////sol_log_compute_units();
    ////msg!"a1 ");
    let a1 = f.c2 * &s1; //     cost 13791 cumulative 84154
                         ////sol_log_compute_units();
                         ////msg!"a2 ");
    let a2 = f.c1 * &s2; //      cost 13791 cumulative 97945
                         ////sol_log_compute_units();
                         ////msg!"a3 ");
    let mut a3 = a1 + &a2; //    cost 182 cumulative 98127
                           ////sol_log_compute_units();
                           ////msg!"a3.1 ");
                           // cost 297 cumulative 98424
    a3 = <ark_ff::fields::models::fp6_3over2::Fp6ParamsWrapper::<ark_bn254::Fq6Parameters>  as CubicExtParameters>::mul_base_field_by_nonresidue(&a3);

    //13895
    let t6_input = f.c0 * &s0 + &a3;

    parse_quad_to_bytes(s0, _quad_range_0);
    parse_quad_to_bytes(s1, _quad_range_1);
    parse_quad_to_bytes(s2, _quad_range_2);
    parse_quad_to_bytes(t6_input, _quad_range_3);

    //}
}

pub fn custom_cubic_inverse_2(
    _cubic_range_0: &mut Vec<u8>,
    _quad_range_0: &Vec<u8>,
    _quad_range_1: &Vec<u8>,
    _quad_range_2: &Vec<u8>,
    _quad_range_3: &Vec<u8>,
) {
    msg!("custom_cubic_inverse_2: ------------------------------------------ ");

    let t6 = parse_quad_from_bytes(_quad_range_3);
    let s0 = parse_quad_from_bytes(_quad_range_0);
    let s1 = parse_quad_from_bytes(_quad_range_1);
    let s2 = parse_quad_from_bytes(_quad_range_2);
    let c0 = t6 * &s0; //      cost 13698
                       ////sol_log_compute_units();
                       ////msg!"c1 ");
    let c1 = t6 * &s1; //      cost 13790
                       ////sol_log_compute_units();
                       ////msg!"c2 ");
    let c2 = t6 * &s2; //      cost 13790
                       ////sol_log_compute_units();
    parse_cubic_to_bytes_sub(
        ark_ff::fields::models::cubic_extension::CubicExtField::<
            ark_ff::fields::models::fp6_3over2::Fp6ParamsWrapper<ark_bn254::Fq6Parameters>,
        >::new(c0, c1, c2),
        _cubic_range_0,
        SOLO_CUBIC_0_RANGE,
    );
}

pub fn custom_quadratic_fp256_inverse_1(_quad_range_3: &Vec<u8>, _fp384_range: &mut Vec<u8>) {
    //if account[quad_range_3[0]..quad_range_3[1]] == [0;(quad_range_3[1] - quad_range_3[0])] {
    //returns none in arkworks maybe we should add a byte to signal a crash
    //} else {
    // Guide to Pairing-based Cryptography, Algorithm 5.19.
    msg!("custom_quadratic_fp256_inverse_1: ------------------------------------------ ");
    let f = parse_quad_from_bytes(_quad_range_3);
    let v1 = f.c1.square(); //      cost 3659

    //   cost 184974
    let v0 = <ark_ff::fields::models::fp2::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>  as QuadExtParameters>::sub_and_mul_base_field_by_nonresidue(&f.c0.square(), &v1);
    parse_fp256_to_bytes(v0, _fp384_range);
    //}
}

pub fn custom_quadratic_fp256_inverse_2(_quad_range_3: &mut Vec<u8>, _fp384_range: &Vec<u8>) {
    msg!("custom_quadratic_fp256_inverse_2: ------------------------------------------ ");

    let v0 = parse_fp256_from_bytes(_fp384_range);
    let f = parse_quad_from_bytes(_quad_range_3);
    v0.inverse().map(|v1| {
        //      cost 181186

        //   cost 184974

        let c0 = f.c0 * &v1; //      cost 4367
                             ////sol_log_compute_units();
                             ////msg!"c1");
        let c1 = -(f.c1 * &v1); //   cost 4471
                                ////sol_log_compute_units();

        let res = QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(c0, c1);
        parse_quad_to_bytes(res, _quad_range_3);
    });
}

/*
pub fn verify_result_and_withdraw(_f1_r_range: &Vec<u8>, account_from: &AccountInfo, account_to: &AccountInfo) {

    //let verifyingkey = get_alpha_g1_beta_g2(); // new verif key val
    let result = parse_f_from_bytes(_f1_r_range);


    assert_eq!(
        result,
        QuadExtField::<ark_ff::Fp12ParamsWrapper::<ark_bls12_381::Fq12Parameters>>::new(
        CubicExtField::<ark_ff::Fp6ParamsWrapper::<ark_bls12_381::Fq6Parameters>>::new(
        QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([9450009823341693778, 7567869168370272456, 18197991852398659280, 14366248363560943150, 1846449964006099810, 679186062217654437])),
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([14423346876892333681, 6730281363705116644, 7424630442455670057, 8270520680913369906, 129074939192573571, 1189318454514002421]))
        ),
        QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([15087111543975820103, 4158933278356405436, 18118066973472597158, 16470418406097340038, 7290763036760663413, 1432986065586497656])),
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([13142509635494830254, 1063120006913468487, 9502319634834262306, 4432211635827318749, 14796654634504989696, 1613239985088916413]))
        ),
        QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([8540514875150680179, 14211944784949309652, 11503029521577459949, 7848771183824012788, 12423583728904914117, 214651426791184375])),
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([12184631478503290601, 3754734004613789564, 18017661725911600748, 2523454429955899187, 477613221123976217, 1872837268341302997]))
        )
        ),
        CubicExtField::<ark_ff::Fp6ParamsWrapper::<ark_bls12_381::Fq6Parameters>>::new(
        QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([3978527081928063346, 16585342381569337288, 5542417872902215541, 14433809004088446654, 4002383117313680726, 1825637594027485805])),
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([12282572081966365605, 4130176373697380094, 11274978495552272244, 12480637929685418438, 8247324291841073554, 1292512561380557691]))
        ),
        QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([14568481364640261896, 13024973709310556877, 388942695468569842, 5165814615778255344, 5461422625881874726, 998729394980441256])),
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([2915405947247214443, 16476827183007826276, 397625141714225917, 7683936378748485792, 7647979677775928833, 1072503992333817859]))
        ),
        QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([16050027345311255108, 10820284375719774599, 7851447608520829924, 16313935448547715969, 6657343977525303700, 224196233976581852])),
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([16673765787484129697, 12992430675039131019, 2485465101194175871, 15715616086842934831, 1003336842210694614, 1058233574352048271]))
    ))), "VERIFICATION FAILED"
    );


    **account_from.try_borrow_mut_lamports().unwrap()   -= 1000000000; // 1 SOL

    **account_to.try_borrow_mut_lamports().unwrap()     += 1000000000;
    msg!("Final Exp successful");

    msg!("Transfer of 1 Sol successful to {:?}", account_to.key);

}
*/

#[cfg(test)]
mod tests {
    use crate::groth16_verifier::final_exponentiation::instructions::{
        conjugate_wrapper, custom_cubic_inverse_1, custom_cubic_inverse_2,
        custom_cyclotomic_square, custom_cyclotomic_square_in_place, custom_f_inverse_1,
        custom_f_inverse_2, custom_f_inverse_3, custom_f_inverse_4, custom_f_inverse_5,
        custom_frobenius_map_1, custom_frobenius_map_2, custom_frobenius_map_3,
        custom_quadratic_fp256_inverse_1, custom_quadratic_fp256_inverse_2, mul_assign_1,
        mul_assign_1_2, mul_assign_2, mul_assign_3, mul_assign_3_4_5, mul_assign_4_1,
        mul_assign_4_2, mul_assign_5,
    };

    use crate::groth16_verifier::final_exponentiation::state::FinalExpBytes;

    use crate::groth16_verifier::parsers::{
        parse_cubic_from_bytes_sub, parse_f_from_bytes, parse_f_to_bytes, parse_quad_from_bytes,
    };

    use crate::groth16_verifier::final_exponentiation::ranges::{
        F_CUBIC_0_RANGE, F_CUBIC_1_RANGE, NAF_VEC, SOLO_CUBIC_0_RANGE,
    };

    use ark_ec::bn::BnParameters;
    use ark_ff::{Field, Fp12};
    use ark_std::{test_rng, One, UniformRand, Zero};

    #[test]
    fn frobenius_map_test_correct() {
        //generating input
        for i in 1..4 {
            let mut rng = test_rng();
            let mut reference_f =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                    &mut rng,
                );

            let mut actual_f = reference_f.clone();
            //simulating the onchain account
            let mut account = vec![0u8; 384];
            parse_f_to_bytes(actual_f, &mut account);
            if i == 1 {
                custom_frobenius_map_1(&mut account);
            } else if i == 2 {
                custom_frobenius_map_2(&mut account);
            } else {
                custom_frobenius_map_3(&mut account);
            }

            //generating reference value for comparison
            reference_f.frobenius_map(i);
            assert_eq!(reference_f, parse_f_from_bytes(&account));
        }
    }
    /*
    #[test]
    fn frobenius_map_test_fails() {
        //generating input
        for i in 1..4 {
            let mut rng = test_rng();
            let mut reference_f = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(&mut rng);

            let mut actual_f = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(&mut rng);
            //simulating the onchain account
            let mut account = vec![0u8;384];
            parse_f_to_bytes(actual_f, &mut account);
            if i == 1 {
                custom_frobenius_map_1_1(&mut account);
                custom_frobenius_map_1_2(&mut account);

            } else if i == 2 {
                custom_frobenius_map_2_1(&mut account);
                custom_frobenius_map_2_2(&mut account);

            } else {
                custom_frobenius_map_3_1(&mut account);
                custom_frobenius_map_3_2(&mut account);

            }

            //generating reference value for comparison
            reference_f.frobenius_map(i);
            assert!(reference_f != parse_f_from_bytes(&account));
        }

    }
    */
    #[test]
    fn custom_cyclotomic_square_test_correct() {
        //generating input

        let mut rng = test_rng();
        let mut reference_f =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                &mut rng,
            );

        let mut actual_f = reference_f.clone();
        //simulating the onchain account
        let mut account = vec![0u8; 384];
        parse_f_to_bytes(actual_f, &mut account);
        let account_tmp = account.clone();
        custom_cyclotomic_square(&account_tmp, &mut account);
        let account_tmp = account.clone();
        custom_cyclotomic_square(&account_tmp, &mut account);

        //generating reference value for comparison
        reference_f = reference_f.cyclotomic_square();
        reference_f = reference_f.cyclotomic_square();
        assert_eq!(reference_f, parse_f_from_bytes(&account));
    }

    #[test]
    fn custom_cyclotomic_square_test_fails() {
        //generating input
        let mut rng = test_rng();
        let mut reference_f =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                &mut rng,
            );

        let mut actual_f = reference_f.clone();
        //simulating the onchain account
        let mut account = vec![0u8; 384];
        parse_f_to_bytes(actual_f, &mut account);
        let account_tmp = account.clone();
        custom_cyclotomic_square(&account_tmp, &mut account);
        let account_tmp = account.clone();
        custom_cyclotomic_square(&account_tmp, &mut account);

        //generating reference value for comparison
        reference_f = reference_f.cyclotomic_square();
        //reference_f = reference_f.cyclotomic_square();
        assert!(reference_f != parse_f_from_bytes(&account));
    }

    #[test]
    fn conjugate_test_correct() {
        //generating input

        let mut rng = test_rng();
        for i in 0..10 {
            let mut reference_f =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                    &mut rng,
                );

            let mut actual_f = reference_f.clone();
            //simulating the onchain account
            let mut account = vec![0u8; 384];
            parse_f_to_bytes(actual_f, &mut account);
            conjugate_wrapper(&mut account);

            //generating reference value for comparison
            reference_f.conjugate();
            assert_eq!(reference_f, parse_f_from_bytes(&account));
        }
    }

    #[test]
    fn conjugate_test_fails() {
        //generating input

        let mut rng = test_rng();
        for i in 0..10 {
            let mut reference_f =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                    &mut rng,
                );

            let mut actual_f =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                    &mut rng,
                );

            //simulating the onchain account
            let mut account = vec![0u8; 384];
            parse_f_to_bytes(actual_f, &mut account);
            conjugate_wrapper(&mut account);

            //generating reference value for comparison
            reference_f.conjugate();
            assert!(reference_f != parse_f_from_bytes(&account));
        }
    }

    #[test]
    fn custom_inverse_test_correct() {
        let mut rng = test_rng();
        let mut reference_f =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                &mut rng,
            );
        let mut actual_f = reference_f.clone();
        let mut account_struct = FinalExpBytes::new();

        parse_f_to_bytes(actual_f, &mut account_struct.f_f2_range_s);

        //1 ---------------------------------------------
        custom_f_inverse_1(
            &account_struct.f_f2_range_s,
            &mut account_struct.cubic_range_1_s,
        );

        //2 ---------------------------------------------
        custom_f_inverse_2(
            &account_struct.f_f2_range_s,
            &mut account_struct.cubic_range_0_s,
            &account_struct.cubic_range_1_s,
        );
        let cubic = parse_cubic_from_bytes_sub(&account_struct.cubic_range_0_s, SOLO_CUBIC_0_RANGE);

        //3 ---------------------------------------------
        custom_cubic_inverse_1(
            &account_struct.cubic_range_0_s,
            &mut account_struct.quad_range_0_s,
            &mut account_struct.quad_range_1_s,
            &mut account_struct.quad_range_2_s,
            &mut account_struct.quad_range_3_s,
        );

        //quad works
        let quad = parse_quad_from_bytes(&account_struct.quad_range_3_s);

        //4 ---------------------------------------------
        //quad inverse is part of cubic Inverse
        custom_quadratic_fp256_inverse_1(
            &account_struct.quad_range_3_s,
            &mut account_struct.fp384_range_s,
        );

        //5 ---------------------------------------------
        custom_quadratic_fp256_inverse_2(
            &mut account_struct.quad_range_3_s,
            &account_struct.fp384_range_s,
        );

        assert_eq!(
            quad.inverse().unwrap(),
            parse_quad_from_bytes(&account_struct.quad_range_3_s),
            "quad inverse failed"
        );

        //6 ---------------------------------------------
        custom_cubic_inverse_2(
            &mut account_struct.cubic_range_0_s,
            &account_struct.quad_range_0_s,
            &account_struct.quad_range_1_s,
            &account_struct.quad_range_2_s,
            &account_struct.quad_range_3_s,
        );
        assert_eq!(
            cubic.inverse().unwrap(),
            parse_cubic_from_bytes_sub(&account_struct.cubic_range_0_s, SOLO_CUBIC_0_RANGE),
            "cubic inverse failed"
        );

        //7 ---------------------------------------------
        custom_f_inverse_3(
            &mut account_struct.cubic_range_1_s,
            &account_struct.cubic_range_0_s,
            &account_struct.f_f2_range_s,
        );

        //8 ---------------------------------------------
        custom_f_inverse_4(
            &mut account_struct.cubic_range_0_s,
            &account_struct.f_f2_range_s,
        );

        //9 ---------------------------------------------
        custom_f_inverse_5(
            &account_struct.cubic_range_0_s,
            &account_struct.cubic_range_1_s,
            &mut account_struct.f_f2_range_s,
        );
        //reference_f;
        //println!("{:?}", reference_f);
        assert_eq!(
            reference_f.inverse().unwrap(),
            parse_f_from_bytes(&account_struct.f_f2_range_s),
            "f inverse failed"
        );
    }

    #[test]
    fn custom_inverse_test_fails() {
        let mut rng = test_rng();
        let mut reference_f =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                &mut rng,
            );
        let mut actual_f =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                &mut rng,
            );
        let mut account_struct = FinalExpBytes::new();

        parse_f_to_bytes(actual_f, &mut account_struct.f_f2_range_s);

        //1 ---------------------------------------------
        custom_f_inverse_1(
            &account_struct.f_f2_range_s,
            &mut account_struct.cubic_range_1_s,
        );

        //2 ---------------------------------------------
        custom_f_inverse_2(
            &account_struct.f_f2_range_s,
            &mut account_struct.cubic_range_0_s,
            &account_struct.cubic_range_1_s,
        );

        //3 ---------------------------------------------
        custom_cubic_inverse_1(
            &account_struct.cubic_range_0_s,
            &mut account_struct.quad_range_0_s,
            &mut account_struct.quad_range_1_s,
            &mut account_struct.quad_range_2_s,
            &mut account_struct.quad_range_3_s,
        );

        //4 ---------------------------------------------
        //quad inverse is part of cubic Inverse
        custom_quadratic_fp256_inverse_1(
            &account_struct.quad_range_3_s,
            &mut account_struct.fp384_range_s,
        );

        //5 ---------------------------------------------
        custom_quadratic_fp256_inverse_2(
            &mut account_struct.quad_range_3_s,
            &account_struct.fp384_range_s,
        );

        //6 ---------------------------------------------
        custom_cubic_inverse_2(
            &mut account_struct.cubic_range_0_s,
            &account_struct.quad_range_0_s,
            &account_struct.quad_range_1_s,
            &account_struct.quad_range_2_s,
            &account_struct.quad_range_3_s,
        );

        //7 ---------------------------------------------
        custom_f_inverse_3(
            &mut account_struct.cubic_range_1_s,
            &account_struct.cubic_range_0_s,
            &account_struct.f_f2_range_s,
        );

        //8 ---------------------------------------------
        custom_f_inverse_4(
            &mut account_struct.cubic_range_0_s,
            &account_struct.f_f2_range_s,
        );

        //9 ---------------------------------------------
        custom_f_inverse_5(
            &account_struct.cubic_range_0_s,
            &account_struct.cubic_range_1_s,
            &mut account_struct.f_f2_range_s,
        );
        //reference_f;
        //println!("{:?}", reference_f);
        assert!(
            reference_f.inverse().unwrap() != parse_f_from_bytes(&account_struct.f_f2_range_s),
            "f inverse failed"
        );
    }

    #[test]
    fn mul_assign_test_correct() {
        let mut rng = test_rng();
        let mut reference_f =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                &mut rng,
            );
        let mut mul_f =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                &mut rng,
            );

        let mut actual_f = reference_f.clone();
        let mut account_struct = FinalExpBytes::new();

        parse_f_to_bytes(actual_f, &mut account_struct.f1_r_range_s);
        parse_f_to_bytes(mul_f, &mut account_struct.f_f2_range_s);

        mul_assign_1_2(
            &account_struct.f1_r_range_s,
            &account_struct.f_f2_range_s,
            &mut account_struct.cubic_range_0_s,
            &mut account_struct.cubic_range_1_s,
        );

        mul_assign_3_4_5(
            &account_struct.f_f2_range_s,
            &account_struct.cubic_range_0_s,
            &account_struct.cubic_range_1_s,
            &mut account_struct.f1_r_range_s,
        );
        reference_f *= mul_f;
        assert_eq!(
            reference_f,
            parse_f_from_bytes(&account_struct.f1_r_range_s),
            "f mulassign failed"
        );
    }

    #[test]
    fn mul_assign_test_fails() {
        let mut rng = test_rng();
        let mut reference_f =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                &mut rng,
            );
        let mut mul_f =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                &mut rng,
            );

        let mut actual_f =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                &mut rng,
            );
        let mut account_struct = FinalExpBytes::new();

        parse_f_to_bytes(actual_f, &mut account_struct.f1_r_range_s);
        parse_f_to_bytes(mul_f, &mut account_struct.f_f2_range_s);

        mul_assign_1_2(
            &account_struct.f1_r_range_s,
            &account_struct.f_f2_range_s,
            &mut account_struct.cubic_range_0_s,
            &mut account_struct.cubic_range_1_s,
        );

        mul_assign_3_4_5(
            &account_struct.f_f2_range_s,
            &account_struct.cubic_range_0_s,
            &account_struct.cubic_range_1_s,
            &mut account_struct.f1_r_range_s,
        );
        reference_f *= mul_f;
        assert!(
            reference_f != parse_f_from_bytes(&account_struct.f1_r_range_s),
            "f mulassign failed"
        );
    }

    pub fn exp_by_neg_x(
        mut f: Fp12<<ark_bn254::Parameters as ark_ec::bn::BnParameters>::Fp12Params>,
    ) -> Fp12<<ark_bn254::Parameters as ark_ec::bn::BnParameters>::Fp12Params> {
        f = f.cyclotomic_exp(&<ark_bn254::Parameters as ark_ec::bn::BnParameters>::X);
        if !<ark_bn254::Parameters as ark_ec::bn::BnParameters>::X_IS_NEGATIVE {
            println!("conjugate");
            f.conjugate();
        }
        f
    }

    #[test]
    fn exp_by_neg_x_test_correct() {
        let mut rng = test_rng();
        let mut reference_f =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                &mut rng,
            );
        let mut actual_f = reference_f.clone();

        let mut account_struct = FinalExpBytes::new();
        parse_f_to_bytes(actual_f, &mut account_struct.f1_r_range_s);
        let mut y1 =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::one();
        parse_f_to_bytes(y1, &mut account_struct.y1_range_s);

        account_struct.i_range_s = account_struct.f1_r_range_s.clone();

        //18
        conjugate_wrapper(&mut account_struct.i_range_s);
        //19
        //this instruction is equivalent with the first loop iteration thus the iteration can be omitted
        account_struct.y1_range_s = account_struct.f1_r_range_s.clone();
        //println!(" {:?}", parse_f_from_bytes(&account_struct.y1_range_s));
        for i in 1..63 {
            //20
            if i == 1 {
                assert_eq!(account_struct.y1_range_s, account_struct.f1_r_range_s);
            }

            //cyclotomic_exp
            if i != 0 {
                //println!("i {}", i);
                custom_cyclotomic_square_in_place(&mut account_struct.y1_range_s);
            }

            if NAF_VEC[i] != 0 {
                if NAF_VEC[i] > 0 {
                    //println!("if i {}", i);
                    //23
                    mul_assign_1(
                        &account_struct.y1_range_s,
                        F_CUBIC_0_RANGE,
                        &account_struct.f1_r_range_s,
                        F_CUBIC_0_RANGE,
                        &mut account_struct.cubic_range_0_s,
                        SOLO_CUBIC_0_RANGE,
                    );

                    //24
                    mul_assign_2(
                        &account_struct.y1_range_s,
                        F_CUBIC_1_RANGE,
                        &account_struct.f1_r_range_s,
                        F_CUBIC_1_RANGE,
                        &mut account_struct.cubic_range_1_s,
                        SOLO_CUBIC_0_RANGE,
                    );

                    //25
                    mul_assign_3(&mut account_struct.y1_range_s);

                    //26
                    mul_assign_4_1(
                        &account_struct.f1_r_range_s,
                        &mut account_struct.cubic_range_2_s,
                    );
                    mul_assign_4_2(
                        &mut account_struct.y1_range_s,
                        F_CUBIC_1_RANGE,
                        &account_struct.cubic_range_2_s,
                    );

                    //27
                    mul_assign_5(
                        &mut account_struct.y1_range_s,
                        &account_struct.cubic_range_0_s,
                        &account_struct.cubic_range_1_s,
                    );
                } else {
                    //println!("else i {}", i);
                    //28
                    mul_assign_1(
                        &account_struct.y1_range_s,
                        F_CUBIC_0_RANGE,
                        &account_struct.i_range_s,
                        F_CUBIC_0_RANGE,
                        &mut account_struct.cubic_range_0_s,
                        SOLO_CUBIC_0_RANGE,
                    );
                    //29
                    mul_assign_2(
                        &account_struct.y1_range_s,
                        F_CUBIC_1_RANGE,
                        &account_struct.i_range_s,
                        F_CUBIC_1_RANGE,
                        &mut account_struct.cubic_range_1_s,
                        SOLO_CUBIC_0_RANGE,
                    );
                    //30
                    mul_assign_3(&mut account_struct.y1_range_s);
                    //31
                    mul_assign_4_1(
                        &account_struct.i_range_s,
                        &mut account_struct.cubic_range_2_s,
                    );
                    mul_assign_4_2(
                        &mut account_struct.y1_range_s,
                        F_CUBIC_1_RANGE,
                        &account_struct.cubic_range_2_s,
                    );
                    //32
                    mul_assign_5(
                        &mut account_struct.y1_range_s,
                        &account_struct.cubic_range_0_s,
                        &account_struct.cubic_range_1_s,
                    );
                }
            }
        }

        //will always conjugate
        if !<ark_bn254::Parameters as ark_ec::bn::BnParameters>::X_IS_NEGATIVE {
            //println!("conjugate");
            //f.conjugate();
            conjugate_wrapper(&mut account_struct.y1_range_s);
        }

        let reference_f = exp_by_neg_x(reference_f);
        assert_eq!(
            reference_f,
            parse_f_from_bytes(&account_struct.y1_range_s),
            "f exp_by_neg_x failed"
        );
        //println!("success");
    }

    #[test]
    fn exp_by_neg_x_test_fails() {
        let mut rng = test_rng();
        let mut reference_f =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                &mut rng,
            );
        let mut actual_f =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                &mut rng,
            );

        let mut account_struct = FinalExpBytes::new();
        parse_f_to_bytes(actual_f, &mut account_struct.f1_r_range_s);
        let mut y1 =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::one();
        parse_f_to_bytes(y1, &mut account_struct.y1_range_s);

        account_struct.i_range_s = account_struct.f1_r_range_s.clone();

        //18
        conjugate_wrapper(&mut account_struct.i_range_s);
        //19
        //this instruction is equivalent with the first loop iteration thus the iteration can be omitted
        account_struct.y1_range_s = account_struct.f1_r_range_s.clone();
        //println!(" {:?}", parse_f_from_bytes(&account_struct.y1_range_s));
        for i in 1..63 {
            //20
            if i == 1 {
                assert_eq!(account_struct.y1_range_s, account_struct.f1_r_range_s);
            }

            //cyclotomic_exp
            if i != 0 {
                //println!("i {}", i);
                custom_cyclotomic_square_in_place(&mut account_struct.y1_range_s);
            }

            if NAF_VEC[i] != 0 {
                if NAF_VEC[i] > 0 {
                    //println!("if i {}", i);
                    //23
                    mul_assign_1(
                        &account_struct.y1_range_s,
                        F_CUBIC_0_RANGE,
                        &account_struct.f1_r_range_s,
                        F_CUBIC_0_RANGE,
                        &mut account_struct.cubic_range_0_s,
                        SOLO_CUBIC_0_RANGE,
                    );

                    //24
                    mul_assign_2(
                        &account_struct.y1_range_s,
                        F_CUBIC_1_RANGE,
                        &account_struct.f1_r_range_s,
                        F_CUBIC_1_RANGE,
                        &mut account_struct.cubic_range_1_s,
                        SOLO_CUBIC_0_RANGE,
                    );

                    //25
                    mul_assign_3(&mut account_struct.y1_range_s);

                    //26
                    mul_assign_4_1(
                        &account_struct.f1_r_range_s,
                        &mut account_struct.cubic_range_2_s,
                    );
                    mul_assign_4_2(
                        &mut account_struct.y1_range_s,
                        F_CUBIC_1_RANGE,
                        &account_struct.cubic_range_2_s,
                    );

                    //27
                    mul_assign_5(
                        &mut account_struct.y1_range_s,
                        &account_struct.cubic_range_0_s,
                        &account_struct.cubic_range_1_s,
                    );
                } else {
                    //println!("else i {}", i);
                    //28
                    mul_assign_1(
                        &account_struct.y1_range_s,
                        F_CUBIC_0_RANGE,
                        &account_struct.i_range_s,
                        F_CUBIC_0_RANGE,
                        &mut account_struct.cubic_range_0_s,
                        SOLO_CUBIC_0_RANGE,
                    );
                    //29
                    mul_assign_2(
                        &account_struct.y1_range_s,
                        F_CUBIC_1_RANGE,
                        &account_struct.i_range_s,
                        F_CUBIC_1_RANGE,
                        &mut account_struct.cubic_range_1_s,
                        SOLO_CUBIC_0_RANGE,
                    );
                    //30
                    mul_assign_3(&mut account_struct.y1_range_s);
                    //31
                    mul_assign_4_1(
                        &account_struct.i_range_s,
                        &mut account_struct.cubic_range_2_s,
                    );
                    mul_assign_4_2(
                        &mut account_struct.y1_range_s,
                        F_CUBIC_1_RANGE,
                        &account_struct.cubic_range_2_s,
                    );
                    //32
                    mul_assign_5(
                        &mut account_struct.y1_range_s,
                        &account_struct.cubic_range_0_s,
                        &account_struct.cubic_range_1_s,
                    );
                }
            }
        }

        //will always conjugate
        if !<ark_bn254::Parameters as ark_ec::bn::BnParameters>::X_IS_NEGATIVE {
            //println!("conjugate");
            //f.conjugate();
            conjugate_wrapper(&mut account_struct.y1_range_s);
        }

        let reference_f = exp_by_neg_x(reference_f);
        assert!(
            reference_f != parse_f_from_bytes(&account_struct.y1_range_s),
            "f exp_by_neg_x failed"
        );
        //println!("success");
    }
}
