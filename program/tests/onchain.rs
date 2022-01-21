use crate::test_utils::tests::{
    create_and_start_program_var, get_proof_from_bytes, get_public_inputs_from_bytes,
    get_vk_from_file, read_test_data, restart_program,
};
use crate::tokio::time::timeout;
use ark_ec::ProjectiveCurve;
use ark_ff::biginteger::BigInteger256;
use ark_ff::Fp256;
use ark_groth16::{prepare_inputs, prepare_verifying_key, verify_proof};
use light_protocol_core::utils::{init_bytes18, prepared_verifying_key::*};
use light_protocol_core::{
    groth16_verifier::{
        final_exponentiation::state::{FinalExpBytes, INSTRUCTION_ORDER_VERIFIER_PART_2},
        miller_loop::state::*,
        parsers::*,
        prepare_inputs::state::PiBytes,
    },
    poseidon_merkle_tree::state::HashBytes,
    poseidon_merkle_tree::state::MERKLE_TREE_ACC_BYTES,
    process_instruction,
};
use serde_json::{Result, Value};
use solana_program::program_pack::Pack;
use solana_program_test::ProgramTestContext;
use std::{fs, time};
use {
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
    },
    solana_program_test::*,
    solana_sdk::{account::Account, signature::Signer, transaction::Transaction},
    std::str::FromStr,
};
mod test_utils;

async fn compute_prepared_inputs(
    program_id: &solana_program::pubkey::Pubkey,
    signer_pubkey: &solana_program::pubkey::Pubkey,
    signer_keypair: &solana_sdk::signature::Keypair,
    tmp_storage_pda_pubkey: &solana_program::pubkey::Pubkey,
    program_context: &mut ProgramTestContext,
    accounts_vector: &mut std::vec::Vec<(
        &solana_program::pubkey::Pubkey,
        usize,
        std::option::Option<std::vec::Vec<u8>>,
    )>,
) {
    // We're supplying i=0; i++ here because
    // we must make sure we're not having the exact same ix_data/ix in the same block.
    // Since the runtime dedupes any exactly equivalent ix within the same block.
    let mut i = 0usize;
    for id in 0..464usize {
        let mut success = false;
        let mut retries_left = 2;
        while retries_left > 0 && success != true {
            let mut transaction = Transaction::new_with_payer(
                &[Instruction::new_with_bincode(
                    *program_id,
                    &vec![98, 99, i],
                    vec![
                        AccountMeta::new(*signer_pubkey, true),
                        AccountMeta::new(*tmp_storage_pda_pubkey, false),
                    ],
                )],
                Some(&signer_pubkey),
            );
            transaction.sign(&[&*signer_keypair], program_context.last_blockhash);
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

                    let mut program_context = restart_program(
                        accounts_vector,
                        &program_id,
                        &signer_pubkey,
                        program_context,
                    )
                    .await;
                }
            }
        }
        i += 1;
    }
}

async fn compute_miller_output(
    program_id: &solana_program::pubkey::Pubkey,
    signer_pubkey: &solana_program::pubkey::Pubkey,
    signer_keypair: &solana_sdk::signature::Keypair,
    tmp_storage_pda_pubkey: &solana_program::pubkey::Pubkey,
    program_context: &mut ProgramTestContext,
    accounts_vector: &mut std::vec::Vec<(
        &solana_program::pubkey::Pubkey,
        usize,
        std::option::Option<std::vec::Vec<u8>>,
    )>,
) {
    let storage_account = program_context
        .banks_client
        .get_account(*tmp_storage_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    let account_data = ML254Bytes::unpack(&storage_account.data.clone()).unwrap();

    // Executes 1973 following ix.
    let mut i = 8888usize;
    for _id in 0..430usize {
        let mut success = false;
        let mut retries_left = 2;
        while retries_left > 0 && success != true {
            let mut transaction = Transaction::new_with_payer(
                &[Instruction::new_with_bincode(
                    *program_id,
                    &[vec![1, 1], usize::to_le_bytes(i).to_vec()].concat(),
                    vec![
                        AccountMeta::new(*signer_pubkey, true),
                        AccountMeta::new(*tmp_storage_pda_pubkey, false),
                    ],
                )],
                Some(&signer_pubkey),
            );
            transaction.sign(&[signer_keypair], program_context.last_blockhash);
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
                    let mut program_context = restart_program(
                        accounts_vector,
                        &program_id,
                        &signer_pubkey,
                        program_context,
                    )
                    .await;
                }
            }
        }
        i += 1;
    }
}

