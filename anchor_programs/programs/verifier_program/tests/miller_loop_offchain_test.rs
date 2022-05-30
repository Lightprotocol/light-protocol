#[cfg(test)]
mod test {
    use ark_ff::QuadExtField;
    use ark_groth16::prepare_verifying_key;
    use serde_json::{Result, Value};
    use std::convert::TryInto;
    use std::fs;

    use ark_bn254::Fr;
    use ark_ec::bn::g2::G2HomProjective;
    use ark_ec::ProjectiveCurve;
    use ark_ff::bytes::FromBytes;
    use ark_ff::Fp2;
    use ark_ff::Fp256;
    use ark_ff::One;
    use ark_groth16::prepare_inputs;
    use solana_program::pubkey::Pubkey;
    use std::borrow::Borrow;
    use std::cell::{RefCell, RefMut};
    use verifier_program::groth16_verifier::miller_loop::{instructions::*, state::*};
    use verifier_program::groth16_verifier::parsers::*;
    use verifier_program::state::VerifierState;

    pub fn new_verifier_state() -> VerifierState {
        VerifierState {
            current_instruction_index: 0,
            signing_address: Pubkey::new(&[0; 32]),
            f_bytes: [0; 384],
            f_bytes1: [0; 384],
            f_bytes2: [0; 384],
            f_bytes3: [0; 384],
            f_bytes4: [0; 384],
            f_bytes5: [0; 384],
            i_bytes: [0; 384],
            fe_instruction_index: 0,
            max_compute: 1_250_000,
            current_compute: 0,
            first_exp_by_neg_x: 0,
            second_exp_by_neg_x: 0,
            third_exp_by_neg_x: 0,
            initialized: 0,
            outer_loop: 1,
            cyclotomic_square_in_place: 0,
            merkle_tree_tmp_account: Pubkey::new(&[0; 32]),
            relayer_fee: 0,
            recipient: Pubkey::new(&[0; 32]),
            amount: [0; 32],
            nullifier_hash: [0; 32],
            root_hash: [0; 32],
            tx_integrity_hash: [0; 32], // is calculated on-chain from recipient, amount, signing_address,
            proof_a_bytes: [0; 64], //ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>,
            proof_b_bytes: [0; 128], //ark_ec::models::bn::g2::G2Affine<ark_bn254::Parameters>,
            proof_c_bytes: [0; 64], //ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>,

            ext_amount: [0; 8],
            fee: [0; 8],
            leaf_left: [0; 32],
            leaf_right: [0; 32],
            nullifier0: [0; 32],
            nullifier1: [0; 32],

            i_1_range: [0; 32],
            x_1_range: [0; 64],
            i_2_range: [0; 32],
            x_2_range: [0; 64],
            i_3_range: [0; 32],
            x_3_range: [0; 64],
            i_4_range: [0; 32],
            x_4_range: [0; 64],
            i_5_range: [0; 32],
            x_5_range: [0; 64],
            i_6_range: [0; 32],
            x_6_range: [0; 64],
            i_7_range: [0; 32],
            x_7_range: [0; 64],

            res_x_range: [0; 32],
            res_y_range: [0; 32],
            res_z_range: [0; 32],

            g_ic_x_range: [0; 32],
            g_ic_y_range: [0; 32],
            g_ic_z_range: [0; 32],
            current_index: 0,

            // miller loop
            r_bytes: [0; 192], //ark_ec::models::bn::g2::G2HomProjective<ark_bn254::Parameters>,//[0;192],
            q1_bytes: [0; 128],
            current_coeff_bytes: [0; 192],

            outer_first_loop_coeff: 0,
            outer_second_coeff: 0,
            inner_first_coeff: 0,

            compute_max_miller_loop: 0,
            outer_first_loop: 0,
            outer_second_loop: 0,
            outer_third_loop: 0,
            first_inner_loop_index: 0,
            second_inner_loop_index: 0,
            square_in_place_executed: 0,

            coeff_index: [0; 3],

            computing_prepared_inputs: false, // 0 prepare inputs // 1 miller loop //
            computing_miller_loop: false,
            computing_final_exponentiation: true,

            merkle_tree_index: 0,
            found_root: 0,
            current_instruction_index_prepare_inputs: 0,
            encrypted_utxos: [0u8;222],
            last_transaction: false,
            merkle_tree_instruction_index:0,
            updating_merkle_tree:false,
        }
    }

