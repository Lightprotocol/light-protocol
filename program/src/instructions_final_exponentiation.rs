use crate::parsers_part_2_254::{
    parse_f_from_bytes_new,
    parse_f_to_bytes_new,
    parse_fp256_to_bytes_new,
    parse_fp256_from_bytes_new,
    parse_quad_to_bytes_new,
    parse_quad_from_bytes_new,
    parse_cubic_to_bytes_new,
    parse_cubic_from_bytes_new,

};
use ark_ff::{
    fields::models::{
        cubic_extension::CubicExtParameters,
        quadratic_extension::{QuadExtParameters, QuadExtField}},
    Field
};
use ark_ec;
use solana_program::{
    msg,
    log::sol_log_compute_units,
};
use crate::ranges_part_2::*;

//conjugate should work onyl wrapper
pub fn conjugate_wrapper(_range: &mut Vec<u8>) {
    let mut f = parse_f_from_bytes_new(_range);
    f.conjugate();
    parse_f_to_bytes_new(f, _range);
}


//multiplication
pub fn mul_assign_1(
        _cubic: &Vec<u8>,
        _cubic_0_range: [usize;2],
        _cubic_other: &Vec<u8>,
        _range_other_c0: [usize; 2],
        _store_cubic: &mut Vec<u8>,
        _store_cubic_range: [usize; 2]
    ){
    let f_c0 = parse_cubic_from_bytes_new(&_cubic, _cubic_0_range);//30000
    //println!("str f_c0 = {:?}", f_c0);
    let other_c0 = parse_cubic_from_bytes_new(&_cubic_other, _range_other_c0);//30000
    //println!("str other_c0 = {:?}", other_c0);

    let v0 = f_c0 * &other_c0;//   cost 86548
    //println!("str v0 = {:?}", v0);
    parse_cubic_to_bytes_new(v0, _store_cubic, _store_cubic_range);//15000
    //total 162000
}

pub fn mul_assign_2(
    _cubic: &Vec<u8>,
    _cubic_1_range: [usize;2],
    _cubic_other: &Vec<u8>,
    range_other_c1: [usize; 2],
    _store_cubic: &mut Vec<u8>,
    _store_cubic_range: [usize; 2]) {
    let mut f_c1 = parse_cubic_from_bytes_new(&_cubic, _cubic_1_range);              //30000
    let other_c1 = parse_cubic_from_bytes_new(&_cubic_other, range_other_c1);      //30000
    let v1 = f_c1 * &other_c1;                                                //86827
    //println!("v1 = {:?}", v1);
    parse_cubic_to_bytes_new(v1, _store_cubic, _store_cubic_range);          //15000
    //total 193000
}

pub fn mul_assign_3(_f_range: &mut Vec<u8>) {
    let mut f = parse_f_from_bytes_new(&_f_range);              //60000
    f.c1 += &f.c0;                                                      //<1000
    parse_f_to_bytes_new(f, _f_range);                     //30000
    //total 93000
}

pub fn mul_assign_4(
        _cubic_1: &mut Vec<u8>,
        _cubic_1_range: [usize;2],
        _f_range_other: &Vec<u8>,

    ) {
    //let mut f = parse_f_from_bytes(&account[..], range_f);
    let mut f_c1 = parse_cubic_from_bytes_new(&_cubic_1, _cubic_1_range); //30000
    let other = parse_f_from_bytes_new(&_f_range_other);          //60000

    //f.c1 += &f.c0;
    //
    //////msg!"4");
    f_c1 *= &(other.c0 + &other.c1);
                                              //87075
    parse_cubic_to_bytes_new(f_c1, _cubic_1, _cubic_1_range);           //15000
    //195000
}

pub fn mul_assign_4_1(
        _f_range_other: &Vec<u8>,
        _store_cubic_range: &mut Vec<u8>,
    ) {
    //let mut f = parse_f_from_bytes(&account[..], range_f);
    //let mut f_c1 = parse_cubic_from_bytes(&account[..], _f1_r_cubic_1_range); //30000
    let other = parse_f_from_bytes_new(&_f_range_other);          //60000
    //println!("str Other res : {:?}", (other.c0 + other.c1));


    //f.c1 += &f.c0;
    //
    //////msg!"4");
    let tmp = &(other.c0 + &other.c1);                                           //87075
    parse_cubic_to_bytes_new(*tmp, _store_cubic_range, solo_cubic_0_range);           //15000
    //195000
}