async fn compute_final_exponentiation(
    program_id: &solana_program::pubkey::Pubkey,
    signer_pubkey: &solana_program::pubkey::Pubkey,
    signer_keypair: &solana_sdk::signature::Keypair,
    tmp_storage_pda_pubkey: &solana_program::pubkey::Pubkey,
    program_context: &mut ProgramTestContext,
    accounts_vector: &mut std::vec::Vec<(
        &solana_program::pubkey::Pubkey,
        usize,
        std::option::Option<std::vec::Vec<u8>>,
    )>,
) {
    let mut i = 0usize;
    for instruction_id in INSTRUCTION_ORDER_VERIFIER_PART_2 {
        let mut success = false;
        let mut retries_left = 2;
        while (retries_left > 0 && success != true) {
            println!("success: {}", success);
            let mut transaction = Transaction::new_with_payer(
                &[Instruction::new_with_bincode(
                    *program_id,
                    &[vec![instruction_id, 2u8], usize::to_le_bytes(i).to_vec()].concat(),
                    vec![
                        AccountMeta::new(*signer_pubkey, true),
                        AccountMeta::new(*tmp_storage_pda_pubkey, false),
                    ],
                )],
                Some(&signer_pubkey),
            );
            transaction.sign(&[signer_keypair], program_context.last_blockhash);
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
                    let mut program_context = restart_program(
                        accounts_vector,
                        &program_id,
                        &signer_pubkey,
                        program_context,
                    )
                    .await;
                }
            }
        }

        i += 1;
    }
}

pub async fn initialize_merkle_tree(
    program_id: &Pubkey,
    merkle_tree_pda_pubkey: &Pubkey,
    signer_keypair: &solana_sdk::signer::keypair::Keypair,
    program_context: &mut ProgramTestContext,
) {
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            *program_id,
            &[vec![240u8, 0u8], usize::to_le_bytes(1000).to_vec()].concat(),
            vec![
                AccountMeta::new(signer_keypair.pubkey(), true),
                AccountMeta::new(*merkle_tree_pda_pubkey, false),
            ],
        )],
        Some(&signer_keypair.pubkey()),
    );
    transaction.sign(&[signer_keypair], program_context.last_blockhash);

    program_context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let merkle_tree_data = program_context
        .banks_client
        .get_account(*merkle_tree_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    assert_eq!(
        init_bytes18::INIT_BYTES_MERKLE_TREE_18,
        merkle_tree_data.data[0..641]
    );
    println!("initializing merkle tree success");
}

pub async fn update_merkle_tree(
    program_id: &Pubkey,
    merkle_tree_pda_pubkey: &Pubkey,
    tmp_storage_pda_pubkey: &Pubkey,
    signer_keypair: &solana_sdk::signer::keypair::Keypair,
    program_context: &mut ProgramTestContext,
    accounts_vector: &mut Vec<(&Pubkey, usize, Option<Vec<u8>>)>,
) {
    let mut i = 0;
    for (instruction_id) in 0..237 {
        let instruction_data: Vec<u8> = vec![i as u8];
        let mut success = false;
        let mut retries_left = 2;
        while (retries_left > 0 && success != true) {
            let mut transaction = Transaction::new_with_payer(
                &[Instruction::new_with_bincode(
                    *program_id,
                    &instruction_data,
                    vec![
                        AccountMeta::new(signer_keypair.pubkey(), true),
                        AccountMeta::new(*tmp_storage_pda_pubkey, false),
                        AccountMeta::new(*merkle_tree_pda_pubkey, false),
                    ],
                )],
                Some(&signer_keypair.pubkey()),
            );
            transaction.sign(&[signer_keypair], program_context.last_blockhash);

            let res_request = timeout(
                time::Duration::from_millis(500),
                program_context
                    .banks_client
                    .process_transaction(transaction),
            )
            .await;

            match res_request {
                Ok(_) => success = true,
                Err(e) => {
                    retries_left -= 1;
                    let mut program_context = restart_program(
                        accounts_vector,
                        &program_id,
                        &signer_keypair.pubkey(),
                        program_context,
                    )
                    .await;
                }
            }
        }
        println!("Instruction index {}", i);
        i += 1;
    }
}

