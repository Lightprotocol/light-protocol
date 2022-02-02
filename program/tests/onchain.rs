use crate::test_utils::tests::{
    get_proof_from_bytes, get_public_inputs_from_bytes,
    get_vk_from_file, read_test_data, restart_program,
    create_and_start_program_var
};
mod merkle_tree_account_data_after_deposit;
mod merkle_tree_account_data_after_transfer;
use crate::merkle_tree_account_data_after_deposit::merkle_tree_account_data_after_deposit::MERKLE_TREE_ACCOUNT_DATA_AFTER_DEPOSIT;
use crate::merkle_tree_account_data_after_transfer::merkle_tree_account_data_after_transfer::MERKLE_TREE_ACCOUNT_DATA_AFTER_TRANSFER;
//use crate::merkle_tree_account_data_after_deposit::MERKLE_TREE_ACCOUNT_DATA_AFTER_DEPOSIT;
//use crate::merkle_tree_account_data_after_transaction::merkle_tree_account_data_after_transaction::MERKLE_TREE_ACCOUNT_DATA_AFTER_TRANSFER;
// };
use crate::tokio::time::timeout;
use ark_ec::ProjectiveCurve;
use ark_ff::biginteger::BigInteger256;
use ark_ff::Fp256;
use ark_groth16::{prepare_inputs, prepare_verifying_key, verify_proof};
use light_protocol_program::utils::{config, prepared_verifying_key::*};
use light_protocol_program::poseidon_merkle_tree::state::TmpStoragePda;

use light_protocol_program::{
    groth16_verifier::{
        final_exponentiation::state::{
            FinalExponentiationState,
        },
        final_exponentiation::ranges::INSTRUCTION_ORDER_VERIFIER_PART_2,
        miller_loop::state::*,
        parsers::*,
        prepare_inputs::state::PrepareInputsState,
    },
    process_instruction,
    state::ChecksAndTransferState,
    utils::config::MERKLE_TREE_ACC_BYTES_ARRAY,
};

use std::convert::TryInto;
use solana_program::bpf_loader::id;
use serde_json::{Result, Value};
use solana_program_test::ProgramTestContext;
use std::{fs, time};
use {
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        sysvar,
    },
    solana_program_test::*,
    solana_sdk::{
        account::Account, signature::Signer, signer::keypair::Keypair, transaction::Transaction,
    },
    std::str::FromStr,
};

use ark_bn254::Fq;
use ark_ff::BigInteger;
use ark_ff::PrimeField;
use ark_std::{test_rng, UniformRand};
use arrayref::{array_ref, array_refs};
use light_protocol_program::poseidon_merkle_tree::state::MerkleTree;
use solana_sdk::account::WritableAccount;
use solana_program::program_pack::Pack;
use solana_sdk::stake_history::Epoch;

const PRIVATE_KEY : [u8; 64]=[17,34,231,31,83,147,93,173,61,164,25,0,204,82,234,91,202,187,228,110,146,97,112,131,180,164,96,220,57,207,65,107,2,99,226,251,88,66,92,33,25,216,211,185,112,203,212,238,105,144,72,121,176,253,106,168,115,158,154,188,62,255,166,81];

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
    for id in 0..463usize {
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
                        None,
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
    let account_data = MillerLoopState::unpack(&storage_account.data.clone()).unwrap();

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
                        None,
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
                        None,
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
        config::INIT_BYTES_MERKLE_TREE_18,
        merkle_tree_data.data[0..642]
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

    for instruction_id in 0..238 {
        //checking merkle tree lock
        if instruction_id != 0 {
            let merkle_tree_pda_account = program_context
                .banks_client
                .get_account(*merkle_tree_pda_pubkey)
                .await
                .expect("get_account")
                .unwrap();
            let merkle_tree_pda_account_data =
                MerkleTree::unpack(&merkle_tree_pda_account.data.clone()).unwrap();
            assert_eq!(
                Pubkey::new(&merkle_tree_pda_account_data.pubkey_locked[..]),
                signer_keypair.pubkey()
            );
        }
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
                        None,
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
    let ix_data = read_test_data(String::from("deposit_0_1_sol.txt"));
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
    let ix_data = read_test_data(String::from("deposit_0_1_sol.txt"));
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
        let current_index = 465 as usize;
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
        let current_index = 895 as usize;
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

async fn check_nullifier_insert_correct(
    nullifier_pubkeys: &Vec<Pubkey>,
    program_context: &mut ProgramTestContext,
) {
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
}

async fn check_leaves_insert_correct(
    two_leaves_pda_pubkey: &Pubkey,
    left_leaf: &[u8],
    right_leaf: &[u8],
    expected_index: usize,
    program_context: &mut ProgramTestContext,
) {
    let two_leaves_pda_account = program_context
        .banks_client
        .get_account(*two_leaves_pda_pubkey)
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
    //expected index
    //assert_eq!(expected_index, usize::from_le_bytes(two_leaves_pda_account.data[2..11].try_into().unwrap()));
    //saved left leaf correctly
    assert_eq!(*left_leaf, two_leaves_pda_account.data[42..74]);
    //saved right leaf correctly
    assert_eq!(*right_leaf, two_leaves_pda_account.data[10..42]);
    //saved merkle tree pubkey in which leaves were insorted
    assert_eq!(MERKLE_TREE_ACC_BYTES_ARRAY[0].0, two_leaves_pda_account.data[74..106]);
}
async fn create_pubkeys_from_ix_data(
    ix_data: &Vec<u8>,
    program_id: &Pubkey,
) -> (Pubkey, Pubkey, Pubkey, Pubkey) {
    // Creates pubkeys for all the PDAs we'll use
    let tmp_storage_pda_pubkey =
        Pubkey::find_program_address(&[&ix_data[105..137], &b"storage"[..]], &program_id).0;
    let two_leaves_pda_pubkey =
        Pubkey::find_program_address(&[&ix_data[105..137], &b"leaves"[..]], program_id).0;

    let nf_pubkey0 = Pubkey::find_program_address(&[&ix_data[105..137], &b"nf"[..]], program_id).0;

    let nf_pubkey1 = Pubkey::find_program_address(&[&ix_data[137..169], &b"nf"[..]], program_id).0;
    (
        tmp_storage_pda_pubkey,
        two_leaves_pda_pubkey,
        nf_pubkey0,
        nf_pubkey1,
    )
}


async fn transact(
    program_id: &Pubkey,
    signer_pubkey: &Pubkey,
    signer_keypair: &Keypair,
    tmp_storage_pda_pubkey: &Pubkey,
    user_pda_token_pubkey: &Pubkey,
    merkle_tree_pda_pubkey: &Pubkey,
    merkle_tree_pda_token_pubkey: &Pubkey,
    expected_authority_pubkey: &Pubkey,
    nullifier_pubkeys: &Vec<Pubkey>,
    two_leaves_pda_pubkey: &Pubkey,
    relayer_pda_token_pubkey_option: Option<&Pubkey>,
    recipient_pubkey_option: Option<&Pubkey>,
    ix_data: Vec<u8>,
    program_context: &mut ProgramTestContext,
    accounts_vector: &mut std::vec::Vec<(
        &solana_program::pubkey::Pubkey,
        usize,
        std::option::Option<std::vec::Vec<u8>>,
    )>,
    token_accounts: &mut Vec<(&Pubkey, &Pubkey, u64)>,
    separator: u8,
    ) -> Result<ProgramTestContext> {
    let tmp_storage_pda_pubkey_copy = (*tmp_storage_pda_pubkey).clone();
    /*
     *
     *
     * Send data to chain and initialize tmp_storage_account
     *
     *
     */

    //sends bytes (public inputs and proof)
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            *program_id,
            &[ix_data[8..].to_vec(), vec![separator]].concat(),
            vec![
                AccountMeta::new(*signer_pubkey, true),
                AccountMeta::new(*tmp_storage_pda_pubkey, false),
                AccountMeta::new_readonly(
                    solana_program::system_program::id(),
                    false,
                ),
                AccountMeta::new_readonly(sysvar::rent::id(), false),
            ],
        )],
        Some(&signer_pubkey),
    );
    transaction.sign(&[signer_keypair], program_context.last_blockhash);
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
    let empty_vec = Vec::<u8>::new();
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            *program_id,
            &empty_vec, //random
            vec![
                AccountMeta::new(*signer_pubkey, true),
                AccountMeta::new(*tmp_storage_pda_pubkey, false),
                AccountMeta::new(*merkle_tree_pda_pubkey, false),
            ],
        )],
        Some(signer_pubkey),
    );
    transaction.sign(&[signer_keypair], program_context.last_blockhash);
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
        program_context,
        accounts_vector,
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
        program_context,
        accounts_vector,
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
        program_context,
        accounts_vector,
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
        program_context,
        accounts_vector,
    )
    .await;

    /*
     *
     *
     * Inserting Merkle root and transferring funds
     *
     *
     */


     let mut program_context = last_tx(
         &program_id,
         &merkle_tree_pda_pubkey,
         &tmp_storage_pda_pubkey,
         &user_pda_token_pubkey,
         &merkle_tree_pda_token_pubkey,
         &expected_authority_pubkey,
         &signer_keypair,
         nullifier_pubkeys,
         two_leaves_pda_pubkey,
         program_context,
         accounts_vector,
         token_accounts,
         relayer_pda_token_pubkey_option,
         recipient_pubkey_option
     )
     .await;
    Ok(program_context)
}