pub fn mul_assign_4_2(
        _f1_r_range: &mut Vec<u8>,
        _f1_r_cubic_1_range: [usize;2],
        _store_cubic_range: &Vec<u8>
    ) {
    //let mut f = parse_f_from_bytes(&account[..], range_f);
    let mut f_c1 = parse_cubic_from_bytes_new(&_f1_r_range, _f1_r_cubic_1_range); //30000
    let other_cs = parse_cubic_from_bytes_new(&_store_cubic_range, solo_cubic_0_range);          //60000
    //println!("str loaded other res : {:?}", other_cs);


    //f.c1 += &f.c0;
    //if these logs are removed a stackoverflow might occur in the overall program
    sol_log_compute_units();
    //msg!("mulassing 4_2 0");
    f_c1 *= other_cs;
    sol_log_compute_units();
                              //87075
    parse_cubic_to_bytes_new(f_c1, _f1_r_range, _f1_r_cubic_1_range);
    //msg!("mulassing 4_2 1");
    sol_log_compute_units();

    //assert_eq!(f_c1, parse_cubic_from_bytes_new(&_f1_r_range, _f1_r_cubic_1_range));           //15000
    //195000
}

pub fn mul_assign_5(
        _f1_r_range: &mut Vec<u8>,
        _cubic_range_0: &Vec<u8>,
        _cubic_range_1: &Vec<u8>
    ) {
    //--------------- additional split -------------------------
    let mut f = parse_f_from_bytes_new(&_f1_r_range);              //60000
    let v0 = parse_cubic_from_bytes_new(&_cubic_range_0, solo_cubic_0_range);       //30000
    let v1 = parse_cubic_from_bytes_new(&_cubic_range_1, solo_cubic_0_range);       //30000
    //////msg!"5");
    f.c1 -= &v0;                                                        //<1000
    //
    //////msg!"6");
    f.c1 -= &v1;                                                        //<1000
    //
    //println!("multi assign: {:?}", f);
    //////msg!"7");
    f.c0 = <ark_ff::fields::models::fp12_2over3over2::Fp12ParamsWrapper::<ark_bn254::Fq12Parameters>  as QuadExtParameters>::add_and_mul_base_field_by_nonresidue(&v0, &v1); //1000

    parse_f_to_bytes_new(f, _f1_r_range);                     //30000
    //153000
}

//frobenius_map should work only wrapper
pub fn custom_frobenius_map_1_1(mut account: &mut Vec<u8>) {
    custom_frobenius_map_1(&mut account, 1);
}

pub fn custom_frobenius_map_1_2(mut account:  &mut Vec<u8>) {
    custom_frobenius_map_2(&mut account,1);
}

pub fn custom_frobenius_map_2_1(mut account:  &mut Vec<u8>){
    custom_frobenius_map_1(&mut account, 2);
}

pub fn custom_frobenius_map_2_2(mut account:  &mut Vec<u8>) {
    custom_frobenius_map_2(&mut account, 2);
}

pub fn custom_frobenius_map_3_1(mut account:  &mut Vec<u8>){
    custom_frobenius_map_1(&mut account, 3);
}

pub fn custom_frobenius_map_3_2(mut account:  &mut Vec<u8>) {
    custom_frobenius_map_2(&mut account, 3);
}

pub fn custom_frobenius_map_1(account:  &mut Vec<u8>, power: usize) {
    let mut f = parse_f_from_bytes_new(&account);
    ////msg!"c0 map");
    f.c0.frobenius_map(power);  //                        cost 40641

    ////msg!"c1 map");
    f.c1.frobenius_map(power);//                          cost 40643
    parse_f_to_bytes_new(f, account);

}

pub fn custom_frobenius_map_2(account:  &mut Vec<u8>, power: usize) {
    let mut f = parse_f_from_bytes_new(&account);
    ////msg!"c0 map");
    <ark_ff::fields::models::fp12_2over3over2::Fp12ParamsWrapper::<ark_bn254::Fq12Parameters>  as QuadExtParameters>::mul_base_field_by_frob_coeff(&mut f.c1, power);//  cost 41569

    parse_f_to_bytes_new(f, account);

}