pub fn get_ref_value(mode: &str) -> Vec<u8> {
    let bytes;
    let ix_data = read_test_data();
    let public_inputs_bytes = ix_data[9..233].to_vec(); // 224 length
    let pvk_unprepped = get_vk_from_file().unwrap();
    let pvk = prepare_verifying_key(&pvk_unprepped);
    let public_inputs = get_public_inputs_from_bytes(&public_inputs_bytes).unwrap();
    let prepared_inputs = prepare_inputs(&pvk, &public_inputs).unwrap();
    if mode == "prepared_inputs" {
        // We must convert to affine here since the program converts projective into affine already as the last step of prepare_inputs.
        // While the native library implementation does the conversion only when the millerloop is called.
        // The reason we're doing it as part of prepare_inputs is that it takes >1 ix to compute the conversion.
        let as_affine = (prepared_inputs).into_affine();
        let mut affine_bytes = vec![0; 64];
        parse_x_group_affine_to_bytes(as_affine, &mut affine_bytes);
        bytes = affine_bytes;
    } else {
        let proof_bytes = ix_data[233..489].to_vec(); // 256 length
        let proof = get_proof_from_bytes(&proof_bytes);
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
        if mode == "miller_output" {
            let mut miller_output_bytes = vec![0; 384];
            parse_f_to_bytes(miller_output, &mut miller_output_bytes);
            bytes = miller_output_bytes;
        } else if mode == "final_exponentiation" {
            let res = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::final_exponentiation(&miller_output).unwrap();
            let mut res_bytes = vec![0; 384];
            parse_f_to_bytes(res, &mut res_bytes);
            bytes = res_bytes;
        } else {
            bytes = vec![];
        }
    }
    bytes
}

