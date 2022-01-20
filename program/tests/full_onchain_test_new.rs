use crate::tokio::time::timeout;
use ark_ff::biginteger::BigInteger256;
use ark_ff::Fp256;
use light_protocol_core::{
    groth16_verifier::{
        final_exponentiation::state::{FinalExpBytes, INSTRUCTION_ORDER_VERIFIER_PART_2},
        miller_loop::state::*,
        parsers::*,
    },
    poseidon_merkle_tree::mt_state::MERKLE_TREE_ACC_BYTES,
    process_instruction,
    utils::init_bytes18,
};
use solana_program::program_pack::Pack;

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

use solana_program_test::ProgramTestContext;
use std::{fs, time};
mod fe_onchain_test;

//mod tests::fe_onchain_test;
//use crate::fe_onchain_test;
mod mt_onchain_test;
mod pi_onchain_test;
// mod fe_offchain_test;
// use crate::fe_offchain_test::tests::get_public_inputs_from_bytes_254;

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
    println!("accounts {:?}", accounts);
    for (pubkey, size, data) in accounts.iter() {
        println!("accounts {:?}, {:?}, {:?}", pubkey, size, data);

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
    let _res_request = program_context
        .banks_client
        .process_transaction(transaction)
        .await;

    program_context
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
    let program_context_new =
        create_and_start_program_var(&accounts_vector, &program_id, &signer_pubkey).await;
    program_context_new
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

async fn compute_prepared_inputs(
    program_id: solana_program::pubkey::Pubkey,
    signer_pubkey: solana_program::pubkey::Pubkey,
    signer_keypair: solana_sdk::signature::Keypair,
    storage_pubkey: solana_program::pubkey::Pubkey,
    mut program_context: ProgramTestContext,
    mut accounts_vector: std::vec::Vec<(
        &solana_program::pubkey::Pubkey,
        usize,
        std::option::Option<std::vec::Vec<u8>>,
    )>,
) {
    let mut i = 0usize;
    for id in 0..464usize {
        let mut success = false;
        let mut retries_left = 2;
        while retries_left > 0 && success != true {
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
                Err(_e) => {
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
        }
        i += 1;
    }
}

async fn compute_miller_output(
    program_id: solana_program::pubkey::Pubkey,
    signer_pubkey: solana_program::pubkey::Pubkey,
    signer_keypair: solana_sdk::signature::Keypair,
    storage_pubkey: solana_program::pubkey::Pubkey,
    mut program_context: ProgramTestContext,
    mut accounts_vector: std::vec::Vec<(
        &solana_program::pubkey::Pubkey,
        usize,
        std::option::Option<std::vec::Vec<u8>>,
    )>,
) {
    let storage_account = program_context
        .banks_client
        .get_account(storage_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    let account_data = ML254Bytes::unpack(&storage_account.data.clone()).unwrap();

    //assert_eq!(true, false);
    // Executes 1973 following ix.
    let mut i = 8888usize;
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
                    program_context = restart_program(
                        &mut accounts_vector,
                        &program_id,
                        &signer_pubkey,
                        program_context,
                    )
                    .await;
                    // program_context = create_and_start_program(
                    // merkle_tree_account.data.to_vec(),
                    // storage_account.data.to_vec(),
                    // &merkle_tree_pubkey,
                    // &storage_pubkey,
                    // &program_id,
                    // &signer_pubkey,
                    // )
                    // .await;
                }
            }
        }
        i += 1;
    }
}