    #[test]
    fn test_miller_loop_coeffs() {
        let pvk_unprepped = get_vk_from_file().unwrap();
        let pvk = prepare_verifying_key(&pvk_unprepped);

        let mut ix_data = read_test_data(String::from("deposit.txt"));
        ix_data = ix_data[9..].to_vec();
        let proof_a = parse_x_group_affine_from_bytes(&ix_data[224..288].try_into().unwrap());
        let proof_b = parse_proof_b_from_bytes(&ix_data[288..416].try_into().unwrap());
        let proof_c = parse_x_group_affine_from_bytes(&ix_data[416..480].try_into().unwrap());
        let mut public_inputs = Vec::new();
        for input in ix_data[..224].chunks(32) {
            public_inputs.push(<Fr as FromBytes>::read(&*input).unwrap());
        }
        let prepared_inputs = prepare_inputs(&pvk, &public_inputs[..]).unwrap();
        let mut prepared_inputs_bytes = [0u8; 64];
        parse_x_group_affine_to_bytes(
            (prepared_inputs).into_affine().into(),
            &mut prepared_inputs_bytes,
        );

        let tmp = RefCell::new(new_verifier_state());
        let mut tmp_account: RefMut<'_, VerifierState> = tmp.borrow_mut();
        tmp_account.proof_a_bytes = ix_data[224..288].try_into().unwrap();
        tmp_account.proof_b_bytes = ix_data[288..416].try_into().unwrap();
        tmp_account.proof_c_bytes = ix_data[416..480].try_into().unwrap();
        tmp_account.x_1_range = prepared_inputs_bytes.try_into().unwrap();
        tmp_account.max_compute = 1_000_000_000;
        tmp_account.r_bytes = parse_r_to_bytes(G2HomProjective {
            x: proof_b.x,
            y: proof_b.y,
            z: Fp2::one(),
        });
        tmp_account.f_bytes[0] = 1;
        tmp_account.compute_max_miller_loop = 1_000_000_000;
        let mut tmp_account_compute = MillerLoopStateCompute::new(tmp_account.borrow());

        _test_coeffs(
            [
                (proof_a.into(), proof_b.into()),
                (
                    (prepared_inputs).into_affine().into(),
                    pvk.gamma_g2_neg_pc.clone(),
                ),
                (proof_c.into(), pvk.delta_g2_neg_pc.clone()),
            ]
            .iter(),
            &mut tmp_account,
            &mut tmp_account_compute,
        );
    }

