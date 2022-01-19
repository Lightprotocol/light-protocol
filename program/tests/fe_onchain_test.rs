use crate::tokio::runtime::Runtime;
use crate::tokio::time::timeout;
use ark_crypto_primitives::crh::TwoToOneCRH;

use ark_ed_on_bn254::Fq;
use ark_ff::bytes::{FromBytes, ToBytes};
use ark_ff::{BigInteger, Fp256, PrimeField, QuadExtField};
use ark_std::One;
use ark_std::{test_rng, UniformRand};
use arkworks_gadgets::poseidon::{
    circom::CircomCRH, sbox::PoseidonSbox, PoseidonError, PoseidonParameters, Rounds,
};
use arkworks_gadgets::utils::{
    get_mds_poseidon_circom_bn254_x5_3, get_rounds_poseidon_circom_bn254_x5_3, parse_vec,
};
use solana_program::program_pack::Pack;
use solana_program_test::ProgramTestError;
use solana_sdk::signer::keypair::Keypair;
use {
    light_protocol_core::{
        groth16_verifier::final_exponentiation::state::INSTRUCTION_ORDER_VERIFIER_PART_2,
        poseidon_merkle_tree::mt_state::{HashBytes, MerkleTree, MERKLE_TREE_ACC_BYTES},
        process_instruction,
    },
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
    },
    solana_program_test::*,
    solana_sdk::{
        account::Account, msg, signature::Signer, transaction::Transaction,
        transport::TransportError,
    },
    std::str::FromStr,
};

use ark_groth16::prepare_verifying_key;

use ark_groth16::prepare_inputs;
use ark_groth16::verify_proof;
use std::{thread, time};

