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
        final_exponentiation::ranges::*,
        final_exponentiation::state::{FinalExpBytes, INSTRUCTION_ORDER_VERIFIER_PART_2},
        miller_loop::state::*,
        parsers::*,
    },
};

use serde_json::{Result, Value};
use {
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
    },
    solana_program_test::*,
    solana_sdk::{account::Account, signature::Signer, transaction::Transaction},
    std::str::FromStr,
};

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

pub async fn create_and_start_program_with_nullfier_pdas(
    merkle_tree_init_bytes: Vec<u8>,
    hash_bytes_init_bytes: Vec<u8>,
    merkle_tree_pubkey: &Pubkey,
    storage_account: &Pubkey,
    two_leaves_pda_pubkey: &Pubkey,
    nullifier_pubkeys: &Vec<Pubkey>,
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
    let mut two_leaves_pda_byte = Account::new(1100000000, 106, &program_id);

    // if two_leaves_pda_bytes_init_bytes.len() == 98 {
    //
    //     two_leaves_pda_byte.data = two_leaves_pda_bytes_init_bytes;
    // }
    program_test.add_account(*two_leaves_pda_pubkey, two_leaves_pda_byte);

    for pubkey in nullifier_pubkeys.iter() {
        program_test.add_account(*pubkey, Account::new(10000000, 2, &program_id));
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

#[tokio::test]
async fn full_test_onchain_new() {
    // Creates program, accounts, setup.
    let program_id = Pubkey::from_str("TransferLamports111111111111111111112111111").unwrap();

    //create pubkey for temporary storage account
    let storage_pubkey = Pubkey::new_unique();
    let merkle_tree_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES);

    let signer_keypair = solana_sdk::signer::keypair::Keypair::new();
    let signer_pubkey = signer_keypair.pubkey();

    let mut program_context = create_and_start_program(
        vec![0],
        vec![0],
        &merkle_tree_pubkey,
        &storage_pubkey,
        &program_id,
        &signer_pubkey,
    )
    .await;
    let ix_data_file = fs::read_to_string("./tests/test_data/deposit_0_1_sol.txt")
        .expect("Something went wrong reading the file");
    let ix_data_json: Value = serde_json::from_str(&ix_data_file).unwrap();
    let mut ix_data = Vec::new();
    // for i in  tx_bytes["bytes"][0].as_str().unwrap().split(',') {
    //     bytes.push((*i).parse::<u8>().unwrap());
    // }
    for i in ix_data_json["bytes"][0].as_str().unwrap().split(',') {
        let j = (*i).parse::<u8>();
        match j {
            Ok(x) => (ix_data.push(x)),
            Err(e) => (),
        }
    }
    println!("{:?}", ix_data);
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
     * Send data to chain
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
            let storage_account = program_context
                .banks_client
                .get_account(storage_pubkey)
                .await
                .expect("get_account")
                .unwrap();
            //println!("")
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
                    // program_context = pi_onchain_test::create_and_start_program(
                    //     storage_account.data.to_vec(),
                    //     storage_pubkey,
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

    println!("account data after p_i: {:?}", unpacked_data);

    // x_1_range: 252..316.
    // Keep in mind that g_ic_reference_value is based on running groth16.prepare_inputs() with 7 hardcoded inputs.
    let g_ic_affine = parse_x_group_affine_from_bytes(&unpacked_data[252..316].to_vec());
    // let g_ic_affine = parse_x_group_affine_from_bytes(&unpacked_data[220..252].to_vec()); // this new

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
    // let i_data0: Vec<u8> = vec![1, 2, 3, 4, 5];
    // let mut transaction = Transaction::new_with_payer(
    //     &[Instruction::new_with_bincode(
    //         program_id,
    //         &[vec![1, 1], i_data0].concat(),
    //         vec![
    //             AccountMeta::new(signer_pubkey, true),
    //             AccountMeta::new(storage_pubkey, false),
    //             //AccountMeta::new(storage_pubkey, false),
    //         ],
    //     )],
    //     Some(&signer_pubkey),
    // );
    // transaction.sign(&[&signer_keypair], program_context.last_blockhash);
    // program_context
    //     .banks_client
    //     .process_transaction(transaction)
    //     .await
    //     .unwrap();

    // Executes second ix: [1]
    // Parses proof_a and proof_c bytes ()
    // let i_data: Vec<u8> = vec![0]; //[proof_a_bytes, proof_c_bytes].concat(); // 128 b
    // let mut transaction = Transaction::new_with_payer(
    //     &[Instruction::new_with_bincode(
    //         program_id,
    //         &[vec![1, 1], i_data].concat(), // 129++
    //         vec![
    //             AccountMeta::new(signer_pubkey, true),
    //             AccountMeta::new(storage_pubkey, false),
    //         ],
    //     )],
    //     Some(&signer_pubkey),
    // );
    // transaction.sign(&[&signer_keypair], program_context.last_blockhash);
    // program_context
    //     .banks_client
    //     .process_transaction(transaction)
    //     .await
    //     .unwrap();

    // Executes third ix [2]
    // Parses proof_b_bytes (2..194) // 128 b
    // let i_data_2: Vec<u8> = vec![1]; //proof_b_bytes[..].to_vec();
    // let mut transaction = Transaction::new_with_payer(
    //     &[Instruction::new_with_bincode(
    //         program_id,
    //         &[vec![1, 1], i_data_2].concat(),
    //         vec![
    //             AccountMeta::new(signer_pubkey, true),
    //             AccountMeta::new(storage_pubkey, false),
    //         ],
    //     )],
    //     Some(&signer_pubkey),
    // );
    // transaction.sign(&[&signer_keypair], program_context.last_blockhash);
    // program_context
    //     .banks_client
    //     .process_transaction(transaction)
    //     .await
    //     .unwrap();

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
    let mut i = 8888usize;
    for _id in 0..430usize {
        // +1 ! 341
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

    // = ark_groth16-miller_output reference
    let reference_f = [
        41, 164, 125, 219, 237, 181, 202, 195, 98, 55, 97, 232, 35, 147, 153, 23, 164, 70, 211,
        144, 151, 9, 219, 197, 234, 13, 164, 242, 67, 59, 148, 5, 132, 108, 82, 161, 228, 167, 20,
        24, 207, 201, 203, 25, 249, 125, 54, 96, 182, 231, 150, 215, 149, 43, 216, 0, 36, 166, 232,
        13, 126, 3, 53, 0, 174, 209, 16, 242, 177, 143, 60, 247, 181, 65, 132, 142, 14, 231, 170,
        52, 3, 34, 70, 49, 210, 158, 211, 173, 165, 155, 219, 80, 225, 32, 64, 8, 65, 139, 16, 138,
        240, 218, 36, 220, 8, 100, 236, 141, 1, 223, 60, 59, 24, 38, 90, 254, 47, 91, 205, 228,
        169, 103, 178, 30, 124, 141, 43, 9, 83, 155, 75, 140, 209, 26, 2, 250, 250, 20, 185, 78,
        53, 54, 68, 178, 88, 78, 246, 132, 97, 167, 124, 253, 96, 26, 213, 99, 157, 155, 40, 9, 60,
        139, 112, 126, 230, 195, 217, 125, 68, 169, 208, 149, 175, 33, 226, 17, 47, 132, 8, 154,
        237, 156, 34, 97, 55, 129, 155, 64, 202, 54, 161, 19, 24, 1, 208, 104, 140, 149, 25, 229,
        96, 239, 202, 24, 235, 221, 133, 137, 30, 226, 62, 112, 26, 58, 1, 85, 207, 182, 41, 213,
        42, 72, 139, 41, 108, 152, 252, 164, 121, 76, 17, 62, 147, 226, 220, 79, 236, 132, 109,
        130, 163, 209, 203, 14, 144, 180, 25, 216, 234, 198, 199, 74, 48, 62, 57, 0, 206, 138, 12,
        130, 25, 12, 187, 216, 86, 208, 84, 198, 58, 204, 6, 161, 93, 63, 68, 121, 173, 129, 255,
        249, 47, 42, 218, 214, 129, 29, 136, 7, 213, 160, 139, 148, 58, 6, 191, 11, 161, 114, 56,
        174, 224, 86, 243, 103, 166, 151, 107, 36, 205, 170, 206, 196, 248, 251, 147, 91, 3, 136,
        208, 36, 3, 51, 84, 102, 139, 252, 193, 9, 172, 113, 116, 50, 242, 70, 26, 115, 166, 252,
        204, 163, 149, 78, 13, 255, 235, 222, 174, 120, 182, 178, 186, 22, 169, 153, 73, 48, 242,
        139, 120, 98, 33, 101, 204, 204, 169, 57, 249, 168, 45, 197, 126, 105, 54, 187, 35, 241,
        253, 4, 33, 70, 246, 206, 32, 17,
    ];
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

    //assert_eq!(expected_result_bytes, result.y1_range_s);

    /*
     *
     * Merkle Tree insert of new utxos
     *
     */

    let two_leaves_pda_pubkey = Pubkey::new_unique();

    //restart is necessary to add pda account
    let mut program_context = mt_onchain_test::create_and_start_program(
        merkle_tree_account.data.to_vec(),
        storage_account.data.clone(),
        &merkle_tree_pubkey,
        &storage_pubkey,
        &two_leaves_pda_pubkey,
        &program_id,
        &signer_pubkey,
    )
    .await;

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
                        program_context = mt_onchain_test::create_and_start_program(
                            merkle_tree_account.data.to_vec(),
                            hash_bytes_account.data.to_vec(),
                            &merkle_tree_pubkey,
                            &storage_pubkey,
                            &two_leaves_pda_pubkey,
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
    /*
    let nullifer0 = <Fq as FromBytes>::read(&*public_inputs_bytes[96..128].to_vec().clone()).unwrap();
    let nullifer1 = <Fq as FromBytes>::read(&*public_inputs_bytes[128..160].to_vec().clone()).unwrap();
    //let hash = <Fq as FromBytes>::read(_instruction_data).unwrap();
    let mut nullifier_pubkeys = Vec::new();

    let pubkey_from_seed =
        Pubkey::create_with_seed(&signer_pubkey, &nullifer0.to_string()[8..23], &program_id)
            .unwrap();
    nullifier_pubkeys.push(pubkey_from_seed);

    let pubkey_from_seed =
        Pubkey::create_with_seed(&signer_pubkey, &nullifer1.to_string()[8..23], &program_id)
            .unwrap();
    nullifier_pubkeys.push(pubkey_from_seed);
    */

    // let nullifer0 = Pubkey::new(&*ix_data[96 + 8..128 + 8].to_vec().clone());
    // let nullifer1 = Pubkey::new(&*ix_data[128 + 8..160 + 8].to_vec().clone());
    let mut nullifier_pubkeys = Vec::new();
    println!("nf0 seed data: {:?}", ix_data[96 + 9..128 + 9].to_vec());
    let pubkey_from_seed = Pubkey::find_program_address(&[&ix_data[96 + 9..128 + 9]], &program_id);
    // let (authority, bumpSeed) = Pubkey::find_program_address(
    //     &[&owner_keypair.pubkey().to_bytes()[..]],
    //     &program_id
    // );
    nullifier_pubkeys.push(pubkey_from_seed.0);

    let pubkey_from_seed = Pubkey::find_program_address(&[&ix_data[128 + 9..160 + 9]], &program_id);
    nullifier_pubkeys.push(pubkey_from_seed.0);
    println!("derriving nullifier pubkeys from: {:?}", nullifier_pubkeys);

    //restart to add nullifer pdas
    // create_and_start_program_with_nullfier_pdas(
    //         merkle_tree_init_bytes: Vec<u8>,
    //         hash_bytes_init_bytes: Vec<u8>,
    //         merkle_tree_pubkey: &Pubkey,
    //         storage_account: &Pubkey,
    //         two_leaves_pda_pubkey: &Pubkey,
    //         nullifier_pubkeys: &Vec<Pubkey>,
    //         program_id: &Pubkey,
    //         signer_pubkey: &Pubkey
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
    // program_context = create_and_start_program_with_nullfier_pdas(
    //     merkle_tree_account_old.data.to_vec(),
    //     storage_account.data.to_vec(),
    //     &merkle_tree_pubkey,
    //     &storage_pubkey,
    //     &two_leaves_pda_pubkey,
    //     &nullifier_pubkeys,
    //     &program_id,
    //     &signer_pubkey,
    // )
    // .await;

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
                AccountMeta::new(
                    Pubkey::from_str("11111111111111111111111111111111").unwrap(),
                    false,
                ),
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
        "deposit success {}",
        merkel_tree_account_new.lamports == merkle_tree_account_old.lamports + 100000000
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