/*exp_by_neg_x is cyclotomic_exp plus possible conjugate
* cyclotomic_exp is:
* - conjugate
* - loop
*   - if naf non-zero -> cyclotomic_square_in_place
*       - only enters if characteristic_square_mod_6_is_one
*   - if part with mul_assign
*/


pub fn custom_cyclotomic_square_in_place( _store_f_range: &mut Vec<u8>) {
    let mut f = parse_f_from_bytes_new(& _store_f_range);
    // cost 46000
    f.cyclotomic_square_in_place();

    parse_f_to_bytes_new(f,_store_f_range);
}
//cyclotomic_square should work only wrapper

pub fn custom_cyclotomic_square(_f_range: &Vec<u8>, _store_f_range: &mut Vec<u8>) {
    let mut f = parse_f_from_bytes_new(& _f_range);
    // cost 90464
    let y0 = f.cyclotomic_square();
    //assert!(f != y0);

    parse_f_to_bytes_new(y0,_store_f_range);
}

//inverse works
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
    let v0 = <ark_ff::fields::models::fp12_2over3over2::Fp12ParamsWrapper::<ark_bn254::Fq12Parameters>  as QuadExtParameters>::sub_and_mul_base_field_by_nonresidue(&f.c0.square(), &v1);
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
    parse_f_to_bytes_new(<ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::new(c0, c1), _f_f2_range);//30

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
        let n5 = <ark_ff::fields::models::fp6_3over2::Fp6ParamsWrapper::<ark_bn254::Fq6Parameters>  as CubicExtParameters>::mul_base_field_by_nonresidue(&t5);
        ////sol_log_compute_units();
        ////msg!"s0");
        let s0 = t0 - &n5;  //      cost 255 cumulative 69593
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
        let a2 = f.c1 * &s2;//      cost 13791 cumulative 97945
        ////sol_log_compute_units();
        ////msg!"a3 ");
        let mut a3 = a1 + &a2;//    cost 182 cumulative 98127
        ////sol_log_compute_units();
        ////msg!"a3.1 ");
        // cost 297 cumulative 98424
        a3 = <ark_ff::fields::models::fp6_3over2::Fp6ParamsWrapper::<ark_bn254::Fq6Parameters>  as CubicExtParameters>::mul_base_field_by_nonresidue(&a3);

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
    parse_cubic_to_bytes_new(ark_ff::fields::models::cubic_extension::CubicExtField::<ark_ff::fields::models::fp6_3over2::Fp6ParamsWrapper<ark_bn254::Fq6Parameters>>::new(c0, c1, c2), _cubic_range_0, solo_cubic_0_range);

}

pub fn custom_quadratic_fp256_inverse_1(
    _quad_range_3: &Vec<u8>,
    _fp384_range: &mut Vec<u8>) {
    //if account[quad_range_3[0]..quad_range_3[1]] == [0;(quad_range_3[1] - quad_range_3[0])] {
        //returns none in arkworks maybe we should add a byte to signal a crash
    //} else {
        // Guide to Pairing-based Cryptography, Algorithm 5.19.
        let f = parse_quad_from_bytes_new(_quad_range_3);
        let v1 = f.c1.square();//      cost 3659

        //   cost 184974
        let v0 = <ark_ff::fields::models::fp2::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>  as QuadExtParameters>::sub_and_mul_base_field_by_nonresidue(&f.c0.square(), &v1);
        parse_fp256_to_bytes_new(v0,_fp384_range);
    //}
}

pub fn custom_quadratic_fp256_inverse_2(
        _quad_range_3: &mut Vec<u8>,
        _fp384_range: &Vec<u8>
        ) {

    let v0 = parse_fp256_from_bytes_new(&_fp384_range);
    let f = parse_quad_from_bytes_new(&_quad_range_3);
    v0.inverse().map(|v1| { //      cost 181186

        //   cost 184974

        let c0 = f.c0 * &v1;//      cost 4367
        ////sol_log_compute_units();
        ////msg!"c1");
        let c1 = -(f.c1 * &v1);//   cost 4471
        ////sol_log_compute_units();

        let res = QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(c0, c1);
        parse_quad_to_bytes_new(res, _quad_range_3);
    });

}