pub async fn last_tx (
    program_id: &Pubkey,
    merkle_tree_pda_pubkey: &Pubkey,
    tmp_storage_pda_pubkey: &Pubkey,
    user_pda_token_pubkey: &Pubkey,
    merkle_tree_pda_token_pubkey: &Pubkey,
    expected_authority_pubkey: &Pubkey,
    signer_keypair: &solana_sdk::signer::keypair::Keypair,
    nullifier_pubkeys: &Vec<Pubkey>,
    two_leaves_pda_pubkey: &Pubkey,
    program_context: &mut ProgramTestContext,
    accounts_vector: &mut Vec<(&Pubkey, usize, Option<Vec<u8>>)>,
    token_accounts: &mut Vec<(&Pubkey, &Pubkey, u64)>,
    relayer_pda_token_pubkey_option: Option<&Pubkey>,
    recipient_pubkey_option: Option<&Pubkey>,
) -> ProgramTestContext {
    let signer_pubkey = signer_keypair.pubkey();
    let mut accounts_vector_local = accounts_vector.clone();
    accounts_vector_local.push((tmp_storage_pda_pubkey, 3900, None));
   let mut program_context = restart_program(
       &mut accounts_vector_local,
       Some(token_accounts),
       &program_id,
       &signer_pubkey,
       program_context,
   )
   .await;


   let mut recipient_pubkey: Pubkey;
   println!("user_pda_token_pubkey: {:?}", user_pda_token_pubkey);
   println!("recipient_pubkey_option: {:?}", recipient_pubkey_option);

   println!("recipient_pubkey_option: {:?}", recipient_pubkey_option);
   let mut ix_vec = Vec::new();
    //deposit case mint wrapped sol tokens and approve a program owned authority
    if recipient_pubkey_option.is_none() &&  relayer_pda_token_pubkey_option.is_none() {
        recipient_pubkey = *user_pda_token_pubkey;

        let approve_instruction = spl_token::instruction::approve(
           &spl_token::id(),
           &user_pda_token_pubkey,
           &expected_authority_pubkey,
           &signer_keypair.pubkey(),
           &[],
           token_accounts[1].2
       ).unwrap();
       ix_vec.push(approve_instruction);
       ix_vec.push(
           Instruction::new_with_bincode(
           *program_id,
           //&[vec![bumpSeed;1],signer_keypair.pubkey().to_bytes()[..].to_vec()].concat(),
           &vec![21],
           vec![
               AccountMeta::new(signer_keypair.pubkey(), true),
               AccountMeta::new(*tmp_storage_pda_pubkey, false),
               AccountMeta::new(*two_leaves_pda_pubkey, false),
               AccountMeta::new(nullifier_pubkeys[0], false),
               AccountMeta::new(nullifier_pubkeys[1], false),
               AccountMeta::new(*merkle_tree_pda_pubkey, false),
               AccountMeta::new(*merkle_tree_pda_token_pubkey, false),
               AccountMeta::new_readonly(solana_program::system_program::id(), false),
               AccountMeta::new_readonly(spl_token::id(), false),
               AccountMeta::new_readonly(sysvar::rent::id(), false),
               AccountMeta::new(*expected_authority_pubkey, false),
               AccountMeta::new(*user_pda_token_pubkey, false),
           ]
           )
       );
   }
   //transfer
   else if recipient_pubkey_option.is_none() {
       recipient_pubkey = *merkle_tree_pda_token_pubkey;
       ix_vec.push(
           Instruction::new_with_bincode(
           *program_id,
           &vec![21],
           vec![
           AccountMeta::new(signer_keypair.pubkey(), true),
           AccountMeta::new(*tmp_storage_pda_pubkey, false),
           AccountMeta::new(*two_leaves_pda_pubkey, false),
           AccountMeta::new(nullifier_pubkeys[0], false),
           AccountMeta::new(nullifier_pubkeys[1], false),
           AccountMeta::new(*merkle_tree_pda_pubkey, false),
           AccountMeta::new(*merkle_tree_pda_token_pubkey, false),
           AccountMeta::new_readonly(solana_program::system_program::id(), false),
           AccountMeta::new_readonly(spl_token::id(), false),
           AccountMeta::new_readonly(sysvar::rent::id(), false),
           AccountMeta::new(*expected_authority_pubkey, false),
           AccountMeta::new(*relayer_pda_token_pubkey_option.unwrap(), false),
       ]
           )
       );
   }
   //withdrawal
   else {
       recipient_pubkey = *merkle_tree_pda_token_pubkey;
       ix_vec.push(
           Instruction::new_with_bincode(
           *program_id,
           &vec![21],
           vec![
           AccountMeta::new(signer_keypair.pubkey(), true),
           AccountMeta::new(*tmp_storage_pda_pubkey, false),
           AccountMeta::new(*two_leaves_pda_pubkey, false),
           AccountMeta::new(nullifier_pubkeys[0], false),
           AccountMeta::new(nullifier_pubkeys[1], false),
           AccountMeta::new(*merkle_tree_pda_pubkey, false),
           AccountMeta::new(*merkle_tree_pda_token_pubkey, false),
           AccountMeta::new_readonly(solana_program::system_program::id(), false),
           AccountMeta::new_readonly(spl_token::id(), false),
           AccountMeta::new_readonly(sysvar::rent::id(), false),
           AccountMeta::new(*expected_authority_pubkey, false),
           AccountMeta::new(*recipient_pubkey_option.unwrap(), false),
           AccountMeta::new(*relayer_pda_token_pubkey_option.unwrap(), false),
       ]
           )
       );
   }



   let mut transaction = Transaction::new_with_payer(
       &ix_vec,
       Some(&signer_keypair.pubkey()),
   );
   transaction.sign(&[signer_keypair], program_context.last_blockhash);

   let _res_request = timeout(
       time::Duration::from_millis(500),
       program_context
           .banks_client
           .process_transaction(transaction),
   )
   .await;

   return program_context;
}

