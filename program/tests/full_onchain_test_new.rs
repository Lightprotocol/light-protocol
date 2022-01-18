use crate::tokio::time::timeout;
use ark_crypto_primitives::Error;
use ark_ff::biginteger::BigInteger256;
use ark_ff::{Fp256, FromBytes};
use solana_program::program_pack::Pack;
use Testing_Hardcoded_Params_devnet_new::{
    poseidon_merkle_tree::mt_state::{HashBytes, MerkleTree, MERKLE_TREE_ACC_BYTES},
    process_instruction,
    utils::init_bytes18,
    Groth16_verifier::{
        final_exponentiation::fe_ranges::*,
        final_exponentiation::fe_state::{FinalExpBytes, INSTRUCTION_ORDER_VERIFIER_PART_2},
        miller_loop::ml_state::*,
        parsers::*,
    },
};

use {
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
    },
    solana_program_test::*,
    solana_sdk::{account::Account, signature::Signer, transaction::Transaction},
    std::str::FromStr,
};
use serde_json::{Result, Value};

use ark_ed_on_bn254::Fq;
use ark_ff::BigInteger;
use solana_program_test::ProgramTestContext;
use std::convert::TryInto;
use std::{fs, time};
mod fe_onchain_test;

//mod tests::fe_onchain_test;
//use crate::fe_onchain_test;
mod mt_onchain_test;
mod pi_onchain_test;
// mod fe_offchain_test;
// use crate::fe_offchain_test::tests::get_public_inputs_from_bytes_254;