pub fn get_mock_state(
    mode: &str,
    signer_keypair: &solana_sdk::signer::keypair::Keypair,
) -> Vec<u8> {
    // start program the program with the exact account state that it would have at the start of the computation.
    let mock_bytes;
    let ix_data = read_test_data();
    let public_inputs_bytes = ix_data[9..233].to_vec(); // 224 length
    let proof_bytes = ix_data[233..489].to_vec(); // 256 length

    let proof = get_proof_from_bytes(&proof_bytes);
    let public_inputs = get_public_inputs_from_bytes(&public_inputs_bytes).unwrap();
    let pvk_unprepped = get_vk_from_file().unwrap(); //?// TODO: check if same vk
    let pvk = prepare_verifying_key(&pvk_unprepped);
    let prepared_inputs = prepare_inputs(&pvk, &public_inputs).unwrap();
    if mode == "miller_output" {
        let as_affine = (prepared_inputs).into_affine();
        let mut affine_bytes = vec![0; 64];
        parse_x_group_affine_to_bytes(as_affine, &mut affine_bytes);
        // mock account state after prepare_inputs (instruction index = 466)
        let mut account_state = vec![0; 3900];
        // set is_initialized: true
        account_state[0] = 1;
        // We need to set the signer since otherwise the signer check fails on-chain
        let signer_pubkey_bytes = signer_keypair.to_bytes();
        for (index, i) in signer_pubkey_bytes[32..].iter().enumerate() {
            account_state[index + 4] = *i;
        }
        // ...The account state (current instruction index,...) must match the
        // state we'd have at the exact instruction we're starting the test at (ix 466 for millerloop)
        let current_index = 466 as usize;
        for (index, i) in current_index.to_le_bytes().iter().enumerate() {
            account_state[index + 212] = *i;
        }
        // for x_1_range alas prepared_inputs.into_affine()
        for (index, i) in affine_bytes.iter().enumerate() {
            account_state[index + 252] = *i;
        }
        // for proof a,b,c
        for (index, i) in proof_bytes.iter().enumerate() {
            account_state[index + 3516] = *i;
        }
        mock_bytes = account_state;
    } else if mode == "final_exponentiation" {
        let mut account_state = vec![0; 3900];
        // set is_initialized:true
        account_state[0] = 1;
        // set current index
        let current_index = 896 as usize;
        for (index, i) in current_index.to_le_bytes().iter().enumerate() {
            account_state[index + 212] = *i;
        }
        let mut miller_loop_bytes = vec![0u8; 384];
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
        parse_f_to_bytes(miller_output.clone(), &mut miller_loop_bytes);

        // set miller_loop data
        for (index, i) in miller_loop_bytes.iter().enumerate() {
            account_state[index + 220] = *i;
        }
        // We need to set the signer since otherwise the signer check fails on-chain
        let signer_pubkey_bytes = signer_keypair.to_bytes();
        for (index, i) in signer_pubkey_bytes[32..].iter().enumerate() {
            account_state[index + 4] = *i;
        }

        mock_bytes = account_state;
    } else {
        mock_bytes = vec![];
    }
    mock_bytes
}