use ark_ec::ProjectiveCurve;
use light_protocol_core::groth16_verifier::final_exponentiation::state::FinalExpBytes;
use light_protocol_core::groth16_verifier::parsers::parse_f_to_bytes;
use light_protocol_core::groth16_verifier::parsers::parse_x_group_affine_from_bytes;
use light_protocol_core::groth16_verifier::parsers::*;
use serde_json::Value;
use std::fs;
#[tokio::test]
async fn test_final_exp_correct() /*-> Result<(), TransportError>*/
{
    let program_id = Pubkey::from_str("TransferLamports111111111111111111111111111").unwrap();
    let storage_pubkey = Pubkey::new_unique();
    // let merkle_tree_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES);
    let signer_keypair = solana_sdk::signer::keypair::Keypair::new();
    let signer_pubkey = signer_keypair.pubkey();
    // start program the program with the exact account state.
    // ...The account state (current instruction index,...) must match the
    // state we'd have at the exact instruction we're starting the test at (ix 466 for millerloop)
    // read proof, public inputs from test file, prepare_inputs
    let ix_data = read_test_data();
    // Pick the data we need from the test file. 9.. bc of input structure
    let public_inputs_bytes = ix_data[9..233].to_vec(); // 224 length
    let proof_bytes = ix_data[233..489].to_vec(); // 256 length
    let pvk_unprepped = get_vk_from_file().unwrap(); //?// TODO: check if same vk
    let pvk = prepare_verifying_key(&pvk_unprepped);
    let proof = get_proof_from_bytes(&proof_bytes);
    let public_inputs = get_public_inputs_from_bytes(&public_inputs_bytes).unwrap(); // TODO: debug
    let prepared_inputs = prepare_inputs(&pvk, &public_inputs).unwrap();
    println!("public_inputs_bytes: {:?}", public_inputs_bytes);
    let res = verify_proof(&pvk, &proof, &public_inputs[..]);
    println!("res {:?}", res);
    println!("public_inputs_bytes: {:?}", public_inputs_bytes);

    panic!("proof incorrect");

    // Calculate miller_ouput with the ark library. Will be used to compare the
    // on-chain output with.
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
    println!("miller_output: {:?}", miller_output);
    // We must convert to affine here since the program converts projective into affine already as the last step of prepare_inputs.
    // While the native library implementation does the conversion only when the millerloop is called.
    // The reason we're doing it as part of prepare_inputs is that it takes >1 ix to compute the conversion.
    // let as_affine = (prepared_inputs).into_affine();
    // let mut affine_bytes = vec![0; 64];
    // parse_x_group_affine_to_bytes(as_affine, &mut affine_bytes);
    // mock account state after prepare_inputs (instruction index = 466)
    let mut account_state = vec![0; 3900];
    // set is_initialized:true
    account_state[0] = 1;
    // for x_1_range alas prepared_inputs.into_affine()
    // for (index, i) in affine_bytes.iter().enumerate() {
    //     account_state[index + 252] = *i;
    // }
    // for proof a,b,c
    // for (index, i) in proof_bytes.iter().enumerate() {
    //     account_state[index + 3516] = *i;
    // }
    // set current index
    let current_index = 896 as usize;
    for (index, i) in current_index.to_le_bytes().iter().enumerate() {
        account_state[index + 212] = *i;
    }
    let mut miller_loop_bytes = vec![0u8; 384];
    parse_f_to_bytes(miller_output.clone(), &mut miller_loop_bytes);

    // set miller_loop data
    for (index, i) in miller_loop_bytes.iter().enumerate() {
        account_state[index + 220] = *i;
    }
    // println!("miller_output: {:?}", account_state[212..212+384].to_vec());
    // panic!("");
    // We need to set the signer since otherwise the signer check fails on-chain
    let signer_pubkey_bytes = signer_keypair.to_bytes();
    for (index, i) in signer_pubkey_bytes[32..].iter().enumerate() {
        account_state[index + 4] = *i;
    }
    let mut accounts_vector = Vec::new();
    // accounts_vector.push((&merkle_tree_pubkey, 16657, None));
    accounts_vector.push((&storage_pubkey, 3900, Some(account_state.clone())));
    let mut program_context =
        create_and_start_program_var(&accounts_vector, &program_id, &signer_pubkey).await;

    //let storage_pubkey = Pubkey::new_unique();

    //let mut program_context = create_and_start_program(INIT_BYTES_FINAL_EXP.to_vec(), storage_pubkey, program_id).await;

    let init_data = program_context
        .banks_client
        .get_account(storage_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    assert_eq!(init_data.data, account_state);

    //padding to make every tx unique otherwise the test will not execute repeated instructions

    let mut i = 0usize;
    for (instruction_id) in INSTRUCTION_ORDER_VERIFIER_PART_2 {
        //println!("instruction data {:?}", [vec![*instruction_id, 0u8], left_input.clone(), right_input.clone(), [i as u8].to_vec() ].concat());
        //let instruction_data: Vec<u8> = [vec![*instruction_id, 1u8], commit.clone(), [i as u8].to_vec() ].concat();
        println!("INSTRUCTION_ORDER_VERIFIER_PART_2: {}", instruction_id);

        let mut success = false;
        let mut retries_left = 2;
        while (retries_left > 0 && success != true) {
            println!("success: {}", success);
            let mut transaction = Transaction::new_with_payer(
                &[Instruction::new_with_bincode(
                    program_id,
                    &[vec![instruction_id, 2u8], usize::to_le_bytes(i).to_vec()].concat(),
                    vec![
                        AccountMeta::new(signer_pubkey, true),
                        AccountMeta::new(storage_pubkey, false),
                        //AccountMeta::new(merkle_tree_pubkey, false),
                    ],
                )],
                Some(&signer_pubkey),
            );
            transaction.sign(&[&signer_keypair], program_context.last_blockhash);
            //tokio::time::timeout(std::time::Duration::from_secs(2), self.process).await
            let res_request = timeout(
                time::Duration::from_millis(500),
                program_context
                    .banks_client
                    .process_transaction(transaction),
            )
            .await;
            //let ten_millis = time::Duration::from_millis(400);

            //thread::sleep(ten_millis);
            //println!("res: {:?}", res_request);
            match res_request {
                Ok(_) => success = true,
                Err(e) => {
                    println!("retries_left {}", retries_left);
                    retries_left -= 1;
                    program_context = restart_program(
                        &mut accounts_vector,
                        &program_id,
                        &signer_pubkey,
                        program_context,
                    )
                    .await;
                }
            }
            // if i == 3 {
            //     println!("aborted at {}", i);
            //     break;
            // }
            i += 1;
        }
    }

    let storage_account = program_context
        .banks_client
        .get_account(storage_pubkey)
        .await
        .expect("get_account")
        .unwrap();

    let unpacked_data = FinalExpBytes::unpack(&storage_account.data).unwrap();
    let res_ref = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::final_exponentiation(&miller_output).unwrap();
    let mut res_ref_bytes = vec![0; 384];
    parse_f_to_bytes(res_ref, &mut res_ref_bytes);
    println!("res_bytes: {:?}", res_ref_bytes);
    assert_eq!(res_ref_bytes, unpacked_data.y1_range_s);
    let mut pvk_ref = vec![0u8; 384];
    parse_f_to_bytes(pvk.alpha_g1_beta_g2, &mut pvk_ref);

    assert_eq!(pvk_ref, unpacked_data.y1_range_s);
}

//init bytes resulting from miller loop execution,
//account is initialized has initial f stored and right instruction index
pub async fn create_and_start_program_var(
    accounts: &Vec<(&Pubkey, usize, Option<Vec<u8>>)>,
    program_id: &Pubkey,
    signer_pubkey: &Pubkey,
) -> ProgramTestContext {
    let mut program_test = ProgramTest::new(
        "light_protocol_core",
        *program_id,
        processor!(process_instruction),
    );
    for (pubkey, size, data) in accounts.iter() {
        let mut account = Account::new(10000000000, *size, &program_id);
        match data {
            Some(d) => (account.data = d.clone()),
            None => (),
        }
        program_test.add_account(**pubkey, account);
        println!("added account {:?}", **pubkey);
    }

    let mut program_context = program_test.start_with_context().await;
    let mut transaction = solana_sdk::system_transaction::transfer(
        &program_context.payer,
        &signer_pubkey,
        10000000000000,
        program_context.last_blockhash,
    );
    transaction.sign(&[&program_context.payer], program_context.last_blockhash);
    let res_request = program_context
        .banks_client
        .process_transaction(transaction)
        .await;

    program_context
}

pub fn read_test_data() -> Vec<u8> {
    let ix_data_file = fs::read_to_string("./tests/test_data/deposit_0_1_sol.txt")
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
    println!("{:?}", ix_data);
    ix_data
}

pub fn get_proof_from_bytes(
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

pub fn get_public_inputs_from_bytes(
    public_inputs_bytes: &Vec<u8>,
) -> Result<Vec<Fp256<ark_ed_on_bn254::FqParameters>>, serde_json::Error> {
    let mut res = Vec::new();
    for i in public_inputs_bytes.chunks(32) {
        //let current_input = &public_inputs_bytes[(i * 32)..((i * 32) + 32)];

        res.push(<Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&i[..]).unwrap())
    }
    Ok(res)
}

pub fn get_vk_from_file() -> Result<
    ark_groth16::data_structures::VerifyingKey<ark_ec::models::bn::Bn<ark_bn254::Parameters>>,
    serde_json::Error,
> {
    let contents = fs::read_to_string("./tests/verification_key_bytes_254.txt")
        .expect("Something went wrong reading the file");
    let v: Value = serde_json::from_str(&contents)?;
    //println!("{}",  v);

    //println!("With text:\n{:?}", v["vk_alpha_1"][1]);

    let mut a_g1_bigints = Vec::new();
    for i in 0..2 {
        let mut bytes: Vec<u8> = Vec::new();
        for i in v["vk_alpha_1"][i].as_str().unwrap().split(',') {
            bytes.push((*i).parse::<u8>().unwrap());
        }
        a_g1_bigints.push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
    }
    let alpha_g1_bigints = ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
        a_g1_bigints[0],
        a_g1_bigints[1],
        false,
    );

    let mut b_g2_bigints = Vec::new();
    //println!("{}",  v["vk_beta_2"]);
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
    for (i, _) in b_g2_bigints.iter().enumerate() {
        //println!("b_g2 {}", b_g2_bigints[i]);
    }

    let mut delta_g2_bytes = Vec::new();
    //println!("{}",  v["vk_delta_2"]);
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

    for (i, _) in delta_g2_bytes.iter().enumerate() {
        //println!("delta_g2 {}", delta_g2_bytes[i]);
    }

    let mut gamma_g2_bytes = Vec::new();
    //println!("{}",  v["vk_gamma_2"]);
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
            for z in v["IC"][i][u].as_str().unwrap().split(',') {
                bytes.push((*z).parse::<u8>().unwrap());
            }
            //println!("bytes.len() {} {}", bytes.len(), bytes[bytes.len() - 1]);
            g1_bytes.push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
        }
        gamma_abc_g1_bigints_bytes.push(
            ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
                g1_bytes[0],
                g1_bytes[1],
                false,
            ),
        );
    }

    Ok(
        ark_groth16::data_structures::VerifyingKey::<ark_ec::models::bn::Bn<ark_bn254::Parameters>> {
            alpha_g1: alpha_g1_bigints,
            beta_g2: beta_g2,
            gamma_g2: gamma_g2,
            delta_g2: delta_g2,
            gamma_abc_g1: gamma_abc_g1_bigints_bytes,
        },
    )
}

pub async fn restart_program(
    accounts_vector: &mut Vec<(&Pubkey, usize, Option<Vec<u8>>)>,
    program_id: &Pubkey,
    signer_pubkey: &Pubkey,
    mut program_context: ProgramTestContext,
) -> ProgramTestContext {
    for (pubkey, _, current_data) in accounts_vector.iter_mut() {
        let account = program_context
            .banks_client
            .get_account(**pubkey)
            .await
            .expect("get_account")
            .unwrap();
        *current_data = Some(account.data.to_vec());
    }
    // accounts_vector[1].2 = Some(storage_account.data.to_vec());
    let mut program_context_new =
        create_and_start_program_var(&accounts_vector, &program_id, &signer_pubkey).await;
    program_context_new
}
