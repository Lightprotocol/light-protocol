use crate::test_utils::tests::{
    create_and_start_program_var,
     read_test_data, restart_program,
};
use crate::tokio::time::timeout;
use ark_ec::ProjectiveCurve;
use ark_ff::BigInteger;
use ark_ff::PrimeField;
use ark_groth16::{prepare_inputs, prepare_verifying_key};
use ark_std::{test_rng, UniformRand};
use light_protocol_program::poseidon_merkle_tree::state::MerkleTree;
use light_protocol_program::utils::config;
use light_protocol_program::{
    process_instruction,
    state::MerkleTreeTmpPda,
    utils::config::{ENCRYPTED_UTXOS_LENGTH, MERKLE_TREE_ACC_BYTES_ARRAY, MERKLE_TREE_TMP_PDA_SIZE},
    IX_ORDER,
};
use serde_json::Result;
use solana_program::program_pack::Pack;
use solana_program::sysvar::rent::Rent;
use solana_program_test::ProgramTestContext;
use std::convert::TryInto;
use std::fs::File;
use std::io::Write;
use std::time;
use {
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        sysvar,
    },
    solana_program_test::*,
    solana_sdk::{
        signers::Signers,
        account::Account, signature::Signer, signer::keypair::Keypair, transaction::Transaction,
    },
    std::str::FromStr,
};
// is necessary to have a consistent signer and relayer otherwise transactions would get rejected
const PRIVATE_KEY: [u8; 64] = [
    17, 34, 231, 31, 83, 147, 93, 173, 61, 164, 25, 0, 204, 82, 234, 91, 202, 187, 228, 110, 146,
    97, 112, 131, 180, 164, 96, 220, 57, 207, 65, 107, 2, 99, 226, 251, 88, 66, 92, 33, 25, 216,
    211, 185, 112, 203, 212, 238, 105, 144, 72, 121, 176, 253, 106, 168, 115, 158, 154, 188, 62,
    255, 166, 81,
];
const PRIV_KEY_DEPOSIT: [u8; 64] = [
    70, 5, 178, 190, 139, 224, 161, 74, 134, 130, 14, 189, 253, 51, 249, 124, 255, 116, 66, 87,
    146, 202, 196, 243, 68, 129, 95, 145, 97, 170, 145, 61, 221, 240, 113, 237, 127, 131, 46, 151,
    40, 236, 223, 8, 124, 162, 170, 56, 71, 105, 233, 43, 196, 129, 63, 145, 13, 2, 210, 251, 197,
    109, 226, 3,
];

