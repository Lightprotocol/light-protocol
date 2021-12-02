#[cfg(test)]
mod tests {

	use ark_serialize::{Read, Write};
	use ark_std::vec::Vec;
	use ark_ed_on_bn254;
	use ark_ed_on_bn254::Fq;
    use ark_bn254;

	use arkworks_gadgets::utils::{
		get_mds_poseidon_circom_bn254_x5_3, get_rounds_poseidon_circom_bn254_x5_3, parse_vec,
	};
	use arkworks_gadgets::poseidon::{PoseidonParameters, PoseidonError, Rounds,circom::CircomCRH, sbox::PoseidonSbox};
	use ark_crypto_primitives::{crh::{TwoToOneCRH, CRH}, Error};
	use std::convert::TryInto;
    use ark_std::{UniformRand, test_rng};


	use ark_std::{One};

    use std::fs;
    use serde_json::{Result, Value};
    use ark_ff::biginteger::{BigInteger384, BigInteger256};
    use serde_json::Value::String;
    use ark_ec;
    use ark_ff::bytes::{ToBytes, FromBytes};
    use ark_ff::QuadExtField;
    use ark_ff::{Fp256};
    use ark_groth16::prepare_verifying_key;
    use ark_groth16::{verify_proof, prepare_inputs, verify_proof_with_prepared_inputs};


    use ark_ec::AffineCurve;
	use ark_ff::Field;

    use Testing_Hardcoded_Params_devnet_new::hard_coded_verifying_key_pvk_254::*;
	use Testing_Hardcoded_Params_devnet_new::init_bytes11;
	use Testing_Hardcoded_Params_devnet_new::processor_merkle_tree;
	use Testing_Hardcoded_Params_devnet_new::state_merkle_tree::{HashBytes, MerkleTree as MerkleTreeOnchain};
	use Testing_Hardcoded_Params_devnet_new::instructions_poseidon::PoseidonCircomRounds3;
	use Testing_Hardcoded_Params_devnet_new::instructions_final_exponentiation::*;
	use Testing_Hardcoded_Params_devnet_new::processor_final_exp::_process_instruction_final_exp;



    fn get_pvk_from_bytes_254() -> Result<ark_groth16::data_structures::VerifyingKey::<ark_ec::models::bn::Bn<ark_bn254::Parameters>>>{
        let contents = fs::read_to_string("./tests/verification_key_bytes_254.txt")
            .expect("Something went wrong reading the file");
        let v: Value = serde_json::from_str(&contents)?;
        //println!("{}",  v);

        //println!("With text:\n{:?}", v["vk_alpha_1"][1]);

        let mut a_g1_bigints = Vec::new();
        for i in 0..2{
            let mut bytes: Vec<u8> = Vec::new();
            for i in  v["vk_alpha_1"][i].as_str().unwrap().split(',') {
                bytes.push((*i).parse::<u8>().unwrap());
            }
            a_g1_bigints.push(<Fp256::<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
        }
        let alpha_g1_bigints =  ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
            a_g1_bigints[0],
            a_g1_bigints[1],
            false
        );
        println!(" alpha_g1 {}", alpha_g1_bigints);

        let mut b_g2_bigints = Vec::new();
        //println!("{}",  v["vk_beta_2"]);
        for i in 0..2 {
            for j in 0..2 {
                let mut bytes: Vec<u8> = Vec::new();
                for z in  v["vk_beta_2"][i][j].as_str().unwrap().split(',') {
                    bytes.push((*z).parse::<u8>().unwrap());
                }
                b_g2_bigints.push(<Fp256::<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
            }
        }

        let beta_g2 = ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters>::new(
    		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
    			b_g2_bigints[0],
                b_g2_bigints[1],
            ),
    		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
                b_g2_bigints[2],
                b_g2_bigints[3],
    		),
    		false
    	);
        for (i, _) in b_g2_bigints.iter().enumerate() {
            //println!("b_g2 {}", b_g2_bigints[i]);
        }