pub async fn create_and_start_program(
    merkle_tree_init_bytes: Vec<u8>,
    hash_bytes_init_bytes: Vec<u8>,
    merkle_tree_pubkey: &Pubkey,
    storage_account: &Pubkey,
    program_id: &Pubkey,
    signer_pubkey: &Pubkey,
) -> ProgramTestContext {
    let mut program_test = ProgramTest::new(
        "Testing_Hardcoded_Params_devnet_new",
        *program_id,
        processor!(process_instruction),
    );
    let mut merkle_tree = Account::new(10000000000, 16657, &program_id);

    if merkle_tree_init_bytes.len() == 16657 {
        merkle_tree.data = merkle_tree_init_bytes;
    }
    program_test.add_account(*merkle_tree_pubkey, merkle_tree);
    let mut hash_byte = Account::new(10000000000, 3900, &program_id);

    if hash_bytes_init_bytes.len() == 3900 {
        hash_byte.data = hash_bytes_init_bytes;
    }
    program_test.add_account(*storage_account, hash_byte);
    //let mut two_leaves_pda_byte = Account::new(10000000000, 98, &program_id);

    // if two_leaves_pda_bytes_init_bytes.len() == 98 {
    //
    //     two_leaves_pda_byte.data = two_leaves_pda_bytes_init_bytes;
    // }
    // program_test.add_account(
    //     *two_leaves_pda_pubkey,
    //     two_leaves_pda_byte,
    // );

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

pub async fn restart_program(
    accounts_vector: &mut Vec<(&Pubkey, usize, Option<Vec<u8>>)>,
    program_id: &Pubkey,
    signer_pubkey: &Pubkey,
    mut program_context: ProgramTestContext
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
    let mut program_context_new = create_and_start_program_var(
        &accounts_vector,
        &program_id,
        &signer_pubkey,
    ).await;
    program_context_new
}

//use core::num::<impl u64>::checked_add;
#[tokio::test]
async fn full_test_onchain_new() {

    //getting instruction data from file
    //this is necessary for Light only supports proof generation with snarkjs
    let ix_data_file = fs::read_to_string("./tests/test_data/deposit_0_1_sol.txt")
        .expect("Something went wrong reading the file");
    let ix_data_json: Value = serde_json::from_str(&ix_data_file).unwrap();
    let mut ix_data = Vec::new();
    // for i in  tx_bytes["bytes"][0].as_str().unwrap().split(',') {
    //     bytes.push((*i).parse::<u8>().unwrap());
    // }
    for i in  ix_data_json["bytes"][0].as_str().unwrap().split(',') {
        let j = (*i).parse::<u8>();
        match j {
            Ok(x) => (ix_data.push(x)),
            Err(e) => (),
        }
    }
    println!("{:?}", ix_data);

    // Creates program, accounts, setup.
    let program_id = Pubkey::from_str("TransferLamports111111111111111111112111111").unwrap();
    let mut accounts_vector = Vec::new();
    //create pubkey for temporary storage account
    let merkle_tree_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES);
    accounts_vector.push((&merkle_tree_pubkey, 16657, None));
    let signer_keypair = solana_sdk::signer::keypair::Keypair::new();
    let signer_pubkey = signer_keypair.pubkey();
    let two_leaves_pda_pubkey = Pubkey::find_program_address(&[&ix_data[105..137], &b"leaves"[..]], &program_id).0;
    println!("leaves add seed: {:?}", [ &ix_data[105..137], &b"leaves"[..]]);
    let storage_pubkey = Pubkey::find_program_address(&[&ix_data[105..137], &b"storage"[..]], &program_id).0;
    println!("storage_pubkey add seed: {:?}", [&ix_data[105..137], &b"storage"[..]]);

    let mut nullifier_pubkeys = Vec::new();
    println!("nf0 seed data: {:?}", ix_data[96 + 9..128 + 9].to_vec());
    let pubkey_from_seed = Pubkey::find_program_address(&[&ix_data[96 + 9..128 + 9], &b"nf"[..]], &program_id);
    nullifier_pubkeys.push(pubkey_from_seed.0);

    let pubkey_from_seed = Pubkey::find_program_address(&[&ix_data[128 + 9..160 + 9], &b"nf"[..]], &program_id);
    nullifier_pubkeys.push(pubkey_from_seed.0);
    println!("derriving nullifier pubkeys from: {:?}", nullifier_pubkeys);

    //panic!();
    let mut program_context = create_and_start_program_var(
        &accounts_vector,
        &program_id,
        &signer_pubkey,
    ).await;
    let merkle_tree_account = program_context
        .banks_client
        .get_account(merkle_tree_pubkey)
        .await
        .expect("get_account")
        .unwrap();

    //let tx_bytes = tx_bytes["bytes"]

    // println!("yy: {:?}", yy);
    //initialize MerkleTree account
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &[vec![240u8, 0u8], usize::to_le_bytes(1000).to_vec()].concat(),
            vec![
                AccountMeta::new(signer_keypair.pubkey(), true),
                AccountMeta::new(merkle_tree_pubkey, false),
            ],
        )],
        Some(&signer_keypair.pubkey()),
    );
    transaction.sign(&[&signer_keypair], program_context.last_blockhash);

    program_context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
    let merkle_tree_account = program_context
        .banks_client
        .get_account(merkle_tree_pubkey)
        .await
        .expect("get_account")
        .unwrap();

    /*
     *
     *
     * Send data to chain and initialize temp storage account
     *
     *
     */
    //first instruction + prepare inputs id + 7 public inputs in bytes = 226 bytes

    //sends bytes
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &ix_data[8..].to_vec(),
            vec![
                AccountMeta::new(signer_pubkey, true),
                AccountMeta::new(storage_pubkey, false),
                AccountMeta::new(Pubkey::from_str("11111111111111111111111111111111").unwrap(), false),
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


    accounts_vector.push((&storage_pubkey, 3900, None));


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
            &ix_data[8..20].to_vec(),//random
            vec![
                AccountMeta::new(signer_pubkey, true),
                AccountMeta::new(storage_pubkey, false),
                AccountMeta::new(merkle_tree_pubkey, false),
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
    //assert!(true == false);

    /*
     *
     *
     * Proof Verification
     *
     *
     */

    let mut i = 0usize;
    for id in 0..464usize {
        // 0..912 @working
        // 0..1808 @
        let mut success = false;
        let mut retries_left = 2;
        while retries_left > 0 && success != true {
            let idd: u8 = id as u8;
            let mut transaction = Transaction::new_with_payer(
                &[Instruction::new_with_bincode(
                    program_id,
                    &vec![98, 99, i],
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
                Err(e) => {
                    println!("retries_left {}", retries_left);
                    retries_left -= 1;

                    program_context = restart_program(
                        &mut accounts_vector,
                        &program_id,
                        &signer_pubkey,
                        program_context
                    ).await;

                }
            }
        }
        i += 1;
    }

    // Gets bytes that resemble x_1_range in the account: g_ic value after final compuation.
    // Compute the affine value from this and compare to the (hardcoded) value that's returned from
    // prepare_inputs lib call/reference.
    let storage_account = program_context
        .banks_client
        .get_account(storage_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    let mut unpacked_data = vec![0u8; 3900];
    unpacked_data = storage_account.data.clone();

    // x_1_range: 252..316.
    // Keep in mind that g_ic_reference_value is based on running groth16.prepare_inputs() with 7 hardcoded inputs.
    let g_ic_projective = parse_x_group_affine_from_bytes(&unpacked_data[252..316].to_vec());
    let g_ic_reference_value =
        ark_ec::short_weierstrass_jacobian::GroupProjective::<ark_bn254::g1::Parameters>::new(
            Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
                4558364185828577028,
                2968328072905581441,
                15831331149718564992,
                1208602698044891702,
            ])), // Cost: 31
            Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
                15482105712819104980,
                10686255431817088435,
                17716373216643709577,
                264028719181254570,
            ])), // Cost: 31
            Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
                13014122463130548586,
                16367981906331090583,
                13731940001588685782,
                2029626530375041604,
            ])), // Cost: 31
        );
    // assert_eq!(
    //     g_ic_projective, g_ic_reference_value,
    //     "different g_ic projective than libray implementation with the same inputs"
    // );

    /*
     *
     *
     *Miller loop
     *
     *
     */

    // Executes first ix: [0]
    let i_data0: Vec<u8> = vec![0; 2];
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &[vec![1, 1], i_data0].concat(),
            vec![
                AccountMeta::new(signer_pubkey, true),
                AccountMeta::new(storage_pubkey, false),
                //AccountMeta::new(storage_pubkey, false),
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

    // Executes second ix: [1]
    // Parses proof_a and proof_c bytes ()
    let i_data: Vec<u8> = vec![0]; //[proof_a_bytes, proof_c_bytes].concat(); // 128 b
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &[vec![1, 1], i_data].concat(), // 129++
            vec![
                AccountMeta::new(signer_pubkey, true),
                AccountMeta::new(storage_pubkey, false),
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

    // Executes third ix [2]
    // Parses proof_b_bytes (2..194) // 128 b
    let i_data_2: Vec<u8> = vec![0]; //proof_b_bytes[..].to_vec();
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &[vec![1, 1], i_data_2].concat(),
            vec![
                AccountMeta::new(signer_pubkey, true),
                AccountMeta::new(storage_pubkey, false),
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

    let storage_account = program_context
        .banks_client
        .get_account(storage_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    let account_data = ML254Bytes::unpack(&storage_account.data.clone()).unwrap();
    println!("init state f_range: {:?}", account_data.f_range);
    println!("init state P1x: {:?}", account_data.p_1_x_range);
    println!("init state P1y: {:?}", account_data.p_1_y_range);

    println!("init state P2x: {:?}", account_data.p_2_x_range);
    println!("init state P2y: {:?}", account_data.p_2_y_range);

    println!("init state P3x: {:?}", account_data.p_3_x_range);
    println!("init state P3y: {:?}", account_data.p_3_y_range);

    println!("init state PROOFB: {:?}", account_data.proof_b);
    //assert_eq!(true, false);
    // Executes 1973 following ix.
    println!("xxxxx");
    let mut i = 0usize;
    for _id in 3..431usize {
        // 3..612 @merging helpers and add step
        // 3..639 @14,15,16 merged
        // 3..693 11,12,13 merged
        // 3..821 @3,4 merged
        // 3..884 @d1-d5 merged
        // 3..1157 @d2-d5 merged
        // 3..1976
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
                    // program_context = create_and_start_program(
                    //     storage_account.data.to_vec(),
                    //     storage_pubkey,
                    //     pi_bytes_pubkey,
                    //     program_id,
                    // )
                    // .await;
                    program_context = create_and_start_program(
                        merkle_tree_account.data.to_vec(),
                        storage_account.data.to_vec(),
                        &merkle_tree_pubkey,
                        &storage_pubkey,
                        &program_id,
                        &signer_pubkey,
                    )
                    .await;
                }
            }
        }
        i += 1;
    }

    // Compute the affine value from this and compare to the (hardcoded) value that's returned from
    // prepare_inputs lib call/reference.
    let storage_account = program_context
        .banks_client
        .get_account(storage_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    let account_data = ML254Bytes::unpack(&storage_account.data.clone()).unwrap();
    //println!("account_data.f_range: {:?}", account_data.f_range);

    // = ark_groth16-miller_output reference
    let reference_f = [106, 144, 87, 58, 158, 145, 226, 63, 202, 195, 194, 54, 236, 52, 22, 124, 12, 243, 67, 209, 110, 138, 149, 137, 17, 147, 150, 72, 148, 22, 101, 47, 122, 190, 109, 122, 161, 30, 171, 178, 184, 89, 27, 154, 54, 115, 196, 79, 92, 15, 217, 7, 74, 206, 234, 6, 126, 240, 126, 205, 197, 167, 144, 24, 34, 158, 136, 249, 93, 100, 136, 41, 219, 142, 182, 14, 170, 147, 73, 190, 40, 241, 16, 103, 202, 220, 24, 78, 103, 103, 240, 211, 137, 231, 213, 17, 234, 17, 19, 74, 235, 65, 155, 173, 26, 145, 147, 137, 52, 42, 89, 125, 41, 233, 73, 85, 242, 208, 149, 19, 19, 82, 84, 141, 218, 109, 136, 18, 70, 18, 11, 3, 153, 147, 17, 37, 171, 63, 128, 126, 102, 178, 76, 135, 16, 184, 100, 240, 195, 7, 51, 21, 194, 170, 149, 105, 27, 145, 230, 14, 18, 172, 134, 204, 66, 85, 62, 91, 111, 119, 233, 16, 244, 27, 78, 58, 245, 78, 47, 170, 16, 94, 101, 102, 107, 6, 203, 99, 180, 148, 42, 1, 147, 112, 231, 85, 35, 119, 60, 179, 99, 13, 9, 39, 165, 184, 125, 221, 221, 59, 4, 203, 220, 32, 120, 57, 192, 35, 42, 0, 89, 20, 77, 34, 125, 35, 28, 101, 127, 195, 73, 148, 201, 28, 22, 50, 88, 53, 65, 158, 60, 227, 253, 27, 204, 82, 214, 150, 129, 75, 57, 74, 22, 43, 43, 15, 254, 162, 118, 165, 240, 45, 110, 35, 244, 97, 240, 218, 57, 148, 236, 60, 248, 234, 68, 31, 213, 236, 21, 55, 224, 59, 236, 5, 54, 89, 39, 38, 238, 70, 134, 88, 195, 238, 15, 204, 151, 184, 61, 152, 210, 119, 143, 81, 209, 46, 134, 243, 207, 108, 207, 30, 217, 56, 167, 230, 118, 143, 85, 15, 180, 102, 18, 130, 107, 128, 118, 5, 108, 181, 192, 200, 46, 63, 73, 147, 55, 84, 193, 251, 24, 177, 79, 206, 82, 104, 156, 138, 197, 202, 245, 2, 236, 253, 116, 107, 179, 247, 76, 70, 206, 73, 248, 6, 219, 46, 217, 134, 253, 222, 205, 200, 230, 21, 149, 140, 244, 106, 194, 8, 203, 232, 243, 12];
    assert_eq!(
        account_data.f_range, reference_f,
        "onchain f result != reference f (hardcoded from lib call)"
    );
    println!("onchain test success");
    // println!("Final exp init bytes:  {:?}", storage_account.data);
    // assert_eq!(true, false);
    //assert_eq!(true, false);

    /*
     *
     * Final Exponentiation
     *
     */

    let mut i = 0usize;
    for (instruction_id) in INSTRUCTION_ORDER_VERIFIER_PART_2 {
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
                    println!("retries_left {}", retries_left);
                    retries_left -= 1;
                    let storage_account = program_context
                        .banks_client
                        .get_account(storage_pubkey)
                        .await
                        .expect("get_account")
                        .unwrap();
                    //println!("data: {:?}", storage_account.data);
                    program_context = create_and_start_program(
                        merkle_tree_account.data.to_vec(),
                        storage_account.data.to_vec(),
                        &merkle_tree_pubkey,
                        &storage_pubkey,
                        &program_id,
                        &signer_pubkey,
                    )
                    .await;
                }
            }
        }
        // if i == 3 {
        //     println!("aborted at {}", i);
        //     break;
        // }
        i += 1;
    }

    let storage_account = program_context
        .banks_client
        .get_account(storage_pubkey)
        .await
        .expect("get_account")
        .unwrap();

    let result = FinalExpBytes::unpack(&storage_account.data.clone()).unwrap();
    let expected_result_bytes = vec![
        198, 242, 4, 28, 9, 35, 146, 101, 152, 133, 231, 128, 253, 46, 174, 170, 116, 96, 135, 45,
        77, 156, 161, 40, 238, 232, 55, 247, 15, 79, 136, 20, 73, 78, 229, 119, 48, 86, 133, 39,
        142, 172, 194, 67, 33, 2, 66, 111, 127, 20, 159, 85, 92, 82, 21, 187, 149, 99, 99, 91, 169,
        57, 127, 10, 238, 159, 54, 204, 152, 63, 242, 50, 16, 39, 141, 61, 149, 81, 36, 246, 69, 1,
        232, 157, 153, 3, 1, 25, 105, 84, 109, 205, 9, 78, 8, 26, 113, 240, 149, 249, 171, 170, 41,
        39, 144, 143, 89, 229, 207, 106, 60, 195, 236, 5, 73, 82, 126, 170, 50, 181, 192, 135, 129,
        217, 185, 227, 223, 0, 50, 203, 114, 165, 128, 252, 58, 245, 74, 48, 92, 144, 199, 108,
        126, 82, 103, 46, 23, 236, 159, 71, 113, 45, 183, 105, 200, 135, 142, 182, 196, 3, 138,
        113, 217, 236, 105, 118, 157, 226, 54, 90, 23, 215, 59, 110, 169, 133, 96, 175, 12, 86, 33,
        94, 130, 8, 57, 246, 139, 86, 246, 147, 174, 17, 57, 27, 122, 247, 174, 76, 162, 173, 26,
        134, 230, 177, 70, 148, 183, 2, 54, 46, 65, 165, 64, 15, 42, 11, 245, 15, 136, 32, 213,
        228, 4, 27, 176, 63, 169, 82, 178, 89, 227, 58, 204, 40, 159, 210, 216, 255, 223, 194, 117,
        203, 57, 49, 152, 42, 162, 80, 248, 55, 92, 240, 231, 192, 161, 14, 169, 65, 231, 215, 238,
        131, 144, 139, 153, 142, 76, 100, 40, 134, 147, 164, 89, 148, 195, 194, 117, 36, 53, 100,
        231, 61, 164, 217, 129, 190, 160, 44, 30, 94, 13, 159, 6, 83, 126, 195, 26, 86, 113, 177,
        101, 79, 110, 143, 220, 57, 110, 235, 91, 73, 189, 191, 253, 187, 76, 214, 232, 86, 132, 6,
        135, 153, 111, 175, 12, 109, 157, 73, 181, 171, 29, 118, 147, 102, 65, 153, 99, 57, 198,
        45, 85, 153, 67, 208, 177, 113, 205, 237, 210, 233, 79, 46, 231, 168, 16, 11, 21, 249, 174,
        127, 70, 3, 32, 60, 115, 188, 192, 101, 159, 85, 66, 193, 194, 157, 76, 121, 108, 222, 128,
        27, 15, 163, 156, 8,
    ];
    println!("result.y1_range_s: {:?}", parse_f_from_bytes(&result.y1_range_s));

    //assert_eq!(expected_result_bytes, result.y1_range_s);

    /*
     *
     * Merkle Tree insert of new utxos
     *
     */

    let commit = vec![0u8; 32]; //vec![143, 120, 199, 24, 26, 175, 31, 125, 154, 127, 245, 235, 132, 57, 229, 4, 60, 255, 3, 234, 105, 16, 109, 207, 16, 139, 73, 235, 137, 17, 240, 2];//get_poseidon_ref_hash(&left_input[..], &right_input[..]);

    let mut i = 0;
    for (instruction_id) in 0..237 {
        //println!("instruction data {:?}", [vec![*instruction_id, 0u8], left_input.clone(), right_input.clone(), [i as u8].to_vec() ].concat());
        let instruction_data: Vec<u8> = [
            vec![instruction_id, 0u8],
            commit.clone(),
            commit.clone(),
            [i as u8].to_vec(),
        ]
        .concat();

        if i == 0 {
            let mut transaction = Transaction::new_with_payer(
                &[Instruction::new_with_bincode(
                    program_id,
                    &instruction_data,
                    vec![
                        AccountMeta::new(signer_keypair.pubkey(), true),
                        AccountMeta::new(storage_pubkey, false),
                        AccountMeta::new(merkle_tree_pubkey, false),
                        AccountMeta::new(storage_pubkey, false),
                    ],
                )],
                Some(&signer_keypair.pubkey()),
            );
            transaction.sign(&[&signer_keypair], program_context.last_blockhash);

            program_context
                .banks_client
                .process_transaction(transaction)
                .await
                .unwrap();
        } else if i == init_bytes18::INSERT_INSTRUCTION_ORDER_18.len() - 1 {
            println!("Last tx ------------------------------");
            let mut transaction = Transaction::new_with_payer(
                &[Instruction::new_with_bincode(
                    program_id,
                    &instruction_data,
                    vec![
                        AccountMeta::new(signer_keypair.pubkey(), true),
                        AccountMeta::new(storage_pubkey, false),
                        AccountMeta::new(merkle_tree_pubkey, false),
                        AccountMeta::new(two_leaves_pda_pubkey, false),
                    ],
                )],
                Some(&signer_keypair.pubkey()),
            );
            transaction.sign(&[&signer_keypair], program_context.last_blockhash);

            program_context
                .banks_client
                .process_transaction(transaction)
                .await
                .unwrap();
        } else {
            let mut success = false;
            let mut retries_left = 2;
            while (retries_left > 0 && success != true) {
                let mut transaction = Transaction::new_with_payer(
                    &[Instruction::new_with_bincode(
                        program_id,
                        &instruction_data,
                        vec![
                            AccountMeta::new(signer_keypair.pubkey(), true),
                            AccountMeta::new(storage_pubkey, false),
                            AccountMeta::new(merkle_tree_pubkey, false),
                        ],
                    )],
                    Some(&signer_keypair.pubkey()),
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
                    Err(e) => {
                        println!("retries_left {}", retries_left);
                        retries_left -= 1;
                        let merkle_tree_account = program_context
                            .banks_client
                            .get_account(merkle_tree_pubkey)
                            .await
                            .expect("get_account")
                            .unwrap();
                        let hash_bytes_account = program_context
                            .banks_client
                            .get_account(storage_pubkey)
                            .await
                            .expect("get_account")
                            .unwrap();
                        //println!("data: {:?}", storage_account.data);
                        //let old_payer = signer_keypair;
                        program_context = create_and_start_program(
                            merkle_tree_account.data.to_vec(),
                            hash_bytes_account.data.to_vec(),
                            &merkle_tree_pubkey,
                            &storage_pubkey,
                            //&two_leaves_pda_pubkey,
                            &program_id,
                            &signer_pubkey,
                        )
                        .await;
                        //assert_eq!(signer_keypair, old_payer);
                        let merkle_tree_account_new = program_context
                            .banks_client
                            .get_account(merkle_tree_pubkey)
                            .await
                            .expect("get_account")
                            .unwrap();
                        let hash_bytes_account_new = program_context
                            .banks_client
                            .get_account(storage_pubkey)
                            .await
                            .expect("get_account")
                            .unwrap();
                        assert_eq!(merkle_tree_account_new.data, merkle_tree_account.data);
                        assert_eq!(hash_bytes_account_new.data, hash_bytes_account.data);
                    }
                }
            }
        }
        println!("Instruction index {}", i);
        i += 1;
    }
    let storage_account = program_context
        .banks_client
        .get_account(merkle_tree_pubkey)
        .await
        .expect("get_account")
        .unwrap();

    let expected_root = [
        247, 16, 124, 67, 44, 62, 195, 226, 182, 62, 41, 237, 78, 64, 195, 249, 67, 169, 200, 24,
        158, 153, 57, 144, 24, 245, 131, 44, 127, 129, 44, 10,
    ];
    //assert_eq!(expected_root, storage_account.data[609 +32..(609+64)]);

    println!("finished merkle tree calculations");

    /*
     *
     *
     * Inserting Merkle root and transferring funds
     *
     *
     */


    let storage_account = program_context
        .banks_client
        .get_account(storage_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    let merkle_tree_account_old = program_context
        .banks_client
        .get_account(merkle_tree_pubkey)
        .await
        .expect("get_account")
        .unwrap();

    let receiver_pubkey = Pubkey::new_unique();

    let merkle_tree_account = program_context
        .banks_client
        .get_account(merkle_tree_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    assert_eq!(merkle_tree_account.data, merkle_tree_account_old.data);
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &[0],
            vec![
                AccountMeta::new(signer_keypair.pubkey(), true),
                AccountMeta::new(storage_pubkey, false),
                AccountMeta::new(two_leaves_pda_pubkey, false),
                AccountMeta::new(nullifier_pubkeys[0], false),
                AccountMeta::new(nullifier_pubkeys[1], false),
                AccountMeta::new(merkle_tree_pubkey, false),
                AccountMeta::new(Pubkey::from_str("11111111111111111111111111111111").unwrap(), false),
                AccountMeta::new(merkle_tree_pubkey, false),
            ],
        )],
        Some(&signer_keypair.pubkey()),
    );
    transaction.sign(&[&signer_keypair], program_context.last_blockhash);

    let res_request = timeout(
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
        .get_account(merkle_tree_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    println!("root[0]: {:?}", merkel_tree_account_new.data[609..641].to_vec());
    println!("root[1]: {:?}", merkel_tree_account_new.data[641..673].to_vec());
    let two_leaves_pda_account = program_context
        .banks_client
        .get_account(two_leaves_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    println!("two_leaves_pda_account.data: {:?}", two_leaves_pda_account.data);
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
        merkel_tree_account_new.lamports , merkle_tree_account_old.lamports + 100000000
    );
    if merkel_tree_account_new.lamports != merkle_tree_account_old.lamports + 100000000 {
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
