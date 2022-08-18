
use std::fs;
use std::io::Write;

use ark_ff::biginteger::{BigInteger384, BigInteger256};
use serde_json::Value::String;
use ark_bls12_381;
use ark_ec;
use ark_ff::bytes::{ToBytes, FromBytes};
use ark_ff::QuadExtField;
use ark_ff::{Fp256, Fp384};
use ark_groth16::prepare_verifying_key;
use ark_ed_on_bls12_381;
use ark_groth16::{verify_proof, prepare_inputs, verify_proof_with_prepared_inputs};

use ark_bn254;
use ark_ed_on_bn254;
use ark_ec::AffineCurve;
use serde_json::{Result, Value};

// mod hard_coded_verifying_key_pvk_254;
// use crate::hard_coded_verifying_key_pvk_254::*;

fn get_pvk_from_bytes_381() -> Result<ark_groth16::data_structures::VerifyingKey::<ark_ec::models::bls12::Bls12<ark_bls12_381::Parameters>>>{
    let contents = fs::read_to_string("verification_key_bytes.txt")
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
        a_g1_bigints.push(<Fp384::<ark_bls12_381::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
    }
    let alpha_g1_bigints =  ark_ec::models::bls12::g1::G1Affine::<ark_bls12_381::Parameters>::new(
        a_g1_bigints[0],
        a_g1_bigints[1],
        false
    );
    //println!(" alpha_g1 {}", alpha_g1_bigints);

    let mut b_g2_bigints = Vec::new();
    //println!("{}",  v["vk_beta_2"]);
    for i in 0..2 {
        for j in 0..2 {
            let mut bytes: Vec<u8> = Vec::new();
            for z in  v["vk_beta_2"][i][j].as_str().unwrap().split(',') {
                bytes.push((*z).parse::<u8>().unwrap());
            }
            b_g2_bigints.push(<Fp384::<ark_bls12_381::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
        }
    }

    let beta_g2 = ark_ec::models::bls12::g2::G2Affine::<ark_bls12_381::Parameters>::new(
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
			b_g2_bigints[0],
            b_g2_bigints[1],
        ),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
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
            delta_g2_bytes.push(<Fp384::<ark_bls12_381::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
        }
    }

    let delta_g2 = ark_ec::models::bls12::g2::G2Affine::<ark_bls12_381::Parameters>::new(
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
			delta_g2_bytes[0],
            delta_g2_bytes[1],
        ),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
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
            gamma_g2_bytes.push(<Fp384::<ark_bls12_381::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
        }
    }
    let gamma_g2 = ark_ec::models::bls12::g2::G2Affine::<ark_bls12_381::Parameters>::new(
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
			gamma_g2_bytes[0],
            gamma_g2_bytes[1],
        ),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
            gamma_g2_bytes[2],
            gamma_g2_bytes[3],
		),
		false
	);

    for (i, _) in gamma_g2_bytes.iter().enumerate() {
        //println!("gamma_g2 {}", gamma_g2_bytes[i]);
    }


    let mut gamma_abc_g1_bigints_bytes = Vec::new();
    //println!("{}",  v["vk_alphabeta_12"]);
    // for i in 0..2 {
    //
    //     for j in 0..3 {
    //         let mut g1_bytes = Vec::new();
    //         for u in 0..2 {
    //
    //             let mut bytes: Vec<u8> = Vec::new();
    //             for z in  v["vk_alphabeta_12"][i][j][u].as_str().unwrap().split(',') {
    //                 bytes.push((*z).parse::<u8>().unwrap());
    //             }
    //             g1_bytes.push(<Fp384::<ark_bls12_381::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
    //         }
    //         gamma_abc_g1_bigints_bytes.push(ark_ec::models::bls12::g1::G1Affine::<ark_bls12_381::Parameters>::new(
    //             g1_bytes[0],
    //             g1_bytes[1],
    //             false
    //         ));
    //     }
    // }
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
                while bytes.len() != 48 {
                    //bytes.insert(0, 0u8);
                    bytes.push(0u8);

                }
                g1_bytes.push(<Fp384::<ark_bls12_381::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
            }
            gamma_abc_g1_bigints_bytes.push(ark_ec::models::bls12::g1::G1Affine::<ark_bls12_381::Parameters>::new(
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

    Ok(ark_groth16::data_structures::VerifyingKey::<ark_ec::models::bls12::Bls12<ark_bls12_381::Parameters>> {
    alpha_g1: alpha_g1_bigints,
    beta_g2: beta_g2,
    gamma_g2: gamma_g2,
    delta_g2: delta_g2,
    gamma_abc_g1:gamma_abc_g1_bigints_bytes
    })
}

fn get_pvk_from_bytes_254() -> Result<ark_groth16::data_structures::VerifyingKey::<ark_ec::models::bn::Bn<ark_bn254::Parameters>>>{
    let contents = fs::read_to_string("verification_key_bytes_mainnet.txt")
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
    //println!(" alpha_g1 {}", alpha_g1_bigints);

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
    //println!("{}",  v["vk_alphabeta_12"]);
    // for i in 0..2 {
    //
    //     for j in 0..3 {
    //         let mut g1_bytes = Vec::new();
    //         for u in 0..2 {
    //
    //             let mut bytes: Vec<u8> = Vec::new();
    //             for z in  v["vk_alphabeta_12"][i][j][u].as_str().unwrap().split(',') {
    //                 bytes.push((*z).parse::<u8>().unwrap());
    //             }
    //             g1_bytes.push(<Fp384::<ark_bls12_381::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
    //         }
    //         gamma_abc_g1_bigints_bytes.push(ark_ec::models::bls12::g1::G1Affine::<ark_bls12_381::Parameters>::new(
    //             g1_bytes[0],
    //             g1_bytes[1],
    //             false
    //         ));
    //     }
    // }
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


fn get_proof_from_bytes() -> Result<ark_groth16::data_structures::Proof::<ark_ec::models::bls12::Bls12<ark_bls12_381::Parameters>>>{
    let contents = fs::read_to_string("proof_bytes.txt")
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
        a_g1_bigints.push(<Fp384::<ark_bls12_381::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
    }
    //println!("{}", a_g1_bigints[0]);
//use ark_ff::fields::models::Fp384;
    //println!("{}", ark_ff::Fp384::<ark_bls12_381::FqParameters>::new(a_g1_bigints[0]));
    //println!("{:?}",hex::decode("0B85DC05397EAC823D25C7C4682A0BE95141A33334C65A857D2491680F972BA4A5D50D9FB71A87E0594E32C02B9484E4").unwrap());
    let a_g1 =  ark_ec::models::bls12::g1::G1Affine::<ark_bls12_381::Parameters>::new(
        //ark_ff::Fp384::<ark_bls12_381::FqParameters>::new(a_g1_bigints[0]),
        //ark_ff::Fp384::<ark_bls12_381::FqParameters>::new(a_g1_bigints[1]),
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
            b_g2_bigints.push(<Fp384::<ark_bls12_381::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
        }
    }
    let b_g2 = ark_ec::models::bls12::g2::G2Affine::<ark_bls12_381::Parameters>::new(
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
			b_g2_bigints[0],
            b_g2_bigints[1],
        ),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
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
        c_g1_bigints.push(<Fp384::<ark_bls12_381::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
    }
    let c_g1 =  ark_ec::models::bls12::g1::G1Affine::<ark_bls12_381::Parameters>::new(
        c_g1_bigints[0],
        c_g1_bigints[1],
        false
    );

    //println!("{:?}", delta_g2);


    Ok(ark_groth16::data_structures::Proof::<ark_ec::models::bls12::Bls12<ark_bls12_381::Parameters>> {
    a: a_g1,
    b: b_g2,
    c: c_g1
    })
}

fn get_proof_from_bytes_254() -> Result<ark_groth16::data_structures::Proof::<ark_ec::models::bn::Bn<ark_bn254::Parameters>>>{
    let contents = fs::read_to_string("proof_bytes_254.txt")
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

fn get_public_inputs_from_bytes() -> Result<Vec<Fp256::<ark_ed_on_bls12_381::FqParameters>>> {
    let contents = fs::read_to_string("public_inputs_bytes.txt")
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
        res.push(<Fp256::<ark_ed_on_bls12_381::FqParameters> as FromBytes>::read(&bytes[..]).unwrap())
    }

    //println!("{:?}", public_inputs);


    Ok(res)
}

fn get_public_inputs_from_bytes_254() -> Result<Vec<Fp256::<ark_ed_on_bn254::FqParameters>>> {
    let contents = fs::read_to_string("public_inputs_254_bytes.txt")
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

    //println!("{:?}", public_inputs);


    Ok(res)
}

//ark_ec::models::bls12::g1::G1Affine::<ark_bls12_381::Parameters> ark_ec::short_weierstrass_jacobian::GroupProjective
fn get_hard_coded_prepared_inputs() -> ark_ec::short_weierstrass_jacobian::GroupProjective<ark_bls12_381::g1::Parameters> {
    let x: [u8;48] = [6 ,83 ,85 ,220 ,67 ,17 ,35 ,155 ,211 ,125 ,160 ,104 ,47 ,246 ,218 ,166 ,14 ,236 ,99 ,35 ,108 ,176 ,35 ,167 ,233 ,151 ,240 ,92 ,161 ,100 ,19 ,199 ,214 ,199 ,156 ,110 ,64 ,91 ,219 ,91 ,88 ,174 ,237 ,49 ,51 ,30 ,109 ,16];
    let y: [u8;48] = [240 ,245 ,17 ,17 ,228 ,245 ,218 ,183 ,20 ,63 ,252 ,27 ,247 ,58 ,221 ,113 ,233 ,107 ,47 ,102 ,228 ,186 ,47 ,13 ,46 ,250 ,125 ,240 ,88 ,21 ,171 ,113 ,9 ,223 ,13 ,184 ,186 ,6 ,207 ,56 ,186 ,214 ,195 ,81 ,209 ,223 ,153 ,17];
    let z: [u8;48] = [150 ,204 ,82 ,43 ,153 ,226 ,3 ,119 ,109 ,131 ,77 ,216 ,15 ,174 ,158 ,115 ,16 ,75 ,54 ,238 ,33 ,142 ,240 ,113 ,217 ,161 ,141 ,203 ,90 ,218 ,45 ,113 ,3 ,170 ,203 ,37 ,54 ,244 ,48 ,99 ,141 ,172 ,225 ,39 ,93 ,179 ,103 ,21];
    ark_ec::short_weierstrass_jacobian::GroupProjective::<ark_bls12_381::g1::Parameters>::new(
        <Fp384::<ark_bls12_381::FqParameters> as FromBytes>::read(&x[..]).unwrap(),
        <Fp384::<ark_bls12_381::FqParameters> as FromBytes>::read(&y[..]).unwrap(),
        <Fp384::<ark_bls12_381::FqParameters> as FromBytes>::read(&z[..]).unwrap()
    )
}
/*
fn test_hardcoded_verifyingkey() {
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

}*/
pub fn get_proof_from_bytes_test(
    proof_bytes: &Vec<u8>,
) -> ark_groth16::data_structures::Proof<ark_ec::models::bn::Bn<ark_bn254::Parameters>> {
    let proof_a = parse_x_group_affine_from_bytes(&proof_bytes[0..64].to_vec());
    let proof_b = parse_proof_b_from_bytes(&proof_bytes[64..192].to_vec());
    let proof_c = parse_x_group_affine_from_bytes(&proof_bytes[192..256].to_vec());
    let proof =
        ark_groth16::data_structures::Proof::<ark_ec::models::bn::Bn<ark_bn254::Parameters>> {
            a: proof_a,
            b: proof_b,
            c: proof_c,
        };
    proof
}

pub fn get_public_inputs_from_bytes_test(
    public_inputs_bytes: &Vec<u8>,
) -> Result<Vec<Fp256<ark_ed_on_bn254::FqParameters>>> {
    let mut res = Vec::new();
    for i in public_inputs_bytes.chunks(32) {
        //let current_input = &public_inputs_bytes[(i * 32)..((i * 32) + 32)];

        res.push(
            <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&i[..])
                .unwrap(),
        )
    }
    Ok(res)
}
pub fn parse_proof_b_from_bytes(
    range: &Vec<u8>,
) -> ark_ec::models::bn::g2::G2Affine<ark_bn254::Parameters> {
    ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters>::new(
        parse_quad_from_bytes(&range[..64].to_vec()),
        parse_quad_from_bytes(&range[64..].to_vec()),
        false,
    )
}

pub fn parse_quad_from_bytes(
    range: &Vec<u8>,
) -> ark_ff::QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>> {
    let start = 0;
    let end = 64;
    let iter = start + 32;

    let quad = QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
        <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[start..iter]).unwrap(),
        <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&range[iter..end]).unwrap(),
    );
    quad
}