#[cfg(test)]
mod tests {
    use crate::instructions_final_exponentiation::{
        custom_frobenius_map_1_1,
        custom_frobenius_map_1_2,
        custom_frobenius_map_2_1,
        custom_frobenius_map_2_2,
        custom_frobenius_map_3_1,
        custom_frobenius_map_3_2,
        custom_cyclotomic_square,
        conjugate_wrapper,
        custom_f_inverse_1,
        custom_f_inverse_2,
        custom_f_inverse_3,
        custom_f_inverse_4,
        custom_f_inverse_5,
        custom_cubic_inverse_1,
        custom_cubic_inverse_2,
        custom_quadratic_fp256_inverse_1,
        custom_quadratic_fp256_inverse_2,
        mul_assign_1,
        mul_assign_2,
        mul_assign_3,
        mul_assign_4_1,
        mul_assign_4_2,
        mul_assign_5,
        custom_cyclotomic_square_in_place,
    };

    use crate::state_final_exp::FinalExpBytes;

    use crate::parsers_part_2_254::{
        parse_f_to_bytes_new,
        parse_f_from_bytes_new,
        parse_quad_from_bytes_new,
        parse_cubic_from_bytes_new
    };

    use crate::ranges_part_2::{
        f_cubic_0_range,
        f_cubic_1_range,
        solo_cubic_0_range,
        naf_vec
    };

    use ark_ff::{Fp12, Field};
    use ark_ec::bn::BnParameters;
    use ark_std::{UniformRand, test_rng, One, Zero};

    #[test]
    fn frobenius_map_test_correct() {
        //generating input
        for i in 1..4 {
            let mut rng = test_rng();
            let mut reference_f = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(&mut rng);

            let mut actual_f = reference_f.clone();
            //simulating the onchain account
            let mut account = vec![0u8;384];
            parse_f_to_bytes_new(actual_f, &mut account);
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
            assert_eq!(reference_f, parse_f_from_bytes_new(&account));
        }

    }

    #[test]
    fn frobenius_map_test_fails() {
        //generating input
        for i in 1..4 {
            let mut rng = test_rng();
            let mut reference_f = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(&mut rng);

            let mut actual_f = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(&mut rng);
            //simulating the onchain account
            let mut account = vec![0u8;384];
            parse_f_to_bytes_new(actual_f, &mut account);
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
            assert!(reference_f != parse_f_from_bytes_new(&account));
        }

    }

    #[test]
    fn custom_cyclotomic_square_test_correct() {
        //generating input

        let mut rng = test_rng();
        let mut reference_f = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(&mut rng);

        let mut actual_f = reference_f.clone();
        //simulating the onchain account
        let mut account = vec![0u8;384];
        parse_f_to_bytes_new(actual_f, &mut account);
        let account_tmp = account.clone();
        custom_cyclotomic_square(&account_tmp, &mut account);
        let account_tmp = account.clone();
        custom_cyclotomic_square(&account_tmp, &mut account);

        //generating reference value for comparison
        reference_f = reference_f.cyclotomic_square();
        reference_f = reference_f.cyclotomic_square();
        assert_eq!(reference_f, parse_f_from_bytes_new(&account));

    }

    #[test]
    fn custom_cyclotomic_square_test_fails() {
        //generating input
        let mut rng = test_rng();
        let mut reference_f = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(&mut rng);

        let mut actual_f = reference_f.clone();
        //simulating the onchain account
        let mut account = vec![0u8;384];
        parse_f_to_bytes_new(actual_f, &mut account);
        let account_tmp = account.clone();
        custom_cyclotomic_square(&account_tmp, &mut account);
        let account_tmp = account.clone();
        custom_cyclotomic_square(&account_tmp, &mut account);

        //generating reference value for comparison
        reference_f = reference_f.cyclotomic_square();
        //reference_f = reference_f.cyclotomic_square();
        assert!(reference_f != parse_f_from_bytes_new(&account));

    }

    #[test]
    fn conjugate_test_correct() {
        //generating input

        let mut rng = test_rng();
        for i in 0..10 {
            let mut reference_f = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(&mut rng);

            let mut actual_f = reference_f.clone();
            //simulating the onchain account
            let mut account = vec![0u8;384];
            parse_f_to_bytes_new(actual_f, &mut account);
            conjugate_wrapper(&mut account);

            //generating reference value for comparison
            reference_f.conjugate();
            assert_eq!(reference_f, parse_f_from_bytes_new(&account));
        }
    }

