use crate::groth16_verifier::{
    final_exponentiation::{ranges::*, state::FinalExponentiationState},
    parsers::{
        parse_cubic_from_bytes_sub, parse_cubic_to_bytes_sub, parse_f_from_bytes, parse_f_to_bytes,
        parse_fp256_from_bytes, parse_fp256_to_bytes, parse_quad_from_bytes, parse_quad_to_bytes,
    },
};
use crate::utils::prepared_verifying_key::ALPHA_G1_BETA_G2;
use ark_ec;
use ark_ff::{
    fields::models::{
        cubic_extension::CubicExtParameters,
        quadratic_extension::{QuadExtField, QuadExtParameters},
    },
    Field,
};

use solana_program::{msg, program_error::ProgramError};

pub fn verify_result(main_account_data: &FinalExponentiationState) -> Result<(), ProgramError> {
    if ALPHA_G1_BETA_G2.to_vec() != main_account_data.y1_range {
        msg!("verification failed");
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

pub fn conjugate_wrapper(_range: &mut Vec<u8>) {
    let mut f = parse_f_from_bytes(_range);
    f.conjugate();
    parse_f_to_bytes(f, _range);
}

//multiplication

pub fn mul_assign_1(
    _f_range: &Vec<u8>,
    _other: &Vec<u8>,
    _store_cubic0: &mut Vec<u8>,
    _store_cubic1: &mut Vec<u8>,
) {
    let f = parse_f_from_bytes(_f_range);
    let other = parse_f_from_bytes(_other);

    let v0 = f.c0 * other.c0;
    let v1 = f.c1 * other.c1;

    parse_cubic_to_bytes_sub(v0, _store_cubic0, SOLO_CUBIC_0_RANGE); //15000
    parse_cubic_to_bytes_sub(v1, _store_cubic1, SOLO_CUBIC_0_RANGE);
}

pub fn mul_assign_2(
    _f_range_other: &Vec<u8>,
    _cubic_range_0: &Vec<u8>,
    _cubic_range_1: &Vec<u8>,
    _f_range: &mut Vec<u8>,
) {
    //3
    let mut f = parse_f_from_bytes(_f_range);
    f.c1 += &f.c0;

    let other = parse_f_from_bytes(_f_range_other);
    f.c1 *= &(other.c0 + other.c1);

    let v0 = parse_cubic_from_bytes_sub(_cubic_range_0, SOLO_CUBIC_0_RANGE); //30000
    let v1 = parse_cubic_from_bytes_sub(_cubic_range_1, SOLO_CUBIC_0_RANGE); //30000
    f.c1 -= &v0;
    f.c1 -= &v1;
    f.c0 = <ark_ff::fields::models::fp12_2over3over2::Fp12ParamsWrapper::<ark_bn254::Fq12Parameters>  as QuadExtParameters>::add_and_mul_base_field_by_nonresidue(&v0, &v1); //1000

    parse_f_to_bytes(f, _f_range);
}

pub fn custom_frobenius_map_1(account: &mut Vec<u8>) {
    let mut f = parse_f_from_bytes(account);
    f.frobenius_map(1);
    parse_f_to_bytes(f, account);
}

pub fn custom_frobenius_map_2(account: &mut Vec<u8>) {
    let mut f = parse_f_from_bytes(account);
    f.frobenius_map(2);
    parse_f_to_bytes(f, account);
}

pub fn custom_frobenius_map_3(account: &mut Vec<u8>) {
    let mut f = parse_f_from_bytes(account);
    f.frobenius_map(3);
    parse_f_to_bytes(f, account);
}

pub fn custom_cyclotomic_square_in_place(_store_f_range: &mut Vec<u8>) {
    let mut f = parse_f_from_bytes(_store_f_range);
    // cost 46000
    f.cyclotomic_square_in_place();

    parse_f_to_bytes(f, _store_f_range);
}

pub fn custom_cyclotomic_square(_f_range: &Vec<u8>, _store_f_range: &mut Vec<u8>) {
    let f = parse_f_from_bytes(_f_range);
    // cost 90464
    let y0 = f.cyclotomic_square();

    parse_f_to_bytes(y0, _store_f_range);
}

pub fn custom_f_inverse_1(_f_f2_range: &Vec<u8>, _cubic_range_1: &mut Vec<u8>) {
    //first part of calculating the inverse to f

    let f = parse_f_from_bytes(_f_f2_range);
    let v1 = f.c1.square();
    parse_cubic_to_bytes_sub(v1, _cubic_range_1, SOLO_CUBIC_0_RANGE);
}

pub fn custom_f_inverse_2(
    _f_f2_range: &Vec<u8>,
    _cubic_range_0: &mut Vec<u8>,
    _cubic_range_1: &Vec<u8>,
) {
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
    let v1 = parse_cubic_from_bytes_sub(_cubic_range_0, SOLO_CUBIC_0_RANGE);
    let f_c0 = parse_cubic_from_bytes_sub(_f_f2_range, F_CUBIC_0_RANGE);

    let c0 = f_c0 * v1; //      cost 87545

    parse_cubic_to_bytes_sub(c0, _cubic_range_1, SOLO_CUBIC_0_RANGE);
}

pub fn custom_f_inverse_4(_cubic: &mut Vec<u8>, _f_f2_range: &Vec<u8>) {
    let v1 = parse_cubic_from_bytes_sub(_cubic, SOLO_CUBIC_0_RANGE); //30
    let f_c1 = parse_cubic_from_bytes_sub(_f_f2_range, F_CUBIC_1_RANGE); //30
    let c1 = -(f_c1 * v1); //   cost 86867
    parse_cubic_to_bytes_sub(c1, _cubic, SOLO_CUBIC_0_RANGE);
}

pub fn custom_f_inverse_5(_cubic_0: &Vec<u8>, _cubic_1: &Vec<u8>, _f_f2_range: &mut Vec<u8>) {
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
    let f = parse_cubic_from_bytes_sub(_cubic_range_0, SOLO_CUBIC_0_RANGE);
    // From "High-Speed Software Implementation of the Optimal Ate AbstractPairing
    // Barreto-Naehrig Curves"; Algorithm 17

    let t0 = f.c0.square(); //  cost 9188
    let t1 = f.c1.square(); //  cost 9255 cumulative 18443
    let t2 = f.c2.square(); //   cost 9255 cumulative 27698
    let t3 = f.c0 * f.c1; //    cost 13790 cumulative 41488
    let t4 = f.c0 * f.c2; //    cost 13789 cumulative 55277

    let t5 = f.c1 * f.c2; //    cost 13791 cumulative 69068

    let n5 = <ark_ff::fields::models::fp6_3over2::Fp6ParamsWrapper::<ark_bn254::Fq6Parameters>  as CubicExtParameters>::mul_base_field_by_nonresidue(&t5);

    let s0 = t0 - n5; //      cost 255 cumulative 69593

    let s1 = <ark_ff::fields::models::fp6_3over2::Fp6ParamsWrapper::<ark_bn254::Fq6Parameters>  as CubicExtParameters>::mul_base_field_by_nonresidue(&t2) - t3;

    let s2 = t1 - t4; // typo in paper referenced above. should be "-" as per Scott, but is "*"

    let a1 = f.c2 * s1; //     cost 13791 cumulative 84154

    let a2 = f.c1 * s2; //      cost 13791 cumulative 97945

    let mut a3 = a1 + a2; //    cost 182 cumulative 98127

    a3 = <ark_ff::fields::models::fp6_3over2::Fp6ParamsWrapper::<ark_bn254::Fq6Parameters>  as CubicExtParameters>::mul_base_field_by_nonresidue(&a3);

    //13895
    let t6_input = f.c0 * s0 + a3;

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
    let t6 = parse_quad_from_bytes(_quad_range_3);
    let s0 = parse_quad_from_bytes(_quad_range_0);
    let s1 = parse_quad_from_bytes(_quad_range_1);
    let s2 = parse_quad_from_bytes(_quad_range_2);
    let c0 = t6 * s0; //      cost 13698

    let c1 = t6 * s1; //      cost 13790

    let c2 = t6 * s2; //      cost 13790
    parse_cubic_to_bytes_sub(
        ark_ff::fields::models::cubic_extension::CubicExtField::<
            ark_ff::fields::models::fp6_3over2::Fp6ParamsWrapper<ark_bn254::Fq6Parameters>,
        >::new(c0, c1, c2),
        _cubic_range_0,
        SOLO_CUBIC_0_RANGE,
    );
}

pub fn custom_quadratic_fp256_inverse_1(_quad_range_3: &Vec<u8>, _fp384_range: &mut Vec<u8>) {
    let f = parse_quad_from_bytes(_quad_range_3);
    // Guide to Pairing-based Cryptography, Algorithm 5.19.
    let v1 = f.c1.square(); //      cost 3659

    //   cost 184974
    let v0 = <ark_ff::fields::models::fp2::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>  as QuadExtParameters>::sub_and_mul_base_field_by_nonresidue(&f.c0.square(), &v1);
    parse_fp256_to_bytes(v0, _fp384_range);
}

#[allow(clippy::option_map_unit_fn)]
pub fn custom_quadratic_fp256_inverse_2(_quad_range_3: &mut Vec<u8>, _fp384_range: &Vec<u8>) {
    let v0 = parse_fp256_from_bytes(_fp384_range);
    let f = parse_quad_from_bytes(_quad_range_3);
    v0.inverse().map(|v1| {
        //      cost 181186

        //   cost 184974

        let c0 = f.c0 * v1; //      cost 4367

        let c1 = -(f.c1 * v1); //   cost 4471

        let res = QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(c0, c1);
        parse_quad_to_bytes(res, _quad_range_3);
    });
}

#[cfg(test)]
mod tests {
    use crate::groth16_verifier::final_exponentiation::instructions::{
        conjugate_wrapper, custom_cubic_inverse_1, custom_cubic_inverse_2,
        custom_cyclotomic_square, custom_f_inverse_1, custom_f_inverse_2, custom_f_inverse_3,
        custom_f_inverse_4, custom_f_inverse_5, custom_frobenius_map_1, custom_frobenius_map_2,
        custom_frobenius_map_3, custom_quadratic_fp256_inverse_1, custom_quadratic_fp256_inverse_2,
        mul_assign_1, mul_assign_2,
    };

    use crate::groth16_verifier::final_exponentiation::state::FinalExponentiationState;

    use crate::groth16_verifier::parsers::{
        parse_cubic_from_bytes_sub, parse_f_from_bytes, parse_f_to_bytes, parse_quad_from_bytes,
    };

    use crate::groth16_verifier::final_exponentiation::ranges::SOLO_CUBIC_0_RANGE;

    use ark_ff::Field;
    use ark_std::{test_rng, UniformRand};

    #[test]
    fn fe_unit_test_frobenius_map_test_correct() {
        //generating input
        for i in 1..4 {
            let mut rng = test_rng();
            let mut reference_f =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                    &mut rng,
                );

            let actual_f = reference_f.clone();
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

    #[test]
    fn fe_unit_test_custom_cyclotomic_square_test_correct() {
        //generating input

        let mut rng = test_rng();
        let mut reference_f =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                &mut rng,
            );

        let actual_f = reference_f.clone();
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
    fn fe_unit_test_custom_cyclotomic_square_test_fails() {
        //generating input
        let mut rng = test_rng();
        let mut reference_f =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                &mut rng,
            );

        let actual_f = reference_f.clone();
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
    fn fe_unit_test_conjugate_test_correct() {
        //generating input

        let mut rng = test_rng();
        for _i in 0..10 {
            let mut reference_f =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                    &mut rng,
                );

            let actual_f = reference_f.clone();
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
    fn fe_unit_test_conjugate_test_fails() {
        //generating input

        let mut rng = test_rng();
        for _i in 0..10 {
            let mut reference_f =
                <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                    &mut rng,
                );

            let actual_f =
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
    fn fe_unit_test_custom_inverse_test_correct() {
        let mut rng = test_rng();
        let reference_f =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                &mut rng,
            );
        let actual_f = reference_f.clone();
        let mut account_struct = FinalExponentiationState::new();

        parse_f_to_bytes(actual_f, &mut account_struct.f_f2_range);

        //1 ---------------------------------------------
        custom_f_inverse_1(
            &account_struct.f_f2_range,
            &mut account_struct.cubic_range_1,
        );

        //2 ---------------------------------------------
        custom_f_inverse_2(
            &account_struct.f_f2_range,
            &mut account_struct.cubic_range_0,
            &account_struct.cubic_range_1,
        );
        let cubic = parse_cubic_from_bytes_sub(&account_struct.cubic_range_0, SOLO_CUBIC_0_RANGE);

        //3 ---------------------------------------------
        custom_cubic_inverse_1(
            &account_struct.cubic_range_0,
            &mut account_struct.quad_range_0,
            &mut account_struct.quad_range_1,
            &mut account_struct.quad_range_2,
            &mut account_struct.quad_range_3,
        );

        //quad works
        let quad = parse_quad_from_bytes(&account_struct.quad_range_3);

        //4 ---------------------------------------------
        //quad inverse is part of cubic Inverse
        custom_quadratic_fp256_inverse_1(
            &account_struct.quad_range_3,
            &mut account_struct.fp256_range,
        );

        //5 ---------------------------------------------
        custom_quadratic_fp256_inverse_2(
            &mut account_struct.quad_range_3,
            &account_struct.fp256_range,
        );

        assert_eq!(
            quad.inverse().unwrap(),
            parse_quad_from_bytes(&account_struct.quad_range_3),
            "quad inverse failed"
        );

        //6 ---------------------------------------------
        custom_cubic_inverse_2(
            &mut account_struct.cubic_range_0,
            &account_struct.quad_range_0,
            &account_struct.quad_range_1,
            &account_struct.quad_range_2,
            &account_struct.quad_range_3,
        );
        assert_eq!(
            cubic.inverse().unwrap(),
            parse_cubic_from_bytes_sub(&account_struct.cubic_range_0, SOLO_CUBIC_0_RANGE),
            "cubic inverse failed"
        );

        //7 ---------------------------------------------
        custom_f_inverse_3(
            &mut account_struct.cubic_range_1,
            &account_struct.cubic_range_0,
            &account_struct.f_f2_range,
        );

        //8 ---------------------------------------------
        custom_f_inverse_4(
            &mut account_struct.cubic_range_0,
            &account_struct.f_f2_range,
        );

        //9 ---------------------------------------------
        custom_f_inverse_5(
            &account_struct.cubic_range_0,
            &account_struct.cubic_range_1,
            &mut account_struct.f_f2_range,
        );
        //reference_f;
        //println!("{:?}", reference_f);
        assert_eq!(
            reference_f.inverse().unwrap(),
            parse_f_from_bytes(&account_struct.f_f2_range),
            "f inverse failed"
        );
    }

    #[test]
    fn fe_unit_test_custom_inverse_test_fails() {
        let mut rng = test_rng();
        let reference_f =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                &mut rng,
            );
        let actual_f =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                &mut rng,
            );
        let mut account_struct = FinalExponentiationState::new();

        parse_f_to_bytes(actual_f, &mut account_struct.f_f2_range);

        //1 ---------------------------------------------
        custom_f_inverse_1(
            &account_struct.f_f2_range,
            &mut account_struct.cubic_range_1,
        );

        //2 ---------------------------------------------
        custom_f_inverse_2(
            &account_struct.f_f2_range,
            &mut account_struct.cubic_range_0,
            &account_struct.cubic_range_1,
        );

        //3 ---------------------------------------------
        custom_cubic_inverse_1(
            &account_struct.cubic_range_0,
            &mut account_struct.quad_range_0,
            &mut account_struct.quad_range_1,
            &mut account_struct.quad_range_2,
            &mut account_struct.quad_range_3,
        );

        //4 ---------------------------------------------
        //quad inverse is part of cubic Inverse
        custom_quadratic_fp256_inverse_1(
            &account_struct.quad_range_3,
            &mut account_struct.fp256_range,
        );

        //5 ---------------------------------------------
        custom_quadratic_fp256_inverse_2(
            &mut account_struct.quad_range_3,
            &account_struct.fp256_range,
        );

        //6 ---------------------------------------------
        custom_cubic_inverse_2(
            &mut account_struct.cubic_range_0,
            &account_struct.quad_range_0,
            &account_struct.quad_range_1,
            &account_struct.quad_range_2,
            &account_struct.quad_range_3,
        );

        //7 ---------------------------------------------
        custom_f_inverse_3(
            &mut account_struct.cubic_range_1,
            &account_struct.cubic_range_0,
            &account_struct.f_f2_range,
        );

        //8 ---------------------------------------------
        custom_f_inverse_4(
            &mut account_struct.cubic_range_0,
            &account_struct.f_f2_range,
        );

        //9 ---------------------------------------------
        custom_f_inverse_5(
            &account_struct.cubic_range_0,
            &account_struct.cubic_range_1,
            &mut account_struct.f_f2_range,
        );
        //reference_f;
        //println!("{:?}", reference_f);
        assert!(
            reference_f.inverse().unwrap() != parse_f_from_bytes(&account_struct.f_f2_range),
            "f inverse failed"
        );
    }

    #[test]
    fn fe_unit_test_mul_assign_test_correct() {
        let mut rng = test_rng();
        let mut reference_f =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                &mut rng,
            );
        let mul_f =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                &mut rng,
            );

        let actual_f = reference_f.clone();
        let mut account_struct = FinalExponentiationState::new();

        parse_f_to_bytes(actual_f, &mut account_struct.f1_r_range);
        parse_f_to_bytes(mul_f, &mut account_struct.f_f2_range);

        mul_assign_1(
            &account_struct.f1_r_range,
            &account_struct.f_f2_range,
            &mut account_struct.cubic_range_0,
            &mut account_struct.cubic_range_1,
        );

        mul_assign_2(
            &account_struct.f_f2_range,
            &account_struct.cubic_range_0,
            &account_struct.cubic_range_1,
            &mut account_struct.f1_r_range,
        );
        reference_f *= mul_f;
        assert_eq!(
            reference_f,
            parse_f_from_bytes(&account_struct.f1_r_range),
            "f mulassign failed"
        );
    }

    #[test]
    fn fe_unit_test_mul_assign_test_fails() {
        let mut rng = test_rng();
        let mut reference_f =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                &mut rng,
            );
        let mul_f =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                &mut rng,
            );

        let actual_f =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::rand(
                &mut rng,
            );
        let mut account_struct = FinalExponentiationState::new();

        parse_f_to_bytes(actual_f, &mut account_struct.f1_r_range);
        parse_f_to_bytes(mul_f, &mut account_struct.f_f2_range);

        mul_assign_1(
            &account_struct.f1_r_range,
            &account_struct.f_f2_range,
            &mut account_struct.cubic_range_0,
            &mut account_struct.cubic_range_1,
        );

        mul_assign_2(
            &account_struct.f_f2_range,
            &account_struct.cubic_range_0,
            &account_struct.cubic_range_1,
            &mut account_struct.f1_r_range,
        );
        reference_f *= mul_f;
        assert!(
            reference_f != parse_f_from_bytes(&account_struct.f1_r_range),
            "f mulassign failed"
        );
    }
    /*
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
    */
}