pub fn read_test_data() -> Vec<u8> {
    let ix_data_file = fs::read_to_string("deposit_0_1_sol.txt")
        .expect("Something went wrong reading the file");
    let ix_data_json: Value = serde_json::from_str(&ix_data_file).unwrap();
    let mut ix_data = Vec::new();
    for i in ix_data_json["bytes"][0].as_str().unwrap().split(',') {
        let j = (*i).parse::<u8>();
        match j {
            Ok(x) => (ix_data.push(x)),
            Err(_e) => (),
        }
    }
    //println!("{:?}", ix_data);
    ix_data
}
pub fn parse_x_group_affine_from_bytes(
    account: &Vec<u8>,
) -> ark_ec::short_weierstrass_jacobian::GroupAffine<ark_bn254::g1::Parameters> {
    let x = ark_ec::short_weierstrass_jacobian::GroupAffine::<ark_bn254::g1::Parameters>::new(
        <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&account[0..32]).unwrap(),
        <Fp256<ark_bn254::FqParameters> as FromBytes>::read(&account[32..64]).unwrap(),
        false,
    );
    x
}

pub fn parse_f_to_bytes(
    f: <ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk,
    range: &mut Vec<u8>,
) {
    let mut iter = 0;
    for i in 0..2_u8 {
        for j in 0..3_u8 {
            for z in 0..2_u8 {
                let tmp = iter;
                iter += 32;
                if i == 0 {
                    if j == 0 && z == 0 {
                        <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
                            &f.c0.c0.c0,
                            &mut range[tmp..iter],
                        )
                        .unwrap();
                    } else if j == 1 && z == 0 {
                        <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
                            &f.c0.c1.c0,
                            &mut range[tmp..iter],
                        )
                        .unwrap();
                    } else if j == 2 && z == 0 {
                        <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
                            &f.c0.c2.c0,
                            &mut range[tmp..iter],
                        )
                        .unwrap();
                    } else if j == 0 && z == 1 {
                        <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
                            &f.c0.c0.c1,
                            &mut range[tmp..iter],
                        )
                        .unwrap();
                    } else if j == 1 && z == 1 {
                        <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
                            &f.c0.c1.c1,
                            &mut range[tmp..iter],
                        )
                        .unwrap();
                    } else if j == 2 && z == 1 {
                        <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
                            &f.c0.c2.c1,
                            &mut range[tmp..iter],
                        )
                        .unwrap();
                    }
                } else if i == 1 {
                    if j == 0 && z == 0 {
                        <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
                            &f.c1.c0.c0,
                            &mut range[tmp..iter],
                        )
                        .unwrap();
                    } else if j == 1 && z == 0 {
                        <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
                            &f.c1.c1.c0,
                            &mut range[tmp..iter],
                        )
                        .unwrap();
                    } else if j == 2 && z == 0 {
                        <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
                            &f.c1.c2.c0,
                            &mut range[tmp..iter],
                        )
                        .unwrap();
                    } else if j == 0 && z == 1 {
                        <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
                            &f.c1.c0.c1,
                            &mut range[tmp..iter],
                        )
                        .unwrap();
                    } else if j == 1 && z == 1 {
                        <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
                            &f.c1.c1.c1,
                            &mut range[tmp..iter],
                        )
                        .unwrap();
                    } else if j == 2 && z == 1 {
                        <Fp256<ark_bn254::FqParameters> as ToBytes>::write(
                            &f.c1.c2.c1,
                            &mut range[tmp..iter],
                        )
                        .unwrap();
                    }
                }
            }
        }
    }
}