    #[test]
    fn conjugate_test_fails() {
        //generating input

        let mut rng = test_rng();
        for i in 0..10 {
            let mut reference_f = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(&mut rng);

            let mut actual_f = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(&mut rng);

            //simulating the onchain account
            let mut account = vec![0u8;384];
            parse_f_to_bytes_new(actual_f, &mut account);
            conjugate_wrapper(&mut account);

            //generating reference value for comparison
            reference_f.conjugate();
            assert!(reference_f != parse_f_from_bytes_new(&account));
        }
    }

    #[test]
    fn custom_inverse_test_correct() {

        let mut rng = test_rng();
        let mut reference_f = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(&mut rng);
        let mut actual_f = reference_f.clone();
        let mut account_struct = FinalExpBytes::new();

        parse_f_to_bytes_new(actual_f, &mut account_struct.f_f2_range_s);

        //1 ---------------------------------------------
        custom_f_inverse_1(&account_struct.f_f2_range_s, &mut account_struct.cubic_range_1_s);

        //2 ---------------------------------------------
        custom_f_inverse_2(&account_struct.f_f2_range_s,&mut account_struct.cubic_range_0_s, &account_struct.cubic_range_1_s);
        let cubic = parse_cubic_from_bytes_new(&account_struct.cubic_range_0_s,solo_cubic_0_range);

        //3 ---------------------------------------------
        custom_cubic_inverse_1(
            &account_struct.cubic_range_0_s,
            &mut account_struct.quad_range_0_s,
            &mut account_struct.quad_range_1_s,
            &mut account_struct.quad_range_2_s,
            &mut account_struct.quad_range_3_s
        );

        //quad works
        let quad = parse_quad_from_bytes_new(&account_struct.quad_range_3_s);

        //4 ---------------------------------------------
        //quad inverse is part of cubic Inverse
        custom_quadratic_fp256_inverse_1(
            &account_struct.quad_range_3_s,
            &mut account_struct.fp384_range_s
        );

        //5 ---------------------------------------------
        custom_quadratic_fp256_inverse_2(
            &mut account_struct.quad_range_3_s,
            & account_struct.fp384_range_s,
        );

        assert_eq!(quad.inverse().unwrap() , parse_quad_from_bytes_new(&account_struct.quad_range_3_s), "quad inverse failed");

        //6 ---------------------------------------------
        custom_cubic_inverse_2(
        &mut account_struct.cubic_range_0_s,
        & account_struct.quad_range_0_s,
        & account_struct.quad_range_1_s,
        & account_struct.quad_range_2_s,
        & account_struct.quad_range_3_s
        );
        assert_eq!(cubic.inverse().unwrap() , parse_cubic_from_bytes_new(&account_struct.cubic_range_0_s, solo_cubic_0_range), "cubic inverse failed");

        //7 ---------------------------------------------
        custom_f_inverse_3(
            &mut account_struct.cubic_range_1_s,
            &account_struct.cubic_range_0_s,
            &account_struct.f_f2_range_s,
        );

        //8 ---------------------------------------------
        custom_f_inverse_4(
            &mut account_struct.cubic_range_0_s,
            &account_struct.f_f2_range_s
        );

        //9 ---------------------------------------------
        custom_f_inverse_5(
            &account_struct.cubic_range_0_s,
            &account_struct.cubic_range_1_s,
            &mut account_struct.f_f2_range_s,
        );
        //reference_f;
        //println!("{:?}", reference_f);
        assert_eq!(reference_f.inverse().unwrap() , parse_f_from_bytes_new(&account_struct.f_f2_range_s), "f inverse failed");
    }

