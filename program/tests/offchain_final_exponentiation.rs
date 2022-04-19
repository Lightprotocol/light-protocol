mod test_utils;

#[cfg(test)]
pub mod tests {
    use crate::test_utils::tests::{get_vk_from_file, read_test_data};
    use ark_bn254;
    use ark_bn254::Fr;
    use ark_ec;
    use ark_ec::ProjectiveCurve;
    use ark_ff::bytes::FromBytes;
    use ark_ff::Field;
    use ark_ff::Fp12;
    use ark_groth16::prepare_verifying_key;
    use ark_groth16::{prepare_inputs, verify_proof};
    use ark_std::vec::Vec;
    use light_protocol_program::groth16_verifier::final_exponentiation::{
        instructions::*, processor::_process_instruction, ranges::*,
        state::FinalExponentiationState,
    };
    use light_protocol_program::groth16_verifier::parsers::*;
    use light_protocol_program::utils::config::ENCRYPTED_UTXOS_LENGTH;
    use serde_json::Result;
    use solana_program::program_pack::Pack;
    pub const INSTRUCTION_ORDER_CONST: [u8; 371] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 10, 11, 14, 15, 15, 15, 15, 16, 17, 15, 15,
        16, 17, 15, 15, 15, 18, 19, 15, 15, 16, 17, 15, 15, 16, 17, 15, 15, 18, 19, 15, 15, 15, 16,
        17, 15, 15, 16, 17, 15, 15, 18, 19, 15, 15, 18, 19, 15, 15, 18, 19, 15, 15, 16, 17, 15, 15,
        15, 15, 16, 17, 15, 15, 15, 16, 17, 15, 15, 16, 17, 15, 15, 16, 17, 15, 15, 18, 19, 15, 15,
        16, 17, 15, 15, 15, 16, 17, 15, 15, 15, 15, 15, 16, 17, 15, 15, 16, 17, 15, 15, 15, 15, 15,
        18, 19, 15, 15, 15, 15, 16, 17, 20, 21, 22, 23, 24, 25, 25, 25, 25, 26, 27, 25, 25, 26, 27,
        25, 25, 25, 28, 29, 25, 25, 26, 27, 25, 25, 26, 27, 25, 25, 28, 29, 25, 25, 25, 26, 27, 25,
        25, 26, 27, 25, 25, 28, 29, 25, 25, 28, 29, 25, 25, 28, 29, 25, 25, 26, 27, 25, 25, 25, 25,
        26, 27, 25, 25, 25, 26, 27, 25, 25, 26, 27, 25, 25, 26, 27, 25, 25, 28, 29, 25, 25, 26, 27,
        25, 25, 25, 26, 27, 25, 25, 25, 25, 25, 26, 27, 25, 25, 26, 27, 25, 25, 25, 25, 25, 28, 29,
        25, 25, 25, 25, 26, 27, 30, 31, 32, 32, 32, 32, 33, 34, 32, 32, 33, 34, 32, 32, 32, 35, 36,
        32, 32, 33, 34, 32, 32, 33, 34, 32, 32, 35, 36, 32, 32, 32, 33, 34, 32, 32, 33, 34, 32, 32,
        35, 36, 32, 32, 35, 36, 32, 32, 35, 36, 32, 32, 33, 34, 32, 32, 32, 32, 33, 34, 32, 32, 32,
        33, 34, 32, 32, 33, 34, 32, 32, 33, 34, 32, 32, 35, 36, 32, 32, 33, 34, 32, 32, 32, 33, 34,
        32, 32, 32, 32, 32, 33, 34, 32, 32, 33, 34, 32, 32, 32, 32, 32, 35, 36, 32, 32, 32, 32, 33,
        34, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 38, 39, 52, 53, 54, 55, 42,
        43,
    ];

    #[test]
    fn verification_test() -> Result<()> {
        let pvk_unprepped = get_vk_from_file()?;
        let pvk = prepare_verifying_key(&pvk_unprepped);
        let mut ix_data = read_test_data(String::from("deposit.txt"));
        ix_data = ix_data[9..].to_vec();
        let proof_a = parse_x_group_affine_from_bytes(&ix_data[224..288].to_vec());
        let proof_b = parse_proof_b_from_bytes(&ix_data[288..416].to_vec());
        let proof_c = parse_x_group_affine_from_bytes(&ix_data[416..480].to_vec());
        let mut public_inputs = Vec::new();
        for input in ix_data[..224].chunks(32) {
            public_inputs.push(<Fr as FromBytes>::read(&*input).unwrap());
        }
        let proof =
            ark_groth16::data_structures::Proof::<ark_ec::models::bn::Bn<ark_bn254::Parameters>> {
                a: proof_a,
                b: proof_b,
                c: proof_c,
            };
        let res = verify_proof(&pvk, &proof, &public_inputs[..]);

        assert!(res.unwrap());
        Ok(())
    }

    #[test]
    fn fe_test_offchain() -> Result<()> {
        let pvk_unprepped = get_vk_from_file()?;
        let pvk = prepare_verifying_key(&pvk_unprepped);

        let mut ix_data = read_test_data(String::from("deposit.txt"));
        ix_data = ix_data[9..].to_vec();
        let proof_a = parse_x_group_affine_from_bytes(&ix_data[224..288].to_vec());
        let proof_b = parse_proof_b_from_bytes(&ix_data[288..416].to_vec());
        let proof_c = parse_x_group_affine_from_bytes(&ix_data[416..480].to_vec());
        let mut public_inputs = Vec::new();
        for input in ix_data[..224].chunks(32) {
            public_inputs.push(<Fr as FromBytes>::read(&*input).unwrap());
        }

        let prepared_inputs = prepare_inputs(&pvk, &public_inputs[..]).unwrap();

        let miller_output =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::miller_loop(
                [
                    (proof_a.into(), proof_b.into()),
                    (
                        (prepared_inputs).into_affine().into(),
                        pvk.gamma_g2_neg_pc.clone(),
                    ),
                    (proof_c.into(), pvk.delta_g2_neg_pc.clone()),
                ]
                .iter(),
            );
        //starting result
        let f = miller_output;

        //library result for reference
        let res_origin = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::final_exponentiation(&f).unwrap();
        let res_custom = final_exponentiation_custom(&f).unwrap();
        assert_eq!(res_origin, res_custom);
        let res_processor = final_exponentiation_test_processor(&f).unwrap();
        assert_eq!(res_origin, res_processor);

        assert_eq!(res_origin, pvk.alpha_g1_beta_g2);
        Ok(())
    }

    #[allow(clippy::let_and_return)]
    fn final_exponentiation_custom(
        f: &<ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk,
    ) -> Option<<ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk> {
        //adapted from ark_ec bn254
        //executes fe_instructions alongside reference implementation and
        //asserts after every step

        //from original repo:
        // Easy part: result = elt^((q^6-1)*(q^2+1)).
        // Follows, e.g., Beuchat et al page 9, by computing result as follows:
        //   elt^((q^6-1)*(q^2+1)) = (conj(elt) * elt^(-1))^(q^2+1)
        let mut instruction_order = Vec::new();
        /*
        	* ------------------------- init -------------------------
        	*instruction 0
        	*/
        let mut account_struct = FinalExponentiationState::new();
        let mut account_struct1 = FinalExponentiationState::new();

        // f1 = r.conjugate() = f^(p^6)
        let mut f1 = *f;
        parse_f_to_bytes(*f, &mut account_struct.f1_r_range);
        parse_f_to_bytes(*f, &mut account_struct1.f_f2_range);

        assert_eq!(
            f1,
            parse_f_from_bytes(&account_struct.f1_r_range),
            "0 failed"
        );
        assert_eq!(account_struct1.y0_range, account_struct.y0_range);
        assert_eq!(account_struct1.y1_range, account_struct.y1_range);
        assert_eq!(account_struct1.y2_range, account_struct.y2_range);
        assert_eq!(account_struct1.y6_range, account_struct.y6_range);
        _process_instruction(&mut account_struct1, 0).unwrap();

        let reference_f = f.clone();
        account_struct.f_f2_range = account_struct.f1_r_range.clone();
        /*
        	* ------------------------- conjugate -------------------------
        	*instruction 0
        	*/
        f1.conjugate();
        conjugate_wrapper(&mut account_struct.f1_r_range);
        instruction_order.push(0);
        assert_eq!(
            f1,
            parse_f_from_bytes(&account_struct.f1_r_range),
            "1 failed"
        );

        /*
        	*
        	* ------------------------- Inverse -------------------------
        	* instruction 1
        	*/
        instruction_order.push(1);
        assert_eq!(account_struct1.y0_range, account_struct.y0_range);
        assert_eq!(account_struct1.y1_range, account_struct.y1_range);
        assert_eq!(account_struct1.y2_range, account_struct.y2_range);
        assert_eq!(account_struct1.y6_range, account_struct.y6_range);
        _process_instruction(&mut account_struct1, 1).unwrap();
        custom_f_inverse_1(
            &account_struct.f_f2_range,
            &mut account_struct.cubic_range_1,
        );

        //instruction 2 ---------------------------------------------
        instruction_order.push(2);
        assert_eq!(account_struct1.y0_range, account_struct.y0_range);
        assert_eq!(account_struct1.y1_range, account_struct.y1_range);
        assert_eq!(account_struct1.y2_range, account_struct.y2_range);
        assert_eq!(account_struct1.y6_range, account_struct.y6_range);
        _process_instruction(&mut account_struct1, 2).unwrap();

        custom_f_inverse_2(
            &account_struct.f_f2_range,
            &mut account_struct.cubic_range_0,
            &account_struct.cubic_range_1,
        );

        //instruction 3 ---------------------------------------------
        instruction_order.push(3);
        assert_eq!(account_struct1.y0_range, account_struct.y0_range);
        assert_eq!(account_struct1.y1_range, account_struct.y1_range);
        assert_eq!(account_struct1.y2_range, account_struct.y2_range);
        assert_eq!(account_struct1.y6_range, account_struct.y6_range);
        _process_instruction(&mut account_struct1, 3).unwrap();

        custom_cubic_inverse_1(
            &account_struct.cubic_range_0,
            &mut account_struct.quad_range_0,
            &mut account_struct.quad_range_1,
            &mut account_struct.quad_range_2,
            &mut account_struct.quad_range_3,
        );

        //instruction 4 ---------------------------------------------
        instruction_order.push(4);
        assert_eq!(account_struct1.y0_range, account_struct.y0_range);
        assert_eq!(account_struct1.y1_range, account_struct.y1_range);
        assert_eq!(account_struct1.y2_range, account_struct.y2_range);
        assert_eq!(account_struct1.y6_range, account_struct.y6_range);
        _process_instruction(&mut account_struct1, 4).unwrap();

        //quad inverse is part of cubic Inverse
        custom_quadratic_fp256_inverse_1(
            &account_struct.quad_range_3,
            &mut account_struct.fp256_range,
        );

        //instruction 5 ---------------------------------------------
        instruction_order.push(5);
        assert_eq!(account_struct1.y0_range, account_struct.y0_range);
        assert_eq!(account_struct1.y1_range, account_struct.y1_range);
        assert_eq!(account_struct1.y2_range, account_struct.y2_range);
        assert_eq!(account_struct1.y6_range, account_struct.y6_range);
        _process_instruction(&mut account_struct1, 5).unwrap();

        custom_quadratic_fp256_inverse_2(
            &mut account_struct.quad_range_3,
            &account_struct.fp256_range,
        );

        //instruction 6 ---------------------------------------------
        instruction_order.push(6);
        assert_eq!(account_struct1.y0_range, account_struct.y0_range);
        assert_eq!(account_struct1.y1_range, account_struct.y1_range);
        assert_eq!(account_struct1.y2_range, account_struct.y2_range);
        assert_eq!(account_struct1.y6_range, account_struct.y6_range);
        _process_instruction(&mut account_struct1, 6).unwrap();

        custom_cubic_inverse_2(
            &mut account_struct.cubic_range_0,
            &account_struct.quad_range_0,
            &account_struct.quad_range_1,
            &account_struct.quad_range_2,
            &account_struct.quad_range_3,
        );

        //instruction 7 ---------------------------------------------
        instruction_order.push(7);
        assert_eq!(account_struct1.y0_range, account_struct.y0_range);
        assert_eq!(account_struct1.y1_range, account_struct.y1_range);
        assert_eq!(account_struct1.y2_range, account_struct.y2_range);
        assert_eq!(account_struct1.y6_range, account_struct.y6_range);
        _process_instruction(&mut account_struct1, 7).unwrap();

        custom_f_inverse_3(
            &mut account_struct.cubic_range_1,
            &account_struct.cubic_range_0,
            &account_struct.f_f2_range,
        );

        //instruction 8 ---------------------------------------------
        instruction_order.push(8);
        assert_eq!(account_struct1.y0_range, account_struct.y0_range);
        assert_eq!(account_struct1.y1_range, account_struct.y1_range);
        assert_eq!(account_struct1.y2_range, account_struct.y2_range);
        assert_eq!(account_struct1.y6_range, account_struct.y6_range);
        _process_instruction(&mut account_struct1, 8).unwrap();

        custom_f_inverse_4(
            &mut account_struct.cubic_range_0,
            &account_struct.f_f2_range,
        );

        //instruction 9 ---------------------------------------------
        instruction_order.push(9);
        assert_eq!(account_struct1.y0_range, account_struct.y0_range);
        assert_eq!(account_struct1.y1_range, account_struct.y1_range);
        assert_eq!(account_struct1.y2_range, account_struct.y2_range);
        assert_eq!(account_struct1.y6_range, account_struct.y6_range);
        _process_instruction(&mut account_struct1, 9).unwrap();

        custom_f_inverse_5(
            &account_struct.cubic_range_0,
            &account_struct.cubic_range_1,
            &mut account_struct.f_f2_range,
        );

        assert_eq!(
            reference_f.inverse().unwrap(),
            parse_f_from_bytes(&account_struct.f_f2_range),
            "f inverse failed"
        );
        assert_eq!(f1, parse_f_from_bytes(&account_struct.f1_r_range));

        f.inverse().map(|mut f2| {
            /*
            	*
            	*
            	* ------------------------- mul -------------------------
            	*
            	*
            	*/
            //from original repo
            // f2 = f^(-1);
            // r = f^(p^6 - 1)

            let mut r = f1 * &f2;

            assert_eq!(f2, parse_f_from_bytes(&account_struct.f_f2_range));
            assert_eq!(f1, parse_f_from_bytes(&account_struct.f1_r_range));
            //instruction 10 ---------------------------------------------
            instruction_order.push(10);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 10).unwrap();

            mul_assign_1(
                &account_struct.f1_r_range,
                &account_struct.f_f2_range,
                &mut account_struct.cubic_range_0,
                &mut account_struct.cubic_range_1,
            );

            //instruction 11 ---------------------------------------------
            instruction_order.push(11);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 11).unwrap();

            mul_assign_2(
                &account_struct.f_f2_range,
                &account_struct.cubic_range_0,
                &account_struct.cubic_range_1,
                &mut account_struct.f1_r_range,
            );

            assert_eq!(
                r,
                parse_f_from_bytes(&account_struct.f1_r_range),
                "f mulassign failed"
            );

            /*
            	*
            	*
            	* ------------------------- assign -------------------------
            	*
            	*
            	*/
            //from original repo
            // f2 = f^(p^6 - 1)
            f2 = r;
            //instruction 12 ---------------------------------------------
            instruction_order.push(12);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 12).unwrap();

            account_struct.f_f2_range = account_struct.f1_r_range.clone();
            assert_eq!(f2, parse_f_from_bytes(&account_struct.f_f2_range));

            /*
            	*
            	*
            	* ------------------------- frobenius_map(2) -------------------------
            	*
            	*
            	*/
            //from original repo
            // r = f^((p^6 - 1)(p^2))
            r.frobenius_map(2);

            //instruction 13 ---------------------------------------------
            instruction_order.push(13);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 13).unwrap();

            custom_frobenius_map_2(&mut account_struct.f1_r_range);

            assert_eq!(r, parse_f_from_bytes(&account_struct.f1_r_range));

            /*
            	*
            	*
            	* ------------------------- mulassign -------------------------
            	*
            	*
            	*/
            //from original repo
            // r = f^((p^6 - 1)(p^2) + (p^6 - 1))
            // r = f^((p^6 - 1)(p^2 + 1))
            r *= &f2;
            //f2 last used here
            //instruction 10 ---------------------------------------------
            instruction_order.push(10);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 10).unwrap();

            mul_assign_1(
                &account_struct.f1_r_range,
                &account_struct.f_f2_range,
                &mut account_struct.cubic_range_0,
                &mut account_struct.cubic_range_1,
            );

            //instruction 11 ---------------------------------------------
            instruction_order.push(11);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 11).unwrap();

            mul_assign_2(
                &account_struct.f_f2_range,
                &account_struct.cubic_range_0,
                &account_struct.cubic_range_1,
                &mut account_struct.f1_r_range,
            );

            assert_eq!(
                r,
                parse_f_from_bytes(&account_struct.f1_r_range),
                "f mulassign failed"
            );

            /*
            	*
            	*
            	* ------------------------- exp_by_neg_x(r) -------------------------
            	*
            	*
            	*/
            //from original repo
            // Hard part follows Laura Fuentes-Castaneda et al. "Faster hashing to G2"
            // by computing:
            //
            // result = elt^(q^3 * (12*z^3 + 6z^2 + 4z - 1) +
            //               q^2 * (12*z^3 + 6z^2 + 6z) +
            //               q   * (12*z^3 + 6z^2 + 4z) +
            //               1   * (12*z^3 + 12z^2 + 6z + 1))
            // which equals
            //
            // result = elt^( 2z * ( 6z^2 + 3z + 1 ) * (q^4 - q^2 + 1)/r ).

            let y0 = exp_by_neg_x(r);

            //init
            //instruction 14 ---------------------------------------------
            instruction_order.push(14);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 14).unwrap();

            account_struct.i_range = account_struct.f1_r_range.clone();
            conjugate_wrapper(&mut account_struct.i_range);
            account_struct.y0_range = account_struct.f1_r_range.clone();

            for i in 1..63 {
                if i == 1 {
                    assert_eq!(account_struct.y0_range, account_struct.f1_r_range);
                }
                println!("i {}", i);
                //cyclotomic_exp
                //instruction 15 ---------------------------------------------
                instruction_order.push(15);
                assert_eq!(account_struct1.y0_range, account_struct.y0_range);
                assert_eq!(account_struct1.y1_range, account_struct.y1_range);
                assert_eq!(account_struct1.y2_range, account_struct.y2_range);
                assert_eq!(account_struct1.y6_range, account_struct.y6_range);
                _process_instruction(&mut account_struct1, 15).unwrap();

                custom_cyclotomic_square_in_place(&mut account_struct.y0_range);

                if NAF_VEC[i] != 0 {
                    if NAF_VEC[i] > 0 {
                        //println!("if i {}", i);
                        //instruction 16 ---------------------------------------------
                        instruction_order.push(16);
                        assert_eq!(account_struct1.y0_range, account_struct.y0_range);
                        assert_eq!(account_struct1.y1_range, account_struct.y1_range);
                        assert_eq!(account_struct1.y2_range, account_struct.y2_range);
                        assert_eq!(account_struct1.y6_range, account_struct.y6_range);
                        _process_instruction(&mut account_struct1, 16).unwrap();
                        mul_assign_1(
                            &account_struct.y0_range,
                            &account_struct.f1_r_range,
                            &mut account_struct.cubic_range_0,
                            &mut account_struct.cubic_range_1,
                        );

                        //instruction 17 ---------------------------------------------
                        instruction_order.push(17);
                        assert_eq!(account_struct1.y0_range, account_struct.y0_range);
                        assert_eq!(account_struct1.y1_range, account_struct.y1_range);
                        assert_eq!(account_struct1.y2_range, account_struct.y2_range);
                        assert_eq!(account_struct1.y6_range, account_struct.y6_range);
                        _process_instruction(&mut account_struct1, 17).unwrap();

                        mul_assign_2(
                            &account_struct.f1_r_range,
                            &account_struct.cubic_range_0,
                            &account_struct.cubic_range_1,
                            &mut account_struct.y0_range,
                        );
                    } else {
                        //instruction 18 ---------------------------------------------
                        instruction_order.push(18);
                        assert_eq!(account_struct1.y0_range, account_struct.y0_range);
                        assert_eq!(account_struct1.y1_range, account_struct.y1_range);
                        assert_eq!(account_struct1.y2_range, account_struct.y2_range);
                        assert_eq!(account_struct1.y6_range, account_struct.y6_range);
                        _process_instruction(&mut account_struct1, 18).unwrap();
                        mul_assign_1(
                            &account_struct.y0_range,
                            &account_struct.i_range,
                            &mut account_struct.cubic_range_0,
                            &mut account_struct.cubic_range_1,
                        );

                        //instruction 19 ---------------------------------------------
                        instruction_order.push(19);
                        assert_eq!(account_struct1.y0_range, account_struct.y0_range);
                        assert_eq!(account_struct1.y1_range, account_struct.y1_range);
                        assert_eq!(account_struct1.y2_range, account_struct.y2_range);
                        assert_eq!(account_struct1.y6_range, account_struct.y6_range);
                        _process_instruction(&mut account_struct1, 19).unwrap();
                        mul_assign_2(
                            &account_struct.i_range,
                            &account_struct.cubic_range_0,
                            &account_struct.cubic_range_1,
                            &mut account_struct.y0_range,
                        );
                    }
                }
            }

            //instruction 20 ---------------------------------------------
            instruction_order.push(20);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 20).unwrap();

            conjugate_wrapper(&mut account_struct.y0_range);
            assert_eq!(
                y0,
                parse_f_from_bytes(&account_struct.y0_range),
                "exp_by_neg_x(r) "
            );

            /*
            	*
            	*
            	* ------------------------- y0.cyclotomic_square() -------------------------
            	* still instruction 20
            	*
            	*/

            let y1 = y0.cyclotomic_square();
            custom_cyclotomic_square(&account_struct.y0_range, &mut account_struct.y1_range);
            assert_eq!(
                y1,
                parse_f_from_bytes(&account_struct.y1_range),
                "exp_by_neg_x(r) "
            );
            //y0 last used

            /*
            	*
            	*
            	* ------------------------- y0.cyclotomic_square() -------------------------
            	*
            	*
            	*/

            let y2 = y1.cyclotomic_square();
            //y2 is stored in y0_range
            //instruction 21 ---------------------------------------------
            instruction_order.push(21);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 21).unwrap();

            custom_cyclotomic_square(&account_struct.y1_range, &mut account_struct.y0_range);
            assert_eq!(
                y2,
                parse_f_from_bytes(&account_struct.y0_range),
                "exp_by_neg_x(r) "
            );

            /*
            	*
            	*
            	* ------------------------- mulassign -------------------------
            	*
            	*
            	*/

            let mut y3 = y2 * &y1;
            //y3 is stored in y0_range

            //instruction 22 ---------------------------------------------
            instruction_order.push(22);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 22).unwrap();
            mul_assign_1(
                &account_struct.y0_range,
                &account_struct.y1_range,
                &mut account_struct.cubic_range_0,
                &mut account_struct.cubic_range_1,
            );

            //instruction 23 ---------------------------------------------
            instruction_order.push(23);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 23).unwrap();
            mul_assign_2(
                &account_struct.y1_range,
                &account_struct.cubic_range_0,
                &account_struct.cubic_range_1,
                &mut account_struct.y0_range,
            );

            assert_eq!(
                y3,
                parse_f_from_bytes(&account_struct.y0_range),
                "mulassign "
            );

            /*
            	*
            	*
            	* ------------------------- y4 = exp_by_neg_x(y3) -------------------------
            	*
            	*
            	*/

            let y4 = exp_by_neg_x(y3);
            //y4 is stored in y2_range

            //init
            //instruction 24 ---------------------------------------------
            instruction_order.push(24);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 24).unwrap();
            account_struct.i_range = account_struct.y0_range.clone();
            conjugate_wrapper(&mut account_struct.i_range);
            account_struct.y2_range = account_struct.y0_range.clone();

            for i in 1..63 {
                //cyclotomic_exp
                //instruction 25 ---------------------------------------------
                instruction_order.push(25);
                assert_eq!(account_struct1.y0_range, account_struct.y0_range);
                assert_eq!(account_struct1.y1_range, account_struct.y1_range);
                assert_eq!(account_struct1.y2_range, account_struct.y2_range);
                assert_eq!(account_struct1.y6_range, account_struct.y6_range);
                _process_instruction(&mut account_struct1, 25).unwrap();
                custom_cyclotomic_square_in_place(&mut account_struct.y2_range);

                if NAF_VEC[i] != 0 {
                    if NAF_VEC[i] > 0 {
                        //instruction 26 ---------------------------------------------
                        instruction_order.push(26);
                        assert_eq!(account_struct1.y0_range, account_struct.y0_range);
                        assert_eq!(account_struct1.y1_range, account_struct.y1_range);
                        assert_eq!(account_struct1.y2_range, account_struct.y2_range);
                        assert_eq!(account_struct1.y6_range, account_struct.y6_range);
                        _process_instruction(&mut account_struct1, 26).unwrap();
                        mul_assign_1(
                            &account_struct.y2_range,
                            &account_struct.y0_range,
                            &mut account_struct.cubic_range_0,
                            &mut account_struct.cubic_range_1,
                        );

                        //instruction 27 ---------------------------------------------
                        instruction_order.push(27);
                        assert_eq!(account_struct1.y0_range, account_struct.y0_range);
                        assert_eq!(account_struct1.y1_range, account_struct.y1_range);
                        assert_eq!(account_struct1.y2_range, account_struct.y2_range);
                        assert_eq!(account_struct1.y6_range, account_struct.y6_range);
                        _process_instruction(&mut account_struct1, 27).unwrap();
                        mul_assign_2(
                            &account_struct.y0_range,
                            &account_struct.cubic_range_0,
                            &account_struct.cubic_range_1,
                            &mut account_struct.y2_range,
                        );
                    } else {
                        //instruction 28 ---------------------------------------------
                        instruction_order.push(28);
                        assert_eq!(account_struct1.y0_range, account_struct.y0_range);
                        assert_eq!(account_struct1.y1_range, account_struct.y1_range);
                        assert_eq!(account_struct1.y2_range, account_struct.y2_range);
                        assert_eq!(account_struct1.y6_range, account_struct.y6_range);
                        _process_instruction(&mut account_struct1, 28).unwrap();
                        mul_assign_1(
                            &account_struct.y2_range,
                            &account_struct.i_range,
                            &mut account_struct.cubic_range_0,
                            &mut account_struct.cubic_range_1,
                        );

                        //instruction 29 ---------------------------------------------
                        instruction_order.push(29);
                        assert_eq!(account_struct1.y0_range, account_struct.y0_range);
                        assert_eq!(account_struct1.y1_range, account_struct.y1_range);
                        assert_eq!(account_struct1.y2_range, account_struct.y2_range);
                        assert_eq!(account_struct1.y6_range, account_struct.y6_range);
                        _process_instruction(&mut account_struct1, 29).unwrap();
                        mul_assign_2(
                            &account_struct.i_range,
                            &account_struct.cubic_range_0,
                            &account_struct.cubic_range_1,
                            &mut account_struct.y2_range,
                        );
                    }
                }
            }

            //instruction 30 ---------------------------------------------
            instruction_order.push(30);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 30).unwrap();
            conjugate_wrapper(&mut account_struct.y2_range);

            assert_eq!(
                y4,
                parse_f_from_bytes(&account_struct.y2_range),
                "exp_by_neg_x(r) "
            );

            /*
            	*
            	*
            	* ------------------------- y4.cyclotomic_square() -------------------------
            	*
            	* r_range: 			r
            	* f_f2_range: 	f2 not used anymore
            	* y0_range:  		y3
            	* y1_range:			y1
            	* y2_range:			y4
            	* still instruction 30
            	*/

            let y5 = y4.cyclotomic_square();
            //y5 is stored in f_f2_range
            custom_cyclotomic_square(&account_struct.y2_range, &mut account_struct.f_f2_range);
            assert_eq!(
                y5,
                parse_f_from_bytes(&account_struct.f_f2_range),
                "cyclotomic_square "
            );

            /*
            	*
            	*
            	* ------------------------- y4 = exp_by_neg_x(y3) -------------------------
            	*
            	* r_range: 			r
            	* f_f2_range: 	y5 			//y5 last used here
            	* y0_range:  		y3
            	* y1_range:			y1
            	* y2_range:			y4
            	* y6_range:			free
            	*/

            let mut y6 = exp_by_neg_x(y5);
            //y4 is stored in y6_range

            //init
            //instruction 31 ---------------------------------------------
            instruction_order.push(31);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 31).unwrap();
            account_struct.i_range = account_struct.f_f2_range.clone();

            conjugate_wrapper(&mut account_struct.i_range);

            account_struct.y6_range = account_struct.f_f2_range.clone();

            for i in 1..63 {
                //cyclotomic_exp
                //instruction 32 ---------------------------------------------
                instruction_order.push(32);
                assert_eq!(account_struct1.y0_range, account_struct.y0_range);
                assert_eq!(account_struct1.y1_range, account_struct.y1_range);
                assert_eq!(account_struct1.y2_range, account_struct.y2_range);
                assert_eq!(account_struct1.y6_range, account_struct.y6_range);
                _process_instruction(&mut account_struct1, 32).unwrap();
                custom_cyclotomic_square_in_place(&mut account_struct.y6_range);

                if NAF_VEC[i] != 0 {
                    if NAF_VEC[i] > 0 {
                        //instruction 33 ---------------------------------------------
                        instruction_order.push(33);
                        assert_eq!(account_struct1.y0_range, account_struct.y0_range);
                        assert_eq!(account_struct1.y1_range, account_struct.y1_range);
                        assert_eq!(account_struct1.y2_range, account_struct.y2_range);
                        assert_eq!(account_struct1.y6_range, account_struct.y6_range);
                        _process_instruction(&mut account_struct1, 33).unwrap();
                        mul_assign_1(
                            &account_struct.y6_range,
                            &account_struct.f_f2_range,
                            &mut account_struct.cubic_range_0,
                            &mut account_struct.cubic_range_1,
                        );

                        //instruction 34 ---------------------------------------------
                        instruction_order.push(34);
                        assert_eq!(account_struct1.y0_range, account_struct.y0_range);
                        assert_eq!(account_struct1.y1_range, account_struct.y1_range);
                        assert_eq!(account_struct1.y2_range, account_struct.y2_range);
                        assert_eq!(account_struct1.y6_range, account_struct.y6_range);
                        _process_instruction(&mut account_struct1, 34).unwrap();
                        mul_assign_2(
                            &account_struct.f_f2_range,
                            &account_struct.cubic_range_0,
                            &account_struct.cubic_range_1,
                            &mut account_struct.y6_range,
                        );
                    } else {
                        //instruction 35 ---------------------------------------------
                        instruction_order.push(35);
                        assert_eq!(account_struct1.y0_range, account_struct.y0_range);
                        assert_eq!(account_struct1.y1_range, account_struct.y1_range);
                        assert_eq!(account_struct1.y2_range, account_struct.y2_range);
                        assert_eq!(account_struct1.y6_range, account_struct.y6_range);
                        _process_instruction(&mut account_struct1, 35).unwrap();
                        mul_assign_1(
                            &account_struct.y6_range,
                            &account_struct.i_range,
                            &mut account_struct.cubic_range_0,
                            &mut account_struct.cubic_range_1,
                        );

                        //instruction 36 ---------------------------------------------
                        instruction_order.push(36);
                        assert_eq!(account_struct1.y0_range, account_struct.y0_range);
                        assert_eq!(account_struct1.y1_range, account_struct.y1_range);
                        assert_eq!(account_struct1.y2_range, account_struct.y2_range);
                        assert_eq!(account_struct1.y6_range, account_struct.y6_range);
                        _process_instruction(&mut account_struct1, 36).unwrap();
                        mul_assign_2(
                            &account_struct.i_range,
                            &account_struct.cubic_range_0,
                            &account_struct.cubic_range_1,
                            &mut account_struct.y6_range,
                        );
                    }
                }
            }
            //instruction 37 ---------------------------------------------
            instruction_order.push(37);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 37).unwrap();

            //y6.conjugate();

            conjugate_wrapper(&mut account_struct.y6_range);

            assert_eq!(
                y6,
                parse_f_from_bytes(&account_struct.y6_range),
                "exp_by_neg_x(r) "
            );

            /*
            	*
            	*
            	* ------------------------- conjugate_wrapper -------------------------
            	*
            	* r_range: 			r
            	* f_f2_range: 	free
            	* y0_range:  		y3
            	* y1_range:			y1
            	* y2_range:			y4
            	* y6_range:			y6
            	* instruction 37
            	*/

            y3.conjugate();
            conjugate_wrapper(&mut account_struct.y0_range);

            y6.conjugate();

            conjugate_wrapper(&mut account_struct.y6_range);

            /*
            	*
            	*
            	* ------------------------- mul_assign -------------------------
            	*
            	* r_range: 			r
            	* f_f2_range: 	free
            	* y0_range:  		y3
            	* y1_range:			y1
            	* y2_range:			y4
            	* y6_range:			y6 last used
            	*/

            let y7 = y6 * &y4;
            // stored in y6_range

            //instruction 38 ---------------------------------------------
            instruction_order.push(38);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 38).unwrap();
            mul_assign_1(
                &account_struct.y6_range,
                &account_struct.y2_range,
                &mut account_struct.cubic_range_0,
                &mut account_struct.cubic_range_1,
            );

            //instruction 39 ---------------------------------------------
            instruction_order.push(39);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 39).unwrap();
            mul_assign_2(
                &account_struct.y2_range,
                &account_struct.cubic_range_0,
                &account_struct.cubic_range_1,
                &mut account_struct.y6_range,
            );

            assert_eq!(
                y7,
                parse_f_from_bytes(&account_struct.y6_range),
                "mulassign "
            );

            /*
            	*
            	*
            	* ------------------------- mul_assign -------------------------
            	*
            	* r_range: 			r
            	* f_f2_range: 	free
            	* y0_range:  		y3 last used
            	* y1_range:			y1
            	* y2_range:			y4
            	* y6_range:			y7 last used
            	*/
            let mut y8 = y7 * &y3;
            // stored in y6_range

            //instruction 40 ---------------------------------------------
            instruction_order.push(40);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 40).unwrap();
            mul_assign_1(
                &account_struct.y6_range,
                &account_struct.y0_range,
                &mut account_struct.cubic_range_0,
                &mut account_struct.cubic_range_1,
            );

            //instruction 41 ---------------------------------------------
            instruction_order.push(41);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 41).unwrap();
            mul_assign_2(
                &account_struct.y0_range,
                &account_struct.cubic_range_0,
                &account_struct.cubic_range_1,
                &mut account_struct.y6_range,
            );

            assert_eq!(
                y8,
                parse_f_from_bytes(&account_struct.y6_range),
                "mulassign "
            );

            /*
            	*
            	*
            	* ------------------------- mul_assign -------------------------
            	*
            	* r_range: 			r
            	* f_f2_range: 	free
            	* y0_range:  		free
            	* y1_range:			y1		last used
            	* y2_range:			y4
            	* y6_range:			y8
            	*/
            let y9 = y8 * &y1;
            // stored in y1_range

            //instruction 42 ---------------------------------------------
            instruction_order.push(42);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 42).unwrap();
            mul_assign_1(
                &account_struct.y1_range,
                &account_struct.y6_range,
                &mut account_struct.cubic_range_0,
                &mut account_struct.cubic_range_1,
            );

            //instruction 43 ---------------------------------------------
            instruction_order.push(43);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 43).unwrap();
            mul_assign_2(
                &account_struct.y6_range,
                &account_struct.cubic_range_0,
                &account_struct.cubic_range_1,
                &mut account_struct.y1_range,
            );

            assert_eq!(
                y9,
                parse_f_from_bytes(&account_struct.y1_range),
                "mulassign "
            );

            /*
            	*
            	*
            	* ------------------------- mul_assign -------------------------
            	*
            	* r_range: 			r
            	* f_f2_range: 	free
            	* y0_range:  		free
            	* y1_range:			y9
            	* y2_range:			y4 last used
            	* y6_range:			y8
            	*/
            let y10 = y8 * &y4;
            // stored in y2_range

            //instruction 44 ---------------------------------------------
            instruction_order.push(44);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 44).unwrap();
            mul_assign_1(
                &account_struct.y2_range,
                &account_struct.y6_range,
                &mut account_struct.cubic_range_0,
                &mut account_struct.cubic_range_1,
            );

            //instruction 45 ---------------------------------------------
            instruction_order.push(45);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 45).unwrap();
            mul_assign_2(
                &account_struct.y6_range,
                &account_struct.cubic_range_0,
                &account_struct.cubic_range_1,
                &mut account_struct.y2_range,
            );

            assert_eq!(
                y10,
                parse_f_from_bytes(&account_struct.y2_range),
                "mulassign "
            );

            /*
            	*
            	*
            	* ------------------------- mul_assign -------------------------
            	*
            	* r_range: 			r
            	* f_f2_range: 	free
            	* y0_range:  		free
            	* y1_range:			y9
            	* y2_range:			y10  last used
            	* y6_range:			y8
            	*/
            let y11 = y10 * &r;
            // stored in y2_range

            //instruction 46 ---------------------------------------------
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 46).unwrap();
            instruction_order.push(46);
            mul_assign_1(
                &account_struct.y2_range,
                &account_struct.f1_r_range,
                &mut account_struct.cubic_range_0,
                &mut account_struct.cubic_range_1,
            );

            //instruction 47 ---------------------------------------------
            instruction_order.push(47);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 47).unwrap();
            mul_assign_2(
                &account_struct.f1_r_range,
                &account_struct.cubic_range_0,
                &account_struct.cubic_range_1,
                &mut account_struct.y2_range,
            );

            assert_eq!(
                y11,
                parse_f_from_bytes(&account_struct.y2_range),
                "mulassign "
            );

            /*
            	*
            	*
            	* ------------------------- assign -------------------------
            	*
            	* r_range: 			r
            	* f_f2_range: 	free
            	* y0_range:  		free
            	* y1_range:			y9
            	* y2_range:			y11
            	* y6_range:			y8
            	*/

            let mut y12 = y9;

            //instruction 48 ---------------------------------------------
            instruction_order.push(48);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 48).unwrap();

            //assert_eq!(y12,  parse_f_from_bytes(&account_struct.y0_range));
            account_struct.y0_range = account_struct.y1_range.clone();
            /*
            	*
            	*
            	* ------------------------- frobenius_map(1) -------------------------
            	*
            	* r_range: 			r
            	* f_f2_range: 	free
            	* y0_range:  		y12
            	* y1_range:			y9
            	* y2_range:			y11
            	* y6_range:			y8
            	*/
            y12.frobenius_map(1);

            custom_frobenius_map_1(&mut account_struct.y0_range);

            assert_eq!(y12, parse_f_from_bytes(&account_struct.y0_range));

            /*
            	*
            	*
            	* ------------------------- mul_assign -------------------------
            	*
            	* r_range: 			r
            	* f_f2_range: 	free
            	* y0_range:  		y12
            	* y1_range:			y9
            	* y2_range:			y11 last used
            	* y6_range:			y8
            	*/
            let y13 = y12 * &y11;
            //y13 stored in y2_range

            //instruction 49 ---------------------------------------------
            instruction_order.push(49);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 49).unwrap();
            mul_assign_1(
                &account_struct.y2_range,
                &account_struct.y0_range,
                &mut account_struct.cubic_range_0,
                &mut account_struct.cubic_range_1,
            );

            //instruction 50 ---------------------------------------------
            instruction_order.push(50);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 50).unwrap();
            mul_assign_2(
                &account_struct.y0_range,
                &account_struct.cubic_range_0,
                &account_struct.cubic_range_1,
                &mut account_struct.y2_range,
            );

            assert_eq!(
                y13,
                parse_f_from_bytes(&account_struct.y2_range),
                "mulassign "
            );

            /*
            	*
            	*
            	* ------------------------- frobenius_map(2) -------------------------
            	*
            	* r_range: 			r
            	* f_f2_range: 	free
            	* y0_range:  		y12
            	* y1_range:			y9
            	* y2_range:			y13
            	* y6_range:			y8
            	*/
            y8.frobenius_map(2);

            //instruction 51 ---------------------------------------------
            instruction_order.push(51);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 51).unwrap();
            custom_frobenius_map_2(&mut account_struct.y6_range);

            assert_eq!(y8, parse_f_from_bytes(&account_struct.y6_range));

            /*
            	*
            	*
            	* ------------------------- mulassign -------------------------
            	*
            	* r_range: 			r
            	* f_f2_range: 	free
            	* y0_range:  		y12
            	* y1_range:			y9
            	* y2_range:			y13
            	* y6_range:			y8 last used
            	*/

            let y14 = y8 * &y13;
            //y14 stored in y6_range
            //instr110uctions already exist

            //instruction 38 ---------------------------------------------
            instruction_order.push(38);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 38).unwrap();
            mul_assign_1(
                &account_struct.y6_range,
                &account_struct.y2_range,
                &mut account_struct.cubic_range_0,
                &mut account_struct.cubic_range_1,
            );

            //instruction 39 ---------------------------------------------
            instruction_order.push(39);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 39).unwrap();
            mul_assign_2(
                &account_struct.y2_range,
                &account_struct.cubic_range_0,
                &account_struct.cubic_range_1,
                &mut account_struct.y6_range,
            );

            assert_eq!(
                y14,
                parse_f_from_bytes(&account_struct.y6_range),
                "mulassign "
            );

            /*
            	*
            	*
            	* ------------------------- conjugate -------------------------
            	*
            	* r_range: 			r
            	* f_f2_range: 	free
            	* y0_range:  		y12
            	* y1_range:			y9
            	* y2_range:			y13
            	* y6_range:			y14
            	*/

            //instruction 52 ---------------------------------------------
            instruction_order.push(52);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 52).unwrap();
            r.conjugate();
            conjugate_wrapper(&mut account_struct.f1_r_range);

            /*
            	*
            	*
            	* ------------------------- mul_assign -------------------------
            	*
            	* r_range: 			r		last used
            	* f_f2_range: 	free
            	* y0_range:  		y12
            	* y1_range:			y9		last used
            	* y2_range:			y13
            	* y6_range:			y14
            	*/

            let mut y15 = r * &y9;
            //y15 stored in y1_range

            //instruction 53 ---------------------------------------------
            instruction_order.push(53);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 53).unwrap();
            mul_assign_1(
                &account_struct.y1_range,
                &account_struct.f1_r_range,
                &mut account_struct.cubic_range_0,
                &mut account_struct.cubic_range_1,
            );

            //instruction 54 ---------------------------------------------
            instruction_order.push(54);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 54).unwrap();
            mul_assign_2(
                &account_struct.f1_r_range,
                &account_struct.cubic_range_0,
                &account_struct.cubic_range_1,
                &mut account_struct.y1_range,
            );

            assert_eq!(
                y15,
                parse_f_from_bytes(&account_struct.y1_range),
                "mulassign "
            );

            /*
            	*
            	*
            	* ------------------------- frobenius_map(3) -------------------------
            	*
            	* r_range: 			r
            	* f_f2_range: 	free
            	* y0_range:  		y12
            	* y1_range:			y15
            	* y2_range:			y13
            	* y6_range:			y14
            	*/

            y15.frobenius_map(3);

            //instruction 55 ---------------------------------------------
            instruction_order.push(55);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 55).unwrap();
            custom_frobenius_map_3(&mut account_struct.y1_range);

            assert_eq!(y15, parse_f_from_bytes(&account_struct.y1_range));

            /*
            	*
            	*
            	* ------------------------- mulassign -------------------------
            	*
            	* r_range: 			r
            	* f_f2_range: 	free
            	* y0_range:  		y12
            	* y1_range:			y15
            	* y2_range:			y13
            	* y6_range:			y14
            	*/
            //not unique second time instruction 83

            let y16 = y15 * &y14;

            //instruction 42 ---------------------------------------------
            instruction_order.push(42);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 42).unwrap();
            mul_assign_1(
                &account_struct.y1_range,
                &account_struct.y6_range,
                &mut account_struct.cubic_range_0,
                &mut account_struct.cubic_range_1,
            );

            //instruction 43 ---------------------------------------------
            instruction_order.push(43);
            assert_eq!(account_struct1.y0_range, account_struct.y0_range);
            assert_eq!(account_struct1.y1_range, account_struct.y1_range);
            assert_eq!(account_struct1.y2_range, account_struct.y2_range);
            assert_eq!(account_struct1.y6_range, account_struct.y6_range);
            _process_instruction(&mut account_struct1, 43).unwrap();
            mul_assign_2(
                &account_struct.y6_range,
                &account_struct.cubic_range_0,
                &account_struct.cubic_range_1,
                &mut account_struct.y1_range,
            );

            assert_eq!(
                y16,
                parse_f_from_bytes(&account_struct.y1_range),
                "mulassign "
            );
            println!(
                "Let instruction order: [u8; {}] = {:?}",
                instruction_order.len(),
                instruction_order
            );
            y16
        })
    }

    fn final_exponentiation_test_processor(
        f: &<ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk,
    ) -> Option<<ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk> {
        let mut account_struct = FinalExponentiationState::new();

        parse_f_to_bytes(*f, &mut account_struct.f_f2_range);
        account_struct.changed_variables[F_F2_RANGE_ITER] = true;
        let mut account_onchain_slice = [0u8; 3900 + ENCRYPTED_UTXOS_LENGTH];
        //in
        account_onchain_slice[1] = 1;
        <FinalExponentiationState as Pack>::pack_into_slice(
            &account_struct,
            &mut account_onchain_slice,
        );
        // create data for onchain
        // let path = "tests/fe_onchain_init_bytes.rs";
        // let mut output = File::create(path).ok()?;
        // write!(
        //     output,
        //     "{}",
        //     format!(
        //         "pub const INIT_BYTES_FINAL_EXP : [u8;{}] = {:?};",
        //         account_onchain_slice.len(),
        //         account_onchain_slice
        //     )
        // );

        for i in INSTRUCTION_ORDER_CONST {
            let mut account_struct_tmp =
                <FinalExponentiationState as Pack>::unpack(&account_onchain_slice).unwrap();
            println!("processor iter : {}", i);
            _process_instruction(&mut account_struct_tmp, i).unwrap();
            account_struct.y1_range = account_struct_tmp.y1_range.clone();

            <FinalExponentiationState as Pack>::pack_into_slice(
                &account_struct_tmp,
                &mut account_onchain_slice,
            );
            assert_eq!(account_struct.y1_range, account_struct_tmp.y1_range);
        }
        println!("result in bytes: {:?}", account_struct.y1_range);
        verify_result(&account_struct).unwrap();
        Some(parse_f_from_bytes(&account_struct.y1_range))
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
}