mod test_utils;

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
                AccountMeta::new_readonly(sysvar::rent::id(), false),
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
    let mut cache_index = 3;
    // 73
    for instruction_id in 0..38 {
        let instruction_data: Vec<u8> = vec![2u8,2u8, i as u8];

        let mut instruction_vec = vec![Instruction::new_with_bincode(
            *program_id,
            &instruction_data,
            vec![
                AccountMeta::new(signer_keypair.pubkey(), true),
                AccountMeta::new(*tmp_storage_pda_pubkey, false),
                AccountMeta::new(*merkle_tree_pda_pubkey, false),
            ],
        )];
        //checking merkle tree lock and add second instruction
        if instruction_id != 0 {
            let instruction_data: Vec<u8> = vec![2u8,1u8, i as u8];
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
                *tmp_storage_pda_pubkey
            );
            let tmp_storage_pda_account = program_context
                .banks_client
                .get_account(*tmp_storage_pda_pubkey)
                .await
                .expect("get_account")
                .unwrap();
            let tmp_storage_pda_account_data =
                MerkleTreeTmpPda::unpack(&tmp_storage_pda_account.data.clone()).unwrap();
            println!("cache_index: {}", cache_index);
            println!("IX_ORDER: {}", IX_ORDER[cache_index]);
            println!("tmp_storage_pda_account_data.current_instruction_index: {}", tmp_storage_pda_account_data.current_instruction_index);

            assert_eq!(
                tmp_storage_pda_account_data.current_instruction_index,
                cache_index
            );
            // always executing 2 instructions except for the first time
            cache_index += 2;

        }
        instruction_vec.push(Instruction::new_with_bincode(
            *program_id,
            &instruction_data,
            vec![
                AccountMeta::new(signer_keypair.pubkey(), true),
                AccountMeta::new(*tmp_storage_pda_pubkey, false),
                AccountMeta::new(*merkle_tree_pda_pubkey, false),
            ],
        ));
        // the 9th byte has to be zero for it is used to enter other instructions,
        // i.e. user account init, the callindex is added to make the transaction unique,
        // equal transactions are not executed by test-bpf
        let mut success = false;
        let mut retries_left = 2;
        while retries_left > 0 && success != true {
            let mut transaction = Transaction::new_with_payer(
                &instruction_vec[..],

                Some(&signer_keypair.pubkey()),
            );
            println!("transaction: update merkle tree {} {:?}", instruction_id, transaction);
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
                    retries_left -= 1;
                    restart_program(
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
        let storage_account = program_context
            .banks_client
            .get_account(*tmp_storage_pda_pubkey)
            .await
            .expect("get_account")
            .unwrap();
        println!("storage_account_unpacked.state[0..32] {:?}", storage_account.data);

        let storage_account_unpacked = MerkleTreeTmpPda::unpack(&storage_account.data).unwrap();

        println!("storage_account_unpacked.state[0..32] {:?}", storage_account_unpacked.state[0..32].to_vec());

        println!("Test Instruction index {}", i);
        i += 1;
    }
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
    encrypted_utxos: &[u8],
    expected_merkle_tree_pubkey: &[u8],
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
    assert_eq!(
        *expected_merkle_tree_pubkey,
        two_leaves_pda_account.data[74..106]
    );
    // saved encrypted_utxos correctly
    assert_eq!(*encrypted_utxos, two_leaves_pda_account.data[106..]);
}
async fn create_pubkeys_from_ix_data(
    ix_data: &Vec<u8>,
    program_id: &Pubkey,
    program_id_merkle_tree: &Pubkey,
) -> (Pubkey, Pubkey, Pubkey, Pubkey, Pubkey) {
    // Creates pubkeys for all the PDAs we'll use
    let verifier_tmp_storage_pda_pubkey =
        Pubkey::find_program_address(&[&ix_data[73..105], &b"storage"[..]], &program_id).0;
    let merkel_tree_tmp_storage_pda_pubkey =
        Pubkey::find_program_address(&[&ix_data[73..105], &b"storage"[..]], &program_id_merkle_tree).0;
    let two_leaves_pda_pubkey =
        Pubkey::find_program_address(&[&ix_data[105..137], &b"leaves"[..]], program_id_merkle_tree).0;

    let nf_pubkey0 = Pubkey::find_program_address(&[&ix_data[105..137], &b"nf"[..]], program_id_merkle_tree).0;

    let nf_pubkey1 = Pubkey::find_program_address(&[&ix_data[137..169], &b"nf"[..]], program_id_merkle_tree).0;
    (
        verifier_tmp_storage_pda_pubkey,
        merkel_tree_tmp_storage_pda_pubkey,
        two_leaves_pda_pubkey,
        nf_pubkey0,
        nf_pubkey1,
    )
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
        MerkleTreeTmpPda::unpack(&tmp_storage_account.data.clone()).unwrap();
    assert_eq!(unpacked_tmp_storage_account.current_instruction_index, 1501);

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
                merkle_account_data_after.unwrap()[((merkle_tree_pda_after.current_root_index - 1)
                    * 32)
                    + 610
                    ..((merkle_tree_pda_after.current_root_index - 1) * 32) + 642]
                    .to_vec()
            );
            assert_eq!(
                unpacked_tmp_storage_account.root_hash,
                merkle_account_data_after.unwrap()[((merkle_tree_pda_after.current_root_index - 1)
                    * 32)
                    + 610
                    ..((merkle_tree_pda_after.current_root_index - 1) * 32) + 642]
                    .to_vec()
            );
        }
    }
}


use light_protocol_program::instructions::MerkleTreeTmpStorageAccInputData;
use borsh::ser::BorshSerialize;
use ark_ed_on_bn254::Fq;
use ark_ff::bytes::ToBytes;