    #[test]
    fn custom_inverse_test_fails() {

        let mut rng = test_rng();
        let mut reference_f = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(&mut rng);
        let mut actual_f = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(&mut rng);
        let mut account_struct = FinalExpBytes::new();

        parse_f_to_bytes_new(actual_f, &mut account_struct.f_f2_range_s);

        //1 ---------------------------------------------
        custom_f_inverse_1(&account_struct.f_f2_range_s, &mut account_struct.cubic_range_1_s);

        //2 ---------------------------------------------
        custom_f_inverse_2(&account_struct.f_f2_range_s,&mut account_struct.cubic_range_0_s, &account_struct.cubic_range_1_s);

        //3 ---------------------------------------------
        custom_cubic_inverse_1(
            &account_struct.cubic_range_0_s,
            &mut account_struct.quad_range_0_s,
            &mut account_struct.quad_range_1_s,
            &mut account_struct.quad_range_2_s,
            &mut account_struct.quad_range_3_s
        );


        //4 ---------------------------------------------
        //quad inverse is part of cubic Inverse
        custom_quadratic_fp256_inverse_1(
            &account_struct.quad_range_3_s,
            &mut account_struct.fp384_range_s
        );

        //5 ---------------------------------------------
        custom_quadratic_fp256_inverse_2(
            &mut account_struct.quad_range_3_s,
            & account_struct.fp384_range_s,
        );


        //6 ---------------------------------------------
        custom_cubic_inverse_2(
        &mut account_struct.cubic_range_0_s,
        & account_struct.quad_range_0_s,
        & account_struct.quad_range_1_s,
        & account_struct.quad_range_2_s,
        & account_struct.quad_range_3_s
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
            &account_struct.f_f2_range_s
        );

        //9 ---------------------------------------------
        custom_f_inverse_5(
            &account_struct.cubic_range_0_s,
            &account_struct.cubic_range_1_s,
            &mut account_struct.f_f2_range_s,
        );
        //reference_f;
        //println!("{:?}", reference_f);
        assert!(reference_f.inverse().unwrap() != parse_f_from_bytes_new(&account_struct.f_f2_range_s), "f inverse failed");
    }

    #[test]
    fn mul_assign_test_correct() {

        let mut rng = test_rng();
        let mut reference_f = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(&mut rng);
        let mut mul_f = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(&mut rng);

        let mut actual_f = reference_f.clone();
        let mut account_struct = FinalExpBytes::new();

        parse_f_to_bytes_new(actual_f, &mut account_struct.f1_r_range_s);
        parse_f_to_bytes_new(mul_f, &mut account_struct.f_f2_range_s);

        mul_assign_1(
            &account_struct.f1_r_range_s,  f_cubic_0_range,
            &account_struct.f_f2_range_s,  f_cubic_0_range,
            &mut account_struct.cubic_range_0_s,  solo_cubic_0_range
        );

        //println!("cubi mul assing1 ref: {:?}", parse_cubic_from_bytes(&account[..], cubic_range_0));
        //9
        mul_assign_2(
            &account_struct.f1_r_range_s,  f_cubic_1_range,
            &account_struct.f_f2_range_s,  f_cubic_1_range,
            &mut account_struct.cubic_range_1_s,  solo_cubic_0_range
        );
        //10
        mul_assign_3(
            &mut account_struct.f1_r_range_s
        );
        //println!("mul assing3 ref: {:?}", parse_f_from_bytes(&account[..], f1_r_range));

        //11
        mul_assign_4_1(
            &account_struct.f_f2_range_s,
            &mut account_struct.cubic_range_2_s,
        );
        //println!("mul assing4_1 ref: {:?}", parse_cubic_from_bytes(&account[..], cubic_range_2));

        mul_assign_4_2(
            &mut account_struct.f1_r_range_s,
             f_cubic_1_range,
            &account_struct.cubic_range_2_s,
        );

        //12
        mul_assign_5(
            &mut account_struct.f1_r_range_s,
            &account_struct.cubic_range_0_s,
            &account_struct.cubic_range_1_s
        );
        reference_f *= mul_f;
        assert_eq!(reference_f,  parse_f_from_bytes_new(&account_struct.f1_r_range_s), "f mulassign failed");
    }

