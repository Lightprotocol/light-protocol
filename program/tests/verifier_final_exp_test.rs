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
	use Testing_Hardcoded_Params_devnet_new::instructions_poseidon::PoseidonCircomRounds3;
	use std::convert::TryInto;
    use ark_std::{UniformRand, test_rng};

	use Testing_Hardcoded_Params_devnet_new::state_merkle_tree::{HashBytes, MerkleTree as MerkleTreeOnchain};

	use ark_std::{One};

	use Testing_Hardcoded_Params_devnet_new::init_bytes11;
	use Testing_Hardcoded_Params_devnet_new::processor_merkle_tree;


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

    use Testing_Hardcoded_Params_devnet_new::hard_coded_verifying_key_pvk_254::*;


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



        // println!("{:?}", res_origin);
        // println!("{:?}", pvk.alpha_g1_beta_g2);
        assert_eq!(res_origin, pvk.alpha_g1_beta_g2);
        Ok(())
    }

	use ark_ff::Field;
    #[allow(clippy::let_and_return)]
    fn final_exponentiation_custom(f: &<ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk) -> Option<<ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk> {
        // Easy part: result = elt^((q^6-1)*(q^2+1)).
        // Follows, e.g., Beuchat et al page 9, by computing result as follows:
        //   elt^((q^6-1)*(q^2+1)) = (conj(elt) * elt^(-1))^(q^2+1)

        // f1 = r.conjugate() = f^(p^6)
        let mut f1 = *f;
        f1.conjugate();

        f.inverse().map(|mut f2| {
            // f2 = f^(-1);
            // r = f^(p^6 - 1)
            let mut r = f1 * &f2;

            // f2 = f^(p^6 - 1)
            f2 = r;
            // r = f^((p^6 - 1)(p^2))
            r.frobenius_map(2);

            // r = f^((p^6 - 1)(p^2) + (p^6 - 1))
            // r = f^((p^6 - 1)(p^2 + 1))
            r *= &f2;

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
            let y1 = y0.cyclotomic_square();
			//y0 last used
            let y2 = y1.cyclotomic_square();
            let mut y3 = y2 * &y1;
			//y2 last used
            let y4 = exp_by_neg_x(y3);
            let y5 = y4.cyclotomic_square();
            let mut y6 = exp_by_neg_x(y5);
			//y5 last used
            y3.conjugate();
            y6.conjugate();
            let y7 = y6 * &y4;
			//y6 last used
            let mut y8 = y7 * &y3;
			//y3 last used
            let y9 = y8 * &y1;
			//y1 last used
            let y10 = y8 * &y4;
			//y4 last used
            let y11 = y10 * &r;
			//y10 last used
            let mut y12 = y9;
            y12.frobenius_map(1);
            let y13 = y12 * &y11;
			//y11 last used
            y8.frobenius_map(2);
            let y14 = y8 * &y13;
			//y8 last used
            r.conjugate();
            let mut y15 = r * &y9;
			//y9 last used
            y15.frobenius_map(3);
            let y16 = y15 * &y14;

            y16
        })
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