#[test]
fn pvk_should_match() {
    let pvk_unprepped = get_vk_from_file().unwrap();
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

#[tokio::test]
async fn deposit_should_succeed() {
    let ix_data = read_test_data();
    // Creates program, accounts, setup.
    let program_id = Pubkey::from_str("TransferLamports111111111111111111112111111").unwrap();
    let mut accounts_vector = Vec::new();
    // Creates pubkey for temporary storage account
    let merkle_tree_pda_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES);
    accounts_vector.push((&merkle_tree_pda_pubkey, 16657, None));
    // Creates random signer
    let signer_keypair = solana_sdk::signer::keypair::Keypair::new();
    let signer_pubkey = signer_keypair.pubkey();

    // start program
    let mut program_context =
        create_and_start_program_var(&accounts_vector, &program_id, &signer_pubkey).await;
    let _merkle_tree_pda = program_context
        .banks_client
        .get_account(merkle_tree_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();

    /*
     *
     *
     * Tx that initializes MerkleTree account
     *
     *
     */
    initialize_merkle_tree(
        &program_id,
        &merkle_tree_pda_pubkey,
        &signer_keypair,
        &mut program_context,
    )
    .await;
    /*
     *
     *
     * Send data to chain and initialize tmp_storage_account
     *
     *
     */
    let tmp_storage_pda_pubkey =
        Pubkey::find_program_address(&[&ix_data[105..137], &b"storage"[..]], &program_id).0;
    accounts_vector.push((&tmp_storage_pda_pubkey, 3900, None));
    //sends bytes (public inputs and proof)
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &ix_data[8..].to_vec(),
            vec![
                AccountMeta::new(signer_pubkey, true),
                AccountMeta::new(tmp_storage_pda_pubkey, false),
                AccountMeta::new(
                    Pubkey::from_str("11111111111111111111111111111111").unwrap(),
                    false,
                ),
            ],
        )],
        Some(&signer_pubkey),
    );
    transaction.sign(&[&signer_keypair], program_context.last_blockhash);
    program_context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    /*
     *
     *
     * check merkle root
     *
     *
     */
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &ix_data[8..20].to_vec(), //random
            vec![
                AccountMeta::new(signer_pubkey, true),
                AccountMeta::new(tmp_storage_pda_pubkey, false),
                AccountMeta::new(merkle_tree_pda_pubkey, false),
            ],
        )],
        Some(&signer_pubkey),
    );
    transaction.sign(&[&signer_keypair], program_context.last_blockhash);
    program_context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    /*
     *
     *
     * Proof verification
     *
     *
     */

    compute_prepared_inputs(
        &program_id,
        &signer_pubkey,
        &signer_keypair,
        &tmp_storage_pda_pubkey,
        &mut program_context,
        &mut accounts_vector,
    )
    .await;

    /*
     *
     *
     *Miller loop
     *
     *
     */
    compute_miller_output(
        &program_id,
        &signer_pubkey,
        &signer_keypair,
        &tmp_storage_pda_pubkey,
        &mut program_context,
        &mut accounts_vector,
    )
    .await;

    /*
     *
     * Final Exponentiation
     *
     */

    // Note that if they verificaton is successful, this will pass. If not, an on-chain check will panic the program
    compute_final_exponentiation(
        &program_id,
        &signer_pubkey,
        &signer_keypair,
        &tmp_storage_pda_pubkey,
        &mut program_context,
        &mut accounts_vector,
    )
    .await;

    // TODO: Add offchain verification here, just to "prove" that the onchain check is legit.
    println!("Onchain Proof Verification success");

    /*
     *
     * Merkle Tree insert of new utxos
     *
     */

    update_merkle_tree(
        &program_id,
        &merkle_tree_pda_pubkey,
        &tmp_storage_pda_pubkey,
        &signer_keypair,
        &mut program_context,
        &mut accounts_vector,
    )
    .await;

    /*
     *
     *
     * Inserting Merkle root and transferring funds
     *
     *
     */
    // Creates pubkeys for all the PDAs we'll use
    let two_leaves_pda_pubkey =
        Pubkey::find_program_address(&[&ix_data[105..137], &b"leaves"[..]], &program_id).0;

    let mut nullifier_pubkeys = Vec::new();
    let pubkey_from_seed =
        Pubkey::find_program_address(&[&ix_data[96 + 9..128 + 9], &b"nf"[..]], &program_id);
    nullifier_pubkeys.push(pubkey_from_seed.0);

    let pubkey_from_seed =
        Pubkey::find_program_address(&[&ix_data[128 + 9..160 + 9], &b"nf"[..]], &program_id);
    nullifier_pubkeys.push(pubkey_from_seed.0);

    let _storage_account = program_context
        .banks_client
        .get_account(tmp_storage_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    let merkle_tree_pda_old = program_context
        .banks_client
        .get_account(merkle_tree_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();

    let receiver_pubkey = Pubkey::new_unique();

    let merkle_tree_pda = program_context
        .banks_client
        .get_account(merkle_tree_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    assert_eq!(merkle_tree_pda.data, merkle_tree_pda_old.data);
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &[0],
            vec![
                AccountMeta::new(signer_keypair.pubkey(), true),
                AccountMeta::new(tmp_storage_pda_pubkey, false),
                AccountMeta::new(two_leaves_pda_pubkey, false),
                AccountMeta::new(nullifier_pubkeys[0], false),
                AccountMeta::new(nullifier_pubkeys[1], false),
                AccountMeta::new(merkle_tree_pda_pubkey, false),
                AccountMeta::new(
                    Pubkey::from_str("11111111111111111111111111111111").unwrap(),
                    false,
                ),
                AccountMeta::new(merkle_tree_pda_pubkey, false),
            ],
        )],
        Some(&signer_keypair.pubkey()),
    );
    transaction.sign(&[&signer_keypair], program_context.last_blockhash);

    let _res_request = timeout(
        time::Duration::from_millis(500),
        program_context
            .banks_client
            .process_transaction(transaction),
    )
    .await;

    let nullifier0_account = program_context
        .banks_client
        .get_account(nullifier_pubkeys[0])
        .await
        .expect("get_account")
        .unwrap();
    let nullifier1_account = program_context
        .banks_client
        .get_account(nullifier_pubkeys[1])
        .await
        .expect("get_account")
        .unwrap();
    println!("nullifier0_account.data {:?}", nullifier0_account.data);
    assert_eq!(nullifier0_account.data[0], 1);
    println!("nullifier0_account.data {:?}", nullifier0_account.data);
    assert_eq!(nullifier1_account.data[0], 1);

    let merkel_tree_account_new = program_context
        .banks_client
        .get_account(merkle_tree_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    println!(
        "root[0]: {:?}",
        merkel_tree_account_new.data[609..641].to_vec()
    );
    println!(
        "root[1]: {:?}",
        merkel_tree_account_new.data[641..673].to_vec()
    );
    let two_leaves_pda_account = program_context
        .banks_client
        .get_account(two_leaves_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    println!(
        "two_leaves_pda_account.data: {:?}",
        two_leaves_pda_account.data
    );
    //account was initialized correctly
    assert_eq!(1, two_leaves_pda_account.data[0]);
    //account type is correct
    assert_eq!(4, two_leaves_pda_account.data[1]);

    //saved left leaf correctly
    // assert_eq!(
    //     public_inputs_bytes[160..192],
    //     two_leaves_pda_account.data[2..34]
    // );
    // //saved right leaf correctly
    // assert_eq!(
    //     public_inputs_bytes[192..224],
    //     two_leaves_pda_account.data[34..66]
    // );
    //saved merkle tree pubkey in which leaves were insorted
    assert_eq!(MERKLE_TREE_ACC_BYTES, two_leaves_pda_account.data[74..106]);

    println!(
        "deposit success {} {}",
        merkel_tree_account_new.lamports,
        merkle_tree_pda_old.lamports + 100000000
    );
    if merkel_tree_account_new.lamports != merkle_tree_pda_old.lamports + 100000000 {
        let receiver_account = program_context
            .banks_client
            .get_account(receiver_pubkey)
            .await
            .expect("get_account")
            .unwrap();

        println!(
            "withdraw success {}",
            receiver_account.lamports == 1000000000,
        );
    }
}

#[tokio::test]
async fn compute_prepared_inputs_should_succeed() {
    // Creates program, accounts, setup.
    let program_id = Pubkey::from_str("TransferLamports111111111111111111112111111").unwrap();
    //create pubkey for temporary storage account
    // let tmp_storage_pda_pubkey = Pubkey::new_unique();

    let merkle_tree_pda_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES);
    let signer_keypair = solana_sdk::signer::keypair::Keypair::new();
    let signer_pubkey = signer_keypair.pubkey();
    // start program the program with the exact account state.
    // ...The account state (current instruction index,...) must match the
    // state we'd have at the exact instruction we're starting the test at (ix 466 for millerloop)
    // read proof, public inputs from test file, prepare_inputs
    let ix_data = read_test_data();
    let tmp_storage_pda_pubkey =
        Pubkey::find_program_address(&[&ix_data[105..137], &b"storage"[..]], &program_id).0;
    // Pick the data we need from the test file. 9.. bc of input structure

    let prepared_inputs_ref = get_ref_value("prepared_inputs");

    let mut accounts_vector = Vec::new();
    accounts_vector.push((&merkle_tree_pda_pubkey, 16657, None));
    let mut program_context =
        create_and_start_program_var(&accounts_vector, &program_id, &signer_pubkey).await;

    // Initialize MerkleTree account
    initialize_merkle_tree(
        &program_id,
        &merkle_tree_pda_pubkey,
        &signer_keypair,
        &mut program_context,
    )
    .await;
    /*
     *
     *
     * Send data to chain
     *
     *
     */
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &ix_data[8..].to_vec(),
            vec![
                AccountMeta::new(signer_pubkey, true),
                AccountMeta::new(tmp_storage_pda_pubkey, false),
                AccountMeta::new(
                    Pubkey::from_str("11111111111111111111111111111111").unwrap(),
                    false,
                ),
            ],
        )],
        Some(&signer_pubkey),
    );
    transaction.sign(&[&signer_keypair], program_context.last_blockhash);
    program_context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
    accounts_vector.push((&tmp_storage_pda_pubkey, 3900, None));

    /*
     *
     *
     * check merkle root
     *
     *
     */
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &ix_data[8..20].to_vec(), //random
            vec![
                AccountMeta::new(signer_pubkey, true),
                AccountMeta::new(tmp_storage_pda_pubkey, false),
                AccountMeta::new(merkle_tree_pda_pubkey, false),
            ],
        )],
        Some(&signer_pubkey),
    );
    transaction.sign(&[&signer_keypair], program_context.last_blockhash);
    program_context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    /*
     *
     *
     *Prepare inputs
     *
     *
     */
    compute_prepared_inputs(
        &program_id,
        &signer_pubkey,
        &signer_keypair,
        &tmp_storage_pda_pubkey,
        &mut program_context,
        &mut accounts_vector,
    )
    .await;
    let storage_account = program_context
        .banks_client
        .get_account(tmp_storage_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();

    let account_data = PiBytes::unpack(&storage_account.data.clone()).unwrap();
    assert_eq!(
        account_data.x_1_range, prepared_inputs_ref,
        "onchain pi result != reference pi.into:affine()"
    );
}