async fn check_tmp_storage_account_state_correct(
    tmp_storage_pda_pubkey: &Pubkey,
    merkle_account_data_before: Option<&Vec<u8>>,
    merkle_account_data_after: Option<&Vec<u8>>,
    program_context: &mut ProgramTestContext,
) {
    let tmp_storage_account = program_context
        .banks_client
        .get_account(*tmp_storage_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();

    let unpacked_tmp_storage_account =
        ChecksAndTransferState::unpack(&tmp_storage_account.data.clone()).unwrap();
    assert_eq!(
        unpacked_tmp_storage_account.current_instruction_index,
        1502
    );

    if merkle_account_data_after.is_some() {
        let merkle_tree_pda_after =
            MerkleTree::unpack(&merkle_account_data_after.unwrap()).unwrap();
        assert_eq!(merkle_tree_pda_after.pubkey_locked, vec![0u8; 32]);
        if merkle_account_data_before.is_some() {
            let merkle_tree_account_before =
                MerkleTree::unpack(&merkle_account_data_before.unwrap()).unwrap();
            assert_eq!(
                merkle_tree_pda_after.current_root_index,
                merkle_tree_account_before.current_root_index + 1
            );
            assert!(merkle_tree_pda_after.roots != merkle_tree_account_before.roots);
            println!(
                "root[0]: {:?}",
                merkle_account_data_after.unwrap()[609..642].to_vec()
            );
            println!(
                "root[{}]: {:?}",
                merkle_tree_pda_after.current_root_index,
                merkle_account_data_after.unwrap()[((merkle_tree_pda_after.current_root_index - 1) * 32)
                    + 610
                    ..((merkle_tree_pda_after.current_root_index - 1) * 32) + 642]
                    .to_vec()
            );
            assert_eq!(unpacked_tmp_storage_account.root_hash, merkle_account_data_after.unwrap()[((merkle_tree_pda_after.current_root_index - 1) * 32) + 610..((merkle_tree_pda_after.current_root_index - 1) * 32) + 642].to_vec());

        }
    }
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
    let mut ix_withdraw_data = read_test_data(std::string::String::from("deposit_0_1_sol.txt"));
    println!("ix_withdraw_data[521..529]: {:?}", ix_withdraw_data[521..529].to_vec());
    let amount: u64 =  i64::from_le_bytes(ix_withdraw_data[521..529].try_into().unwrap()).try_into().unwrap();
    println!("amount: {:?}", amount);
    assert_eq!(ix_withdraw_data.len(), 602);
    // Creates program, accounts, setup.
    let program_id = Pubkey::from_str("TransferLamports111111111111111111112111111").unwrap();
    let mut accounts_vector = Vec::new();
    // Creates pubkey for tmporary storage account
    let merkle_tree_pda_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY[0].0);
    accounts_vector.push((
        &merkle_tree_pda_pubkey,
        16658,
        None,
    ));
    //private key is hardcoded to have a deterministic signer as relayer
    // Creates random signer
    let signer_keypair = solana_sdk::signer::keypair::Keypair::from_bytes(&PRIVATE_KEY).unwrap();
    let signer_pubkey = signer_keypair.pubkey();
    // assign relayer key to signer otherwise it fails relayer check
    for (i, elem) in ix_withdraw_data[529..561].iter_mut().enumerate() {
        *elem = signer_pubkey.to_bytes()[i];

    }

    let (tmp_storage_pda_pubkey, two_leaves_pda_pubkey, nf_pubkey0, nf_pubkey1) =
        create_pubkeys_from_ix_data(&ix_withdraw_data, &program_id).await;
    let mut nullifier_pubkeys = Vec::new();
    nullifier_pubkeys.push(nf_pubkey0);
    nullifier_pubkeys.push(nf_pubkey1);

    //is hardcoded onchain
    let authority_seed = program_id.to_bytes();
    let (expected_authority_pubkey, authority_bump_seed) = Pubkey::find_program_address(&[&authority_seed], &program_id);

    // let (merkle_tree_pda_token_pubkey, bumpSeed_merkle_tree) = Pubkey::find_program_address(
    //    &[&merkle_tree_pda_pubkey.to_bytes()[..]],
    //    &program_id
    // );
    let merkle_tree_pda_token_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY
        [ix_withdraw_data[601] as usize]
    .1);
    let user_pda_token_pubkey =  Keypair::new().pubkey();
    let relayer_pda_token_pubkey =  Keypair::new().pubkey();

    let mut token_accounts = Vec::new();
    token_accounts.push((&merkle_tree_pda_token_pubkey, &expected_authority_pubkey, 0));
    token_accounts.push((&user_pda_token_pubkey, &signer_pubkey, amount ));
    token_accounts.push((&relayer_pda_token_pubkey, &merkle_tree_pda_pubkey, 0));


    // start program
    let mut program_context =
        create_and_start_program_var(&accounts_vector, None, &program_id, &signer_pubkey).await;
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

    let merkle_tree_pda_before = program_context
        .banks_client
        .get_account(merkle_tree_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();

    //deposit shielded pool
    let mut program_context = transact(
        &program_id,
        &signer_pubkey,
        &signer_keypair,
        &tmp_storage_pda_pubkey,
        &user_pda_token_pubkey,
        &merkle_tree_pda_pubkey,
        &merkle_tree_pda_token_pubkey,
        &expected_authority_pubkey,
        &nullifier_pubkeys,
        &two_leaves_pda_pubkey,
        None,
        None,
        ix_withdraw_data.clone(),
        &mut program_context,
        &mut accounts_vector,
        &mut token_accounts,
        1u8,
    )
    .await.unwrap();

    check_nullifier_insert_correct(&nullifier_pubkeys, &mut program_context).await;

    let merkle_tree_pda_after = program_context
        .banks_client
        .get_account(merkle_tree_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();

    check_tmp_storage_account_state_correct(
        &tmp_storage_pda_pubkey,
        Some(&merkle_tree_pda_before.data),
        Some(&merkle_tree_pda_after.data),
        &mut program_context,
    )
    .await;

    check_leaves_insert_correct(
        &two_leaves_pda_pubkey,
        &ix_withdraw_data[192 + 9..224 + 9], //left leaf todo change order
        &ix_withdraw_data[160 + 9..192 + 9], //right leaf
        0,
        &mut program_context,
    )
    .await;

    let user_pda_token_account = program_context.banks_client
        .get_account(user_pda_token_pubkey)
        .await
        .expect("get_account").unwrap();
    let user_pda_token_account_data = spl_token::state::Account::unpack(&user_pda_token_account.data).unwrap();
    println!("\nuser_pda_token: {:?} \n", user_pda_token_pubkey);

    println!("user_pda_token_account_data: {:?}", user_pda_token_account_data);
    assert_eq!(user_pda_token_account_data.amount, 0);

    println!("\n merkle_tree_pda_token_pubkey: {:?} \n", merkle_tree_pda_token_pubkey);
    let merkle_tree_pda_token_account = program_context.banks_client
            .get_account(merkle_tree_pda_token_pubkey)
            .await
            .expect("get_account").unwrap();
    let merkle_tree_pda_token_account_data = spl_token::state::Account::unpack(&merkle_tree_pda_token_account.data).unwrap();

    println!("merkle_tree_pda_token_account_data: {:?}", merkle_tree_pda_token_account_data);
    assert_eq!(merkle_tree_pda_token_account_data.amount, amount);

    let relayer_pda_token_account = program_context.banks_client
            .get_account(relayer_pda_token_pubkey)
            .await
            .expect("get_account").unwrap();
    let relayer_pda_token_account_data = spl_token::state::Account::unpack(&relayer_pda_token_account.data).unwrap();

    let merkle_tree_account_data = program_context.banks_client
            .get_account(merkle_tree_pda_pubkey)
            .await
            .expect("get_account").unwrap();
    //println!("merkle_tree_account_data: {:?}", merkle_tree_account_data.data);
    let path = "tests/merkle_tree_account_data_after_deposit.rs";
    let mut output = File::create(path).ok().unwrap();
    write!(
        output,
        "{}",
        format!(
            "#[cfg(test)]
            pub mod merkle_tree_account_data_after_deposit {{
                pub const MERKLE_TREE_ACCOUNT_DATA_AFTER_DEPOSIT : [u8;{}] = {:?};
            }}",
            merkle_tree_account_data.data.len(),
            merkle_tree_account_data.data
        )
    ).unwrap();

}
use std::fs::File;
use std::io::Write;
#[tokio::test]
async fn internal_transfer_should_succeed() {
    let mut ix_withdraw_data = read_test_data(std::string::String::from("internal_transfer.txt"));
    let recipient = Pubkey::new(&ix_withdraw_data[489..521].to_vec());
    println!("ix_withdraw_data[521..529]: {:?} ", ix_withdraw_data[521..529].to_vec());
    println!("ix_withdraw_data[529..561]: {:?} ", Pubkey::new(&ix_withdraw_data[529..561].to_vec()));
    //panic!("");
    let amount: u64 =  0;//(-i64::from_le_bytes(ix_withdraw_data[521..529].try_into().unwrap())).try_into().unwrap();
    println!("amount: {:?}", amount);
    let fees: u64 =  u64::from_le_bytes(ix_withdraw_data[561..569].try_into().unwrap());
    // Creates program, accounts, setup.
    let program_id = Pubkey::from_str("TransferLamports111111111111111111112111111").unwrap();
    let mut accounts_vector = Vec::new();
    // Creates pubkey for tmporary storage account
    let merkle_tree_pda_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY[0].0);
    accounts_vector.push((
        &merkle_tree_pda_pubkey,
        16658,
        Some(MERKLE_TREE_ACCOUNT_DATA_AFTER_DEPOSIT.to_vec()),
    ));
    //private key is hardcoded to have a deterministic signer as relayer
    // Creates random signer
    let signer_keypair = solana_sdk::signer::keypair::Keypair::from_bytes(&PRIVATE_KEY).unwrap();
    let signer_pubkey = signer_keypair.pubkey();
    // assign relayer key to signer otherwise it fails relayer check
    for (i, elem) in ix_withdraw_data[529..561].iter_mut().enumerate() {
        *elem = signer_pubkey.to_bytes()[i];

    }

    let (tmp_storage_pda_pubkey, two_leaves_pda_pubkey, nf_pubkey0, nf_pubkey1) =
        create_pubkeys_from_ix_data(&ix_withdraw_data, &program_id).await;
    let mut nullifier_pubkeys = Vec::new();
    nullifier_pubkeys.push(nf_pubkey0);
    nullifier_pubkeys.push(nf_pubkey1);

    //is hardcoded onchain
    let authority_seed = program_id.to_bytes();
    let (expected_authority_pubkey, authority_bump_seed) = Pubkey::find_program_address(&[&authority_seed], &program_id);

    // let (merkle_tree_pda_token_pubkey, bumpSeed_merkle_tree) = Pubkey::find_program_address(
    //    &[&merkle_tree_pda_pubkey.to_bytes()[..]],
    //    &program_id
    // );
    let merkle_tree_pda_token_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY
        [ix_withdraw_data[601] as usize]
    .1);
    let user_pda_token_pubkey =  Keypair::new().pubkey();
    let random_user_owner_pubkey =  Keypair::new().pubkey();

    let relayer_pda_token_pubkey =  Keypair::new().pubkey();

    let mut token_accounts = Vec::new();
    token_accounts.push((&merkle_tree_pda_token_pubkey, &expected_authority_pubkey, fees ));
    token_accounts.push((&user_pda_token_pubkey, &random_user_owner_pubkey, 0));
    token_accounts.push((&relayer_pda_token_pubkey, &signer_pubkey, 0));


    // start program
    let mut program_context =
        create_and_start_program_var(&accounts_vector, None, &program_id, &signer_pubkey).await;
    let _merkle_tree_pda = program_context
        .banks_client
        .get_account(merkle_tree_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();

    let merkle_tree_pda_before = program_context
        .banks_client
        .get_account(merkle_tree_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();

    //transact in shielded pool
    let mut program_context = transact(
        &program_id,
        &signer_pubkey,
        &signer_keypair,
        &tmp_storage_pda_pubkey,
        &user_pda_token_pubkey,
        &merkle_tree_pda_pubkey,
        &merkle_tree_pda_token_pubkey,
        &expected_authority_pubkey,
        &nullifier_pubkeys,
        &two_leaves_pda_pubkey,
        Some(&relayer_pda_token_pubkey),
        None,
        ix_withdraw_data.clone(),
        &mut program_context,
        &mut accounts_vector,
        &mut token_accounts,
        1u8,
    )
    .await.unwrap();

    check_nullifier_insert_correct(&nullifier_pubkeys, &mut program_context).await;

    let merkle_tree_pda_after = program_context
        .banks_client
        .get_account(merkle_tree_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();

    check_tmp_storage_account_state_correct(
        &tmp_storage_pda_pubkey,
        Some(&merkle_tree_pda_before.data),
        Some(&merkle_tree_pda_after.data),
        &mut program_context,
    )
    .await;

    check_leaves_insert_correct(
        &two_leaves_pda_pubkey,
        &ix_withdraw_data[192 + 9..224 + 9], //left leaf todo change order
        &ix_withdraw_data[160 + 9..192 + 9], //right leaf
        0,
        &mut program_context,
    )
    .await;

    let user_pda_token_account = program_context.banks_client
        .get_account(user_pda_token_pubkey)
        .await
        .expect("get_account").unwrap();
    let user_pda_token_account_data = spl_token::state::Account::unpack(&user_pda_token_account.data).unwrap();
    println!("\nuser_pda_token: {:?} \n", user_pda_token_pubkey);

    println!("user_pda_token_account_data: {:?}", user_pda_token_account_data);
    assert_eq!(user_pda_token_account_data.amount, 0);

    println!("\n merkle_tree_pda_token_pubkey: {:?} \n", merkle_tree_pda_token_pubkey);
    let merkle_tree_pda_token_account = program_context.banks_client
            .get_account(merkle_tree_pda_token_pubkey)
            .await
            .expect("get_account").unwrap();
    let merkle_tree_pda_token_account_data = spl_token::state::Account::unpack(&merkle_tree_pda_token_account.data).unwrap();

    println!("merkle_tree_pda_token_account_data: {:?}", merkle_tree_pda_token_account_data);
    assert_eq!(merkle_tree_pda_token_account_data.amount, 0);

    let relayer_pda_token_account = program_context.banks_client
            .get_account(relayer_pda_token_pubkey)
            .await
            .expect("get_account").unwrap();
    let relayer_pda_token_account_data = spl_token::state::Account::unpack(&relayer_pda_token_account.data).unwrap();

    assert_eq!(relayer_pda_token_account_data.amount, fees);
    let merkle_tree_account_data = program_context.banks_client
            .get_account(merkle_tree_pda_pubkey)
            .await
            .expect("get_account").unwrap();

    //println!("relayer test disabled {:?}", merkle_tree_pda_account.data);
    let path = "tests/merkle_tree_account_data_after_transfer.rs";
    let mut output = File::create(path).ok().unwrap();
    write!(
        output,
        "{}",
        format!(
            "#[cfg(test)]\n
            pub mod merkle_tree_account_data_after_transfer {{ \n
            \t pub const MERKLE_TREE_ACCOUNT_DATA_AFTER_TRANSFER : [u8;{}] = {:?};
            \n }}",
            merkle_tree_account_data.data.len(),
            merkle_tree_account_data.data
        )
    ).unwrap();

}
#[tokio::test]
async fn withdrawal_should_succeed() {
    let ix_withdraw_data = read_test_data(std::string::String::from("withdraw_0_1_sol.txt"));
    let recipient = Pubkey::new(&ix_withdraw_data[489..521]);
    let amount: u64 =  (-i64::from_le_bytes(ix_withdraw_data[521..529].try_into().unwrap())).try_into().unwrap();
    println!("amount: {:?}", amount);
    let fees: u64 =  u64::from_le_bytes(ix_withdraw_data[561..569].try_into().unwrap());

    // Creates program, accounts, setup.
    let program_id = Pubkey::from_str("TransferLamports111111111111111111112111111").unwrap();
    let mut accounts_vector = Vec::new();
    // Creates pubkey for tmporary storage account
    let merkle_tree_pda_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY[0].0);
    accounts_vector.push((
        &merkle_tree_pda_pubkey,
        16658,
        Some(MERKLE_TREE_ACCOUNT_DATA_AFTER_TRANSFER.to_vec()),
    ));

    let signer_keypair = solana_sdk::signer::keypair::Keypair::from_bytes(&PRIVATE_KEY).unwrap();
    let signer_pubkey = signer_keypair.pubkey();

    let (tmp_storage_pda_pubkey, two_leaves_pda_pubkey, nf_pubkey0, nf_pubkey1) =
        create_pubkeys_from_ix_data(&ix_withdraw_data, &program_id).await;
    let mut nullifier_pubkeys = Vec::new();
    nullifier_pubkeys.push(nf_pubkey0);
    nullifier_pubkeys.push(nf_pubkey1);

    //is hardcoded onchain
    let authority_seed = program_id.to_bytes();
    let (expected_authority_pubkey, authority_bump_seed) = Pubkey::find_program_address(&[&authority_seed], &program_id);

    // let (merkle_tree_pda_token_pubkey, bumpSeed_merkle_tree) = Pubkey::find_program_address(
    //    &[&merkle_tree_pda_pubkey.to_bytes()[..]],
    //    &program_id
    // );
    let merkle_tree_pda_token_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY
        [ix_withdraw_data[601] as usize]
    .1);
    let user_pda_token_pubkey =  Keypair::new().pubkey();

    let relayer_pda_token_pubkey =  Keypair::new().pubkey();

    let mut token_accounts = Vec::new();
    token_accounts.push((&merkle_tree_pda_token_pubkey, &expected_authority_pubkey, amount + fees ));
    token_accounts.push((&recipient, &signer_pubkey, 0));
    token_accounts.push((&relayer_pda_token_pubkey, &signer_pubkey, 0));


    // start program
    let mut program_context =
        create_and_start_program_var(&accounts_vector, None, &program_id, &signer_pubkey).await;
    let _merkle_tree_pda = program_context
        .banks_client
        .get_account(merkle_tree_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();

    let merkle_tree_pda_before = program_context
        .banks_client
        .get_account(merkle_tree_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();

    //withdraw from shielded pool
    let mut program_context = transact(
        &program_id,
        &signer_pubkey,
        &signer_keypair,
        &tmp_storage_pda_pubkey,
        &recipient,
        &merkle_tree_pda_pubkey,
        &merkle_tree_pda_token_pubkey,
        &expected_authority_pubkey,
        &nullifier_pubkeys,
        &two_leaves_pda_pubkey,
        Some(&relayer_pda_token_pubkey),
        Some(&recipient),
        ix_withdraw_data.clone(),
        &mut program_context,
        &mut accounts_vector,
        &mut token_accounts,
        1u8,
    )
    .await.unwrap();

        check_nullifier_insert_correct(&nullifier_pubkeys, &mut program_context).await;

        let merkle_tree_pda_after = program_context
            .banks_client
            .get_account(merkle_tree_pda_pubkey)
            .await
            .expect("get_account")
            .unwrap();

        check_tmp_storage_account_state_correct(
            &tmp_storage_pda_pubkey,
            Some(&merkle_tree_pda_before.data),
            Some(&merkle_tree_pda_after.data),
            &mut program_context,
        )
        .await;

        check_leaves_insert_correct(
            &two_leaves_pda_pubkey,
            &ix_withdraw_data[192 + 9..224 + 9], //left leaf todo change order
            &ix_withdraw_data[160 + 9..192 + 9], //right leaf
            0,
            &mut program_context,
        )
        .await;

        let recipient_pda_token_account = program_context.banks_client
            .get_account(recipient)
            .await
            .expect("get_account").unwrap();
        let recipient_token_account_data = spl_token::state::Account::unpack(&recipient_pda_token_account.data).unwrap();
        println!("\recipient: {:?} \n", recipient);

        println!("recipient_token_account_data: {:?}", recipient_token_account_data);
        assert_eq!(recipient_token_account_data.amount, amount);

        println!("\n merkle_tree_pda_token_pubkey: {:?} \n", merkle_tree_pda_token_pubkey);
        let merkle_tree_pda_token_account = program_context.banks_client
                .get_account(merkle_tree_pda_token_pubkey)
                .await
                .expect("get_account").unwrap();
        let merkle_tree_pda_token_account_data = spl_token::state::Account::unpack(&merkle_tree_pda_token_account.data).unwrap();

        println!("merkle_tree_pda_token_account_data: {:?}", merkle_tree_pda_token_account_data);
        assert_eq!(merkle_tree_pda_token_account_data.amount, 0);

        let relayer_pda_token_account = program_context.banks_client
                .get_account(relayer_pda_token_pubkey)
                .await
                .expect("get_account").unwrap();
        let relayer_pda_token_account_data = spl_token::state::Account::unpack(&relayer_pda_token_account.data).unwrap();
        println!("relayer test disabled");

        //assert_eq!(relayer_pda_token_account_data.amount, 1);
}

#[tokio::test]
async fn double_spend_should_not_succeed() {
    let ix_withdraw_data = read_test_data(std::string::String::from("withdraw_0_1_sol.txt"));
    let recipient = Pubkey::from_str("8r4HLLzJkv4WCG5LiAcR4yb5S3uY3X7sqSaQxnDxQ36y").unwrap();
    let amount: u64 =  (-i64::from_le_bytes(ix_withdraw_data[521..529].try_into().unwrap())).try_into().unwrap();
    println!("amount: {:?}", amount);
    let fees: u64 =  u64::from_le_bytes(ix_withdraw_data[561..569].try_into().unwrap());

    // Creates program, accounts, setup.
    let program_id = Pubkey::from_str("TransferLamports111111111111111111112111111").unwrap();
    let mut accounts_vector = Vec::new();
    // Creates pubkey for tmporary storage account
    let merkle_tree_pda_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY[0].0);
    accounts_vector.push((
        &merkle_tree_pda_pubkey,
        16658,
        Some(MERKLE_TREE_ACCOUNT_DATA_AFTER_TRANSFER.to_vec()),
    ));

    let signer_keypair = solana_sdk::signer::keypair::Keypair::from_bytes(&PRIVATE_KEY).unwrap();
    let signer_pubkey = signer_keypair.pubkey();

    let (tmp_storage_pda_pubkey, two_leaves_pda_pubkey, nf_pubkey0, nf_pubkey1) =
        create_pubkeys_from_ix_data(&ix_withdraw_data, &program_id).await;
    let mut nullifier_pubkeys = Vec::new();
    nullifier_pubkeys.push(nf_pubkey0);
    nullifier_pubkeys.push(nf_pubkey1);
    //add nullifier_pubkeys to account vector to mimic their invalidation
    accounts_vector.push((&nullifier_pubkeys[0], 2, Some(vec![1, 0])));
    accounts_vector.push((&nullifier_pubkeys[1], 2, Some(vec![1, 0])));

    //is hardcoded onchain
    let authority_seed = program_id.to_bytes();
    let (expected_authority_pubkey, authority_bump_seed) = Pubkey::find_program_address(&[&authority_seed], &program_id);

    // let (merkle_tree_pda_token_pubkey, bumpSeed_merkle_tree) = Pubkey::find_program_address(
    //    &[&merkle_tree_pda_pubkey.to_bytes()[..]],
    //    &program_id
    // );
    let merkle_tree_pda_token_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY
        [ix_withdraw_data[601] as usize]
    .1);
    let user_pda_token_pubkey =  Keypair::new().pubkey();

    let relayer_pda_token_pubkey =  Keypair::new().pubkey();

    let mut token_accounts = Vec::new();
    token_accounts.push((&merkle_tree_pda_token_pubkey, &expected_authority_pubkey, amount + fees ));
    token_accounts.push((&recipient, &signer_pubkey, 0));
    token_accounts.push((&relayer_pda_token_pubkey, &signer_pubkey, 0));


    // start program
    let mut program_context =
        create_and_start_program_var(&accounts_vector, None, &program_id, &signer_pubkey).await;
    let _merkle_tree_pda = program_context
        .banks_client
        .get_account(merkle_tree_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();

    let merkle_tree_pda_before = program_context
        .banks_client
        .get_account(merkle_tree_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();

    //withdraw from shielded pool
    let mut program_context = transact(
        &program_id,
        &signer_pubkey,
        &signer_keypair,
        &tmp_storage_pda_pubkey,
        &recipient,
        &merkle_tree_pda_pubkey,
        &merkle_tree_pda_token_pubkey,
        &expected_authority_pubkey,
        &nullifier_pubkeys,
        &two_leaves_pda_pubkey,
        Some(&relayer_pda_token_pubkey),
        Some(&recipient),
        ix_withdraw_data.clone(),
        &mut program_context,
        &mut accounts_vector,
        &mut token_accounts,
        1u8,
    )
    .await.unwrap();

    check_nullifier_insert_correct(&nullifier_pubkeys, &mut program_context).await;

    let merkel_tree_pda_after = program_context
        .banks_client
        .get_account(merkle_tree_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();

    //assert current root is the same
    assert_eq!(merkel_tree_pda_after.data[642 + 32..673 + 32], merkle_tree_pda_before.data[642 + 32..673 + 32]);
    //assert root index did not increase

    //checking that no leaves were inserted
    let two_leaves_pda_account = program_context
        .banks_client
        .get_account(two_leaves_pda_pubkey)
        .await
        .unwrap();
    assert_eq!(two_leaves_pda_account.is_none(), true);

    let recipient_pda_token_account = program_context.banks_client
        .get_account(recipient)
        .await
        .expect("get_account").unwrap();
    let recipient_token_account_data = spl_token::state::Account::unpack(&recipient_pda_token_account.data).unwrap();
    println!("\recipient: {:?} \n", recipient);

    println!("recipient_token_account_data: {:?}", recipient_token_account_data);
    assert_eq!(recipient_token_account_data.amount, 0);

}

#[tokio::test]
async fn compute_prepared_inputs_should_succeed() {
    // Creates program, accounts, setup.
    let program_id = Pubkey::from_str("TransferLamports111111111111111111112111111").unwrap();
    //create pubkey for tmporary storage account
    // let tmp_storage_pda_pubkey = Pubkey::new_unique();

    let merkle_tree_pda_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY[0].0);
    let signer_keypair = solana_sdk::signer::keypair::Keypair::from_bytes(&PRIVATE_KEY).unwrap();
    let signer_pubkey = signer_keypair.pubkey();
    // start program the program with the exact account state.
    // ...The account state (current instruction index,...) must match the
    // state we'd have at the exact instruction we're starting the test at (ix 466 for millerloop)
    // read proof, public inputs from test file, prepare_inputs
    let ix_data = read_test_data(String::from("deposit_0_1_sol.txt"));
    let tmp_storage_pda_pubkey =
        Pubkey::find_program_address(&[&ix_data[105..137], &b"storage"[..]], &program_id).0;
    // Pick the data we need from the test file. 9.. bc of input structure

    let prepared_inputs_ref = get_ref_value("prepared_inputs");

    let mut accounts_vector = Vec::new();
    accounts_vector.push((&merkle_tree_pda_pubkey, 16658, None));
    let mut program_context =
        create_and_start_program_var(&accounts_vector, None, &program_id, &signer_pubkey).await;

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
                    solana_program::system_program::id(),
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

    let account_data = PrepareInputsState::unpack(&storage_account.data.clone()).unwrap();
    assert_eq!(
        account_data.x_1_range, prepared_inputs_ref,
        "onchain pi result != reference pi.into:affine()"
    );
}

#[tokio::test]
async fn compute_miller_output_should_succeed() {
    // Creates program, accounts, setup.
    let program_id = Pubkey::from_str("TransferLamports111111111111111111112111111").unwrap();
    // let merkle_tree_pda_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY[0].0);
    let signer_keypair = solana_sdk::signer::keypair::Keypair::from_bytes(&PRIVATE_KEY).unwrap();
    let signer_pubkey = signer_keypair.pubkey();
    // start program the program with the exact account state.
    // ...The account state (current instruction index,...) must match the
    // state we'd have at the exact instruction we're starting the test at (ix 466 for millerloop)
    // read proof, public inputs from test file, prepare_inputs
    let ix_data = read_test_data(String::from("deposit_0_1_sol.txt"));
    //create pubkey for tmporary storage account
    let tmp_storage_pda_pubkey =
        Pubkey::find_program_address(&[&ix_data[105..137], &b"storage"[..]], &program_id).0;

    let account_state = get_mock_state("miller_output", &signer_keypair);
    let mut accounts_vector = Vec::new();
    accounts_vector.push((&tmp_storage_pda_pubkey, 3900, Some(account_state)));
    let mut program_context =
        create_and_start_program_var(&accounts_vector, None, &program_id, &signer_pubkey).await;

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
    let account_data = MillerLoopState::unpack(&storage_account.data.clone()).unwrap();

    let miller_output_ref = get_ref_value("miller_output");
    assert_eq!(
        account_data.f_range, miller_output_ref,
        "onchain f result != reference f"
    );
    println!("onchain test success");
}

#[tokio::test]
async fn compute_final_exponentiation_should_succeed() {
    let program_id = Pubkey::from_str("TransferLamports111111111111111111111111111").unwrap();
    let ix_data = read_test_data(String::from("deposit_0_1_sol.txt"));
    //create pubkey for tmporary storage account
    let tmp_storage_pda_pubkey =
        Pubkey::find_program_address(&[&ix_data[105..137], &b"storage"[..]], &program_id).0;
    let signer_keypair = solana_sdk::signer::keypair::Keypair::from_bytes(&PRIVATE_KEY).unwrap();
    let signer_pubkey = signer_keypair.pubkey();
    let f_ref = get_ref_value("final_exponentiation");
    let account_state = get_mock_state("final_exponentiation", &signer_keypair);
    let mut accounts_vector = Vec::new();
    accounts_vector.push((&tmp_storage_pda_pubkey, 3900, Some(account_state.clone())));
    let mut program_context =
        create_and_start_program_var(&accounts_vector,None, &program_id, &signer_pubkey).await;

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

    let unpacked_data = FinalExponentiationState::unpack(&storage_account.data).unwrap();

    assert_eq!(f_ref, unpacked_data.y1_range);

    // // checking pvk ref
    // let mut pvk_ref = vec![0u8; 384];
    // parse_f_to_bytes(pvk.alpha_g1_beta_g2, &mut pvk_ref);
    // assert_eq!(pvk_ref, unpacked_data.y1_range);
}

#[tokio::test]
async fn submit_proof_with_wrong_root_should_not_succeed() {
    let mut ix_data = read_test_data(String::from("deposit_0_1_sol.txt"));

    //generate random value
    let mut rng = test_rng();
    let rnd_value = Fq::rand(&mut rng).into_repr().to_bytes_le();
    println!("{:?}", ix_data[..32].to_vec());
    //change root in ix_data for random value
    for i in 0..32 {
        ix_data[i] = rnd_value[i];
    }

    // Creates program, accounts, setup.
    let program_id = Pubkey::from_str("TransferLamports111111111111111111112111111").unwrap();
    let mut accounts_vector = Vec::new();
    // Creates pubkey for tmporary storage account
    let merkle_tree_pda_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY[0].0);
    accounts_vector.push((&merkle_tree_pda_pubkey, 16658, None));

    let signer_keypair = solana_sdk::signer::keypair::Keypair::from_bytes(&PRIVATE_KEY).unwrap();
    let signer_pubkey = signer_keypair.pubkey();

    let tmp_storage_pda_pubkey =
        Pubkey::find_program_address(&[&ix_data[105..137], &b"storage"[..]], &program_id).0;

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

    // start program
    let mut program_context =
        create_and_start_program_var(&accounts_vector, None, &program_id, &signer_pubkey).await;

    //push tmp_storage_pda_pubkey after creating program contex such that it is not initialized
    //it will be initialized in the first instruction onchain
    accounts_vector.push((&tmp_storage_pda_pubkey, 3900, None));

    let merkle_tree_pda_before = program_context
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
     * initialize tmporary storage account
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
                    solana_program::system_program::id(),
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
        .expect_err("invalid account data for instruction");

}

#[tokio::test]
async fn signer_acc_not_in_first_place_should_not_succeed() {
    let mut ix_data = read_test_data(String::from("deposit_0_1_sol.txt"));

    //generate random value
    let mut rng = test_rng();
    let rnd_value = Fq::rand(&mut rng).into_repr().to_bytes_le();
    //change root in ix_data for random value
    for i in 0..32 {
        ix_data[i] = rnd_value[i];
    }

    // Creates program, accounts, setup.
    let program_id = Pubkey::from_str("TransferLamports111111111111111111112111111").unwrap();
    let mut accounts_vector = Vec::new();
    // Creates pubkey for tmporary storage account
    let merkle_tree_pda_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY[0].0);
    accounts_vector.push((&merkle_tree_pda_pubkey, 16658, None));

    let signer_keypair = solana_sdk::signer::keypair::Keypair::from_bytes(&PRIVATE_KEY).unwrap();
    let signer_pubkey = signer_keypair.pubkey();

    let tmp_storage_pda_pubkey =
        Pubkey::find_program_address(&[&ix_data[105..137], &b"storage"[..]], &program_id).0;

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

    // start program
    let mut program_context =
        create_and_start_program_var(&accounts_vector, None, &program_id, &signer_pubkey).await;

    //push tmp_storage_pda_pubkey after creating program contex such that it is not initialized
    //it will be initialized in the first instruction onchain
    accounts_vector.push((&tmp_storage_pda_pubkey, 3900, None));

    let merkle_tree_pda_before = program_context
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
     * initialize tmporary storage account
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
                    solana_program::system_program::id(),
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

    let empty_vec = Vec::<u8>::new();
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &[1], //random
            vec![
                AccountMeta::new(signer_pubkey, false),
                AccountMeta::new(tmp_storage_pda_pubkey, false),
                AccountMeta::new(merkle_tree_pda_pubkey, false),
                AccountMeta::new(program_context.payer.pubkey(), true),
            ],
        )],
        Some(&program_context.payer.pubkey()),
    );
    transaction.sign(&[&program_context.payer], program_context.last_blockhash);
    program_context
        .banks_client
        .process_transaction(transaction)
        .await
        .expect_err("Signer in last place is not allowed.");
}