async fn compute_final_exponentiation(
    program_id: solana_program::pubkey::Pubkey,
    signer_pubkey: solana_program::pubkey::Pubkey,
    signer_keypair: solana_sdk::signature::Keypair,
    storage_pubkey: solana_program::pubkey::Pubkey,
    mut program_context: ProgramTestContext,
    mut accounts_vector: std::vec::Vec<(
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
                    program_id,
                    &[vec![instruction_id, 2u8], usize::to_le_bytes(i).to_vec()].concat(),
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
                    // program_context = create_and_start_program(
                    //     merkle_tree_account.data.to_vec(),
                    //     storage_account.data.to_vec(),
                    //     &merkle_tree_pubkey,
                    //     &storage_pubkey,
                    //     &program_id,
                    //     &signer_pubkey,
                    // )
                    // .await;
                    program_context = restart_program(
                        &mut accounts_vector,
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

#[tokio::test]
async fn onchain_verification_should_succeed() {
    //getting instruction data from file
    // Creates program, accounts, setup.
    let program_id = Pubkey::from_str("TransferLamports111111111111111111112111111").unwrap();
    let ix_data = read_test_data();
    let mut accounts_vector = Vec::new();
    //create pubkey for temporary storage account
    let merkle_tree_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES);
    accounts_vector.push((&merkle_tree_pubkey, 16657, None));

    let signer_keypair = solana_sdk::signer::keypair::Keypair::new();
    let signer_pubkey = signer_keypair.pubkey();

    // create pubkeys for all the PDAs we'll use
    let two_leaves_pda_pubkey =
        Pubkey::find_program_address(&[&ix_data[105..137], &b"leaves"[..]], &program_id).0;
    println!(
        "leaves add seed: {:?}",
        [&ix_data[105..137], &b"leaves"[..]]
    );
    let storage_pubkey =
        Pubkey::find_program_address(&[&ix_data[105..137], &b"storage"[..]], &program_id).0;
    println!(
        "storage_pubkey add seed: {:?}",
        [&ix_data[105..137], &b"storage"[..]]
    );

    let mut nullifier_pubkeys = Vec::new();
    let pubkey_from_seed =
        Pubkey::find_program_address(&[&ix_data[96 + 9..128 + 9], &b"nf"[..]], &program_id);
    nullifier_pubkeys.push(pubkey_from_seed.0);

    let pubkey_from_seed =
        Pubkey::find_program_address(&[&ix_data[128 + 9..160 + 9], &b"nf"[..]], &program_id);
    nullifier_pubkeys.push(pubkey_from_seed.0);
    println!("deriving nullifier pubkeys from: {:?}", nullifier_pubkeys);

    // start program
    let mut program_context =
        create_and_start_program_var(&accounts_vector, &program_id, &signer_pubkey).await;
    let _merkle_tree_account = program_context
        .banks_client
        .get_account(merkle_tree_pubkey)
        .await
        .expect("get_account")
        .unwrap();

    // Tx that initializes MerkleTree account
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
     * Send data to chain and initialize tmp_storage_account
     *
     *
     */
    //sends bytes (public inputs and proof)
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &ix_data[8..].to_vec(),
            vec![
                AccountMeta::new(signer_pubkey, true),
                AccountMeta::new(storage_pubkey, false),
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

    /*
     *
     *
     * Proof verification
     *
     *
     */

    compute_prepared_inputs(
        program_id,
        signer_pubkey,
        signer_keypair,
        storage_pubkey,
        program_context,
        accounts_vector,
    );

    /*
     *
     *
     *Miller loop
     *
     *
     */
    compute_miller_output(
        program_id,
        signer_pubkey,
        signer_keypair,
        storage_pubkey,
        program_context,
        accounts_vector,
    );

    /*
     *
     * Final Exponentiation
     *
     */

    // Note that if they verificaton is succesful, this will pass. If not, an on-chain check will panic the program
    compute_final_exponentiation(
        program_id,
        signer_pubkey,
        signer_keypair,
        storage_pubkey,
        program_context,
        accounts_vector,
    );

    // TODO: Add offchain verification here, just to "prove" that the onchain check is legit.
    println!("Onchain Proof Verification success");

    // TODO: Jorrit: below is part of the extra logic /deposit/withdrawal/merkletree; A pure "verifciation test" should stop here.
    // So if you keep this here, feel free to change the test name from "onchain_verification_should_succeed" to something like "deposit_should_succeed"...

    /*
     *
     * Merkle Tree insert of new utxos
     *
     */
    let commit = vec![0u8; 32];
    let mut i = 0;
    for instruction_id in 0..237 {
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
            while retries_left > 0 && success != true {
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
                    Err(_e) => {
                        println!("retries_left {}", retries_left);
                        retries_left -= 1;
                        let merkle_tree_account = program_context
                            .banks_client
                            .get_account(merkle_tree_pubkey)
                            .await
                            .expect("get_account")
                            .unwrap();
                        let tmp_storage_account = program_context
                            .banks_client
                            .get_account(storage_pubkey)
                            .await
                            .expect("get_account")
                            .unwrap();
                        //println!("data: {:?}", storage_account.data);
                        //let old_payer = signer_keypair;
                        program_context = create_and_start_program(
                            merkle_tree_account.data.to_vec(),
                            tmp_storage_account.data.to_vec(),
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
                        let tmp_storage_account_new = program_context
                            .banks_client
                            .get_account(storage_pubkey)
                            .await
                            .expect("get_account")
                            .unwrap();
                        assert_eq!(merkle_tree_account_new.data, merkle_tree_account.data);
                        assert_eq!(tmp_storage_account_new.data, tmp_storage_account.data);
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
    let _storage_account = program_context
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
        "deposit success {} {}",
        merkel_tree_account_new.lamports,
        merkle_tree_account_old.lamports + 100000000
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
