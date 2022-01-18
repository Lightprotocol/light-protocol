use crate::tokio::time::timeout;
use ark_ff::biginteger::BigInteger256;
use ark_ff::{Fp256, FromBytes};
use serde_json::Value;
use solana_program::program_pack::Pack;
use Testing_Hardcoded_Params_devnet_new::{
    Groth16_verifier::{
        parsers::*,
        miller_loop::ml_state::*,
    },
    process_instruction,
};
use {
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        program_error::ProgramError
    },
    solana_program_test::*,
    solana_sdk::{account::Account, signature::Signer, transaction::Transaction},
    std::str::FromStr,
};

use solana_program_test::ProgramTestContext;
use std::{fs, time};
use ark_ff::QuadExtField;
use ark_groth16::prepare_verifying_key;
use ark_groth16::prepare_inputs;
use ark_ec::ProjectiveCurve;
async fn create_and_start_program(
    account_init_bytes: Vec<u8>,
    ml_bytes_pubkey: Pubkey,
    pi_bytes_pubkey: Pubkey,
    program_id: Pubkey,
) -> ProgramTestContext {
    let mut program_test = ProgramTest::new(
        "Testing_Hardcoded_Params_devnet_new",
        program_id,
        processor!(process_instruction),
    );

    // Not used after all.
    let g_ic_reference_value: ark_ec::short_weierstrass_jacobian::GroupAffine<
        ark_bn254::g1::Parameters,
    > = ark_ec::short_weierstrass_jacobian::GroupProjective::<ark_bn254::g1::Parameters>::new(
        Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
            471223499747275859,
            7569467397224972520,
            7773394081695017935,
            2286200356768260157,
        ])), // Cost: 31
        Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
            3547148427379123761,
            953159675554207994,
            12994789713999071316,
            608791868936298975,
        ])), // Cost: 31
        Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
            18420557557863729053,
            2004103336708983265,
            11245578246982574736,
            2207358309629838870,
        ])), // Cost: 31
    )
    // .into_affine()
    .into();

    // Initializes acc based on x state and y pubkey.
    let mut account_ml = Account::new(10000000000, 3900, &program_id);
    account_ml.data = account_init_bytes;
    program_test.add_account(ml_bytes_pubkey, account_ml);

    // Inits gic account.
    let mut account_pi = Account::new(10000000000, 3900, &program_id);
    let mut pi_acc_init_bytes: [u8; 3900] = [0; 3900];
    // Keep in mind that g_ic_reference_value is based on running groth16.prepare_inputs() with 7 hardcoded inputs.
    // This done in preproc.
    let mut g_ic_init_bytes: Vec<u8> = vec![0; 64];
    parse_x_group_affine_to_bytes(g_ic_reference_value, &mut g_ic_init_bytes);
    println!("g_ic_init_bytes: {:?}", g_ic_init_bytes);

    // Tmp test: hardcoded bytes == offchain bytes of preparedinputs/gic etc
    let init_p2x = [
        100, 40, 113, 154, 67, 121, 202, 201, 76, 235, 138, 200, 226, 180, 215, 158, 50, 175, 84,
        220, 140, 125, 70, 226, 143, 61, 4, 218, 156, 132, 189, 26,
    ];
    let init_p2y = [
        102, 134, 203, 168, 30, 25, 139, 190, 23, 227, 191, 231, 63, 83, 110, 168, 17, 250, 213,
        214, 90, 94, 40, 179, 94, 56, 209, 32, 129, 142, 178, 11,
    ];
    let init_p: Vec<u8> = vec![init_p2x.to_vec(), init_p2y.to_vec()].concat();
    // Where x_1_range starts
    for i in 252..316 {
        pi_acc_init_bytes[i] = init_p[i - 252];
    }
    println!("pi_acc_init_bytes g_ic_range: {:?}", pi_acc_init_bytes);

    account_pi.data = pi_acc_init_bytes[..].to_vec();
    println!("adding new pi acc");
    program_test.add_account(pi_bytes_pubkey, account_pi);
    println!("new pi acc added");

    let tmp = program_test.start_with_context().await;
    println!("program started w/ context");
    tmp
}