#[tokio::test]
async fn merkle_tree_tmp_account_init_should_succeed() {
    // let program_id = Pubkey::from_str("TransferLamports111111111111111111111111111").unwrap();
    let program_id_merkle_tree = Pubkey::from_str("TransferLamports111111111111111111111111122").unwrap();
    let merkle_tree_pda_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY[0].0);



    let signer_keypair = solana_sdk::signer::keypair::Keypair::from_bytes(&PRIVATE_KEY).unwrap();
    let signer_pubkey = signer_keypair.pubkey();

    let mut ix_withdraw_data = vec![1u8;1000];
    let mut account_data = MerkleTreeTmpStorageAccInputData::new(
            ix_withdraw_data[0..32].to_vec(),
            ix_withdraw_data[32..64].to_vec(),
            ix_withdraw_data[32..64].to_vec(),
            merkle_tree_pda_pubkey.to_bytes().to_vec(),
            signer_pubkey.to_bytes().to_vec(),
            vec![0u8;32]//verifier_tmp_storage_pda_pubkey.to_bytes().to_vec()
        ).unwrap();

    account_data.node_left = vec![2u8;32];
    account_data.node_right = vec![2u8;32];
    account_data.root_hash = vec![0u8;32];
    account_data.merkle_tree_pda_pubkey =
        Pubkey::find_program_address(&[&account_data.node_left, &b"storage"[..]], &program_id_merkle_tree).0.to_bytes().to_vec();
    println!("account_data.merkle_tree_pda_pubkey: {:?}", Pubkey::find_program_address(&[&account_data.node_left, &b"leaves"[..]], &program_id_merkle_tree).0);
    account_data.relayer = signer_pubkey.to_bytes().to_vec();
    let two_leaves_pda_pubkey = Pubkey::find_program_address(&[&account_data.node_left, &b"leaves"[..]], &program_id_merkle_tree).0;

    ix_withdraw_data = account_data.return_ix_data().unwrap();
    println!("ix_withdraw_data: {:?}", ix_withdraw_data);
    let (
        verifier_tmp_storage_pda_pubkey,
        merkle_tree_tmp_storage_pda_pubkey,
        _two_leaves_pda_pubkey,
        nf_pubkey0,
        nf_pubkey1
        ) =
        create_pubkeys_from_ix_data(&[vec![0u8;9],ix_withdraw_data.to_vec()].concat(), &program_id_merkle_tree, &program_id_merkle_tree).await;
    let mut nullifier_pubkeys = Vec::new();
    nullifier_pubkeys.push(nf_pubkey0);
    nullifier_pubkeys.push(nf_pubkey1);


    let mut accounts_vector = Vec::new();
    accounts_vector.push(
        (&merkle_tree_pda_pubkey, 16658, None)
    );

    let merkle_tree_tmp_storage_pda_pubkey =
            Pubkey::new(&account_data.merkle_tree_pda_pubkey);

    let mut program_context =
        create_and_start_program_var(&accounts_vector, None, &program_id_merkle_tree, &signer_pubkey).await;
    println!("here");
    initialize_merkle_tree(
        &program_id_merkle_tree,
        &merkle_tree_pda_pubkey,
        &signer_keypair,
        &mut program_context,
    )
    .await;

    println!("\n ix_withdraw_data: {:?} \n", ix_withdraw_data);
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id_merkle_tree,
            &[vec![1u8], ix_withdraw_data.to_vec()].concat(),
            vec![
                AccountMeta::new(signer_pubkey, true),
                AccountMeta::new(verifier_tmp_storage_pda_pubkey, false),
                AccountMeta::new(merkle_tree_tmp_storage_pda_pubkey, false),
                AccountMeta::new_readonly(solana_program::system_program::id(), false),
                AccountMeta::new_readonly(sysvar::rent::id(), false),
            ],
        )],
        Some(&signer_pubkey),
    );
    println!("0");
    transaction.sign(&[&signer_keypair], program_context.last_blockhash);
    println!("transaction: {:?}", transaction);
    program_context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();


    let storage_account = program_context
        .banks_client
        .get_account(merkle_tree_tmp_storage_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    let storage_account_unpacked = MerkleTreeTmpPda::unpack(&storage_account.data).unwrap();
    assert_eq!(storage_account_unpacked.node_left, [2u8;32]);
    assert_eq!(storage_account_unpacked.node_right, [2u8;32]);
    println!("-------------- update_merkle_tree -----------------------");
    update_merkle_tree(
        &program_id_merkle_tree,
        &merkle_tree_pda_pubkey,
        &merkle_tree_tmp_storage_pda_pubkey,
        &signer_keypair,
        &mut program_context,
        &mut accounts_vector,
    )
    .await;

    let storage_account = program_context
        .banks_client
        .get_account(merkle_tree_tmp_storage_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    let storage_account_unpacked = MerkleTreeTmpPda::unpack(&storage_account.data).unwrap();

    assert_eq!(
        [141, 69, 80, 56, 132, 104, 54, 29, 244, 1, 168, 24, 51, 53, 162, 230, 208, 149, 158, 156, 84, 167, 67, 171, 234, 58, 128, 14, 0, 179, 97, 46],
        storage_account_unpacked.state[0..32]
    );
    println!("initializing merkle tree success: {:?}", storage_account_unpacked);


    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id_merkle_tree,
            &[vec![2u8], vec![2u8;ENCRYPTED_UTXOS_LENGTH]].concat(),
            vec![
                AccountMeta::new(signer_keypair.pubkey(), true),
                AccountMeta::new(merkle_tree_tmp_storage_pda_pubkey, false),
                AccountMeta::new(two_leaves_pda_pubkey, false),
                AccountMeta::new(merkle_tree_pda_pubkey, false),
                AccountMeta::new_readonly(solana_program::system_program::id(), false),
                AccountMeta::new_readonly(sysvar::rent::id(), false),
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

        check_leaves_insert_correct(
        &two_leaves_pda_pubkey,
        &account_data.node_left, //left leaf todo change order
        &account_data.node_right, //right leaf
        &vec![2u8; ENCRYPTED_UTXOS_LENGTH], //encrypted_utxos
        &merkle_tree_tmp_storage_pda_pubkey.to_bytes(),
        &mut program_context,
        )
        .await;

        let storage_account = program_context
            .banks_client
            .get_account(merkle_tree_pda_pubkey)
            .await
            .expect("get_account")
            .unwrap();
        let storage_account_unpacked = MerkleTree::unpack(&storage_account.data).unwrap();

        assert_eq!(
            vec![141, 69, 80, 56, 132, 104, 54, 29, 244, 1, 168, 24, 51, 53, 162, 230, 208, 149, 158, 156, 84, 167, 67, 171, 234, 58, 128, 14, 0, 179, 97, 46],
            storage_account_unpacked.roots
        );


}

#[tokio::test]
async fn sol_transfer_should_succeed() {
    let amount = 100_000_000;
    let program_id = Pubkey::from_str("TransferLamports111111111111111111111111111").unwrap();

    let merkle_tree_pda_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES_ARRAY[0].0);

    let signer_keypair = solana_sdk::signer::keypair::Keypair::new();
    let signer_pubkey = signer_keypair.pubkey();

    let tmp_storage_pda_pubkey = solana_sdk::signer::keypair::Keypair::new().pubkey();
    let merkle_tree_pda_pubkey = solana_sdk::signer::keypair::Keypair::new().pubkey();
    let user_ecrow_acc = Pubkey::find_program_address(
        &[&tmp_storage_pda_pubkey.to_bytes(), &b"escrow"[..]],
        &program_id,
    )
    .0;

    let mut accounts_vector = Vec::new();
    accounts_vector.push((&merkle_tree_pda_pubkey, 16658, None));
    accounts_vector.push((&tmp_storage_pda_pubkey, 0, None));
    accounts_vector.push((&merkle_tree_pda_pubkey, 0, None));

    let mut program_context =
        create_and_start_program_var(&accounts_vector, None, &program_id, &signer_pubkey).await;

    let merkle_tree_data_prior = program_context
        .banks_client
        .get_account(merkle_tree_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();

    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &[vec![3u8, 1u8], u64::to_le_bytes(amount).to_vec()].concat(),
            vec![
                AccountMeta::new(signer_keypair.pubkey(), true),
                AccountMeta::new(tmp_storage_pda_pubkey, false),
                AccountMeta::new_readonly(solana_program::system_program::id(), false),
                AccountMeta::new_readonly(sysvar::rent::id(), false),
                AccountMeta::new(merkle_tree_pda_pubkey, false),
                AccountMeta::new(user_ecrow_acc, false),
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

    let merkle_tree_data = program_context
        .banks_client
        .get_account(merkle_tree_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    println!("merkle_tree_data_prior: {:?}", merkle_tree_data_prior);

    println!("merkle_tree_data: {:?}", merkle_tree_data);
    assert_eq!(merkle_tree_data.lamports, merkle_tree_data_prior.lamports + 890880 + amount, "Deposit failed.");


    let merkle_tree_data_prior = program_context
        .banks_client
        .get_account(merkle_tree_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();

    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &[vec![3u8, 2u8], u64::to_le_bytes(amount).to_vec()].concat(),
            vec![
                AccountMeta::new(signer_keypair.pubkey(), true),
                AccountMeta::new(tmp_storage_pda_pubkey, false),
                AccountMeta::new(merkle_tree_pda_pubkey, false),
                AccountMeta::new(user_ecrow_acc, false),
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

    let merkle_tree_data = program_context
        .banks_client
        .get_account(merkle_tree_pda_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    println!("merkle_tree_data_prior: {:?}", merkle_tree_data_prior);

    println!("merkle_tree_data: {:?}", merkle_tree_data);
    assert_eq!(merkle_tree_data.lamports, merkle_tree_data_prior.lamports - amount, "Withdrawal failed.");

}