#[tokio::test]
async fn compute_miller_output_should_succeed() {
    // Creates program, accounts, setup.
    let program_id = Pubkey::from_str("TransferLamports111111111111111111112111111").unwrap();
    // let merkle_tree_pda_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES);
    let signer_keypair = solana_sdk::signer::keypair::Keypair::new();
    let signer_pubkey = signer_keypair.pubkey();
    // start program the program with the exact account state.
    // ...The account state (current instruction index,...) must match the
    // state we'd have at the exact instruction we're starting the test at (ix 466 for millerloop)
    // read proof, public inputs from test file, prepare_inputs
    let ix_data = read_test_data();
    //create pubkey for temporary storage account
    let tmp_storage_pda_pubkey =
        Pubkey::find_program_address(&[&ix_data[105..137], &b"storage"[..]], &program_id).0;

    let account_state = get_mock_state("miller_output", &signer_keypair);
    let mut accounts_vector = Vec::new();
    accounts_vector.push((&tmp_storage_pda_pubkey, 3900, Some(account_state)));
    let mut program_context =
        create_and_start_program_var(&accounts_vector, &program_id, &signer_pubkey).await;

    /*
     *
     *
     *Miller loop
     *
     *
     */
    compute_miller_output(
        &program_id,
        &signer_pubkey,
        &signer_keypair,
        &tmp_storage_pda_pubkey,
        &mut program_context,
        &mut accounts_vector,
    )
    .await;
    let storage_account = program_context
        .banks_client
        .get_account(tmp_storage_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    let account_data = ML254Bytes::unpack(&storage_account.data.clone()).unwrap();

    let miller_output_ref = get_ref_value("miller_output");
    assert_eq!(
        account_data.f_range, miller_output_ref,
        "onchain f result != reference f"
    );
    println!("onchain test success");
}

#[tokio::test]
async fn compute_final_exponentiation_should_succeed() /*-> Result<(), TransportError>*/
{
    let program_id = Pubkey::from_str("TransferLamports111111111111111111111111111").unwrap();
    let ix_data = read_test_data();
    //create pubkey for temporary storage account
    let tmp_storage_pda_pubkey =
        Pubkey::find_program_address(&[&ix_data[105..137], &b"storage"[..]], &program_id).0;
    let signer_keypair = solana_sdk::signer::keypair::Keypair::new();
    let signer_pubkey = signer_keypair.pubkey();
    let f_ref = get_ref_value("final_exponentiation");
    let account_state = get_mock_state("final_exponentiation", &signer_keypair);
    let mut accounts_vector = Vec::new();
    accounts_vector.push((&tmp_storage_pda_pubkey, 3900, Some(account_state.clone())));
    let mut program_context =
        create_and_start_program_var(&accounts_vector, &program_id, &signer_pubkey).await;

    let init_data = program_context
        .banks_client
        .get_account(tmp_storage_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    assert_eq!(init_data.data, account_state);

    compute_final_exponentiation(
        &program_id,
        &signer_pubkey,
        &signer_keypair,
        &tmp_storage_pda_pubkey,
        &mut program_context,
        &mut accounts_vector,
    )
    .await;

    let storage_account = program_context
        .banks_client
        .get_account(tmp_storage_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();

    let unpacked_data = FinalExpBytes::unpack(&storage_account.data).unwrap();

    assert_eq!(f_ref, unpacked_data.y1_range_s);

    // // checking pvk ref
    // let mut pvk_ref = vec![0u8; 384];
    // parse_f_to_bytes(pvk.alpha_g1_beta_g2, &mut pvk_ref);
    // assert_eq!(pvk_ref, unpacked_data.y1_range_s);
}

#[tokio::test]
// TODO: Jorrit: rename?
async fn merkle_tree_root_check_should_succeed() {
    let program_id = Pubkey::from_str("TransferLamports111111111111111111111111111").unwrap();

    let tmp_storage_pda_pubkey = Pubkey::new_unique();
    let merkle_tree_pda_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES);
    let signer_keypair = solana_sdk::signer::keypair::Keypair::new();
    let signer_pubkey = signer_keypair.pubkey();

    let mut account_state = vec![0u8; 3900];
    let x = usize::to_le_bytes(801 + 466);
    for i in 212..220 {
        account_state[i] = x[i - 212];
    }
    account_state[0] = 1;
    // We need to set the signer since otherwise the signer check fails on-chain
    let signer_pubkey_bytes = signer_keypair.to_bytes();
    for (index, i) in signer_pubkey_bytes[32..].iter().enumerate() {
        account_state[index + 4] = *i;
    }
    //a random commitment to be used as leaves for merkle tree test update
    let commit = vec![
        143, 120, 199, 24, 26, 175, 31, 125, 154, 127, 245, 235, 132, 57, 229, 4, 60, 255, 3, 234,
        105, 16, 109, 207, 16, 139, 73, 235, 137, 17, 240, 2,
    ];
    //inserting commitment as left leaf
    for i in 3772..3804 {
        account_state[i] = commit[i - 3772];
    }
    //inserting commitment as right leaf
    for i in 3804..3836 {
        account_state[i] = commit[i - 3804];
    }
    let mut accounts_vector = Vec::new();
    accounts_vector.push((&merkle_tree_pda_pubkey, 16657, None));
    accounts_vector.push((&tmp_storage_pda_pubkey, 3900, Some(account_state.clone())));

    let mut program_context =
        create_and_start_program_var(&accounts_vector, &program_id, &signer_pubkey).await;

    //initialize MerkleTree account
    initialize_merkle_tree(
        &program_id,
        &merkle_tree_pda_pubkey,
        &signer_keypair,
        &mut program_context,
    )
    .await;
    //calculate new merkle tree root by updating the two first leaves
    update_merkle_tree(
        &program_id,
        &merkle_tree_pda_pubkey,
        &tmp_storage_pda_pubkey,
        &signer_keypair,
        &mut program_context,
        &mut accounts_vector,
    )
    .await;

    let storage_account = program_context
        .banks_client
        .get_account(tmp_storage_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();

    //expected root after one merkle tree height 18 update with specified leaves
    let expected_root = vec![
        247, 16, 124, 67, 44, 62, 195, 226, 182, 62, 41, 237, 78, 64, 195, 249, 67, 169, 200, 24,
        158, 153, 57, 144, 24, 245, 131, 44, 127, 129, 44, 10,
    ];
    let storage_account_unpacked = HashBytes::unpack(&storage_account.data).unwrap();
    assert_eq!(storage_account_unpacked.state[0], expected_root);
}