    #[test]
    fn mul_assign_test_fails() {

        let mut rng = test_rng();
        let mut reference_f = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(&mut rng);
        let mut mul_f = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(&mut rng);

        let mut actual_f = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(&mut rng);

        let mut account_struct = FinalExpBytes::new();

        parse_f_to_bytes_new(actual_f, &mut account_struct.f1_r_range_s);
        parse_f_to_bytes_new(mul_f, &mut account_struct.f_f2_range_s);

        mul_assign_1(
            &account_struct.f1_r_range_s,  f_cubic_0_range,
            &account_struct.f_f2_range_s,  f_cubic_0_range,
            &mut account_struct.cubic_range_0_s,  solo_cubic_0_range
        );

        //println!("cubi mul assing1 ref: {:?}", parse_cubic_from_bytes(&account[..], cubic_range_0));
        //9
        mul_assign_2(
            &account_struct.f1_r_range_s,  f_cubic_1_range,
            &account_struct.f_f2_range_s,  f_cubic_1_range,
            &mut account_struct.cubic_range_1_s,  solo_cubic_0_range
        );
        //10
        mul_assign_3(
            &mut account_struct.f1_r_range_s
        );
        //println!("mul assing3 ref: {:?}", parse_f_from_bytes(&account[..], f1_r_range));

        //11
        mul_assign_4_1(
            &account_struct.f_f2_range_s,
            &mut account_struct.cubic_range_2_s,
        );
        //println!("mul assing4_1 ref: {:?}", parse_cubic_from_bytes(&account[..], cubic_range_2));

        mul_assign_4_2(
            &mut account_struct.f1_r_range_s,
             f_cubic_1_range,
            &account_struct.cubic_range_2_s,
        );

        //12
        mul_assign_5(
            &mut account_struct.f1_r_range_s,
            &account_struct.cubic_range_0_s,
            &account_struct.cubic_range_1_s
        );
        reference_f *= mul_f;
        assert!(reference_f != parse_f_from_bytes_new(&account_struct.f1_r_range_s), "f mulassign failed");
    }