#[tokio::test]
async fn submit_proof_with_wrong_signer_should_not_succeed() {
    let mut ix_data = read_test_data(String::from("deposit_0_1_sol.txt"));

    //generate random value
    let mut rng = test_rng();
    let rnd_value = Fq::rand(&mut rng).into_repr().to_bytes_le();
    println!("{:?}", ix_data[..32].to_vec());
    //change root in ix_data for random value
    for i in 0..32 {
        ix_data[i] = rnd_value[i];
    }

    // Creates program, accounts, setup.
    let program_id = Pubkey::from_str("TransferLamports111111111111111111112111111").unwrap();
    let mut accounts_vector = Vec::new();
    // Creates pubkey for tmporary storage account
    let merkle_tree_pda_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY[0].0);
    accounts_vector.push((&merkle_tree_pda_pubkey, 16658, None));

    let signer_keypair = solana_sdk::signer::keypair::Keypair::from_bytes(&PRIVATE_KEY).unwrap();
    let signer_pubkey = signer_keypair.pubkey();

    let tmp_storage_pda_pubkey =
        Pubkey::find_program_address(&[&ix_data[105..137], &b"storage"[..]], &program_id).0;

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

    // start program
    let mut program_context =
        create_and_start_program_var(&accounts_vector, None, &program_id, &signer_pubkey).await;

    //push tmp_storage_pda_pubkey after creating program contex such that it is not initialized
    //it will be initialized in the first instruction onchain
    accounts_vector.push((&tmp_storage_pda_pubkey, 3900, None));

    let merkle_tree_pda_old = program_context
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
     * initialize tmporary storage account
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
                    solana_program::system_program::id(),
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
                AccountMeta::new(program_context.payer.pubkey(), false),
                AccountMeta::new(tmp_storage_pda_pubkey, false),
                AccountMeta::new(merkle_tree_pda_pubkey, false),
            ],
        )],
        Some(&program_context.payer.pubkey()),
    );
    transaction.sign(&[&program_context.payer], program_context.last_blockhash);
    program_context
        .banks_client
        .process_transaction(transaction)
        .await
        .expect_err("This signer is not allowed.");
}

