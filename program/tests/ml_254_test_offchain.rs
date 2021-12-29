#[cfg(test)]
pub mod tests {
    use ark_bn254;
    use ark_ed_on_bn254;
    use ark_ed_on_bn254::Fq;
    use ark_std::vec::Vec;

    use ark_ec;
    use ark_ff::biginteger::{BigInteger256, BigInteger384};
    use ark_ff::bytes::{FromBytes, ToBytes};
    use ark_ff::Fp256;
    use ark_ff::QuadExtField;
    use ark_groth16::{
        prepare_inputs, prepare_verifying_key, verify_proof, verify_proof_with_prepared_inputs,
    };
    use ark_std::One;
    use serde_json::{Result, Value};
    use solana_program::{program_error::ProgramError, program_pack::Pack};
    use std::fs;

    use ark_ec::{AffineCurve, ProjectiveCurve};

    use Testing_Hardcoded_Params_devnet_new::{
        utils::prepared_verifying_key::*,
        Groth16_verifier::{
            parsers::*,
            miller_loop::{
                ml_processor::*,
                ml_state::*,
            }
        }
    };

    // For native miller loop implementation
    use ark_bn254::Fq12Parameters;
    use ark_ff::fields::models::fp6_3over2::{Fp6, Fp6Parameters};
    use ark_ff::fields::models::quadratic_extension::QuadExtParameters;
    use ark_ff::fields::{
        fp12_2over3over2::{Fp12, Fp12Parameters},
        Field, Fp2,
    };
    use ark_ff::Fp12ParamsWrapper;
    pub const ML_IX_ORDER: [u8; 430] = [
        0, 1, 2, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7,
        4, 5, 6, 3, 7, 4, 5, 6, 9, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 8,
        4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 9, 4, 5, 6, 3, 7, 4, 5, 6,
        3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5,
        6, 3, 7, 4, 5, 6, 9, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7,
        4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 9, 4, 5, 6, 3,
        7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6,
        8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 9, 4, 5,
        6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 9, 4,
        5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7,
        4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 9, 4, 5, 6, 3,
        7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 9, 4, 5, 6, 3, 7, 4, 5, 6,
        3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5, 6, 8, 4, 5, 6, 3, 7, 4, 5, 6, 3, 7, 4, 5,
        6, 3, 7, 4, 5, 6, 10, 4, 5, 6, 11, 4, 5, 6,
    ];

    fn get_pvk_from_bytes_254() -> Result<
        ark_groth16::data_structures::VerifyingKey<ark_ec::models::bn::Bn<ark_bn254::Parameters>>,
    > {
        let contents = fs::read_to_string("./tests/verification_key_bytes_254.txt")
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
        for (i, _) in b_g2_bigints.iter().enumerate() {}

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

        for (i, _) in delta_g2_bytes.iter().enumerate() {}

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

        for (i, _) in gamma_g2_bytes.iter().enumerate() {}

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

    fn get_proof_from_bytes_254(
    ) -> Result<ark_groth16::data_structures::Proof<ark_ec::models::bn::Bn<ark_bn254::Parameters>>>
    {
        let contents = fs::read_to_string("./tests/proof_bytes_254.txt")
            .expect("Something went wrong reading the file");
        let v: Value = serde_json::from_str(&contents)?;

        let mut a_g1_bigints = Vec::new();
        for i in 0..3 {
            let mut bytes: Vec<u8> = Vec::new();
            for i in v["pi_a"][i].as_str().unwrap().split(',') {
                bytes.push((*i).parse::<u8>().unwrap());
            }
            a_g1_bigints
                .push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
        }
        let a_g1 = ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
            a_g1_bigints[0],
            a_g1_bigints[1],
            false,
        );
        let mut b_g2_bigints = Vec::new();
        for i in 0..2 {
            for j in 0..2 {
                let mut bytes: Vec<u8> = Vec::new();
                for z in v["pi_b"][i][j].as_str().unwrap().split(',') {
                    bytes.push((*z).parse::<u8>().unwrap());
                }
                b_g2_bigints
                    .push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
            }
        }
        let b_g2 = ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters>::new(
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

        let mut c_g1_bigints = Vec::new();
        for i in 0..2 {
            let mut bytes: Vec<u8> = Vec::new();
            for i in v["pi_c"][i].as_str().unwrap().split(',') {
                bytes.push((*i).parse::<u8>().unwrap());
            }
            c_g1_bigints
                .push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
        }
        let c_g1 = ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
            c_g1_bigints[0],
            c_g1_bigints[1],
            false,
        );

        Ok(
            ark_groth16::data_structures::Proof::<ark_ec::models::bn::Bn<ark_bn254::Parameters>> {
                a: a_g1,
                b: b_g2,
                c: c_g1,
            },
        )
    }

    fn get_public_inputs_from_bytes_254() -> Result<Vec<Fp256<ark_ed_on_bn254::FqParameters>>> {
        let contents = fs::read_to_string("./tests/public_inputs_254_bytes.txt")
            .expect("Something went wrong reading the file");
        let v: Value = serde_json::from_str(&contents)?;
        let mut res = Vec::new();
        for i in 0..7 {
            let mut bytes: Vec<u8> = Vec::new();
            for i in v[i].as_str().unwrap().split(',') {
                bytes.push((*i).parse::<u8>().unwrap());
            }
            res.push(<Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap())
        }

        Ok(res)
    }