        let mut delta_g2_bytes = Vec::new();
        //println!("{}",  v["vk_delta_2"]);
        for i in 0..2 {
            for j in 0..2 {
                let mut bytes: Vec<u8> = Vec::new();
                for z in  v["vk_delta_2"][i][j].as_str().unwrap().split(',') {
                    bytes.push((*z).parse::<u8>().unwrap());
                }
                delta_g2_bytes.push(<Fp256::<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
            }
        }

        let delta_g2 = ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters>::new(
    		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
    			delta_g2_bytes[0],
                delta_g2_bytes[1],
            ),
    		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
                delta_g2_bytes[2],
                delta_g2_bytes[3],
    		),
    		false
    	);


        for (i, _) in delta_g2_bytes.iter().enumerate() {
            //println!("delta_g2 {}", delta_g2_bytes[i]);
        }

        let mut gamma_g2_bytes = Vec::new();
        //println!("{}",  v["vk_gamma_2"]);
        for i in 0..2 {
            for j in 0..2 {
                let mut bytes: Vec<u8> = Vec::new();
                for z in  v["vk_gamma_2"][i][j].as_str().unwrap().split(',') {
                    bytes.push((*z).parse::<u8>().unwrap());
                }
                gamma_g2_bytes.push(<Fp256::<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
            }
        }
        let gamma_g2 = ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters>::new(
    		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
    			gamma_g2_bytes[0],
                gamma_g2_bytes[1],
            ),
    		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
                gamma_g2_bytes[2],
                gamma_g2_bytes[3],
    		),
    		false
    	);

        for (i, _) in gamma_g2_bytes.iter().enumerate() {
            //println!("gamma_g2 {}", gamma_g2_bytes[i]);
        }


        let mut gamma_abc_g1_bigints_bytes = Vec::new();

        for i in 0..8 {

            //for j in 0..1 {
                let mut g1_bytes = Vec::new();
                //println!("{:?}", v["IC"][i][j]);
                //println!("Iter: {}", i);
                for u in 0..2 {
                    //println!("{:?}", v["IC"][i][u]);
                    let mut bytes: Vec<u8> = Vec::new();
                    for z in  v["IC"][i][u].as_str().unwrap().split(',') {
                        bytes.push((*z).parse::<u8>().unwrap());
                    }
                    //println!("bytes.len() {} {}", bytes.len(), bytes[bytes.len() - 1]);
                    g1_bytes.push(<Fp256::<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
                }
                gamma_abc_g1_bigints_bytes.push(ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
                    g1_bytes[0],
                    g1_bytes[1],
                    false
                ));
            //}
        }
        //println!("{:?}", gamma_abc_g1_bigints_bytes);

        //let string: str = String(v["vk_alpha_1"][1].to_string());

        // println!("{:?}", bytes);

        // let test_bigint: BigInteger384 = b"1303703767005851722774629343754215733783027212042241028503844419575444644222078222865715146086337294141748947680479".into();
        // println!("With text:\n{}", test_bigint);

        Ok(ark_groth16::data_structures::VerifyingKey::<ark_ec::models::bn::Bn<ark_bn254::Parameters>> {
        alpha_g1: alpha_g1_bigints,
        beta_g2: beta_g2,
        gamma_g2: gamma_g2,
        delta_g2: delta_g2,
        gamma_abc_g1:gamma_abc_g1_bigints_bytes
        })
    }

    fn get_proof_from_bytes_254() -> Result<ark_groth16::data_structures::Proof::<ark_ec::models::bn::Bn<ark_bn254::Parameters>>>{
        let contents = fs::read_to_string("./tests/proof_bytes_254.txt")
            .expect("Something went wrong reading the file");
        let v: Value = serde_json::from_str(&contents)?;
        //println!("{}",  v);

        //println!("With text:\n{:?}", v["vk_alpha_1"][1]);

        let mut a_g1_bigints = Vec::new();
        for i in 0..2{
            let mut bytes: Vec<u8> = Vec::new();
            for i in  v["pi_a"][i].as_str().unwrap().split(',') {
                bytes.push((*i).parse::<u8>().unwrap());
            }
            //println!("{}",v["pi_a"][i]);
            //println!("{:?}", bytes);
            a_g1_bigints.push(<Fp256::<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
        }
        //println!("{}", a_g1_bigints[0]);
    //use ark_ff::fields::models::Fp256;
        //println!("{}", ark_ff::Fp256::<ark_bn254::FqParameters>::new(a_g1_bigints[0]));
        //println!("{:?}",hex::decode("0B85DC05397EAC823D25C7C4682A0BE95141A33334C65A857D2491680F972BA4A5D50D9FB71A87E0594E32C02B9484E4").unwrap());
        let a_g1 =  ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
            //ark_ff::Fp256::<ark_bn254::FqParameters>::new(a_g1_bigints[0]),
            //ark_ff::Fp256::<ark_bn254::FqParameters>::new(a_g1_bigints[1]),
            a_g1_bigints[0],
            a_g1_bigints[1],
            false
        );
        //println!("{}", alpha_g1_bigints);

        let mut b_g2_bigints = Vec::new();
        //println!("{}",  v["vk_beta_2"]);
        for i in 0..2 {
            for j in 0..2 {
                let mut bytes: Vec<u8> = Vec::new();
                for z in  v["pi_b"][i][j].as_str().unwrap().split(',') {
                    bytes.push((*z).parse::<u8>().unwrap());
                }
                b_g2_bigints.push(<Fp256::<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
            }
        }
        let b_g2 = ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters>::new(
    		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
    			b_g2_bigints[0],
                b_g2_bigints[1],
            ),
    		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
                b_g2_bigints[2],
                b_g2_bigints[3],
    		),
    		false
    	);

        //println!("{:?}", beta_g2);
        let mut c_g1_bigints = Vec::new();
        for i in 0..2{
            let mut bytes: Vec<u8> = Vec::new();
            for i in  v["pi_c"][i].as_str().unwrap().split(',') {
                bytes.push((*i).parse::<u8>().unwrap());
            }
            c_g1_bigints.push(<Fp256::<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
        }
        let c_g1 =  ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
            c_g1_bigints[0],
            c_g1_bigints[1],
            false
        );

        //println!("{:?}", delta_g2);


        Ok(ark_groth16::data_structures::Proof::<ark_ec::models::bn::Bn<ark_bn254::Parameters>> {
        a: a_g1,
        b: b_g2,
        c: c_g1
        })
    }

    fn get_public_inputs_from_bytes_254() -> Result<Vec<Fp256::<ark_ed_on_bn254::FqParameters>>> {
        let contents = fs::read_to_string("./tests/public_inputs_254_bytes.txt")
            .expect("Something went wrong reading the file");
        let v: Value = serde_json::from_str(&contents)?;
        //println!("{}",  v);

        //println!("With text:\n{:?}", v["vk_alpha_1"][1]);
        //let mut public_inputs = Vec::new();

        //println!("{}", v[0]);
        let mut res = Vec::new();
        for i in 0..7{
            let mut bytes: Vec<u8> = Vec::new();
            for i in  v[i].as_str().unwrap().split(',') {
                bytes.push((*i).parse::<u8>().unwrap());
            }
            res.push(<Fp256::<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap())
        }

        Ok(res)
    }

    #[test]
    fn hardcoded_verifyingkey_test() {
        let pvk_unprepped = get_pvk_from_bytes_254().unwrap();
        let pvk = prepare_verifying_key(&pvk_unprepped);
        assert_eq!(get_gamma_abc_g1_0(), pvk.vk.gamma_abc_g1[0]);
        assert_eq!(get_gamma_abc_g1_1(), pvk.vk.gamma_abc_g1[1]);
        assert_eq!(get_gamma_abc_g1_2(), pvk.vk.gamma_abc_g1[2]);
        assert_eq!(get_gamma_abc_g1_3(), pvk.vk.gamma_abc_g1[3]);
        assert_eq!(get_gamma_abc_g1_4(), pvk.vk.gamma_abc_g1[4]);
        assert_eq!(get_gamma_g2_neg_pc_0() , pvk.gamma_g2_neg_pc.ell_coeffs[0]);
        assert_eq!(get_gamma_g2_neg_pc_1() , pvk.gamma_g2_neg_pc.ell_coeffs[1]);
        assert_eq!(get_gamma_g2_neg_pc_2() , pvk.gamma_g2_neg_pc.ell_coeffs[2]);
        assert_eq!(get_gamma_g2_neg_pc_3() , pvk.gamma_g2_neg_pc.ell_coeffs[3]);
        assert_eq!(get_gamma_g2_neg_pc_4() , pvk.gamma_g2_neg_pc.ell_coeffs[4]);
        assert_eq!(get_gamma_g2_neg_pc_5() , pvk.gamma_g2_neg_pc.ell_coeffs[5]);
        assert_eq!(get_gamma_g2_neg_pc_6() , pvk.gamma_g2_neg_pc.ell_coeffs[6]);
        assert_eq!(get_gamma_g2_neg_pc_7() , pvk.gamma_g2_neg_pc.ell_coeffs[7]);
        assert_eq!(get_gamma_g2_neg_pc_8() , pvk.gamma_g2_neg_pc.ell_coeffs[8]);
        assert_eq!(get_gamma_g2_neg_pc_9() , pvk.gamma_g2_neg_pc.ell_coeffs[9]);
        assert_eq!(get_gamma_g2_neg_pc_10() , pvk.gamma_g2_neg_pc.ell_coeffs[10]);
        assert_eq!(get_gamma_g2_neg_pc_11() , pvk.gamma_g2_neg_pc.ell_coeffs[11]);
        assert_eq!(get_gamma_g2_neg_pc_12() , pvk.gamma_g2_neg_pc.ell_coeffs[12]);
        assert_eq!(get_gamma_g2_neg_pc_13() , pvk.gamma_g2_neg_pc.ell_coeffs[13]);
        assert_eq!(get_gamma_g2_neg_pc_14() , pvk.gamma_g2_neg_pc.ell_coeffs[14]);
        assert_eq!(get_gamma_g2_neg_pc_15() , pvk.gamma_g2_neg_pc.ell_coeffs[15]);
        assert_eq!(get_gamma_g2_neg_pc_16() , pvk.gamma_g2_neg_pc.ell_coeffs[16]);
        assert_eq!(get_gamma_g2_neg_pc_17() , pvk.gamma_g2_neg_pc.ell_coeffs[17]);
        assert_eq!(get_gamma_g2_neg_pc_18() , pvk.gamma_g2_neg_pc.ell_coeffs[18]);
        assert_eq!(get_gamma_g2_neg_pc_19() , pvk.gamma_g2_neg_pc.ell_coeffs[19]);
        assert_eq!(get_gamma_g2_neg_pc_20() , pvk.gamma_g2_neg_pc.ell_coeffs[20]);
        assert_eq!(get_gamma_g2_neg_pc_21() , pvk.gamma_g2_neg_pc.ell_coeffs[21]);
        assert_eq!(get_gamma_g2_neg_pc_22() , pvk.gamma_g2_neg_pc.ell_coeffs[22]);
        assert_eq!(get_gamma_g2_neg_pc_23() , pvk.gamma_g2_neg_pc.ell_coeffs[23]);
        assert_eq!(get_gamma_g2_neg_pc_24() , pvk.gamma_g2_neg_pc.ell_coeffs[24]);
        assert_eq!(get_gamma_g2_neg_pc_25() , pvk.gamma_g2_neg_pc.ell_coeffs[25]);
        assert_eq!(get_gamma_g2_neg_pc_26() , pvk.gamma_g2_neg_pc.ell_coeffs[26]);
        assert_eq!(get_gamma_g2_neg_pc_27() , pvk.gamma_g2_neg_pc.ell_coeffs[27]);
        assert_eq!(get_gamma_g2_neg_pc_28() , pvk.gamma_g2_neg_pc.ell_coeffs[28]);
        assert_eq!(get_gamma_g2_neg_pc_29() , pvk.gamma_g2_neg_pc.ell_coeffs[29]);
        assert_eq!(get_gamma_g2_neg_pc_30() , pvk.gamma_g2_neg_pc.ell_coeffs[30]);
        assert_eq!(get_gamma_g2_neg_pc_31() , pvk.gamma_g2_neg_pc.ell_coeffs[31]);
        assert_eq!(get_gamma_g2_neg_pc_32() , pvk.gamma_g2_neg_pc.ell_coeffs[32]);
        assert_eq!(get_gamma_g2_neg_pc_33() , pvk.gamma_g2_neg_pc.ell_coeffs[33]);
        assert_eq!(get_gamma_g2_neg_pc_34() , pvk.gamma_g2_neg_pc.ell_coeffs[34]);
        assert_eq!(get_gamma_g2_neg_pc_35() , pvk.gamma_g2_neg_pc.ell_coeffs[35]);
        assert_eq!(get_gamma_g2_neg_pc_36() , pvk.gamma_g2_neg_pc.ell_coeffs[36]);
        assert_eq!(get_gamma_g2_neg_pc_37() , pvk.gamma_g2_neg_pc.ell_coeffs[37]);
        assert_eq!(get_gamma_g2_neg_pc_38() , pvk.gamma_g2_neg_pc.ell_coeffs[38]);
        assert_eq!(get_gamma_g2_neg_pc_39() , pvk.gamma_g2_neg_pc.ell_coeffs[39]);
        assert_eq!(get_gamma_g2_neg_pc_40() , pvk.gamma_g2_neg_pc.ell_coeffs[40]);
        assert_eq!(get_gamma_g2_neg_pc_41() , pvk.gamma_g2_neg_pc.ell_coeffs[41]);
        assert_eq!(get_gamma_g2_neg_pc_42() , pvk.gamma_g2_neg_pc.ell_coeffs[42]);
        assert_eq!(get_gamma_g2_neg_pc_43() , pvk.gamma_g2_neg_pc.ell_coeffs[43]);
        assert_eq!(get_gamma_g2_neg_pc_44() , pvk.gamma_g2_neg_pc.ell_coeffs[44]);
        assert_eq!(get_gamma_g2_neg_pc_45() , pvk.gamma_g2_neg_pc.ell_coeffs[45]);
        assert_eq!(get_gamma_g2_neg_pc_46() , pvk.gamma_g2_neg_pc.ell_coeffs[46]);
        assert_eq!(get_gamma_g2_neg_pc_47() , pvk.gamma_g2_neg_pc.ell_coeffs[47]);
        assert_eq!(get_gamma_g2_neg_pc_48() , pvk.gamma_g2_neg_pc.ell_coeffs[48]);
        assert_eq!(get_gamma_g2_neg_pc_49() , pvk.gamma_g2_neg_pc.ell_coeffs[49]);
        assert_eq!(get_gamma_g2_neg_pc_50() , pvk.gamma_g2_neg_pc.ell_coeffs[50]);
        assert_eq!(get_gamma_g2_neg_pc_51() , pvk.gamma_g2_neg_pc.ell_coeffs[51]);
        assert_eq!(get_gamma_g2_neg_pc_52() , pvk.gamma_g2_neg_pc.ell_coeffs[52]);
        assert_eq!(get_gamma_g2_neg_pc_53() , pvk.gamma_g2_neg_pc.ell_coeffs[53]);
        assert_eq!(get_gamma_g2_neg_pc_54() , pvk.gamma_g2_neg_pc.ell_coeffs[54]);
        assert_eq!(get_gamma_g2_neg_pc_55() , pvk.gamma_g2_neg_pc.ell_coeffs[55]);
        assert_eq!(get_gamma_g2_neg_pc_56() , pvk.gamma_g2_neg_pc.ell_coeffs[56]);
        assert_eq!(get_gamma_g2_neg_pc_57() , pvk.gamma_g2_neg_pc.ell_coeffs[57]);
        assert_eq!(get_gamma_g2_neg_pc_58() , pvk.gamma_g2_neg_pc.ell_coeffs[58]);
        assert_eq!(get_gamma_g2_neg_pc_59() , pvk.gamma_g2_neg_pc.ell_coeffs[59]);
        assert_eq!(get_gamma_g2_neg_pc_60() , pvk.gamma_g2_neg_pc.ell_coeffs[60]);
        assert_eq!(get_gamma_g2_neg_pc_61() , pvk.gamma_g2_neg_pc.ell_coeffs[61]);
        assert_eq!(get_gamma_g2_neg_pc_62() , pvk.gamma_g2_neg_pc.ell_coeffs[62]);
        assert_eq!(get_gamma_g2_neg_pc_63() , pvk.gamma_g2_neg_pc.ell_coeffs[63]);
        assert_eq!(get_gamma_g2_neg_pc_64() , pvk.gamma_g2_neg_pc.ell_coeffs[64]);
        assert_eq!(get_gamma_g2_neg_pc_65() , pvk.gamma_g2_neg_pc.ell_coeffs[65]);
        assert_eq!(get_gamma_g2_neg_pc_66() , pvk.gamma_g2_neg_pc.ell_coeffs[66]);
        assert_eq!(get_gamma_g2_neg_pc_67() , pvk.gamma_g2_neg_pc.ell_coeffs[67]);
        assert_eq!(get_delta_g2_neg_pc_0() , pvk.delta_g2_neg_pc.ell_coeffs[0]);
        assert_eq!(get_delta_g2_neg_pc_1() , pvk.delta_g2_neg_pc.ell_coeffs[1]);
        assert_eq!(get_delta_g2_neg_pc_2() , pvk.delta_g2_neg_pc.ell_coeffs[2]);
        assert_eq!(get_delta_g2_neg_pc_3() , pvk.delta_g2_neg_pc.ell_coeffs[3]);
        assert_eq!(get_delta_g2_neg_pc_4() , pvk.delta_g2_neg_pc.ell_coeffs[4]);
        assert_eq!(get_delta_g2_neg_pc_5() , pvk.delta_g2_neg_pc.ell_coeffs[5]);
        assert_eq!(get_delta_g2_neg_pc_6() , pvk.delta_g2_neg_pc.ell_coeffs[6]);
        assert_eq!(get_delta_g2_neg_pc_7() , pvk.delta_g2_neg_pc.ell_coeffs[7]);
        assert_eq!(get_delta_g2_neg_pc_8() , pvk.delta_g2_neg_pc.ell_coeffs[8]);
        assert_eq!(get_delta_g2_neg_pc_9() , pvk.delta_g2_neg_pc.ell_coeffs[9]);
        assert_eq!(get_delta_g2_neg_pc_10() , pvk.delta_g2_neg_pc.ell_coeffs[10]);
        assert_eq!(get_delta_g2_neg_pc_11() , pvk.delta_g2_neg_pc.ell_coeffs[11]);
        assert_eq!(get_delta_g2_neg_pc_12() , pvk.delta_g2_neg_pc.ell_coeffs[12]);
        assert_eq!(get_delta_g2_neg_pc_13() , pvk.delta_g2_neg_pc.ell_coeffs[13]);
        assert_eq!(get_delta_g2_neg_pc_14() , pvk.delta_g2_neg_pc.ell_coeffs[14]);
        assert_eq!(get_delta_g2_neg_pc_15() , pvk.delta_g2_neg_pc.ell_coeffs[15]);
        assert_eq!(get_delta_g2_neg_pc_16() , pvk.delta_g2_neg_pc.ell_coeffs[16]);
        assert_eq!(get_delta_g2_neg_pc_17() , pvk.delta_g2_neg_pc.ell_coeffs[17]);
        assert_eq!(get_delta_g2_neg_pc_18() , pvk.delta_g2_neg_pc.ell_coeffs[18]);
        assert_eq!(get_delta_g2_neg_pc_19() , pvk.delta_g2_neg_pc.ell_coeffs[19]);
        assert_eq!(get_delta_g2_neg_pc_20() , pvk.delta_g2_neg_pc.ell_coeffs[20]);
        assert_eq!(get_delta_g2_neg_pc_21() , pvk.delta_g2_neg_pc.ell_coeffs[21]);
        assert_eq!(get_delta_g2_neg_pc_22() , pvk.delta_g2_neg_pc.ell_coeffs[22]);
        assert_eq!(get_delta_g2_neg_pc_23() , pvk.delta_g2_neg_pc.ell_coeffs[23]);
        assert_eq!(get_delta_g2_neg_pc_24() , pvk.delta_g2_neg_pc.ell_coeffs[24]);
        assert_eq!(get_delta_g2_neg_pc_25() , pvk.delta_g2_neg_pc.ell_coeffs[25]);
        assert_eq!(get_delta_g2_neg_pc_26() , pvk.delta_g2_neg_pc.ell_coeffs[26]);
        assert_eq!(get_delta_g2_neg_pc_27() , pvk.delta_g2_neg_pc.ell_coeffs[27]);
        assert_eq!(get_delta_g2_neg_pc_28() , pvk.delta_g2_neg_pc.ell_coeffs[28]);
        assert_eq!(get_delta_g2_neg_pc_29() , pvk.delta_g2_neg_pc.ell_coeffs[29]);
        assert_eq!(get_delta_g2_neg_pc_30() , pvk.delta_g2_neg_pc.ell_coeffs[30]);
        assert_eq!(get_delta_g2_neg_pc_31() , pvk.delta_g2_neg_pc.ell_coeffs[31]);
        assert_eq!(get_delta_g2_neg_pc_32() , pvk.delta_g2_neg_pc.ell_coeffs[32]);
        assert_eq!(get_delta_g2_neg_pc_33() , pvk.delta_g2_neg_pc.ell_coeffs[33]);
        assert_eq!(get_delta_g2_neg_pc_34() , pvk.delta_g2_neg_pc.ell_coeffs[34]);
        assert_eq!(get_delta_g2_neg_pc_35() , pvk.delta_g2_neg_pc.ell_coeffs[35]);
        assert_eq!(get_delta_g2_neg_pc_36() , pvk.delta_g2_neg_pc.ell_coeffs[36]);
        assert_eq!(get_delta_g2_neg_pc_37() , pvk.delta_g2_neg_pc.ell_coeffs[37]);
        assert_eq!(get_delta_g2_neg_pc_38() , pvk.delta_g2_neg_pc.ell_coeffs[38]);
        assert_eq!(get_delta_g2_neg_pc_39() , pvk.delta_g2_neg_pc.ell_coeffs[39]);
        assert_eq!(get_delta_g2_neg_pc_40() , pvk.delta_g2_neg_pc.ell_coeffs[40]);
        assert_eq!(get_delta_g2_neg_pc_41() , pvk.delta_g2_neg_pc.ell_coeffs[41]);
        assert_eq!(get_delta_g2_neg_pc_42() , pvk.delta_g2_neg_pc.ell_coeffs[42]);
        assert_eq!(get_delta_g2_neg_pc_43() , pvk.delta_g2_neg_pc.ell_coeffs[43]);
        assert_eq!(get_delta_g2_neg_pc_44() , pvk.delta_g2_neg_pc.ell_coeffs[44]);
        assert_eq!(get_delta_g2_neg_pc_45() , pvk.delta_g2_neg_pc.ell_coeffs[45]);
        assert_eq!(get_delta_g2_neg_pc_46() , pvk.delta_g2_neg_pc.ell_coeffs[46]);
        assert_eq!(get_delta_g2_neg_pc_47() , pvk.delta_g2_neg_pc.ell_coeffs[47]);
        assert_eq!(get_delta_g2_neg_pc_48() , pvk.delta_g2_neg_pc.ell_coeffs[48]);
        assert_eq!(get_delta_g2_neg_pc_49() , pvk.delta_g2_neg_pc.ell_coeffs[49]);
        assert_eq!(get_delta_g2_neg_pc_50() , pvk.delta_g2_neg_pc.ell_coeffs[50]);
        assert_eq!(get_delta_g2_neg_pc_51() , pvk.delta_g2_neg_pc.ell_coeffs[51]);
        assert_eq!(get_delta_g2_neg_pc_52() , pvk.delta_g2_neg_pc.ell_coeffs[52]);
        assert_eq!(get_delta_g2_neg_pc_53() , pvk.delta_g2_neg_pc.ell_coeffs[53]);
        assert_eq!(get_delta_g2_neg_pc_54() , pvk.delta_g2_neg_pc.ell_coeffs[54]);
        assert_eq!(get_delta_g2_neg_pc_55() , pvk.delta_g2_neg_pc.ell_coeffs[55]);
        assert_eq!(get_delta_g2_neg_pc_56() , pvk.delta_g2_neg_pc.ell_coeffs[56]);
        assert_eq!(get_delta_g2_neg_pc_57() , pvk.delta_g2_neg_pc.ell_coeffs[57]);
        assert_eq!(get_delta_g2_neg_pc_58() , pvk.delta_g2_neg_pc.ell_coeffs[58]);
        assert_eq!(get_delta_g2_neg_pc_59() , pvk.delta_g2_neg_pc.ell_coeffs[59]);
        assert_eq!(get_delta_g2_neg_pc_60() , pvk.delta_g2_neg_pc.ell_coeffs[60]);
        assert_eq!(get_delta_g2_neg_pc_61() , pvk.delta_g2_neg_pc.ell_coeffs[61]);
        assert_eq!(get_delta_g2_neg_pc_62() , pvk.delta_g2_neg_pc.ell_coeffs[62]);
        assert_eq!(get_delta_g2_neg_pc_63() , pvk.delta_g2_neg_pc.ell_coeffs[63]);
        assert_eq!(get_delta_g2_neg_pc_64() , pvk.delta_g2_neg_pc.ell_coeffs[64]);
        assert_eq!(get_delta_g2_neg_pc_65() , pvk.delta_g2_neg_pc.ell_coeffs[65]);
        assert_eq!(get_delta_g2_neg_pc_66() , pvk.delta_g2_neg_pc.ell_coeffs[66]);
        assert_eq!(get_delta_g2_neg_pc_67() , pvk.delta_g2_neg_pc.ell_coeffs[67]);

    }

    #[test]
    fn verification_test() -> Result<()>{

        let pvk_unprepped = get_pvk_from_bytes_254()?;
        let pvk = prepare_verifying_key(&pvk_unprepped);
        let proof = get_proof_from_bytes_254()?;

        let public_inputs = get_public_inputs_from_bytes_254()?;


        let res = verify_proof(
            &pvk,
            &proof,
            &public_inputs[..]
        );

        //println!("{:?}", pvk);
        Ok(())
    }

    use ark_ec::ProjectiveCurve;
	use Testing_Hardcoded_Params_devnet_new::parsers_part_2_254::*;
	use Testing_Hardcoded_Params_devnet_new::state_final_exp::FinalExpBytes;
	use Testing_Hardcoded_Params_devnet_new::ranges_part_2::*;

	#[test]
    fn final_exp_offchain() -> Result<()> {

        let pvk_unprepped = get_pvk_from_bytes_254()?;
        let pvk = prepare_verifying_key(&pvk_unprepped);
        let proof = get_proof_from_bytes_254()?;

        let public_inputs = get_public_inputs_from_bytes_254()?;

        let prepared_inputs = prepare_inputs(&pvk, &public_inputs).unwrap();

        let miller_output = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::miller_loop(
            [
                (proof.a.into(), proof.b.into()),
                (
                    (prepared_inputs).into_affine().into(),
                    pvk.gamma_g2_neg_pc.clone(),
                ),
                (proof.c.into(), pvk.delta_g2_neg_pc.clone()),
            ]
            .iter(),

        );
        //starting result
        let mut f = miller_output;

        //library result for reference
        let res_origin = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::final_exponentiation(&f).unwrap();
		let res_custom = final_exponentiation_custom(&f).unwrap();
		assert_eq!(res_origin,res_custom);
		let res_processor = final_exponentiation_test_processor(&f).unwrap();
		assert_eq!(res_origin,res_processor);


        // println!("{:?}", res_origin);
        // println!("{:?}", pvk.alpha_g1_beta_g2);
        assert_eq!(res_origin, pvk.alpha_g1_beta_g2);
        Ok(())
    }

    #[allow(clippy::let_and_return)]
    fn final_exponentiation_custom(f: &<ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk) -> Option<<ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk> {
		//adapted from ark_ec bn254
		//executes instructions_final_exponentiation alongside reference implementation and
		//asserts after every step

		//from original repo:
	        // Easy part: result = elt^((q^6-1)*(q^2+1)).
	        // Follows, e.g., Beuchat et al page 9, by computing result as follows:
	        //   elt^((q^6-1)*(q^2+1)) = (conj(elt) * elt^(-1))^(q^2+1)
		//let mut instruction_order = Vec::new();
		/*
		* ------------------------- init -------------------------
		*instruction 0
		*/
		let mut account_struct = FinalExpBytes::new();
		let mut account_struct1 = FinalExpBytes::new();


        // f1 = r.conjugate() = f^(p^6)
        let mut f1 = *f;
		parse_f_to_bytes_new(*f, &mut account_struct.f1_r_range_s);
		parse_f_to_bytes_new(*f, &mut account_struct1.f1_r_range_s);

		assert_eq!(f1, parse_f_from_bytes_new(&account_struct.f1_r_range_s), "0 failed");
		assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,0);

		let reference_f = f.clone();
		account_struct.f_f2_range_s = account_struct.f1_r_range_s.clone();
		/*
		* ------------------------- conjugate -------------------------
		*instruction 0
		*/
        f1.conjugate();
		conjugate_wrapper(&mut account_struct.f1_r_range_s);
		assert_eq!(f1, parse_f_from_bytes_new(&account_struct.f1_r_range_s), "1 failed");

		/*
		*
		* ------------------------- Inverse -------------------------
		* instruction 1
		*/
		assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,1);
		custom_f_inverse_1(&account_struct.f_f2_range_s, &mut account_struct.cubic_range_1_s);

		//instruction 2 ---------------------------------------------
		assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,2);

		custom_f_inverse_2(&account_struct.f_f2_range_s,&mut account_struct.cubic_range_0_s, &account_struct.cubic_range_1_s);

		//instruction 3 ---------------------------------------------
		assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,3);

		custom_cubic_inverse_1(
			&account_struct.cubic_range_0_s,
			&mut account_struct.quad_range_0_s,
			&mut account_struct.quad_range_1_s,
			&mut account_struct.quad_range_2_s,
			&mut account_struct.quad_range_3_s
		);

		//instruction 4 ---------------------------------------------
		assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,4);

		//quad inverse is part of cubic Inverse
		custom_quadratic_fp256_inverse_1(
			&account_struct.quad_range_3_s,
			&mut account_struct.fp384_range_s
		);

		//instruction 5 ---------------------------------------------
		assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,5);

		custom_quadratic_fp256_inverse_2(
			&mut account_struct.quad_range_3_s,
			& account_struct.fp384_range_s,
		);

		//instruction 6 ---------------------------------------------
		assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,6);

		custom_cubic_inverse_2(
		&mut account_struct.cubic_range_0_s,
		& account_struct.quad_range_0_s,
		& account_struct.quad_range_1_s,
		& account_struct.quad_range_2_s,
		& account_struct.quad_range_3_s
		);


		//instruction 7 ---------------------------------------------
		assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,7);

		custom_f_inverse_3(
			&mut account_struct.cubic_range_1_s,
			&account_struct.cubic_range_0_s,
			&account_struct.f_f2_range_s,
		);

		//instruction 8 ---------------------------------------------
		assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,8);

		custom_f_inverse_4(
			&mut account_struct.cubic_range_0_s,
			&account_struct.f_f2_range_s
		);

		//instruction 9 ---------------------------------------------
		assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,9);

		custom_f_inverse_5(
			&account_struct.cubic_range_0_s,
			&account_struct.cubic_range_1_s,
			&mut account_struct.f_f2_range_s,
		);

		assert_eq!(reference_f.inverse().unwrap() , parse_f_from_bytes_new(&account_struct.f_f2_range_s), "f inverse failed");
		assert_eq!(f1, parse_f_from_bytes_new(&account_struct.f1_r_range_s));

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

			assert_eq!(f2, parse_f_from_bytes_new(&account_struct.f_f2_range_s));
			assert_eq!(f1, parse_f_from_bytes_new(&account_struct.f1_r_range_s));
			//instruction 10 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,10);

			mul_assign_1(
				&account_struct.f1_r_range_s,  f_cubic_0_range,
				&account_struct.f_f2_range_s,  f_cubic_0_range,
				&mut account_struct.cubic_range_0_s,  solo_cubic_0_range
			);

			//instruction 11 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,11);

			mul_assign_2(
				&account_struct.f1_r_range_s,  f_cubic_1_range,
				&account_struct.f_f2_range_s,  f_cubic_1_range,
				&mut account_struct.cubic_range_1_s,  solo_cubic_0_range
			);

			//instruction 12 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,12);

			mul_assign_3(
				&mut account_struct.f1_r_range_s
			);

			//instruction 13 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,13);

			mul_assign_4_1(
				&account_struct.f_f2_range_s,
				&mut account_struct.cubic_range_2_s,
			);

			//instruction 14 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,14);

			mul_assign_4_2(
				&mut account_struct.f1_r_range_s,
				 f_cubic_1_range,
				&account_struct.cubic_range_2_s,
			);

			//instruction 15 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,15);

			mul_assign_5(
				&mut account_struct.f1_r_range_s,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s
			);

			assert_eq!(r,  parse_f_from_bytes_new(&account_struct.f1_r_range_s), "f mulassign failed");

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
			//instruction 16 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,16);

			account_struct.f_f2_range_s = account_struct.f1_r_range_s.clone();
			assert_eq!(f2,  parse_f_from_bytes_new(&account_struct.f_f2_range_s));

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

			//instruction 17 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,17);

			custom_frobenius_map_2_1(&mut account_struct.f1_r_range_s);
			//instruction 18 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,18);

			custom_frobenius_map_2_2(&mut account_struct.f1_r_range_s);

			assert_eq!(r,  parse_f_from_bytes_new(&account_struct.f1_r_range_s));

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
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,10);

			mul_assign_1(
				&account_struct.f1_r_range_s,  f_cubic_0_range,
				&account_struct.f_f2_range_s,  f_cubic_0_range,
				&mut account_struct.cubic_range_0_s,  solo_cubic_0_range
			);

			//instruction 11 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,11);

			mul_assign_2(
				&account_struct.f1_r_range_s,  f_cubic_1_range,
				&account_struct.f_f2_range_s,  f_cubic_1_range,
				&mut account_struct.cubic_range_1_s,  solo_cubic_0_range
			);

			//instruction 12 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,12);

			mul_assign_3(
				&mut account_struct.f1_r_range_s
			);

			//instruction 13 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,13);

			mul_assign_4_1(
				&account_struct.f_f2_range_s,
				&mut account_struct.cubic_range_2_s,
			);

			//instruction 14 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,14);

			mul_assign_4_2(
				&mut account_struct.f1_r_range_s,
				 f_cubic_1_range,
				&account_struct.cubic_range_2_s,
			);

			//instruction 15 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,15);

			mul_assign_5(
				&mut account_struct.f1_r_range_s,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s
			);

			assert_eq!(r,  parse_f_from_bytes_new(&account_struct.f1_r_range_s), "f mulassign failed");

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
			//instruction 19 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,19);

			account_struct.i_range_s = account_struct.f1_r_range_s.clone();
			conjugate_wrapper(&mut account_struct.i_range_s);
			account_struct.y0_range_s = account_struct.f1_r_range_s.clone();

			for i in 1..63 {
				if i == 1 {
					assert_eq!(account_struct.y0_range_s, account_struct.f1_r_range_s);
				}

				//cyclotomic_exp
				//instruction 20 ---------------------------------------------
				assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,20);

				custom_cyclotomic_square_in_place(&mut account_struct.y0_range_s);


				if naf_vec[i] != 0 {
					if naf_vec[i] > 0 {
						//println!("if i {}", i);
						//instruction 21 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,21);

						mul_assign_1(
							&account_struct.y0_range_s, f_cubic_0_range,
							&account_struct.f1_r_range_s, f_cubic_0_range,
							&mut account_struct.cubic_range_0_s, solo_cubic_0_range
						);

						//instruction 22 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,22);

						mul_assign_2(
							&account_struct.y0_range_s, f_cubic_1_range,
							&account_struct.f1_r_range_s, f_cubic_1_range,
							&mut account_struct.cubic_range_1_s, solo_cubic_0_range
						);

						//instruction 23 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,23);

						mul_assign_3(
							&mut account_struct.y0_range_s
						);


						//instruction 24 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,24);

						mul_assign_4_1(
							&account_struct.f1_r_range_s,
							&mut account_struct.cubic_range_2_s,
						);

						//instruction 25 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,25);

						mul_assign_4_2(
							&mut account_struct.y0_range_s,
							f_cubic_1_range,
							&account_struct.cubic_range_2_s,
						);

						//instruction 26 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,26);

						mul_assign_5(
							&mut account_struct.y0_range_s,
							&account_struct.cubic_range_0_s,
							&account_struct.cubic_range_1_s
						);

					} else {

						//instruction 27 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,27);

						mul_assign_1(
							&account_struct.y0_range_s, f_cubic_0_range,
							&account_struct.i_range_s, f_cubic_0_range,
							&mut account_struct.cubic_range_0_s, solo_cubic_0_range
						);

						//instruction 28 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,28);

						mul_assign_2(
							&account_struct.y0_range_s, f_cubic_1_range,
							&account_struct.i_range_s, f_cubic_1_range,
							&mut account_struct.cubic_range_1_s, solo_cubic_0_range
						);

						//instruction 29 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,29);

						mul_assign_3(
							&mut account_struct.y0_range_s
						);

						//instruction 30 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,30);

						mul_assign_4_1(
							&account_struct.i_range_s,
							&mut account_struct.cubic_range_2_s,
						);

						//instruction 31 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,31);

						mul_assign_4_2(
							&mut account_struct.y0_range_s,
							f_cubic_1_range,
							&account_struct.cubic_range_2_s,
						);

						//instruction 32 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,32);

						mul_assign_5(
							&mut account_struct.y0_range_s,
							&account_struct.cubic_range_0_s,
							&account_struct.cubic_range_1_s
						);
					}
				}
			}

			//instruction 33 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,33);

			conjugate_wrapper(&mut account_struct.y0_range_s);
			assert_eq!(y0,  parse_f_from_bytes_new(&account_struct.y0_range_s), "exp_by_neg_x(r) ");

			/*
			*
			*
			* ------------------------- y0.cyclotomic_square() -------------------------
			* still instruction 33
			*
			*/

            let y1 = y0.cyclotomic_square();
			custom_cyclotomic_square(&account_struct.y0_range_s, &mut account_struct.y1_range_s);
			assert_eq!(y1,  parse_f_from_bytes_new(&account_struct.y1_range_s), "exp_by_neg_x(r) ");
			//y0 last used

			/*
			*
			*
			* ------------------------- y0.cyclotomic_square() -------------------------
			*
			*
			*/

            let y2 = y1.cyclotomic_square();
			//y2 is stored in y0_range_s
			//instruction 34 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,34);

			custom_cyclotomic_square(&account_struct.y1_range_s , &mut account_struct.y0_range_s);
			assert_eq!(y2,  parse_f_from_bytes_new(&account_struct.y0_range_s), "exp_by_neg_x(r) ");

			/*
			*
			*
			* ------------------------- mulassign -------------------------
			*
			*
			*/

            let mut y3 = y2 * &y1;
			//y3 is stored in y0_range_s

			//instruction 35 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,35);

			mul_assign_1(
				&account_struct.y0_range_s,  f_cubic_0_range,
				&account_struct.y1_range_s,  f_cubic_0_range,
				&mut account_struct.cubic_range_0_s,  solo_cubic_0_range
			);

			//instruction 36 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,36);

			mul_assign_2(
				&account_struct.y0_range_s,  f_cubic_1_range,
				&account_struct.y1_range_s,  f_cubic_1_range,
				&mut account_struct.cubic_range_1_s,  solo_cubic_0_range
			);

			//instruction 37 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,37);

			mul_assign_3(
				&mut account_struct.y0_range_s
			);

			//instruction 38 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,38);
			mul_assign_4_1(
				&account_struct.y1_range_s,
				&mut account_struct.cubic_range_2_s,
			);

			//instruction 39 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,39);
			mul_assign_4_2(
				&mut account_struct.y0_range_s,
				 f_cubic_1_range,
				&account_struct.cubic_range_2_s,
			);

			//instruction 40 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,40);
			mul_assign_5(
				&mut account_struct.y0_range_s,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s
			);

			assert_eq!(y3,  parse_f_from_bytes_new(&account_struct.y0_range_s), "mulassign ");

			/*
			*
			*
			* ------------------------- y4 = exp_by_neg_x(y3) -------------------------
			*
			*
			*/

            let y4 = exp_by_neg_x(y3);
			//y4 is stored in y2_range_s


			//init
			//instruction 41 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,41);
			account_struct.i_range_s = account_struct.y0_range_s.clone();
			conjugate_wrapper(&mut account_struct.i_range_s);
			account_struct.y2_range_s = account_struct.y0_range_s.clone();

			for i in 1..63 {

				//cyclotomic_exp
				//instruction 42 ---------------------------------------------
				assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,42);
				custom_cyclotomic_square_in_place(&mut account_struct.y2_range_s);


				if naf_vec[i] != 0 {
					if naf_vec[i] > 0 {

						//instruction 43 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,43);
						mul_assign_1(
							&account_struct.y2_range_s, f_cubic_0_range,
							&account_struct.y0_range_s, f_cubic_0_range,
							&mut account_struct.cubic_range_0_s, solo_cubic_0_range
						);

						//instruction 44 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,44);
						mul_assign_2(
							&account_struct.y2_range_s, f_cubic_1_range,
							&account_struct.y0_range_s, f_cubic_1_range,
							&mut account_struct.cubic_range_1_s, solo_cubic_0_range
						);

						//instruction 45 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,45);
						mul_assign_3(
							&mut account_struct.y2_range_s
						);

						//instruction 46 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,46);
						mul_assign_4_1(
							&account_struct.y0_range_s,
							&mut account_struct.cubic_range_2_s,
						);

						//instruction 47 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,47);
						mul_assign_4_2(
							&mut account_struct.y2_range_s,
							f_cubic_1_range,
							&account_struct.cubic_range_2_s,
						);

						//instruction 48 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,48);
						mul_assign_5(
							&mut account_struct.y2_range_s,
							&account_struct.cubic_range_0_s,
							&account_struct.cubic_range_1_s
						);

					} else {

						//instruction 49 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,49);
						mul_assign_1(
							&account_struct.y2_range_s, f_cubic_0_range,
							&account_struct.i_range_s, f_cubic_0_range,
							&mut account_struct.cubic_range_0_s, solo_cubic_0_range
						);

						//instruction 50 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,50);
						mul_assign_2(
							&account_struct.y2_range_s, f_cubic_1_range,
							&account_struct.i_range_s, f_cubic_1_range,
							&mut account_struct.cubic_range_1_s, solo_cubic_0_range
						);

						//instruction 51 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,51);
						mul_assign_3(
							&mut account_struct.y2_range_s
						);

						//instruction 52 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,52);
						mul_assign_4_1(
							&account_struct.i_range_s,
							&mut account_struct.cubic_range_2_s,
						);

						//instruction 53 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,53);
						mul_assign_4_2(
							&mut account_struct.y2_range_s,
							f_cubic_1_range,
							&account_struct.cubic_range_2_s,
						);

						//instruction 54 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,54);
						mul_assign_5(
							&mut account_struct.y2_range_s,
							&account_struct.cubic_range_0_s,
							&account_struct.cubic_range_1_s
						);
					}
				}
			}

			//instruction 55 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,55);
			conjugate_wrapper(&mut account_struct.y2_range_s);

			assert_eq!(y4,  parse_f_from_bytes_new(&account_struct.y2_range_s), "exp_by_neg_x(r) ");


			/*
			*
			*
			* ------------------------- y4.cyclotomic_square() -------------------------
			*
			* r_range: 			r
			* f_f2_range_s: 	f2 not used anymore
			* y0_range:  		y3
			* y1_range:			y1
			* y2_range:			y4
			* still instruction 55
			*/


            let y5 = y4.cyclotomic_square();
			//y5 is stored in f_f2_range_s
			custom_cyclotomic_square(&account_struct.y2_range_s, &mut account_struct.f_f2_range_s);
			assert_eq!(y5,  parse_f_from_bytes_new(&account_struct.f_f2_range_s), "cyclotomic_square ");

			/*
			*
			*
			* ------------------------- y4 = exp_by_neg_x(y3) -------------------------
			*
			* r_range: 			r
			* f_f2_range_s: 	y5 			//y5 last used here
			* y0_range:  		y3
			* y1_range:			y1
			* y2_range:			y4
			* y6_range:			free
			*/

            let mut y6 = exp_by_neg_x(y5);
			//y4 is stored in y6_range


			//init
			//instruction 56 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,56);
			account_struct.i_range_s = account_struct.f_f2_range_s.clone();

			conjugate_wrapper(&mut account_struct.i_range_s);

			account_struct.y6_range = account_struct.f_f2_range_s.clone();

			for i in 1..63 {
				//cyclotomic_exp
				//instruction 57 ---------------------------------------------
				assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,57);
				custom_cyclotomic_square_in_place(&mut account_struct.y6_range);


				if naf_vec[i] != 0 {
					if naf_vec[i] > 0 {

						//instruction 58 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,58);
						mul_assign_1(
							&account_struct.y6_range, f_cubic_0_range,
							&account_struct.f_f2_range_s, f_cubic_0_range,
							&mut account_struct.cubic_range_0_s, solo_cubic_0_range
						);

						//instruction 59 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,59);
						mul_assign_2(
							&account_struct.y6_range, f_cubic_1_range,
							&account_struct.f_f2_range_s, f_cubic_1_range,
							&mut account_struct.cubic_range_1_s, solo_cubic_0_range
						);

						//instruction 60 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,60);
						mul_assign_3(
							&mut account_struct.y6_range
						);


						//instruction 61 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,61);
						mul_assign_4_1(
							&account_struct.f_f2_range_s,
							&mut account_struct.cubic_range_2_s,
						);

						//instruction 62 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,62);
						mul_assign_4_2(
							&mut account_struct.y6_range,
							f_cubic_1_range,
							&account_struct.cubic_range_2_s,
						);

						//instruction 63 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,63);
						mul_assign_5(
							&mut account_struct.y6_range,
							&account_struct.cubic_range_0_s,
							&account_struct.cubic_range_1_s
						);

					} else {

						//instruction 64 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,64);
						mul_assign_1(
							&account_struct.y6_range, f_cubic_0_range,
							&account_struct.i_range_s, f_cubic_0_range,
							&mut account_struct.cubic_range_0_s, solo_cubic_0_range
						);

						//instruction 65 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,65);
						mul_assign_2(
							&account_struct.y6_range, f_cubic_1_range,
							&account_struct.i_range_s, f_cubic_1_range,
							&mut account_struct.cubic_range_1_s, solo_cubic_0_range
						);

						//instruction 66 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,66);
						mul_assign_3(
							&mut account_struct.y6_range
						);

						//instruction 67 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,67);
						mul_assign_4_1(
							&account_struct.i_range_s,
							&mut account_struct.cubic_range_2_s,
						);

						//instruction 68 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,68);
						mul_assign_4_2(
							&mut account_struct.y6_range,
							f_cubic_1_range,
							&account_struct.cubic_range_2_s,
						);

						//instruction 69 ---------------------------------------------
						assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,69);
						mul_assign_5(
							&mut account_struct.y6_range,
							&account_struct.cubic_range_0_s,
							&account_struct.cubic_range_1_s
						);
					}
				}
			}
			//instruction 70 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,70);

			conjugate_wrapper(&mut account_struct.y6_range);

			assert_eq!(y6,  parse_f_from_bytes_new(&account_struct.y6_range), "exp_by_neg_x(r) ");

			/*
			*
			*
			* ------------------------- conjugate_wrapper -------------------------
			*
			* r_range: 			r
			* f_f2_range_s: 	free
			* y0_range:  		y3
			* y1_range:			y1
			* y2_range:			y4
			* y6_range:			y6
			* instruction 70
			*/

            y3.conjugate();
			conjugate_wrapper(&mut account_struct.y0_range_s);

            y6.conjugate();

			conjugate_wrapper(&mut account_struct.y6_range);

			/*
			*
			*
			* ------------------------- mul_assign -------------------------
			*
			* r_range: 			r
			* f_f2_range_s: 	free
			* y0_range:  		y3
			* y1_range:			y1
			* y2_range:			y4
			* y6_range:			y6 last used
			*/

            let y7 = y6 * &y4;
			// stored in y6_range

			//instruction 71 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,71);
			mul_assign_1(
				&account_struct.y6_range,  f_cubic_0_range,
				&account_struct.y2_range_s,  f_cubic_0_range,
				&mut account_struct.cubic_range_0_s,  solo_cubic_0_range
			);

 			//instruction 72 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,72);
			mul_assign_2(
				&account_struct.y6_range,  f_cubic_1_range,
				&account_struct.y2_range_s,  f_cubic_1_range,
				&mut account_struct.cubic_range_1_s,  solo_cubic_0_range
			);

			//instruction 73 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,73);
			mul_assign_3(
				&mut account_struct.y6_range
			);

			//instruction 74 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,74);
			mul_assign_4_1(
				&account_struct.y2_range_s,
				&mut account_struct.cubic_range_2_s,
			);

			//instruction 75 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,75);
			mul_assign_4_2(
				&mut account_struct.y6_range,
				 f_cubic_1_range,
				&account_struct.cubic_range_2_s,
			);

			//instruction 76 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,76);
			mul_assign_5(
				&mut account_struct.y6_range,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s
			);

			assert_eq!(y7,  parse_f_from_bytes_new(&account_struct.y6_range), "mulassign ");

			/*
			*
			*
			* ------------------------- mul_assign -------------------------
			*
			* r_range: 			r
			* f_f2_range_s: 	free
			* y0_range:  		y3 last used
			* y1_range:			y1
			* y2_range:			y4
			* y6_range:			y7 last used
			*/
            let mut y8 = y7 * &y3;
			// stored in y6_range

			//instruction 77 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,77);
			mul_assign_1(
				&account_struct.y6_range,  f_cubic_0_range,
				&account_struct.y0_range_s,  f_cubic_0_range,
				&mut account_struct.cubic_range_0_s,  solo_cubic_0_range
			);

			//instruction 78 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,78);
			mul_assign_2(
				&account_struct.y6_range,  f_cubic_1_range,
				&account_struct.y0_range_s,  f_cubic_1_range,
				&mut account_struct.cubic_range_1_s,  solo_cubic_0_range
			);

			//instruction 79 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,79);
			mul_assign_3(
				&mut account_struct.y6_range
			);

			//instruction 80 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,80);
			mul_assign_4_1(
				&account_struct.y0_range_s,
				&mut account_struct.cubic_range_2_s,
			);

			//instruction 81 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,81);
			mul_assign_4_2(
				&mut account_struct.y6_range,
				 f_cubic_1_range,
				&account_struct.cubic_range_2_s,
			);

			//instruction 82 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,82);
			mul_assign_5(
				&mut account_struct.y6_range,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s
			);

			assert_eq!(y8,  parse_f_from_bytes_new(&account_struct.y6_range), "mulassign ");

			/*
			*
			*
			* ------------------------- mul_assign -------------------------
			*
			* r_range: 			r
			* f_f2_range_s: 	free
			* y0_range:  		free
			* y1_range:			y1		last used
			* y2_range:			y4
			* y6_range:			y8
			*/
            let y9 = y8 * &y1;
			// stored in y1_range

			//instruction 83 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,83);
			mul_assign_1(
				&account_struct.y1_range_s,  f_cubic_0_range,
				&account_struct.y6_range,  f_cubic_0_range,
				&mut account_struct.cubic_range_0_s,  solo_cubic_0_range
			);

			//instruction 84 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,84);
			mul_assign_2(
				&account_struct.y1_range_s,  f_cubic_1_range,
				&account_struct.y6_range,  f_cubic_1_range,
				&mut account_struct.cubic_range_1_s,  solo_cubic_0_range
			);

			//instruction 85 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,85);
			mul_assign_3(
				&mut account_struct.y1_range_s
			);

			//instruction 86 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,86);
			mul_assign_4_1(
				&account_struct.y6_range,
				&mut account_struct.cubic_range_2_s,
			);

			//instruction 87 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,87);
			mul_assign_4_2(
				&mut account_struct.y1_range_s,
				 f_cubic_1_range,
				&account_struct.cubic_range_2_s,
			);

			//instruction 88 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,88);
			mul_assign_5(
				&mut account_struct.y1_range_s,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s
			);

			assert_eq!(y9,  parse_f_from_bytes_new(&account_struct.y1_range_s), "mulassign ");

			/*
			*
			*
			* ------------------------- mul_assign -------------------------
			*
			* r_range: 			r
			* f_f2_range_s: 	free
			* y0_range:  		free
			* y1_range:			y9
			* y2_range:			y4 last used
			* y6_range:			y8
			*/
			let y10 = y8 * &y4;
			// stored in y2_range_s

			//instruction 89 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,89);
			mul_assign_1(
				&account_struct.y2_range_s,  f_cubic_0_range,
				&account_struct.y6_range,  f_cubic_0_range,
				&mut account_struct.cubic_range_0_s,  solo_cubic_0_range
			);

			//instruction 90 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,90);
			mul_assign_2(
				&account_struct.y2_range_s,  f_cubic_1_range,
				&account_struct.y6_range,  f_cubic_1_range,
				&mut account_struct.cubic_range_1_s,  solo_cubic_0_range
			);

			//instruction 91 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,91);
			mul_assign_3(
				&mut account_struct.y2_range_s
			);

			//instruction 92 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,92);
			mul_assign_4_1(
				&account_struct.y6_range,
				&mut account_struct.cubic_range_2_s,
			);

			//instruction 93 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,93);
			mul_assign_4_2(
				&mut account_struct.y2_range_s,
				 f_cubic_1_range,
				&account_struct.cubic_range_2_s,
			);

			//instruction 94 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,94);
			mul_assign_5(
				&mut account_struct.y2_range_s,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s
			);

			assert_eq!(y10,  parse_f_from_bytes_new(&account_struct.y2_range_s), "mulassign ");

			/*
			*
			*
			* ------------------------- mul_assign -------------------------
			*
			* r_range: 			r
			* f_f2_range_s: 	free
			* y0_range:  		free
			* y1_range:			y9
			* y2_range:			y10  last used
			* y6_range:			y8
			*/
            let y11 = y10 * &r;
			// stored in y2_range_s

			//instruction 95 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,95);
			mul_assign_1(
				&account_struct.y2_range_s,  f_cubic_0_range,
				&account_struct.f1_r_range_s,  f_cubic_0_range,
				&mut account_struct.cubic_range_0_s,  solo_cubic_0_range
			);

			//instruction 96 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,96);
			mul_assign_2(
				&account_struct.y2_range_s,  f_cubic_1_range,
				&account_struct.f1_r_range_s,  f_cubic_1_range,
				&mut account_struct.cubic_range_1_s,  solo_cubic_0_range
			);

			//instruction 97 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,97);
			mul_assign_3(
				&mut account_struct.y2_range_s
			);

			//instruction 98 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,98);
			mul_assign_4_1(
				&account_struct.f1_r_range_s,
				&mut account_struct.cubic_range_2_s,
			);

			//instruction 99 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,99);
			mul_assign_4_2(
				&mut account_struct.y2_range_s,
				 f_cubic_1_range,
				&account_struct.cubic_range_2_s,
			);

			//instruction 100 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,100);
			mul_assign_5(
				&mut account_struct.y2_range_s,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s
			);

			assert_eq!(y11,  parse_f_from_bytes_new(&account_struct.y2_range_s), "mulassign ");

			/*
			*
			*
			* ------------------------- assign -------------------------
			*
			* r_range: 			r
			* f_f2_range_s: 	free
			* y0_range:  		free
			* y1_range:			y9
			* y2_range:			y11
			* y6_range:			y8
			*/

            let mut y12 = y9;

			//instruction 101 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,101);
			account_struct.y0_range_s = account_struct.y1_range_s.clone();

			/*
			*
			*
			* ------------------------- frobenius_map(1) -------------------------
			*
			* r_range: 			r
			* f_f2_range_s: 	free
			* y0_range:  		y12
			* y1_range:			y9
			* y2_range:			y11
			* y6_range:			y8
			*/
            y12.frobenius_map(1);

			//instruction 102 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,102);
			custom_frobenius_map_1_1(&mut account_struct.y0_range_s);
			//instruction 103 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,103);
			custom_frobenius_map_1_2(&mut account_struct.y0_range_s);

			assert_eq!(y12,  parse_f_from_bytes_new(&account_struct.y0_range_s));

			/*
			*
			*
			* ------------------------- mul_assign -------------------------
			*
			* r_range: 			r
			* f_f2_range_s: 	free
			* y0_range:  		y12
			* y1_range:			y9
			* y2_range:			y11 last used
			* y6_range:			y8
			*/
            let y13 = y12 * &y11;
			//y13 stored in y2_range_s

			//instruction 104 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,104);
			mul_assign_1(
				&account_struct.y2_range_s,  f_cubic_0_range,
				&account_struct.y0_range_s,  f_cubic_0_range,
				&mut account_struct.cubic_range_0_s,  solo_cubic_0_range
			);

			//instruction 105 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,105);
			mul_assign_2(
				&account_struct.y2_range_s,  f_cubic_1_range,
				&account_struct.y0_range_s,  f_cubic_1_range,
				&mut account_struct.cubic_range_1_s,  solo_cubic_0_range
			);

			//instruction 106 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,106);
			mul_assign_3(
				&mut account_struct.y2_range_s
			);

			//instruction 107 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,107);
			mul_assign_4_1(
				&account_struct.y0_range_s,
				&mut account_struct.cubic_range_2_s,
			);

			//instruction 108 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,108);
			mul_assign_4_2(
				&mut account_struct.y2_range_s,
				 f_cubic_1_range,
				&account_struct.cubic_range_2_s,
			);

			//instruction 109 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,109);
			mul_assign_5(
				&mut account_struct.y2_range_s,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s
			);

			assert_eq!(y13,  parse_f_from_bytes_new(&account_struct.y2_range_s), "mulassign ");

			/*
			*
			*
			* ------------------------- frobenius_map(2) -------------------------
			*
			* r_range: 			r
			* f_f2_range_s: 	free
			* y0_range:  		y12
			* y1_range:			y9
			* y2_range:			y13
			* y6_range:			y8
			*/
            y8.frobenius_map(2);


			//instruction 110 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,110);
			custom_frobenius_map_2_1(&mut account_struct.y6_range);
			//instruction 111 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,111);
			custom_frobenius_map_2_2(&mut account_struct.y6_range);

			assert_eq!(y8,  parse_f_from_bytes_new(&account_struct.y6_range));


			/*
			*
			*
			* ------------------------- mulassign -------------------------
			*
			* r_range: 			r
			* f_f2_range_s: 	free
			* y0_range:  		y12
			* y1_range:			y9
			* y2_range:			y13
			* y6_range:			y8 last used
			*/

            let y14 = y8 * &y13;
			//y14 stored in y6_range
			//instructions already exist

			//instruction 71 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,71);
			mul_assign_1(
				&account_struct.y6_range,  f_cubic_0_range,
				&account_struct.y2_range_s,  f_cubic_0_range,
				&mut account_struct.cubic_range_0_s,  solo_cubic_0_range
			);

			//instruction 72 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,72);
			mul_assign_2(
				&account_struct.y6_range,  f_cubic_1_range,
				&account_struct.y2_range_s,  f_cubic_1_range,
				&mut account_struct.cubic_range_1_s,  solo_cubic_0_range
			);

			//instruction 73 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,73);
			mul_assign_3(
				&mut account_struct.y6_range
			);

			//instruction 74 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,74);
			mul_assign_4_1(
				&account_struct.y2_range_s,
				&mut account_struct.cubic_range_2_s,
			);

			//instruction 75 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,75);
			mul_assign_4_2(
				&mut account_struct.y6_range,
				 f_cubic_1_range,
				&account_struct.cubic_range_2_s,
			);

			//instruction 76 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,76);
			mul_assign_5(
				&mut account_struct.y6_range,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s
			);

			assert_eq!(y14,  parse_f_from_bytes_new(&account_struct.y6_range), "mulassign ");

			/*
			*
			*
			* ------------------------- conjugate -------------------------
			*
			* r_range: 			r
			* f_f2_range_s: 	free
			* y0_range:  		y12
			* y1_range:			y9
			* y2_range:			y13
			* y6_range:			y14
			*/

			//instruction 112 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,112);
        	r.conjugate();
			conjugate_wrapper(&mut account_struct.f1_r_range_s);

			/*
			*
			*
			* ------------------------- mul_assign -------------------------
			*
			* r_range: 			r		last used
			* f_f2_range_s: 	free
			* y0_range:  		y12
			* y1_range:			y9		last used
			* y2_range:			y13
			* y6_range:			y14
			*/

            let mut y15 = r * &y9;
			//y15 stored in y1_range

			//instruction 113 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,113);
			mul_assign_1(
				&account_struct.y1_range_s,  f_cubic_0_range,
				&account_struct.f1_r_range_s,  f_cubic_0_range,
				&mut account_struct.cubic_range_0_s,  solo_cubic_0_range
			);

			//instruction 114 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,114);
			mul_assign_2(
				&account_struct.y1_range_s,  f_cubic_1_range,
				&account_struct.f1_r_range_s,  f_cubic_1_range,
				&mut account_struct.cubic_range_1_s,  solo_cubic_0_range
			);

			//instruction 115 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,115);
			mul_assign_3(
				&mut account_struct.y1_range_s
			);

			//instruction 116 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,116);
			mul_assign_4_1(
				&account_struct.f1_r_range_s,
				&mut account_struct.cubic_range_2_s,
			);

			//instruction 117 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,117);
			mul_assign_4_2(
				&mut account_struct.y1_range_s,
				 f_cubic_1_range,
				&account_struct.cubic_range_2_s,
			);

			//instruction 118 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,118);
			mul_assign_5(
				&mut account_struct.y1_range_s,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s
			);

			assert_eq!(y15,  parse_f_from_bytes_new(&account_struct.y1_range_s), "mulassign ");

			/*
			*
			*
			* ------------------------- frobenius_map(3) -------------------------
			*
			* r_range: 			r
			* f_f2_range_s: 	free
			* y0_range:  		y12
			* y1_range:			y15
			* y2_range:			y13
			* y6_range:			y14
			*/

            y15.frobenius_map(3);

			//instruction 119 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,119);
			custom_frobenius_map_3_1(&mut account_struct.y1_range_s);
			//instruction 120 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,120);
			custom_frobenius_map_3_2(&mut account_struct.y1_range_s);

			assert_eq!(y15,  parse_f_from_bytes_new(&account_struct.y1_range_s));

			/*
			*
			*
			* ------------------------- mulassign -------------------------
			*
			* r_range: 			r
			* f_f2_range_s: 	free
			* y0_range:  		y12
			* y1_range:			y15
			* y2_range:			y13
			* y6_range:			y14
			*/
			//not unique second time instruction 83

            let y16 = y15 * &y14;

			//instruction 83 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,83);
			mul_assign_1(
				&account_struct.y1_range_s,  f_cubic_0_range,
				&account_struct.y6_range,  f_cubic_0_range,
				&mut account_struct.cubic_range_0_s,  solo_cubic_0_range
			);

			//instruction 84 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,84);
			mul_assign_2(
				&account_struct.y1_range_s,  f_cubic_1_range,
				&account_struct.y6_range,  f_cubic_1_range,
				&mut account_struct.cubic_range_1_s,  solo_cubic_0_range
			);

			//instruction 85 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,85);
			mul_assign_3(
				&mut account_struct.y1_range_s
			);

			//instruction 86 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,86);
			mul_assign_4_1(
				&account_struct.y6_range,
				&mut account_struct.cubic_range_2_s,
			);

			//instruction 87 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,87);
			mul_assign_4_2(
				&mut account_struct.y1_range_s,
				 f_cubic_1_range,
				&account_struct.cubic_range_2_s,
			);

			//instruction 88 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s,account_struct.y0_range_s); assert_eq!(account_struct1.y1_range_s,account_struct.y1_range_s); assert_eq!(account_struct1.y2_range_s,account_struct.y2_range_s);  assert_eq!(account_struct1.y6_range,account_struct.y6_range); _process_instruction_final_exp(&mut account_struct1,88);
			mul_assign_5(
				&mut account_struct.y1_range_s,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s
			);

			assert_eq!(y16,  parse_f_from_bytes_new(&account_struct.y1_range_s), "mulassign ");
			//println!("Let instruction order: [u8: {}] = {:?}",instruction_order.len(), instruction_order);
            y16
        })
    }

	pub const instruction_order: [u8; 700] = [0,1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 10, 11, 12, 13, 14, 15, 19, 20, 20, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 20, 27, 28, 29, 30, 31, 32, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 27, 28, 29, 30, 31, 32, 20, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 27, 28, 29, 30, 31, 32, 20, 20, 27, 28, 29, 30, 31, 32, 20, 20, 27, 28, 29, 30, 31, 32, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 27, 28, 29, 30, 31, 32, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 20, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 21, 22, 23, 24, 25, 26, 20, 20, 20, 20, 20, 27, 28, 29, 30, 31, 32, 20, 20, 20, 20, 21, 22, 23, 24, 25, 26, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 42, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 42, 49, 50, 51, 52, 53, 54, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 49, 50, 51, 52, 53, 54, 42, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 49, 50, 51, 52, 53, 54, 42, 42, 49, 50, 51, 52, 53, 54, 42, 42, 49, 50, 51, 52, 53, 54, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 49, 50, 51, 52, 53, 54, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 42, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 43, 44, 45, 46, 47, 48, 42, 42, 42, 42, 42, 49, 50, 51, 52, 53, 54, 42, 42, 42, 42, 43, 44, 45, 46, 47, 48, 55, 56, 57, 57, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 57, 64, 65, 66, 67, 68, 69, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 64, 65, 66, 67, 68, 69, 57, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 64, 65, 66, 67, 68, 69, 57, 57, 64, 65, 66, 67, 68, 69, 57, 57, 64, 65, 66, 67, 68, 69, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 64, 65, 66, 67, 68, 69, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 57, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 58, 59, 60, 61, 62, 63, 57, 57, 57, 57, 57, 64, 65, 66, 67, 68, 69, 57, 57, 57, 57, 58, 59, 60, 61, 62, 63, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 71, 72, 73, 74, 75, 76, 112, 113, 114, 115, 116, 117, 118, 119, 120, 83, 84, 85, 86, 87, 88];

	fn final_exponentiation_test_processor(f: &<ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk) -> Option<<ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk> {

			let mut account_struct = FinalExpBytes::new();

		parse_f_to_bytes_new(*f, &mut account_struct.f1_r_range_s);

		for i in instruction_order {
			_process_instruction_final_exp(&mut account_struct, i );
		}

		Some(parse_f_from_bytes_new(&account_struct.y1_range_s))

	}
	use ark_ff::Fp12;
	use ark_ec::bn::BnParameters;
	pub fn exp_by_neg_x(mut f: Fp12::<<ark_bn254::Parameters as ark_ec::bn::BnParameters>::Fp12Params>) -> Fp12::<<ark_bn254::Parameters as ark_ec::bn::BnParameters>::Fp12Params> {
        f = f.cyclotomic_exp(&<ark_bn254::Parameters as ark_ec::bn::BnParameters>::X);
        if !<ark_bn254::Parameters as ark_ec::bn::BnParameters>::X_IS_NEGATIVE {
			println!("conjugate");
			f.conjugate();
        }
        f
    }
}
