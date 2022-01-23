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
use std::convert::TryInto;
use std::{thread, time};
use {
    light_protocol_core::{
        poseidon_merkle_tree::state::{MerkleTree, TempStoragePda, MERKLE_TREE_ACC_BYTES},
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

pub async fn create_and_start_program(
    user_account_pubkey: &Pubkey,
    program_id: &Pubkey,
    signer_pubkey: &Pubkey,
) -> ProgramTestContext {
    let mut program_test = ProgramTest::new(
        "light_protocol_core",
        *program_id,
        processor!(process_instruction),
    );
    let mut user_account = Account::new(10000000000, 34 + SIZE_UTXO as usize * 10, &program_id);

    program_test.add_account(*user_account_pubkey, user_account);

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
use light_protocol_core::user_account::state::SIZE_UTXO;

#[tokio::test]
async fn user_account_onchain_test() {
    let program_id = Pubkey::from_str("TransferLamports111111111111111111111111111").unwrap();

    let user_account_pubkey = Pubkey::new_unique();

    let signer_keypair = solana_sdk::signer::keypair::Keypair::new();
    let signer_pubkey = signer_keypair.pubkey();

    let mut program_context =
        create_and_start_program(&user_account_pubkey, &program_id, &signer_pubkey).await;

    //initialize user_account account

    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &[vec![100u8, 0u8], usize::to_le_bytes(1000).to_vec()].concat(),
            vec![
                AccountMeta::new(signer_keypair.pubkey(), true),
                AccountMeta::new(user_account_pubkey, false),
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

    let user_account_data_init = program_context
        .banks_client
        .get_account(user_account_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    assert_eq!(1u8, user_account_data_init.data[0]);

    assert_eq!(
        signer_keypair.pubkey(),
        Pubkey::new(&user_account_data_init.data[2..34])
    );

    //assert_eq!(true, false);
    println!("initializing user account success");

    //modify user_account account

    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &[
                vec![101u8],
                usize::to_le_bytes(0).to_vec(),
                vec![1u8; SIZE_UTXO.try_into().unwrap()],
            ]
            .concat(),
            vec![
                AccountMeta::new(signer_keypair.pubkey(), true),
                AccountMeta::new(user_account_pubkey, false),
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

    let user_account_data_modified = program_context
        .banks_client
        .get_account(user_account_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    //println!("user_account_data_modified: {:?}", user_account_data_modified.data[0..200].to_vec());
    assert_eq!(vec![1u8; 64], user_account_data_modified.data[34..98]);
    println!("modifying user account success");
}

#[tokio::test]
async fn test_user_account_checks() {
    let program_id = Pubkey::from_str("TransferLamports111111111111111111111111111").unwrap();

    let user_account_pubkey = Pubkey::new_unique();

    let signer_keypair = solana_sdk::signer::keypair::Keypair::new();
    let signer_pubkey = signer_keypair.pubkey();

    let other_keypair = solana_sdk::signer::keypair::Keypair::new();
    let other_pubkey = other_keypair.pubkey();

    let mut program_context =
        create_and_start_program(&user_account_pubkey, &program_id, &signer_pubkey).await;

    //initialize user_account account

    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &[vec![100u8, 0u8], usize::to_le_bytes(1000).to_vec()].concat(),
            vec![
                AccountMeta::new(signer_keypair.pubkey(), true),
                AccountMeta::new(user_account_pubkey, false),
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

    let user_account_data_init = program_context
        .banks_client
        .get_account(user_account_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    assert_eq!(1u8, user_account_data_init.data[0]);

    assert_eq!(
        signer_keypair.pubkey(),
        Pubkey::new(&user_account_data_init.data[2..34])
    );

    //try initialize user_account account again

    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &[vec![100u8, 0u8], usize::to_le_bytes(1000).to_vec()].concat(),
            vec![
                AccountMeta::new(program_context.payer.pubkey(), true),
                AccountMeta::new(user_account_pubkey, false),
            ],
        )],
        Some(&program_context.payer.pubkey()),
    );
    transaction.sign(&[&program_context.payer], program_context.last_blockhash);

    program_context
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let user_account_data_init = program_context
        .banks_client
        .get_account(user_account_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    //println!("merkletree: {:?}", merkle_tree_data);
    assert_eq!(1u8, user_account_data_init.data[0]);
    //println!("user_account_pubkey: {:?}", user_account_pubkey);

    assert_eq!(
        signer_keypair.pubkey(),
        Pubkey::new(&user_account_data_init.data[2..34])
    );

    //try modifying user_account

    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &[vec![101u8], usize::to_le_bytes(0).to_vec(), vec![1u8; 64]].concat(),
            vec![
                AccountMeta::new(program_context.payer.pubkey(), true),
                AccountMeta::new(user_account_pubkey, false),
            ],
        )],
        Some(&program_context.payer.pubkey()),
    );
    transaction.sign(&[&program_context.payer], program_context.last_blockhash);

    program_context
        .banks_client
        .process_transaction(transaction)
        .await;

    let user_account_data_modified = program_context
        .banks_client
        .get_account(user_account_pubkey)
        .await
        .expect("get_account")
        .unwrap();
    //println!("user_account_data_modified: {:?}", user_account_data_modified.data[0..200].to_vec());
    assert_eq!(vec![0u8; 64], user_account_data_modified.data[34..98]);
    println!("user account was not modified success");
}