fn main() -> Result<()>{

    let curve = "bn";
    let verify:bool = false;

    if curve == "bls" {
        let pvk_unprepped = get_pvk_from_bytes_381()?;
        let pvk = prepare_verifying_key(&pvk_unprepped);
        let proof = get_proof_from_bytes()?;
        //println!("proof {:?}", proof);
        //public_inputs_bytes.txt
        let public_inputs = get_public_inputs_from_bytes()?;
        for i in 0..7 {
            println!("public_inputs {}", public_inputs[i]);

        }

        let res = verify_proof(
            &pvk,
            &proof,
            &public_inputs
        );
        println!("res {:?}", res);
    } else {

        let pvk_unprepped = get_pvk_from_bytes_254()?;
        let pvk = prepare_verifying_key(&pvk_unprepped);
        if verify {
            let ix_data = read_test_data();
            // Pick the data we need from the test file. 9.. bc of input structure
            let public_inputs_bytes = ix_data[9..233].to_vec(); // 224 length
            let proof_bytes = ix_data[233..489].to_vec(); // 256 length
            // let pvk_unprepped = get_vk_from_file().unwrap(); //?// TODO: check if same vk
            // let pvk = prepare_verifying_key(&pvk_unprepped);
            let proof_test = get_proof_from_bytes_test(&proof_bytes);
            let public_inputs_test = get_public_inputs_from_bytes_test(&public_inputs_bytes).unwrap(); // TODO: debug
            let mut checking_bytes = vec![0u8;384];
            parse_f_to_bytes(pvk.alpha_g1_beta_g2, &mut checking_bytes);
            // println!("pvk {:?}",checking_bytes );
            // panic!("");
            let proof = get_proof_from_bytes_254()?;
            println!("proof {:?}", proof);
            //public_inputs_bytes.txt
            let public_inputs = get_public_inputs_from_bytes_254()?;
            // for i in 0..7 {
            //     println!("public_inputs {}", public_inputs[i]);
            //
            // }
            assert_eq!(proof, proof_test);
            assert_eq!(public_inputs, public_inputs_test);

            let res = verify_proof(
                &pvk,
                &proof_test,
                &public_inputs_test[..]
            );

        }

            //println!("res {:?}", res);
    let mut file = fs::File::create("prepared_verifying_key.txt").unwrap();

    // Write a &str in the file (ignoring the result).
    writeln!(&mut file,"{:?}", pvk).unwrap();
    let mut alpha_g1_beta_g2_bytes = vec![0u8;384];
    parse_f_to_bytes(pvk.alpha_g1_beta_g2, &mut alpha_g1_beta_g2_bytes);
    writeln!(&mut file,"pub const ALPHA_G1_BETA_G2: [u8;384] = ").unwrap();

    writeln!(&mut file,"\t{:?}", alpha_g1_beta_g2_bytes).unwrap();
    write!(&mut file,";").unwrap();

    // // Write a byte string.
    // file.write(b"Bytes\n").unwrap();
    //     fs::open("prepared_verifying_key.txt")
    //     fs::writeFile(pvk);
    //     //test_hardcoded_verifyingkey();
    }


    //println!("{:?}", pvk);
    Ok(())
}