#[tokio::test]
async fn merkle_tree_insert_should_succeed() {
    let program_id = Pubkey::from_str("TransferLamports111111111111111111111111111").unwrap();

    let tmp_storage_pda_pubkey = Pubkey::new_unique();
    let merkle_tree_pda_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY[0].0);

    let signer_keypair = solana_sdk::signer::keypair::Keypair::from_bytes(&PRIVATE_KEY).unwrap();
    let signer_pubkey = signer_keypair.pubkey();

    let mut account_state = vec![0u8; 3900];
    let x = usize::to_le_bytes(801 + 465);
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
    accounts_vector.push((&merkle_tree_pda_pubkey, 16658, None));
    accounts_vector.push((&tmp_storage_pda_pubkey, 3900, Some(account_state.clone())));

    let mut program_context =
        create_and_start_program_var(&accounts_vector, None, &program_id, &signer_pubkey).await;

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
    let storage_account_unpacked = TmpStoragePda::unpack(&storage_account.data).unwrap();
    assert_eq!(storage_account_unpacked.state[0], expected_root);
}


#[tokio::test]
async fn merkle_tree_init_with_wrong_signer_should_not_succeed() {
    let program_id = Pubkey::from_str("TransferLamports111111111111111111111111111").unwrap();

    let tmp_storage_pda_pubkey = Pubkey::new_unique();
    let merkle_tree_pda_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY[0].0);

    let signer_keypair = solana_sdk::signer::keypair::Keypair::new();
    let signer_pubkey = signer_keypair.pubkey();

    let mut account_state = vec![0u8; 3900];
        let mut accounts_vector = Vec::new();
    accounts_vector.push((&merkle_tree_pda_pubkey, 16658, None));

    let mut program_context =
        create_and_start_program_var(&accounts_vector, None, &program_id, &signer_pubkey).await;

    //initialize MerkleTree account
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &[vec![240u8, 0u8], usize::to_le_bytes(1000).to_vec()].concat(),
            vec![
                AccountMeta::new(signer_keypair.pubkey(), true),
                AccountMeta::new(merkle_tree_pda_pubkey, false),
            ],
        )],
        Some(&signer_keypair.pubkey()),
    );
    program_context
        .banks_client
        .process_transaction(transaction)
        .await
        .expect_err("Signer is not program authority.");

    let merkle_tree_data = program_context
        .banks_client
        .get_account(merkle_tree_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    assert_eq!(
        [0u8;642],
        merkle_tree_data.data[0..642]
    );
    //println!("initializing merkle tree success");

}
