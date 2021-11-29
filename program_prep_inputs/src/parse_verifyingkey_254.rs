// use ark_bn254;
// use ark_ec;
// use ark_ec::AffineCurve;
// use ark_ed_on_bn254;
// use ark_ff::biginteger::{BigInteger256, BigInteger384};
// use ark_ff::bytes::{FromBytes, ToBytes};
// use ark_ff::QuadExtField;
// use ark_ff::{Fp256, Fp384};
// use ark_groth16::prepare_verifying_key;
// use ark_groth16::{prepare_inputs, verify_proof, verify_proof_with_prepared_inputs};
// use serde_json::Value::String;
// use serde_json::{Result, Value};
// use std::fs;

// pub fn get_pvk_from_bytes_254(
// ) -> Result<ark_groth16::data_structures::VerifyingKey<ark_ec::models::bn::Bn<ark_bn254::Parameters>>>
// {
//     let contents = fs::read_to_string("./src/verifyingkey_254_bytes.txt")
//         .expect("Something went wrong reading the file");
//     let v: Value = serde_json::from_str(&contents)?;

//     let mut a_g1_bigints = Vec::new();
//     for i in 0..2 {
//         let mut bytes: Vec<u8> = Vec::new();
//         for i in v["vk_alpha_1"][i].as_str().unwrap().split(',') {
//             bytes.push((*i).parse::<u8>().unwrap());
//         }
//         a_g1_bigints.push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
//     }
//     let alpha_g1_bigints = ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
//         a_g1_bigints[0],
//         a_g1_bigints[1],
//         false,
//     );
//     println!(" alpha_g1 {}", alpha_g1_bigints);

//     let mut b_g2_bigints = Vec::new();
//     for i in 0..2 {
//         for j in 0..2 {
//             let mut bytes: Vec<u8> = Vec::new();
//             for z in v["vk_beta_2"][i][j].as_str().unwrap().split(',') {
//                 bytes.push((*z).parse::<u8>().unwrap());
//             }
//             b_g2_bigints
//                 .push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
//         }
//     }

//     let beta_g2 = ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters>::new(
//         QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
//             b_g2_bigints[0],
//             b_g2_bigints[1],
//         ),
//         QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
//             b_g2_bigints[2],
//             b_g2_bigints[3],
//         ),
//         false,
//     );
//     for (i, _) in b_g2_bigints.iter().enumerate() {
//         //println!("b_g2 {}", b_g2_bigints[i]);
//     }

//     let mut delta_g2_bytes = Vec::new();
//     for i in 0..2 {
//         for j in 0..2 {
//             let mut bytes: Vec<u8> = Vec::new();
//             for z in v["vk_delta_2"][i][j].as_str().unwrap().split(',') {
//                 bytes.push((*z).parse::<u8>().unwrap());
//             }
//             delta_g2_bytes
//                 .push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
//         }
//     }

//     let delta_g2 = ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters>::new(
//         QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
//             delta_g2_bytes[0],
//             delta_g2_bytes[1],
//         ),
//         QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
//             delta_g2_bytes[2],
//             delta_g2_bytes[3],
//         ),
//         false,
//     );

//     for (i, _) in delta_g2_bytes.iter().enumerate() {}

//     let mut gamma_g2_bytes = Vec::new();
//     for i in 0..2 {
//         for j in 0..2 {
//             let mut bytes: Vec<u8> = Vec::new();
//             for z in v["vk_gamma_2"][i][j].as_str().unwrap().split(',') {
//                 bytes.push((*z).parse::<u8>().unwrap());
//             }
//             gamma_g2_bytes
//                 .push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
//         }
//     }
//     let gamma_g2 = ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters>::new(
//         QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
//             gamma_g2_bytes[0],
//             gamma_g2_bytes[1],
//         ),
//         QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
//             gamma_g2_bytes[2],
//             gamma_g2_bytes[3],
//         ),
//         false,
//     );

//     for (i, _) in gamma_g2_bytes.iter().enumerate() {
//         //println!("gamma_g2 {}", gamma_g2_bytes[i]);
//     }

//     let mut gamma_abc_g1_bigints_bytes = Vec::new();

//     for i in 0..8 {
//         let mut g1_bytes = Vec::new();
//         for u in 0..2 {
//             let mut bytes: Vec<u8> = Vec::new();
//             for z in v["IC"][i][u].as_str().unwrap().split(',') {
//                 bytes.push((*z).parse::<u8>().unwrap());
//             }
//             g1_bytes.push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
//         }
//         gamma_abc_g1_bigints_bytes.push(
//             ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
//                 g1_bytes[0],
//                 g1_bytes[1],
//                 false,
//             ),
//         );
//     }

//     Ok(
//         ark_groth16::data_structures::VerifyingKey::<ark_ec::models::bn::Bn<ark_bn254::Parameters>> {
//             alpha_g1: alpha_g1_bigints,
//             beta_g2: beta_g2,
//             gamma_g2: gamma_g2,
//             delta_g2: delta_g2,
//             gamma_abc_g1: gamma_abc_g1_bigints_bytes,
//         },
//     )
// }