    // Testing the integrity of hardcoded verifying key and vk bytes file.
    // They must be == to ensure we test against the right reference in our ML tests.
    #[test]
    fn hardcoded_verifying_key_254_test() {
        let pvk_unprepped = get_pvk_from_bytes_254().unwrap();
        let pvk = prepare_verifying_key(&pvk_unprepped);
        assert_eq!(get_gamma_abc_g1_0(), pvk.vk.gamma_abc_g1[0]);
        assert_eq!(get_gamma_abc_g1_1(), pvk.vk.gamma_abc_g1[1]);
        assert_eq!(get_gamma_abc_g1_2(), pvk.vk.gamma_abc_g1[2]);
        assert_eq!(get_gamma_abc_g1_3(), pvk.vk.gamma_abc_g1[3]);
        assert_eq!(get_gamma_abc_g1_4(), pvk.vk.gamma_abc_g1[4]);
        assert_eq!(get_gamma_abc_g1_5(), pvk.vk.gamma_abc_g1[5]);
        assert_eq!(get_gamma_abc_g1_6(), pvk.vk.gamma_abc_g1[6]);
        assert_eq!(get_gamma_abc_g1_7(), pvk.vk.gamma_abc_g1[7]);
        assert_eq!(get_gamma_g2_neg_pc_0(), pvk.gamma_g2_neg_pc.ell_coeffs[0]);
        assert_eq!(get_gamma_g2_neg_pc_1(), pvk.gamma_g2_neg_pc.ell_coeffs[1]);
        assert_eq!(get_gamma_g2_neg_pc_2(), pvk.gamma_g2_neg_pc.ell_coeffs[2]);
        assert_eq!(get_gamma_g2_neg_pc_3(), pvk.gamma_g2_neg_pc.ell_coeffs[3]);
        assert_eq!(get_gamma_g2_neg_pc_4(), pvk.gamma_g2_neg_pc.ell_coeffs[4]);
        assert_eq!(get_gamma_g2_neg_pc_5(), pvk.gamma_g2_neg_pc.ell_coeffs[5]);
        assert_eq!(get_gamma_g2_neg_pc_6(), pvk.gamma_g2_neg_pc.ell_coeffs[6]);
        assert_eq!(get_gamma_g2_neg_pc_7(), pvk.gamma_g2_neg_pc.ell_coeffs[7]);
        assert_eq!(get_gamma_g2_neg_pc_8(), pvk.gamma_g2_neg_pc.ell_coeffs[8]);
        assert_eq!(get_gamma_g2_neg_pc_9(), pvk.gamma_g2_neg_pc.ell_coeffs[9]);
        assert_eq!(get_gamma_g2_neg_pc_10(), pvk.gamma_g2_neg_pc.ell_coeffs[10]);
        assert_eq!(get_gamma_g2_neg_pc_11(), pvk.gamma_g2_neg_pc.ell_coeffs[11]);
        assert_eq!(get_gamma_g2_neg_pc_12(), pvk.gamma_g2_neg_pc.ell_coeffs[12]);
        assert_eq!(get_gamma_g2_neg_pc_13(), pvk.gamma_g2_neg_pc.ell_coeffs[13]);
        assert_eq!(get_gamma_g2_neg_pc_14(), pvk.gamma_g2_neg_pc.ell_coeffs[14]);
        assert_eq!(get_gamma_g2_neg_pc_15(), pvk.gamma_g2_neg_pc.ell_coeffs[15]);
        assert_eq!(get_gamma_g2_neg_pc_16(), pvk.gamma_g2_neg_pc.ell_coeffs[16]);
        assert_eq!(get_gamma_g2_neg_pc_17(), pvk.gamma_g2_neg_pc.ell_coeffs[17]);
        assert_eq!(get_gamma_g2_neg_pc_18(), pvk.gamma_g2_neg_pc.ell_coeffs[18]);
        assert_eq!(get_gamma_g2_neg_pc_19(), pvk.gamma_g2_neg_pc.ell_coeffs[19]);
        assert_eq!(get_gamma_g2_neg_pc_20(), pvk.gamma_g2_neg_pc.ell_coeffs[20]);
        assert_eq!(get_gamma_g2_neg_pc_21(), pvk.gamma_g2_neg_pc.ell_coeffs[21]);
        assert_eq!(get_gamma_g2_neg_pc_22(), pvk.gamma_g2_neg_pc.ell_coeffs[22]);
        assert_eq!(get_gamma_g2_neg_pc_23(), pvk.gamma_g2_neg_pc.ell_coeffs[23]);
        assert_eq!(get_gamma_g2_neg_pc_24(), pvk.gamma_g2_neg_pc.ell_coeffs[24]);
        assert_eq!(get_gamma_g2_neg_pc_25(), pvk.gamma_g2_neg_pc.ell_coeffs[25]);
        assert_eq!(get_gamma_g2_neg_pc_26(), pvk.gamma_g2_neg_pc.ell_coeffs[26]);
        assert_eq!(get_gamma_g2_neg_pc_27(), pvk.gamma_g2_neg_pc.ell_coeffs[27]);
        assert_eq!(get_gamma_g2_neg_pc_28(), pvk.gamma_g2_neg_pc.ell_coeffs[28]);
        assert_eq!(get_gamma_g2_neg_pc_29(), pvk.gamma_g2_neg_pc.ell_coeffs[29]);
        assert_eq!(get_gamma_g2_neg_pc_30(), pvk.gamma_g2_neg_pc.ell_coeffs[30]);
        assert_eq!(get_gamma_g2_neg_pc_31(), pvk.gamma_g2_neg_pc.ell_coeffs[31]);
        assert_eq!(get_gamma_g2_neg_pc_32(), pvk.gamma_g2_neg_pc.ell_coeffs[32]);
        assert_eq!(get_gamma_g2_neg_pc_33(), pvk.gamma_g2_neg_pc.ell_coeffs[33]);
        assert_eq!(get_gamma_g2_neg_pc_34(), pvk.gamma_g2_neg_pc.ell_coeffs[34]);
        assert_eq!(get_gamma_g2_neg_pc_35(), pvk.gamma_g2_neg_pc.ell_coeffs[35]);
        assert_eq!(get_gamma_g2_neg_pc_36(), pvk.gamma_g2_neg_pc.ell_coeffs[36]);
        assert_eq!(get_gamma_g2_neg_pc_37(), pvk.gamma_g2_neg_pc.ell_coeffs[37]);
        assert_eq!(get_gamma_g2_neg_pc_38(), pvk.gamma_g2_neg_pc.ell_coeffs[38]);
        assert_eq!(get_gamma_g2_neg_pc_39(), pvk.gamma_g2_neg_pc.ell_coeffs[39]);
        assert_eq!(get_gamma_g2_neg_pc_40(), pvk.gamma_g2_neg_pc.ell_coeffs[40]);
        assert_eq!(get_gamma_g2_neg_pc_41(), pvk.gamma_g2_neg_pc.ell_coeffs[41]);
        assert_eq!(get_gamma_g2_neg_pc_42(), pvk.gamma_g2_neg_pc.ell_coeffs[42]);
        assert_eq!(get_gamma_g2_neg_pc_43(), pvk.gamma_g2_neg_pc.ell_coeffs[43]);
        assert_eq!(get_gamma_g2_neg_pc_44(), pvk.gamma_g2_neg_pc.ell_coeffs[44]);
        assert_eq!(get_gamma_g2_neg_pc_45(), pvk.gamma_g2_neg_pc.ell_coeffs[45]);
        assert_eq!(get_gamma_g2_neg_pc_46(), pvk.gamma_g2_neg_pc.ell_coeffs[46]);
        assert_eq!(get_gamma_g2_neg_pc_47(), pvk.gamma_g2_neg_pc.ell_coeffs[47]);
        assert_eq!(get_gamma_g2_neg_pc_48(), pvk.gamma_g2_neg_pc.ell_coeffs[48]);
        assert_eq!(get_gamma_g2_neg_pc_49(), pvk.gamma_g2_neg_pc.ell_coeffs[49]);
        assert_eq!(get_gamma_g2_neg_pc_50(), pvk.gamma_g2_neg_pc.ell_coeffs[50]);
        assert_eq!(get_gamma_g2_neg_pc_51(), pvk.gamma_g2_neg_pc.ell_coeffs[51]);
        assert_eq!(get_gamma_g2_neg_pc_52(), pvk.gamma_g2_neg_pc.ell_coeffs[52]);
        assert_eq!(get_gamma_g2_neg_pc_53(), pvk.gamma_g2_neg_pc.ell_coeffs[53]);
        assert_eq!(get_gamma_g2_neg_pc_54(), pvk.gamma_g2_neg_pc.ell_coeffs[54]);
        assert_eq!(get_gamma_g2_neg_pc_55(), pvk.gamma_g2_neg_pc.ell_coeffs[55]);
        assert_eq!(get_gamma_g2_neg_pc_56(), pvk.gamma_g2_neg_pc.ell_coeffs[56]);
        assert_eq!(get_gamma_g2_neg_pc_57(), pvk.gamma_g2_neg_pc.ell_coeffs[57]);
        assert_eq!(get_gamma_g2_neg_pc_58(), pvk.gamma_g2_neg_pc.ell_coeffs[58]);
        assert_eq!(get_gamma_g2_neg_pc_59(), pvk.gamma_g2_neg_pc.ell_coeffs[59]);
        assert_eq!(get_gamma_g2_neg_pc_60(), pvk.gamma_g2_neg_pc.ell_coeffs[60]);
        assert_eq!(get_gamma_g2_neg_pc_61(), pvk.gamma_g2_neg_pc.ell_coeffs[61]);
        assert_eq!(get_gamma_g2_neg_pc_62(), pvk.gamma_g2_neg_pc.ell_coeffs[62]);
        assert_eq!(get_gamma_g2_neg_pc_63(), pvk.gamma_g2_neg_pc.ell_coeffs[63]);
        assert_eq!(get_gamma_g2_neg_pc_64(), pvk.gamma_g2_neg_pc.ell_coeffs[64]);
        assert_eq!(get_gamma_g2_neg_pc_65(), pvk.gamma_g2_neg_pc.ell_coeffs[65]);
        assert_eq!(get_gamma_g2_neg_pc_66(), pvk.gamma_g2_neg_pc.ell_coeffs[66]);
        assert_eq!(get_gamma_g2_neg_pc_67(), pvk.gamma_g2_neg_pc.ell_coeffs[67]);
        assert_eq!(get_gamma_g2_neg_pc_68(), pvk.gamma_g2_neg_pc.ell_coeffs[68]);
        assert_eq!(get_gamma_g2_neg_pc_69(), pvk.gamma_g2_neg_pc.ell_coeffs[69]);
        assert_eq!(get_gamma_g2_neg_pc_70(), pvk.gamma_g2_neg_pc.ell_coeffs[70]);
        assert_eq!(get_gamma_g2_neg_pc_71(), pvk.gamma_g2_neg_pc.ell_coeffs[71]);
        assert_eq!(get_gamma_g2_neg_pc_72(), pvk.gamma_g2_neg_pc.ell_coeffs[72]);
        assert_eq!(get_gamma_g2_neg_pc_73(), pvk.gamma_g2_neg_pc.ell_coeffs[73]);
        assert_eq!(get_gamma_g2_neg_pc_74(), pvk.gamma_g2_neg_pc.ell_coeffs[74]);
        assert_eq!(get_gamma_g2_neg_pc_75(), pvk.gamma_g2_neg_pc.ell_coeffs[75]);
        assert_eq!(get_gamma_g2_neg_pc_76(), pvk.gamma_g2_neg_pc.ell_coeffs[76]);
        assert_eq!(get_gamma_g2_neg_pc_77(), pvk.gamma_g2_neg_pc.ell_coeffs[77]);
        assert_eq!(get_gamma_g2_neg_pc_78(), pvk.gamma_g2_neg_pc.ell_coeffs[78]);
        assert_eq!(get_gamma_g2_neg_pc_79(), pvk.gamma_g2_neg_pc.ell_coeffs[79]);
        assert_eq!(get_gamma_g2_neg_pc_80(), pvk.gamma_g2_neg_pc.ell_coeffs[80]);
        assert_eq!(get_gamma_g2_neg_pc_81(), pvk.gamma_g2_neg_pc.ell_coeffs[81]);
        assert_eq!(get_gamma_g2_neg_pc_82(), pvk.gamma_g2_neg_pc.ell_coeffs[82]);
        assert_eq!(get_gamma_g2_neg_pc_83(), pvk.gamma_g2_neg_pc.ell_coeffs[83]);
        assert_eq!(get_gamma_g2_neg_pc_84(), pvk.gamma_g2_neg_pc.ell_coeffs[84]);
        assert_eq!(get_gamma_g2_neg_pc_85(), pvk.gamma_g2_neg_pc.ell_coeffs[85]);
        assert_eq!(get_gamma_g2_neg_pc_86(), pvk.gamma_g2_neg_pc.ell_coeffs[86]);
        assert_eq!(get_gamma_g2_neg_pc_87(), pvk.gamma_g2_neg_pc.ell_coeffs[87]);
        assert_eq!(get_gamma_g2_neg_pc_88(), pvk.gamma_g2_neg_pc.ell_coeffs[88]);
        assert_eq!(get_gamma_g2_neg_pc_89(), pvk.gamma_g2_neg_pc.ell_coeffs[89]);
        assert_eq!(get_gamma_g2_neg_pc_90(), pvk.gamma_g2_neg_pc.ell_coeffs[90]);
        assert_eq!(get_delta_g2_neg_pc_0(), pvk.delta_g2_neg_pc.ell_coeffs[0]);
        assert_eq!(get_delta_g2_neg_pc_1(), pvk.delta_g2_neg_pc.ell_coeffs[1]);
        assert_eq!(get_delta_g2_neg_pc_2(), pvk.delta_g2_neg_pc.ell_coeffs[2]);
        assert_eq!(get_delta_g2_neg_pc_3(), pvk.delta_g2_neg_pc.ell_coeffs[3]);
        assert_eq!(get_delta_g2_neg_pc_4(), pvk.delta_g2_neg_pc.ell_coeffs[4]);
        assert_eq!(get_delta_g2_neg_pc_5(), pvk.delta_g2_neg_pc.ell_coeffs[5]);
        assert_eq!(get_delta_g2_neg_pc_6(), pvk.delta_g2_neg_pc.ell_coeffs[6]);
        assert_eq!(get_delta_g2_neg_pc_7(), pvk.delta_g2_neg_pc.ell_coeffs[7]);
        assert_eq!(get_delta_g2_neg_pc_8(), pvk.delta_g2_neg_pc.ell_coeffs[8]);
        assert_eq!(get_delta_g2_neg_pc_9(), pvk.delta_g2_neg_pc.ell_coeffs[9]);
        assert_eq!(get_delta_g2_neg_pc_10(), pvk.delta_g2_neg_pc.ell_coeffs[10]);
        assert_eq!(get_delta_g2_neg_pc_11(), pvk.delta_g2_neg_pc.ell_coeffs[11]);
        assert_eq!(get_delta_g2_neg_pc_12(), pvk.delta_g2_neg_pc.ell_coeffs[12]);
        assert_eq!(get_delta_g2_neg_pc_13(), pvk.delta_g2_neg_pc.ell_coeffs[13]);
        assert_eq!(get_delta_g2_neg_pc_14(), pvk.delta_g2_neg_pc.ell_coeffs[14]);
        assert_eq!(get_delta_g2_neg_pc_15(), pvk.delta_g2_neg_pc.ell_coeffs[15]);
        assert_eq!(get_delta_g2_neg_pc_16(), pvk.delta_g2_neg_pc.ell_coeffs[16]);
        assert_eq!(get_delta_g2_neg_pc_17(), pvk.delta_g2_neg_pc.ell_coeffs[17]);
        assert_eq!(get_delta_g2_neg_pc_18(), pvk.delta_g2_neg_pc.ell_coeffs[18]);
        assert_eq!(get_delta_g2_neg_pc_19(), pvk.delta_g2_neg_pc.ell_coeffs[19]);
        assert_eq!(get_delta_g2_neg_pc_20(), pvk.delta_g2_neg_pc.ell_coeffs[20]);
        assert_eq!(get_delta_g2_neg_pc_21(), pvk.delta_g2_neg_pc.ell_coeffs[21]);
        assert_eq!(get_delta_g2_neg_pc_22(), pvk.delta_g2_neg_pc.ell_coeffs[22]);
        assert_eq!(get_delta_g2_neg_pc_23(), pvk.delta_g2_neg_pc.ell_coeffs[23]);
        assert_eq!(get_delta_g2_neg_pc_24(), pvk.delta_g2_neg_pc.ell_coeffs[24]);
        assert_eq!(get_delta_g2_neg_pc_25(), pvk.delta_g2_neg_pc.ell_coeffs[25]);
        assert_eq!(get_delta_g2_neg_pc_26(), pvk.delta_g2_neg_pc.ell_coeffs[26]);
        assert_eq!(get_delta_g2_neg_pc_27(), pvk.delta_g2_neg_pc.ell_coeffs[27]);
        assert_eq!(get_delta_g2_neg_pc_28(), pvk.delta_g2_neg_pc.ell_coeffs[28]);
        assert_eq!(get_delta_g2_neg_pc_29(), pvk.delta_g2_neg_pc.ell_coeffs[29]);
        assert_eq!(get_delta_g2_neg_pc_30(), pvk.delta_g2_neg_pc.ell_coeffs[30]);
        assert_eq!(get_delta_g2_neg_pc_31(), pvk.delta_g2_neg_pc.ell_coeffs[31]);
        assert_eq!(get_delta_g2_neg_pc_32(), pvk.delta_g2_neg_pc.ell_coeffs[32]);
        assert_eq!(get_delta_g2_neg_pc_33(), pvk.delta_g2_neg_pc.ell_coeffs[33]);
        assert_eq!(get_delta_g2_neg_pc_34(), pvk.delta_g2_neg_pc.ell_coeffs[34]);
        assert_eq!(get_delta_g2_neg_pc_35(), pvk.delta_g2_neg_pc.ell_coeffs[35]);
        assert_eq!(get_delta_g2_neg_pc_36(), pvk.delta_g2_neg_pc.ell_coeffs[36]);
        assert_eq!(get_delta_g2_neg_pc_37(), pvk.delta_g2_neg_pc.ell_coeffs[37]);
        assert_eq!(get_delta_g2_neg_pc_38(), pvk.delta_g2_neg_pc.ell_coeffs[38]);
        assert_eq!(get_delta_g2_neg_pc_39(), pvk.delta_g2_neg_pc.ell_coeffs[39]);
        assert_eq!(get_delta_g2_neg_pc_40(), pvk.delta_g2_neg_pc.ell_coeffs[40]);
        assert_eq!(get_delta_g2_neg_pc_41(), pvk.delta_g2_neg_pc.ell_coeffs[41]);
        assert_eq!(get_delta_g2_neg_pc_42(), pvk.delta_g2_neg_pc.ell_coeffs[42]);
        assert_eq!(get_delta_g2_neg_pc_43(), pvk.delta_g2_neg_pc.ell_coeffs[43]);
        assert_eq!(get_delta_g2_neg_pc_44(), pvk.delta_g2_neg_pc.ell_coeffs[44]);
        assert_eq!(get_delta_g2_neg_pc_45(), pvk.delta_g2_neg_pc.ell_coeffs[45]);
        assert_eq!(get_delta_g2_neg_pc_46(), pvk.delta_g2_neg_pc.ell_coeffs[46]);
        assert_eq!(get_delta_g2_neg_pc_47(), pvk.delta_g2_neg_pc.ell_coeffs[47]);
        assert_eq!(get_delta_g2_neg_pc_48(), pvk.delta_g2_neg_pc.ell_coeffs[48]);
        assert_eq!(get_delta_g2_neg_pc_49(), pvk.delta_g2_neg_pc.ell_coeffs[49]);
        assert_eq!(get_delta_g2_neg_pc_50(), pvk.delta_g2_neg_pc.ell_coeffs[50]);
        assert_eq!(get_delta_g2_neg_pc_51(), pvk.delta_g2_neg_pc.ell_coeffs[51]);
        assert_eq!(get_delta_g2_neg_pc_52(), pvk.delta_g2_neg_pc.ell_coeffs[52]);
        assert_eq!(get_delta_g2_neg_pc_53(), pvk.delta_g2_neg_pc.ell_coeffs[53]);
        assert_eq!(get_delta_g2_neg_pc_54(), pvk.delta_g2_neg_pc.ell_coeffs[54]);
        assert_eq!(get_delta_g2_neg_pc_55(), pvk.delta_g2_neg_pc.ell_coeffs[55]);
        assert_eq!(get_delta_g2_neg_pc_56(), pvk.delta_g2_neg_pc.ell_coeffs[56]);
        assert_eq!(get_delta_g2_neg_pc_57(), pvk.delta_g2_neg_pc.ell_coeffs[57]);
        assert_eq!(get_delta_g2_neg_pc_58(), pvk.delta_g2_neg_pc.ell_coeffs[58]);
        assert_eq!(get_delta_g2_neg_pc_59(), pvk.delta_g2_neg_pc.ell_coeffs[59]);
        assert_eq!(get_delta_g2_neg_pc_60(), pvk.delta_g2_neg_pc.ell_coeffs[60]);
        assert_eq!(get_delta_g2_neg_pc_61(), pvk.delta_g2_neg_pc.ell_coeffs[61]);
        assert_eq!(get_delta_g2_neg_pc_62(), pvk.delta_g2_neg_pc.ell_coeffs[62]);
        assert_eq!(get_delta_g2_neg_pc_63(), pvk.delta_g2_neg_pc.ell_coeffs[63]);
        assert_eq!(get_delta_g2_neg_pc_64(), pvk.delta_g2_neg_pc.ell_coeffs[64]);
        assert_eq!(get_delta_g2_neg_pc_65(), pvk.delta_g2_neg_pc.ell_coeffs[65]);
        assert_eq!(get_delta_g2_neg_pc_66(), pvk.delta_g2_neg_pc.ell_coeffs[66]);
        assert_eq!(get_delta_g2_neg_pc_67(), pvk.delta_g2_neg_pc.ell_coeffs[67]);
        assert_eq!(get_delta_g2_neg_pc_68(), pvk.delta_g2_neg_pc.ell_coeffs[68]);
        assert_eq!(get_delta_g2_neg_pc_69(), pvk.delta_g2_neg_pc.ell_coeffs[69]);
        assert_eq!(get_delta_g2_neg_pc_70(), pvk.delta_g2_neg_pc.ell_coeffs[70]);
        assert_eq!(get_delta_g2_neg_pc_71(), pvk.delta_g2_neg_pc.ell_coeffs[71]);
        assert_eq!(get_delta_g2_neg_pc_72(), pvk.delta_g2_neg_pc.ell_coeffs[72]);
        assert_eq!(get_delta_g2_neg_pc_73(), pvk.delta_g2_neg_pc.ell_coeffs[73]);
        assert_eq!(get_delta_g2_neg_pc_74(), pvk.delta_g2_neg_pc.ell_coeffs[74]);
        assert_eq!(get_delta_g2_neg_pc_75(), pvk.delta_g2_neg_pc.ell_coeffs[75]);
        assert_eq!(get_delta_g2_neg_pc_76(), pvk.delta_g2_neg_pc.ell_coeffs[76]);
        assert_eq!(get_delta_g2_neg_pc_77(), pvk.delta_g2_neg_pc.ell_coeffs[77]);
        assert_eq!(get_delta_g2_neg_pc_78(), pvk.delta_g2_neg_pc.ell_coeffs[78]);
        assert_eq!(get_delta_g2_neg_pc_79(), pvk.delta_g2_neg_pc.ell_coeffs[79]);
        assert_eq!(get_delta_g2_neg_pc_80(), pvk.delta_g2_neg_pc.ell_coeffs[80]);
        assert_eq!(get_delta_g2_neg_pc_81(), pvk.delta_g2_neg_pc.ell_coeffs[81]);
        assert_eq!(get_delta_g2_neg_pc_82(), pvk.delta_g2_neg_pc.ell_coeffs[82]);
        assert_eq!(get_delta_g2_neg_pc_83(), pvk.delta_g2_neg_pc.ell_coeffs[83]);
        assert_eq!(get_delta_g2_neg_pc_84(), pvk.delta_g2_neg_pc.ell_coeffs[84]);
        assert_eq!(get_delta_g2_neg_pc_85(), pvk.delta_g2_neg_pc.ell_coeffs[85]);
        assert_eq!(get_delta_g2_neg_pc_86(), pvk.delta_g2_neg_pc.ell_coeffs[86]);
        assert_eq!(get_delta_g2_neg_pc_87(), pvk.delta_g2_neg_pc.ell_coeffs[87]);
        assert_eq!(get_delta_g2_neg_pc_88(), pvk.delta_g2_neg_pc.ell_coeffs[88]);
        assert_eq!(get_delta_g2_neg_pc_89(), pvk.delta_g2_neg_pc.ell_coeffs[89]);
        assert_eq!(get_delta_g2_neg_pc_90(), pvk.delta_g2_neg_pc.ell_coeffs[90]);
    }

