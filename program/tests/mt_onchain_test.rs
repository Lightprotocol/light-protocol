use crate::tokio::runtime::Runtime;
use crate::tokio::time::timeout;
use ark_crypto_primitives::crh::TwoToOneCRH;
use ark_ed_on_bn254::Fq;
use ark_ff::bytes::{FromBytes, ToBytes};
use ark_ff::{BigInteger, Fp256, PrimeField};
use ark_std::One;
use ark_std::{test_rng, UniformRand};
use arkworks_gadgets::poseidon::{
    circom::CircomCRH, sbox::PoseidonSbox, PoseidonError, PoseidonParameters, Rounds,
};
use arkworks_gadgets::utils::{
    get_mds_poseidon_circom_bn254_x5_3, get_rounds_poseidon_circom_bn254_x5_3, parse_vec,
};
use solana_program::program_pack::Pack;
use solana_program_test::ProgramTestContext;
use solana_program_test::ProgramTestError;
use solana_sdk::signer::keypair::Keypair;
use std::{thread, time};
use {
    light_protocol_core::{
        poseidon_merkle_tree::mt_state::{HashBytes, MerkleTree, MERKLE_TREE_ACC_BYTES},
        process_instruction,
        utils::init_bytes18,
    },
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
    },
    solana_program_test::*,
    solana_sdk::{account::Account, msg, signature::Signer, transaction::Transaction},
    std::str::FromStr,
};

#[tokio::test]
async fn merkle_tree_onchain_test() {
    let program_id = Pubkey::from_str("TransferLamports111111111111111111111111111").unwrap();

    let storage_pubkey = Pubkey::new_unique();
    let merkle_tree_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES);
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
    accounts_vector.push((&merkle_tree_pubkey, 16657, None));
    accounts_vector.push((&storage_pubkey, 3900, Some(account_state.clone())));

    let mut program_context = create_and_start_program_var(&accounts_vector, &program_id, &signer_pubkey).await;

    //initialize MerkleTree account
    initialize_merkle_tree(
        &program_id,
        &merkle_tree_pubkey,
        &signer_keypair,
        &mut program_context,
    ).await;
    //calculate new merkle tree root by updating the two first leaves
    update_merkle_tree(
        &program_id,
        &merkle_tree_pubkey,
        &storage_pubkey,
        &signer_keypair,
        &mut program_context,
        &mut accounts_vector,
    ).await;

    let storage_account = program_context
        .banks_client
        .get_account(storage_pubkey)
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

    //pda is inserted in last tx cannot be tested without all data
    // let storage_account = program_context
    //     .banks_client
    //     .get_account(two_leaves_pda_pubkey)
    //     .await
    //     .expect("get_account")
    //     .unwrap();
    //
    // assert_eq!(1, storage_account.data[0]);
    // assert_eq!(4, storage_account.data[1]);
    // println!("pda_account_data = : {:?}", storage_account.data);
    //
    // assert_eq!(commit, storage_account.data[10..42]);
    // assert_eq!(commit, storage_account.data[42..74]);
    // assert_eq!(MERKLE_TREE_ACC_BYTES, storage_account.data[74..106]);

}

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

pub async fn restart_program(
    accounts_vector: &mut Vec<(&Pubkey, usize, Option<Vec<u8>>)>,
    program_id: &Pubkey,
    signer_pubkey: &Pubkey,
    mut program_context: &mut ProgramTestContext,
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
    let mut program_context_new =
        create_and_start_program_var(&accounts_vector, &program_id, &signer_pubkey).await;
    program_context_new
}


pub async fn initialize_merkle_tree(
    program_id: &Pubkey,
    merkle_tree_pubkey: &Pubkey,
    signer_keypair:&solana_sdk::signer::keypair::Keypair,
    program_context: &mut ProgramTestContext
) {
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            *program_id,
            &[vec![240u8, 0u8], usize::to_le_bytes(1000).to_vec()].concat(),
            vec![
                AccountMeta::new(signer_keypair.pubkey(), true),
                AccountMeta::new(*merkle_tree_pubkey, false),
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
        .get_account(*merkle_tree_pubkey)
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
    merkle_tree_pubkey: &Pubkey,
    storage_pubkey: &Pubkey,
    signer_keypair:&solana_sdk::signer::keypair::Keypair,
    program_context: &mut ProgramTestContext,
    accounts_vector: &mut Vec<(&Pubkey, usize, Option<Vec<u8>>)>
) {
    let mut i = 0;
    for (instruction_id) in 0..237 {
        let instruction_data: Vec<u8> =vec![i as u8];
            let mut success = false;
            let mut retries_left = 2;
            while (retries_left > 0 && success != true) {
                let mut transaction = Transaction::new_with_payer(
                    &[Instruction::new_with_bincode(
                        *program_id,
                        &instruction_data,
                        vec![
                            AccountMeta::new(signer_keypair.pubkey(), true),
                            AccountMeta::new(*storage_pubkey, false),
                            AccountMeta::new(*merkle_tree_pubkey, false),
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
                        retries_left -=1;
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