pub async fn create_and_start_program_var(
    accounts: &Vec<(&Pubkey, usize, Option<Vec<u8>>)>,
    program_id: &Pubkey,
    signer_pubkey: &Pubkey,
) -> ProgramTestContext {
    let mut program_test = ProgramTest::new(
        "Testing_Hardcoded_Params_devnet_new",
        *program_id,
        processor!(process_instruction),
    );
    println!("accounts {:?}", accounts);
    for (pubkey, size, data) in accounts.iter() {
        println!("accounts {:?}, {:?}, {:?}", pubkey, size, data);

        let mut account = Account::new(10000000000, *size, &program_id);
        match data {
            Some(d) => (account.data = d.clone()),
            None => ()
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

pub fn get_public_inputs_from_bytes(
    public_inputs_bytes: &Vec<u8>,
) -> Result<Vec<Fp256<ark_ed_on_bn254::FqParameters>>, serde_json::Error> {
    let mut res = Vec::new();
    for i in 0..7 {
        let current_input = &public_inputs_bytes[(i * 32)..((i * 32) + 32)];

        res.push(
            <Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&current_input[..])
                .unwrap(),
        )
    }
    Ok(res)
}
pub fn get_vk_from_file() -> Result<
        ark_groth16::data_structures::VerifyingKey<ark_ec::models::bn::Bn<ark_bn254::Parameters>>,serde_json::Error> {
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
            a_g1_bigints
                .push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
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

#[tokio::test]
async fn ml_test_onchain_new() {
    // Creates program, accounts, setup.
    let program_id = Pubkey::from_str("TransferLamports111111111111111111112111111").unwrap();
    //create pubkey for temporary storage account
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
    // We must convert to affine here since the program converts projective into affine already as the last step of prepare_inputs.
    // While the native library implementation does the conversion only when the millerloop is called.
    // The reason we're doing it as part of prepare_inputs is that it takes >1 ix to compute the conversion.
    let as_affine = (prepared_inputs).into_affine();
    let mut affine_bytes = vec![0; 64];
    parse_x_group_affine_to_bytes(as_affine, &mut affine_bytes);
    // mock account state after prepare_inputs (instruction index = 466)
    let mut account_state = vec![0; 3900];
    // set is_initialized:true
    account_state[0] = 1;
    // for x_1_range alas prepared_inputs.into_affine()
    for (index, i) in affine_bytes.iter().enumerate() {
        account_state[index + 252] = *i;
    }
    // for proof a,b,c
    for (index, i) in proof_bytes.iter().enumerate() {
        account_state[index + 3516] = *i;
    }
    // set current index
    let current_index = 466 as usize;
    for (index, i) in current_index.to_le_bytes().iter().enumerate() {
        account_state[index + 212] = *i;
    }
    // We need to set the signer since otherwise the signer check fails on-chain
    let signer_pubkey_bytes = signer_keypair.to_bytes();
    for (index, i) in signer_pubkey_bytes[32..].iter().enumerate() {
        account_state[index + 4] = *i;
    }
    let mut accounts_vector = Vec::new();
    // accounts_vector.push((&merkle_tree_pubkey, 16657, None));
    accounts_vector.push((&storage_pubkey, 3900, Some(account_state)));
    let mut program_context =
        create_and_start_program_var(&accounts_vector, &program_id, &signer_pubkey).await;

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

    /*
     *
     *
     *Miller loop
     *
     *
     */
    // Just making sure we're not having the exact same ix_data/ix in the same block.
    // Since the runtime dedupes any exactly equivalent ix within the same block.
    // We need program restart logic here since we're firing 300+ ix and
    // the program_context seems to melt down every couple of hundred ix.
    // It basically just picks up the account state where it left off and restarts the client
    let mut i = 0usize;
    for _id in 0..430usize {
        let mut success = false;
        let mut retries_left = 2;
        while retries_left > 0 && success != true {
            let mut transaction = Transaction::new_with_payer(
                &[Instruction::new_with_bincode(
                    program_id,
                    &[vec![1, 1], usize::to_le_bytes(i).to_vec()].concat(),
                    vec![
                        AccountMeta::new(signer_pubkey, true),
                        AccountMeta::new(storage_pubkey, false),
                    ],
                )],
                Some(&signer_pubkey),
            );
            transaction.sign(&[&signer_keypair], program_context.last_blockhash);
            let res_request = timeout(
                time::Duration::from_millis(500),
                program_context
                    .banks_client
                    .process_transaction(transaction),
            )
            .await;
            match res_request {
                Ok(_) => success = true,
                Err(_e) => {
                    println!("retries_left {}", retries_left);
                    retries_left -= 1;
                    let storage_account = program_context
                        .banks_client
                        .get_account(storage_pubkey)
                        .await
                        .expect("get_account")
                        .unwrap();

                    // start program
                    let mut accounts_vector = Vec::new();
                    // accounts_vector.push((
                    //     &merkle_tree_pubkey,
                    //     16657,
                    //     Some(merkle_tree_account.data.to_vec()),
                    // ));
                    accounts_vector.push((
                        &storage_pubkey,
                        3900,
                        Some(storage_account.data.to_vec()),
                    ));
                    program_context =
                        create_and_start_program_var(&accounts_vector, &program_id, &signer_pubkey)
                            .await;
                }
            }
        }
        i += 1;
    }

    let storage_account = program_context
        .banks_client
        .get_account(storage_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    let account_data = ML254Bytes::unpack(&storage_account.data.clone()).unwrap();
    // TODO:compare with miller_output
    let mut ref_f = vec![0; 384];
    parse_f_to_bytes(miller_output, &mut ref_f);
    println!("account_data.f_range: {:?}", pvk.alpha_g1_beta_g2);
    assert_eq!(
        account_data.f_range, ref_f,
        "onchain f result != reference f (hardcoded from lib call)"
    );
    println!("onchain test success");
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
//"get_proof_from_bytes"
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
// #[tokio::test]
// async fn test_ml_254_onchain() {
//     // Creates program, accounts, setup.
//     let program_id = Pubkey::from_str("TransferLamports111111111111111111112111111").unwrap();
//     let ml_bytes_pubkey = Pubkey::new_unique();
//     let pi_bytes_pubkey = Pubkey::new_unique();
//     let instruction_index = usize::to_le_bytes(465).to_vec();
//     let mut init_bytes_ml = vec![0; 3900];
//     for i in 212..220 {
//         init_bytes_ml[i] = instruction_index[i - 212].clone();
//     }
//     //init_bytes_ml[212..220] = usize::to_le_bytes(465).to_vec();
//     let mut program_context = create_and_start_program(
//         init_bytes_ml.to_vec(),
//         ml_bytes_pubkey,
//         pi_bytes_pubkey,
//         program_id,
//     )
//     .await;
//
//     // Preparing inputs datas like in client (g_ic from prpd inputs, proof.a.b.c from client)
//     let proof_a_bytes = [
//         69, 130, 7, 152, 173, 46, 198, 166, 181, 14, 22, 145, 185, 13, 203, 6, 137, 135, 214, 126,
//         20, 88, 220, 3, 105, 33, 77, 120, 104, 159, 197, 32, 103, 123, 208, 55, 205, 101, 80, 10,
//         180, 216, 217, 177, 14, 196, 164, 108, 249, 131, 207, 100, 192, 194, 74, 200, 16, 192, 219,
//         4, 161, 93, 141,
//         15,
//         // 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//         // 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//     ];
//     let proof_c_bytes = [
//         187, 25, 7, 191, 235, 134, 124, 225, 209, 30, 66, 253, 195, 106, 121, 199, 99, 89, 183,
//         179, 203, 75, 203, 177, 10, 104, 149, 210, 7, 63, 131, 24, 197, 174, 244, 228, 219, 108,
//         228, 249, 71, 84, 209, 158, 244, 104, 179, 116, 118, 246, 158, 237, 87, 197, 134, 24, 140,
//         103, 27, 203, 108, 245, 42,
//         1,
//         //1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//         //0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//     ];
//     // coeffs1 -- ix 2
//     let proof_b_bytes = [
//         32, 255, 161, 204, 195, 74, 249, 196, 139, 193, 49, 109, 241, 230, 145, 100, 91, 134, 188,
//         102, 83, 190, 140, 12, 84, 21, 107, 182, 225, 139, 23, 16, 64, 152, 20, 230, 245, 127, 35,
//         113, 194, 4, 161, 242, 179, 131, 135, 66, 70, 179, 115, 118, 237, 158, 246, 97, 35, 85, 25,
//         13, 30, 21, 183, 18, 254, 194, 12, 96, 211, 37, 160, 170, 7, 173, 208, 52, 22, 169, 113,
//         149, 235, 85, 90, 20, 14, 171, 22, 22, 247, 254, 71, 236, 207, 18, 90, 29, 236, 211, 193,
//         206, 15, 107, 89, 218, 207, 62, 76, 75, 88, 71, 9, 45, 114, 212, 43, 127, 163, 183, 245,
//         213, 117, 216, 64, 56, 26, 102, 15,
//         37,
//         //1, 0, 0, 0, 0, 0, 0, 0,
//         // 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//         // 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
//     ];
//
//     // Testing integrity of the hardcoded bytes:
//     let contents = fs::read_to_string("./tests/proof_bytes_254.txt")
//         .expect("Something went wrong reading the file");
//     let v: Value = serde_json::from_str(&contents).unwrap();
//
//     let mut bytes: Vec<u8> = Vec::new();
//     for i in 0..2 {
//         for i in v["pi_a"][i].as_str().unwrap().split(',') {
//             bytes.push((*i).parse::<u8>().unwrap());
//         }
//     }
//     assert_eq!(
//         bytes,
//         proof_a_bytes[0..64],
//         "parsed proof.a != hardcoded proof.a"
//     );
//
//     let mut bytes: Vec<u8> = Vec::new();
//     for i in 0..2 {
//         for j in 0..2 {
//             for z in v["pi_b"][i][j].as_str().unwrap().split(',') {
//                 bytes.push((*z).parse::<u8>().unwrap());
//             }
//         }
//     }
//     assert_eq!(
//         bytes,
//         proof_b_bytes[0..128],
//         "parsed proof.b != hardcoded proof.b"
//     );
//     let mut bytes: Vec<u8> = Vec::new();
//     for i in 0..2 {
//         for i in v["pi_c"][i].as_str().unwrap().split(',') {
//             bytes.push((*i).parse::<u8>().unwrap());
//         }
//     }
//     assert_eq!(
//         bytes,
//         proof_c_bytes[0..64],
//         "parsed proof.c != hardcoded proof.c"
//     );
//
//     // Executes first ix: [0]
//     // Parses in initialized pi_account 2nd. Preprocessor then reads g_ic from that.
//     let i_data0: Vec<u8> = vec![0; 2];
//     let mut transaction = Transaction::new_with_payer(
//         &[Instruction::new_with_bincode(
//             program_id,
//             &[vec![1, 1], i_data0].concat(),
//             vec![
//                 AccountMeta::new(program_context.payer.pubkey(), true),
//                 AccountMeta::new(ml_bytes_pubkey, false),
//                 AccountMeta::new(pi_bytes_pubkey, false),
//             ],
//         )],
//         Some(&program_context.payer.pubkey()),
//     );
//     transaction.sign(&[&program_context.payer], program_context.last_blockhash);
//     program_context
//         .banks_client
//         .process_transaction(transaction)
//         .await
//         .unwrap();
//
//     // Executes second ix: [1]
//     // Parses proof_a and proof_c bytes ()
//     let i_data: Vec<u8> = [proof_a_bytes, proof_c_bytes].concat(); // 128 b
//     let mut transaction = Transaction::new_with_payer(
//         &[Instruction::new_with_bincode(
//             program_id,
//             &[vec![1, 1], i_data].concat(), // 129++
//             vec![
//                 AccountMeta::new(program_context.payer.pubkey(), true),
//                 AccountMeta::new(ml_bytes_pubkey, false),
//             ],
//         )],
//         Some(&program_context.payer.pubkey()),
//     );
//     transaction.sign(&[&program_context.payer], program_context.last_blockhash);
//     program_context
//         .banks_client
//         .process_transaction(transaction)
//         .await
//         .unwrap();
//
//     // Executes third ix [2]
//     // Parses proof_b_bytes (2..194) // 128 b
//     let i_data_2: Vec<u8> = proof_b_bytes[..].to_vec();
//     let mut transaction = Transaction::new_with_payer(
//         &[Instruction::new_with_bincode(
//             program_id,
//             &[vec![1, 1], i_data_2].concat(),
//             vec![
//                 AccountMeta::new(program_context.payer.pubkey(), true),
//                 AccountMeta::new(ml_bytes_pubkey, false),
//             ],
//         )],
//         Some(&program_context.payer.pubkey()),
//     );
//     transaction.sign(&[&program_context.payer], program_context.last_blockhash);
//     program_context
//         .banks_client
//         .process_transaction(transaction)
//         .await
//         .unwrap();
//
//     let storage_account = program_context
//         .banks_client
//         .get_account(ml_bytes_pubkey)
//         .await
//         .expect("get_account")
//         .unwrap();
//     let account_data = ML254Bytes::unpack(&storage_account.data.clone()).unwrap();
//     println!("init state f_range: {:?}", account_data.f_range);
//     println!("init state P1x: {:?}", account_data.p_1_x_range);
//     println!("init state P1y: {:?}", account_data.p_1_y_range);
//
//     println!("init state P2x: {:?}", account_data.p_2_x_range);
//     println!("init state P2y: {:?}", account_data.p_2_y_range);
//
//     println!("init state P3x: {:?}", account_data.p_3_x_range);
//     println!("init state P3y: {:?}", account_data.p_3_y_range);
//
//     println!("init state PROOFB: {:?}", account_data.proof_b);
//     //assert_eq!(true, false);
//     // Executes 1973 following ix.
//     println!("xxxxx");
//     let mut i = 0usize;
//     for _id in 3..430usize {
//         // 3..612 @merging helpers and add step
//         // 3..639 @14,15,16 merged
//         // 3..693 11,12,13 merged
//         // 3..821 @3,4 merged
//         // 3..884 @d1-d5 merged
//         // 3..1157 @d2-d5 merged
//         // 3..1976
//         let mut success = false;
//         let mut retries_left = 2;
//         while retries_left > 0 && success != true {
//             let mut transaction = Transaction::new_with_payer(
//                 &[Instruction::new_with_bincode(
//                     program_id,
//                     &[vec![1, 1], usize::to_le_bytes(i).to_vec()].concat(),
//                     vec![
//                         AccountMeta::new(program_context.payer.pubkey(), true),
//                         AccountMeta::new(ml_bytes_pubkey, false),
//                     ],
//                 )],
//                 Some(&program_context.payer.pubkey()),
//             );
//             transaction.sign(&[&program_context.payer], program_context.last_blockhash);
//             let res_request = timeout(
//                 time::Duration::from_millis(500),
//                 program_context
//                     .banks_client
//                     .process_transaction(transaction),
//             )
//             .await;
//             match res_request {
//                 Ok(_) => success = true,
//                 Err(_e) => {
//                     println!("retries_left {}", retries_left);
//                     retries_left -= 1;
//                     let storage_account = program_context
//                         .banks_client
//                         .get_account(ml_bytes_pubkey)
//                         .await
//                         .expect("get_account")
//                         .unwrap();
//                     program_context = create_and_start_program(
//                         storage_account.data.to_vec(),
//                         ml_bytes_pubkey,
//                         pi_bytes_pubkey,
//                         program_id,
//                     )
//                     .await;
//                 }
//             }
//         }
//         i += 1;
//     }
//
//     // Compute the affine value from this and compare to the (hardcoded) value that's returned from
//     // prepare_inputs lib call/reference.
//     let storage_account = program_context
//         .banks_client
//         .get_account(ml_bytes_pubkey)
//         .await
//         .expect("get_account")
//         .unwrap();
//     let account_data = ML254Bytes::unpack(&storage_account.data.clone()).unwrap();
//
//     // = ark_groth16-miller_output reference
//     let reference_f = [
//         41, 164, 125, 219, 237, 181, 202, 195, 98, 55, 97, 232, 35, 147, 153, 23, 164, 70, 211,
//         144, 151, 9, 219, 197, 234, 13, 164, 242, 67, 59, 148, 5, 132, 108, 82, 161, 228, 167, 20,
//         24, 207, 201, 203, 25, 249, 125, 54, 96, 182, 231, 150, 215, 149, 43, 216, 0, 36, 166, 232,
//         13, 126, 3, 53, 0, 174, 209, 16, 242, 177, 143, 60, 247, 181, 65, 132, 142, 14, 231, 170,
//         52, 3, 34, 70, 49, 210, 158, 211, 173, 165, 155, 219, 80, 225, 32, 64, 8, 65, 139, 16, 138,
//         240, 218, 36, 220, 8, 100, 236, 141, 1, 223, 60, 59, 24, 38, 90, 254, 47, 91, 205, 228,
//         169, 103, 178, 30, 124, 141, 43, 9, 83, 155, 75, 140, 209, 26, 2, 250, 250, 20, 185, 78,
//         53, 54, 68, 178, 88, 78, 246, 132, 97, 167, 124, 253, 96, 26, 213, 99, 157, 155, 40, 9, 60,
//         139, 112, 126, 230, 195, 217, 125, 68, 169, 208, 149, 175, 33, 226, 17, 47, 132, 8, 154,
//         237, 156, 34, 97, 55, 129, 155, 64, 202, 54, 161, 19, 24, 1, 208, 104, 140, 149, 25, 229,
//         96, 239, 202, 24, 235, 221, 133, 137, 30, 226, 62, 112, 26, 58, 1, 85, 207, 182, 41, 213,
//         42, 72, 139, 41, 108, 152, 252, 164, 121, 76, 17, 62, 147, 226, 220, 79, 236, 132, 109,
//         130, 163, 209, 203, 14, 144, 180, 25, 216, 234, 198, 199, 74, 48, 62, 57, 0, 206, 138, 12,
//         130, 25, 12, 187, 216, 86, 208, 84, 198, 58, 204, 6, 161, 93, 63, 68, 121, 173, 129, 255,
//         249, 47, 42, 218, 214, 129, 29, 136, 7, 213, 160, 139, 148, 58, 6, 191, 11, 161, 114, 56,
//         174, 224, 86, 243, 103, 166, 151, 107, 36, 205, 170, 206, 196, 248, 251, 147, 91, 3, 136,
//         208, 36, 3, 51, 84, 102, 139, 252, 193, 9, 172, 113, 116, 50, 242, 70, 26, 115, 166, 252,
//         204, 163, 149, 78, 13, 255, 235, 222, 174, 120, 182, 178, 186, 22, 169, 153, 73, 48, 242,
//         139, 120, 98, 33, 101, 204, 204, 169, 57, 249, 168, 45, 197, 126, 105, 54, 187, 35, 241,
//         253, 4, 33, 70, 246, 206, 32, 17,
//     ];
//     assert_eq!(
//         account_data.f_range, reference_f,
//         "onchain f result != reference f (hardcoded from lib call)"
//     );
//     println!("onchain test success");
// }