    #[test]
    fn miller_loop_offchain() -> Result<()> {
        // First, run the library function, print i_order et cetera.
        // We'll then compare the f value to the result of
        // the 2nd part (instruction implementation)
        let pvk_unprepped = get_pvk_from_bytes_254()?;
        let pvk = prepare_verifying_key(&pvk_unprepped);
        let proof = get_proof_from_bytes_254()?;
        // trn proof into bytes, compare
        let public_inputs = get_public_inputs_from_bytes_254()?;
        let prepared_inputs = prepare_inputs(&pvk, &public_inputs).unwrap();
        // uses millerloop implementation for bn254. Prints inside library.
        println!("Prepared inputs done");
        let miller_output =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::miller_loop(
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
        let f = miller_output;
        println!("f (library ): {:?}", f);

        // This part mocks the on-chain behavior off-chain:
        // State variable, r/w with the state struct.
        // Ix_order hardcoded. pvk, inputs/proof bytes r ::file
        // passing proof, inputs as bytes. (simulating data ix)
        // processor handles rest..

        let mock_account = [0; 3900];
        // test assumes that unpack is working
        let mut account_data = ML254Bytes::unpack(&mock_account).unwrap();

        // p2: handled in preprocessor: (prepared_inputs) -- ix 0
        // would be reading g_ic ...
        let into_test: ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters> =
            prepared_inputs.into();

        parse_fp256_to_bytes(
            into_test.x, // prepared_inputs.into_affine().x,
            &mut account_data.p_2_x_range,
        );
        parse_fp256_to_bytes(
            into_test.y, // prepared_inputs.into_affine().y,
            &mut account_data.p_2_y_range,
        );

        // Initing Proof bytes. Normally, these would be parsed in ix 0/1/2.
        // p1,p3 -- ix 1
        let proof_a_bytes = [
            69, 130, 7, 152, 173, 46, 198, 166, 181, 14, 22, 145, 185, 13, 203, 6, 137, 135, 214,
            126, 20, 88, 220, 3, 105, 33, 77, 120, 104, 159, 197, 32, 103, 123, 208, 55, 205, 101,
            80, 10, 180, 216, 217, 177, 14, 196, 164, 108, 249, 131, 207, 100, 192, 194, 74, 200,
            16, 192, 219, 4, 161, 93, 141, 15, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let proof_c_bytes = [
            187, 25, 7, 191, 235, 134, 124, 225, 209, 30, 66, 253, 195, 106, 121, 199, 99, 89, 183,
            179, 203, 75, 203, 177, 10, 104, 149, 210, 7, 63, 131, 24, 197, 174, 244, 228, 219,
            108, 228, 249, 71, 84, 209, 158, 244, 104, 179, 116, 118, 246, 158, 237, 87, 197, 134,
            24, 140, 103, 27, 203, 108, 245, 42, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        // coeffs1 -- ix 2
        let proof_b_bytes = [
            32, 255, 161, 204, 195, 74, 249, 196, 139, 193, 49, 109, 241, 230, 145, 100, 91, 134,
            188, 102, 83, 190, 140, 12, 84, 21, 107, 182, 225, 139, 23, 16, 64, 152, 20, 230, 245,
            127, 35, 113, 194, 4, 161, 242, 179, 131, 135, 66, 70, 179, 115, 118, 237, 158, 246,
            97, 35, 85, 25, 13, 30, 21, 183, 18, 254, 194, 12, 96, 211, 37, 160, 170, 7, 173, 208,
            52, 22, 169, 113, 149, 235, 85, 90, 20, 14, 171, 22, 22, 247, 254, 71, 236, 207, 18,
            90, 29, 236, 211, 193, 206, 15, 107, 89, 218, 207, 62, 76, 75, 88, 71, 9, 45, 114, 212,
            43, 127, 163, 183, 245, 213, 117, 216, 64, 56, 26, 102, 15, 37, 1, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        // Testing integrity of the hardcoded bytes:
        // Beware that the asserts only account for the first 64/128 bytes.
        let contents = fs::read_to_string("./tests/proof_bytes_254.txt")
            .expect("Something went wrong reading the file");
        let v: Value = serde_json::from_str(&contents)?;

        let mut bytes: Vec<u8> = Vec::new();
        for i in 0..2 {
            for i in v["pi_a"][i].as_str().unwrap().split(',') {
                bytes.push((*i).parse::<u8>().unwrap());
            }
        }
        assert_eq!(
            bytes,
            proof_a_bytes[0..64], // 64 without 1,0,... 32 more
            "parsed proof.a != hardcoded proof.a"
        );

        let mut bytes: Vec<u8> = Vec::new();
        for i in 0..2 {
            for j in 0..2 {
                for z in v["pi_b"][i][j].as_str().unwrap().split(',') {
                    bytes.push((*z).parse::<u8>().unwrap());
                }
            }
        }
        assert_eq!(
            bytes,
            proof_b_bytes[0..128],
            "parsed proof.b != hardcoded proof.b"
        );
        let mut bytes: Vec<u8> = Vec::new();
        for i in 0..2 {
            for i in v["pi_c"][i].as_str().unwrap().split(',') {
                bytes.push((*i).parse::<u8>().unwrap());
            }
        }
        assert_eq!(
            bytes,
            proof_c_bytes[0..64],
            "parsed proof.c != hardcoded proof.c"
        );

        // Creating coeffs with library to test against.
        // pairs = [(p1,coeffs0),(p2, coeffs1),(p3, coeffs2)]
        // pairs = [(p1, ((1,2,3), (1,2,3)....)), (p2,...)] x91
        let mut pairs = vec![];

        let tuple0: (
            ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>,
            ark_ec::bn::G2Prepared<ark_bn254::Parameters>,
        ) = (proof.a.into(), proof.b.into());

        let tuple1: (
            ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>,
            ark_ec::bn::G2Prepared<ark_bn254::Parameters>,
        ) = (
            (prepared_inputs).into_affine().into(),
            pvk.gamma_g2_neg_pc.clone(),
        );
        let tuple2: (
            ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>,
            ark_ec::bn::G2Prepared<ark_bn254::Parameters>,
        ) = (proof.c.into(), pvk.delta_g2_neg_pc.clone());

        if !tuple0.1.is_zero() {
            pairs.push((tuple0.0, tuple0.1.ell_coeffs.iter()));
        }
        if !tuple1.1.is_zero() {
            pairs.push((tuple1.0, tuple1.1.ell_coeffs.iter()));
        }
        if !tuple2.1.is_zero() {
            pairs.push((tuple2.0, tuple2.1.ell_coeffs.iter()));
        }
        assert_eq!(pairs[0].1.len(), 91, "coeffs_0.len() != 91");
        assert_eq!(pairs[1].1.len(), 91, "coeffs_1.len() != 91");
        assert_eq!(pairs[2].1.len(), 91, "coeffs_2.len() != 91");

        // Storing copy for further coeff integrity checks (since pairs gets used up).
        let mut pairs_copy = pairs.clone();
        // Calling and asserting native implementation as a whole.
        let f_native_0 = miller_loop_native();
        assert_eq!(f_native_0, f, "native0 milleroutput != lib milleroutput");

        // Replicating native implementation step-by-step, unit-testing ix.
        // The native implementation uses the coeffs that we prepared with the library above
        const ATE_LOOP_COUNT: &'static [i8] = &[
            0, 0, 0, 1, 0, 1, 0, -1, 0, 0, 1, -1, 0, 0, 1, 0, 0, 1, 1, 0, -1, 0, 0, 1, 0, -1, 0, 0,
            0, 0, 1, 1, 1, 0, 0, -1, 0, 0, 1, 0, 0, 0, 0, 0, -1, 0, 0, 1, 1, 0, 0, -1, 0, 0, 0, 1,
            1, 0, -1, 0, 0, 1, 0, 1, 1,
        ];
        let mut f_native =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::one();
        for i in (1..ATE_LOOP_COUNT.len()).rev() {
            if i != ATE_LOOP_COUNT.len() - 1 {
                f_native.square_in_place();
            }
            for (p, ref mut coeffs) in &mut pairs {
                ell(&mut f_native, coeffs.next().unwrap(), &p);
            }
            let bit = ATE_LOOP_COUNT[i - 1];
            match bit {
                1 => {
                    for &mut (p, ref mut coeffs) in &mut pairs {
                        ell(&mut f_native, coeffs.next().unwrap(), &p);
                    }
                }
                -1 => {
                    for &mut (p, ref mut coeffs) in &mut pairs {
                        ell(&mut f_native, coeffs.next().unwrap(), &p);
                    }
                }
                _ => continue,
            };
        }
        for &mut (p, ref mut coeffs) in &mut pairs {
            ell(&mut f_native, coeffs.next().unwrap(), &p);
        }
        for &mut (p, ref mut coeffs) in &mut pairs {
            ell(&mut f_native, coeffs.next().unwrap(), &p);
        }
        // Asserting inline implementation.
        assert_eq!(f_native, f, "native milleroutput != lib milleroutput");

        // Testing manual implementation.
        // Replicates rpc calls.
        for i in 0..ML_IX_ORDER.len() {
            // Checking integrity of coeffs w/ the current computed coeff in ix.
            if ML_IX_ORDER[i] == 20 {
                // d1 always should have clean coeffs already.
                let c0 = parse_quad_from_bytes(&account_data.coeff_0_range);
                let c1 = parse_quad_from_bytes(&account_data.coeff_1_range);
                let c2 = parse_quad_from_bytes(&account_data.coeff_2_range);
                let current_coeff_ref = pairs_copy[0].1.next().unwrap();
                // temp test: CORRECTS false coeffs in 20!
                assert_eq!(c2, current_coeff_ref.2, "pairs[0].2 - i20 -{:?}", i);
                assert_eq!(c1, current_coeff_ref.1, "pairs[0].1 - i20 -{:?}", i);
                assert_eq!(c0, current_coeff_ref.0, "pairs[0].0 - i20 -{:?}", i);
            }
            if ML_IX_ORDER[i] == 21 {
                // d1 always should have clean coeffs already.
                let c0 = parse_quad_from_bytes(&account_data.coeff_0_range);
                let c1 = parse_quad_from_bytes(&account_data.coeff_1_range);
                let c2 = parse_quad_from_bytes(&account_data.coeff_2_range);
                let current_coeff_ref = pairs_copy[1].1.next().unwrap();
                assert_eq!(c0, current_coeff_ref.0, "pairs[1].0 - i21 -{:?}", i);
                assert_eq!(c1, current_coeff_ref.1, "pairs[1].1 - i21 -{:?}", i);
                assert_eq!(c2, current_coeff_ref.2, "pairs[1].2 - i21 -{:?}", i);
            }
            if ML_IX_ORDER[i] == 22 {
                // d1 always should have clean coeffs already.
                let c0 = parse_quad_from_bytes(&account_data.coeff_0_range);
                let c1 = parse_quad_from_bytes(&account_data.coeff_1_range);
                let c2 = parse_quad_from_bytes(&account_data.coeff_2_range);
                let current_coeff_ref = pairs_copy[2].1.next().unwrap();
                assert_eq!(c0, current_coeff_ref.0, "pairs[2].0 - i22 -{:?}", i);
                assert_eq!(c1, current_coeff_ref.1, "pairs[2].1 - i22 -{:?}", i);
                assert_eq!(c2, current_coeff_ref.2, "pairs[2].2 - i22 -{:?}", i);
            }
            _process_instruction(
                ML_IX_ORDER[i],
                &mut account_data,
                // &proof_b_bytes.to_vec(),
                // &proof_a_bytes.to_vec(),
                // &proof_c_bytes.to_vec(),
            );
            if ML_IX_ORDER[i] == 2 {
                println!("init state f_range: {:?}", account_data.f_range);
                println!("init state P1x: {:?}", account_data.p_1_x_range);
                println!("init state P1y: {:?}", account_data.p_1_y_range);

                println!("init state P2x: {:?}", account_data.p_2_x_range);
                println!("init state P2y: {:?}", account_data.p_2_y_range);

                println!("init state P3x: {:?}", account_data.p_3_x_range);
                println!("init state P3y: {:?}", account_data.p_3_y_range);

                println!("init state PROOFB: {:?}", account_data.proof_b);
            }
            if i == 66 {}
        }
        // Replicating last ix (255, reading f).
        let manual_f = parse_f_from_bytes(&account_data.f_range.to_vec());

        // Check integrity of the whole ix implementation against the library implementation.
        println!("f result in bytes: {:?}", &account_data.f_range.to_vec());
        println!("f result: {:?}", manual_f);
        assert_eq!(manual_f, f, "man milleroutput != lib milleroutput");
        println!("OK!");
        // TODO: add pack for pack testing.

        Ok(())
    }

    // Native implementation of library functions. Used as assert_reference.
    fn miller_loop_native() -> QuadExtField<Fp12ParamsWrapper<Fq12Parameters>> {
        let pvk_unprepped = get_pvk_from_bytes_254().unwrap();
        let pvk = prepare_verifying_key(&pvk_unprepped);
        let proof = get_proof_from_bytes_254().unwrap();
        // trn proof into bytes, compare
        let public_inputs = get_public_inputs_from_bytes_254().unwrap();
        let prepared_inputs = prepare_inputs(&pvk, &public_inputs).unwrap();
        let mut pairs = vec![];
        let tuple0: (
            ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>,
            ark_ec::bn::G2Prepared<ark_bn254::Parameters>,
        ) = (proof.a.into(), proof.b.into());
        let tuple1: (
            ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>,
            ark_ec::bn::G2Prepared<ark_bn254::Parameters>,
        ) = (
            (prepared_inputs).into_affine().into(),
            pvk.gamma_g2_neg_pc.clone(),
        );
        let tuple2: (
            ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>, // ark_ec::bn::G1Prepared<ark_bn254::Parameters>,
            ark_ec::bn::G2Prepared<ark_bn254::Parameters>,
        ) = (proof.c.into(), pvk.delta_g2_neg_pc.clone());
        pairs.push((tuple0.0, tuple0.1.ell_coeffs.iter()));
        pairs.push((tuple1.0, tuple1.1.ell_coeffs.iter()));
        pairs.push((tuple2.0, tuple2.1.ell_coeffs.iter()));

        // Const taken from the library.
        const ATE_LOOP_COUNT: &'static [i8] = &[
            0, 0, 0, 1, 0, 1, 0, -1, 0, 0, 1, -1, 0, 0, 1, 0, 0, 1, 1, 0, -1, 0, 0, 1, 0, -1, 0, 0,
            0, 0, 1, 1, 1, 0, 0, -1, 0, 0, 1, 0, 0, 0, 0, 0, -1, 0, 0, 1, 1, 0, 0, -1, 0, 0, 0, 1,
            1, 0, -1, 0, 0, 1, 0, 1, 1,
        ];
        let mut f =
            <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::one();
        for i in (1..ATE_LOOP_COUNT.len()).rev() {
            if i != ATE_LOOP_COUNT.len() - 1 {
                f.square_in_place();
            }
            for (p, ref mut coeffs) in &mut pairs {
                ell(&mut f, coeffs.next().unwrap(), &p);
            }
            let bit = ATE_LOOP_COUNT[i - 1];
            match bit {
                1 => {
                    for &mut (p, ref mut coeffs) in &mut pairs {
                        ell(&mut f, coeffs.next().unwrap(), &p);
                    }
                }
                -1 => {
                    for &mut (p, ref mut coeffs) in &mut pairs {
                        ell(&mut f, coeffs.next().unwrap(), &p);
                    }
                }
                _ => continue,
            };
        }
        for &mut (p, ref mut coeffs) in &mut pairs {
            ell(&mut f, coeffs.next().unwrap(), &p);
        }
        for &mut (p, ref mut coeffs) in &mut pairs {
            ell(&mut f, coeffs.next().unwrap(), &p);
        }
        f
    }

    fn ell(
        f: &mut QuadExtField<Fp12ParamsWrapper<Fq12Parameters>>,
        coeffs: &(
            QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
            QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
            QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
        ),
        p: &ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters>,
    ) {
        let mut c0 = coeffs.0;
        let mut c1 = coeffs.1;
        let c2 = coeffs.2;

        c0.mul_assign_by_fp(&p.y);
        c1.mul_assign_by_fp(&p.x);
        mul_by_034(f, &c0, &c1, &c2);
        // match ark_bn254::Parameters::TWIST_TYPE {
        //     TwistType::M => {
        //         // Shouldn't be M twist in this implementation.
        //         assert_eq!(true, false, "Twist M");
        //     }
        //     TwistType::D => {
        //         c0.mul_assign_by_fp(&p.y);
        //         c1.mul_assign_by_fp(&p.x);
        //         mul_by_034(&mut f, &c0, &c1, &c2);
        //         // println!("7,");
        //         // println!("8,");
        //         // println!("9,");
        //         // println!("10,");
        //     }
        // }
    }
    pub fn mul_by_034(
        f: &mut QuadExtField<Fp12ParamsWrapper<Fq12Parameters>>,
        c0: &QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
        c3: &QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
        c4: &QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>,
    ) {
        // D2
        let a0 = f.c0.c0 * c0;
        let a1 = f.c0.c1 * c0;
        let a2 = f.c0.c2 * c0;
        let a = Fp6::new(a0, a1, a2);
        // D3
        let mut b = f.c1;
        b.mul_by_01(&c3, &c4);
        // D4
        let c0 = *c0 + c3;
        let c1 = c4;
        let mut e = f.c0 + &f.c1;
        e.mul_by_01(&c0, &c1);
        // D5
        f.c1 = e - &(a + &b);
        f.c0 = a + <ark_bn254::fq12::Fq12Parameters as Fp12Parameters>::mul_fp6_by_nonresidue(&b);
    }
}