    fn _test_coeffs<'a, I>(
            i: I,
            tmp_account: &mut RefMut<'_, VerifierState>,
            tmp_account_compute: &mut MillerLoopStateCompute

        )
        where
            I: IntoIterator<Item = &'a (<ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::G1Prepared, <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::G2Prepared)>,
        {
        let mut total_steps: u64 = 0;
        let mut pairs = vec![];
        for (p, q) in i {
            if !p.is_zero() && !q.is_zero() {
                pairs.push((p, q.ell_coeffs.iter()));
            }
        }
        // println!("{:?}", pairs[0].1.next());
        for (index, coeff) in pairs[0].1.clone().enumerate() {
            assert_eq!(
                *coeff,
                get_coeff(0, tmp_account, &mut total_steps, tmp_account_compute).unwrap(),
                "failed at {}",
                index
            );
            println!("\ncoeff_index {:?}\n", tmp_account.coeff_index);
            println!("\ninner_first_coeff {}\n", tmp_account.inner_first_coeff);
            println!(
                "\nouter_first_loop_coeff {}\n",
                tmp_account.outer_first_loop_coeff
            );
            println!("\nouter_second_coeff {}\n", tmp_account.outer_second_coeff);
        }
        for (index, coeff) in pairs[1].1.clone().enumerate() {
            assert_eq!(
                *coeff,
                get_coeff(1, tmp_account, &mut total_steps, tmp_account_compute).unwrap(),
                "failed at {}",
                index
            );
        }
        for (index, coeff) in pairs[2].1.clone().enumerate() {
            assert_eq!(
                *coeff,
                get_coeff(2, tmp_account, &mut total_steps, tmp_account_compute).unwrap(),
                "failed at {}",
                index
            );
        }
    }

    pub fn get_vk_from_file() -> Result<
        ark_groth16::data_structures::VerifyingKey<ark_ec::models::bn::Bn<ark_bn254::Parameters>>,
    > {
        let contents = fs::read_to_string("./tests/test_data/verification_key_bytes_254.txt")
            .expect("Something went wrong reading the file");
        let v: Value = serde_json::from_str(&contents)?;
        let mut a_g1_bigints = Vec::new();
        for i in 0..2 {
            let mut bytes: Vec<u8> = Vec::new();
            for i in v["vk_alpha_1"][i].as_str().unwrap().split(',') {
                bytes.push((*i).parse::<u8>().unwrap());
            }
            a_g1_bigints
                .push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
        }
        let alpha_g1_bigints = ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
            a_g1_bigints[0],
            a_g1_bigints[1],
            false,
        );

        let mut b_g2_bigints = Vec::new();
        for i in 0..2 {
            for j in 0..2 {
                let mut bytes: Vec<u8> = Vec::new();
                for z in v["vk_beta_2"][i][j].as_str().unwrap().split(',') {
                    bytes.push((*z).parse::<u8>().unwrap());
                }
                b_g2_bigints
                    .push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
            }
        }

        let beta_g2 = ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters>::new(
            QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
                b_g2_bigints[0],
                b_g2_bigints[1],
            ),
            QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
                b_g2_bigints[2],
                b_g2_bigints[3],
            ),
            false,
        );

        let mut delta_g2_bytes = Vec::new();
        for i in 0..2 {
            for j in 0..2 {
                let mut bytes: Vec<u8> = Vec::new();
                for z in v["vk_delta_2"][i][j].as_str().unwrap().split(',') {
                    bytes.push((*z).parse::<u8>().unwrap());
                }
                delta_g2_bytes
                    .push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
            }
        }

        let delta_g2 = ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters>::new(
            QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
                delta_g2_bytes[0],
                delta_g2_bytes[1],
            ),
            QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
                delta_g2_bytes[2],
                delta_g2_bytes[3],
            ),
            false,
        );

        let mut gamma_g2_bytes = Vec::new();
        for i in 0..2 {
            for j in 0..2 {
                let mut bytes: Vec<u8> = Vec::new();
                for z in v["vk_gamma_2"][i][j].as_str().unwrap().split(',') {
                    bytes.push((*z).parse::<u8>().unwrap());
                }
                gamma_g2_bytes
                    .push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
            }
        }
        let gamma_g2 = ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters>::new(
            QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
                gamma_g2_bytes[0],
                gamma_g2_bytes[1],
            ),
            QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
                gamma_g2_bytes[2],
                gamma_g2_bytes[3],
            ),
            false,
        );

        let mut gamma_abc_g1_bigints_bytes = Vec::new();

        for i in 0..8 {
            let mut g1_bytes = Vec::new();
            for u in 0..2 {
                let mut bytes: Vec<u8> = Vec::new();
                for z in v["IC"][i][u].as_str().unwrap().split(',') {
                    bytes.push((*z).parse::<u8>().unwrap());
                }
                g1_bytes
                    .push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
            }
            gamma_abc_g1_bigints_bytes.push(ark_ec::models::bn::g1::G1Affine::<
                ark_bn254::Parameters,
            >::new(g1_bytes[0], g1_bytes[1], false));
        }

        Ok(ark_groth16::data_structures::VerifyingKey::<
            ark_ec::models::bn::Bn<ark_bn254::Parameters>,
        > {
            alpha_g1: alpha_g1_bigints,
            beta_g2: beta_g2,
            gamma_g2: gamma_g2,
            delta_g2: delta_g2,
            gamma_abc_g1: gamma_abc_g1_bigints_bytes,
        })
    }

    pub fn read_test_data(file_name: std::string::String) -> Vec<u8> {
        let mut path = std::string::String::from("./tests/test_data/");
        path.push_str(&file_name);
        println!("reading file: {:?}", path);
        let ix_data_file = fs::read_to_string(path).expect("Something went wrong reading the file");
        let ix_data_json: Value = serde_json::from_str(&ix_data_file).unwrap();
        let mut ix_data = Vec::new();
        for i in ix_data_json["bytes"][0].as_str().unwrap().split(',') {
            let j = (*i).parse::<u8>();
            match j {
                Ok(x) => (ix_data.push(x)),
                Err(_e) => (),
            }
        }
        println!("Appending merkle tree bytes and merkle tree index");
        // for i in 0..32 {
        //     ix_data.push(MERKLE_TREE_ACC_BYTES_ARRAY[0].0[i]);
        // }
        // //pushing merkle tree index
        // ix_data.push(0);

        println!("{:?}", ix_data);
        ix_data
    }
}