    pub fn exp_by_neg_x(mut f: Fp12::<<ark_bn254::Parameters as ark_ec::bn::BnParameters>::Fp12Params>) -> Fp12::<<ark_bn254::Parameters as ark_ec::bn::BnParameters>::Fp12Params> {
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
        let mut reference_f = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(&mut rng);
        let mut actual_f = reference_f.clone();

        let mut account_struct = FinalExpBytes::new();
        parse_f_to_bytes_new(actual_f, &mut account_struct.f1_r_range_s);
        let mut y1 = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::one();
        parse_f_to_bytes_new(y1, &mut account_struct.y1_range_s);

        account_struct.i_range_s = account_struct.f1_r_range_s.clone();


        //18
        conjugate_wrapper(&mut account_struct.i_range_s);
        //19
        //this instruction is equivalent with the first loop iteration thus the iteration can be omitted
        account_struct.y1_range_s = account_struct.f1_r_range_s.clone();
        //println!(" {:?}", parse_f_from_bytes_new(&account_struct.y1_range_s));
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

            if naf_vec[i] != 0 {
                if naf_vec[i] > 0 {
                    //println!("if i {}", i);
                    //23
                    mul_assign_1(
                        &account_struct.y1_range_s, f_cubic_0_range,
                        &account_struct.f1_r_range_s, f_cubic_0_range,
                        &mut account_struct.cubic_range_0_s, solo_cubic_0_range
                    );

                    //24
                    mul_assign_2(
                        &account_struct.y1_range_s, f_cubic_1_range,
                        &account_struct.f1_r_range_s, f_cubic_1_range,
                        &mut account_struct.cubic_range_1_s, solo_cubic_0_range
                    );

                    //25
                    mul_assign_3(
                        &mut account_struct.y1_range_s
                    );


                    //26
                    mul_assign_4_1(
                        &account_struct.f1_r_range_s,
                        &mut account_struct.cubic_range_2_s,
                    );
                    mul_assign_4_2(
                        &mut account_struct.y1_range_s,
                        f_cubic_1_range,
                        &account_struct.cubic_range_2_s,
                    );

                    //27
                    mul_assign_5(
                        &mut account_struct.y1_range_s,
                        &account_struct.cubic_range_0_s,
                        &account_struct.cubic_range_1_s
                    );

                } else {
                    //println!("else i {}", i);
                    //28
                    mul_assign_1(
                        &account_struct.y1_range_s, f_cubic_0_range,
                        &account_struct.i_range_s, f_cubic_0_range,
                        &mut account_struct.cubic_range_0_s, solo_cubic_0_range
                    );
                    //29
                    mul_assign_2(
                        &account_struct.y1_range_s, f_cubic_1_range,
                        &account_struct.i_range_s, f_cubic_1_range,
                        &mut account_struct.cubic_range_1_s, solo_cubic_0_range
                    );
                    //30
                    mul_assign_3(
                        &mut account_struct.y1_range_s
                    );
                    //31
                    mul_assign_4_1(
                        &account_struct.i_range_s,
                        &mut account_struct.cubic_range_2_s,
                    );
                    mul_assign_4_2(
                        &mut account_struct.y1_range_s,
                        f_cubic_1_range,
                        &account_struct.cubic_range_2_s,
                    );
                    //32
                    mul_assign_5(
                        &mut account_struct.y1_range_s,
                        &account_struct.cubic_range_0_s,
                        &account_struct.cubic_range_1_s
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
        assert_eq!(reference_f, parse_f_from_bytes_new(&account_struct.y1_range_s), "f exp_by_neg_x failed");
        //println!("success");
    }

    #[test]
    fn exp_by_neg_x_test_fails() {

        let mut rng = test_rng();
        let mut reference_f = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(&mut rng);
        let mut actual_f = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(&mut rng);

        let mut account_struct = FinalExpBytes::new();
        parse_f_to_bytes_new(actual_f, &mut account_struct.f1_r_range_s);
        let mut y1 = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::one();
        parse_f_to_bytes_new(y1, &mut account_struct.y1_range_s);

        account_struct.i_range_s = account_struct.f1_r_range_s.clone();


        //18
        conjugate_wrapper(&mut account_struct.i_range_s);
        //19
        //this instruction is equivalent with the first loop iteration thus the iteration can be omitted
        account_struct.y1_range_s = account_struct.f1_r_range_s.clone();
        //println!(" {:?}", parse_f_from_bytes_new(&account_struct.y1_range_s));
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

            if naf_vec[i] != 0 {
                if naf_vec[i] > 0 {
                    //println!("if i {}", i);
                    //23
                    mul_assign_1(
                        &account_struct.y1_range_s, f_cubic_0_range,
                        &account_struct.f1_r_range_s, f_cubic_0_range,
                        &mut account_struct.cubic_range_0_s, solo_cubic_0_range
                    );

                    //24
                    mul_assign_2(
                        &account_struct.y1_range_s, f_cubic_1_range,
                        &account_struct.f1_r_range_s, f_cubic_1_range,
                        &mut account_struct.cubic_range_1_s, solo_cubic_0_range
                    );

                    //25
                    mul_assign_3(
                        &mut account_struct.y1_range_s
                    );


                    //26
                    mul_assign_4_1(
                        &account_struct.f1_r_range_s,
                        &mut account_struct.cubic_range_2_s,
                    );
                    mul_assign_4_2(
                        &mut account_struct.y1_range_s,
                        f_cubic_1_range,
                        &account_struct.cubic_range_2_s,
                    );

                    //27
                    mul_assign_5(
                        &mut account_struct.y1_range_s,
                        &account_struct.cubic_range_0_s,
                        &account_struct.cubic_range_1_s
                    );

                } else {
                    //println!("else i {}", i);
                    //28
                    mul_assign_1(
                        &account_struct.y1_range_s, f_cubic_0_range,
                        &account_struct.i_range_s, f_cubic_0_range,
                        &mut account_struct.cubic_range_0_s, solo_cubic_0_range
                    );
                    //29
                    mul_assign_2(
                        &account_struct.y1_range_s, f_cubic_1_range,
                        &account_struct.i_range_s, f_cubic_1_range,
                        &mut account_struct.cubic_range_1_s, solo_cubic_0_range
                    );
                    //30
                    mul_assign_3(
                        &mut account_struct.y1_range_s
                    );
                    //31
                    mul_assign_4_1(
                        &account_struct.i_range_s,
                        &mut account_struct.cubic_range_2_s,
                    );
                    mul_assign_4_2(
                        &mut account_struct.y1_range_s,
                        f_cubic_1_range,
                        &account_struct.cubic_range_2_s,
                    );
                    //32
                    mul_assign_5(
                        &mut account_struct.y1_range_s,
                        &account_struct.cubic_range_0_s,
                        &account_struct.cubic_range_1_s
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
        assert!(reference_f != parse_f_from_bytes_new(&account_struct.y1_range_s), "f exp_by_neg_x failed");
        //println!("success");
    }


}
