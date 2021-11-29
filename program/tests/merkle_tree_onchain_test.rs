use {
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
    },
    solana_program_test::*,
    solana_sdk::{account::Account, signature::Signer, transaction::Transaction, msg},
    Testing_Hardcoded_Params_devnet_new::{process_instruction, state_merkle_tree::{MerkleTree,HashBytes,MERKLE_TREE_ACC_BYTES}, init_bytes11},
    std::str::FromStr,
};
use solana_program_test::ProgramTestError;
use solana_program::program_pack::Pack;
use solana_sdk::signer::keypair::Keypair;
use crate::tokio::runtime::Runtime;
use ark_ed_on_bn254::Fq;
use ark_std::{One};
use ark_ff::{PrimeField, BigInteger, Fp256};
use arkworks_gadgets::poseidon::{PoseidonError, PoseidonParameters, Rounds,circom::CircomCRH, sbox::PoseidonSbox};
use ark_crypto_primitives::{crh::{TwoToOneCRH}};
use ark_ff::bytes::{FromBytes, ToBytes};
use arkworks_gadgets::utils::{
	get_mds_poseidon_circom_bn254_x5_3, get_rounds_poseidon_circom_bn254_x5_3, parse_vec,
};
use ark_std::{UniformRand, test_rng};

#[derive(Default, Clone)]
pub struct PoseidonCircomRounds3;

impl Rounds for PoseidonCircomRounds3 {
    const FULL_ROUNDS: usize = 8;
    const PARTIAL_ROUNDS: usize = 57;
    const SBOX: PoseidonSbox = PoseidonSbox::Exponentiation(5);
    const WIDTH: usize = 3;
}

pub type PoseidonCircomCRH3 = CircomCRH<Fq, PoseidonCircomRounds3>;


pub fn get_poseidon_ref_hash(left_input: &[u8], right_input: &[u8]) -> Vec<u8> {
    let rounds = get_rounds_poseidon_circom_bn254_x5_3::<Fq>();
	let mds = get_mds_poseidon_circom_bn254_x5_3::<Fq>();
    let params = PoseidonParameters::<Fq>::new(rounds, mds);
        let poseidon_res =
        <PoseidonCircomCRH3 as TwoToOneCRH>::evaluate(&params, &left_input, &right_input)
            .unwrap();
    //assert_eq!(res[0], poseidon_res, "{} != {}", res[0], poseidon_res);
    println!("Arkworks gadget hash 2 inputs {}", poseidon_res );

    let mut out_bytes = [0u8;32];
    <Fq as ToBytes>::write(&poseidon_res, &mut out_bytes[..]);
    out_bytes.to_vec()
}

#[tokio::test]
async fn test_merkle_tree_correct()/*-> io::Result<()>*/ {
    let program_id = Pubkey::from_str("TransferLamports111111111111111111111111111").unwrap();

    let hash_bytes_pubkey = Pubkey::new_unique();
    let merkle_tree_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES);

    let mut program_test = ProgramTest::new(
        "Testing_Hardcoded_Params_devnet_new",
        program_id,
        processor!(process_instruction),
    );

    program_test.add_account(
        hash_bytes_pubkey,
        Account::new(10000000000, 233, &program_id),
    );

    program_test.add_account(
        merkle_tree_pubkey,
        Account::new(10000000000, 135057, &program_id),
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    //initialize MerkleTree account

    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &[240, 0],
            vec![
                AccountMeta::new(payer.pubkey(),true),
                AccountMeta::new(merkle_tree_pubkey, false),
            ],
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);

    banks_client.process_transaction(transaction).await.unwrap();

    let merkle_tree_data = banks_client
        .get_account(merkle_tree_pubkey)
        .await
        .expect("get_account").unwrap();
    //println!("merkletree: {:?}", merkle_tree_data);
    assert_eq!(init_bytes11::INIT_BYTES_MERKLE_TREE_11, merkle_tree_data.data[0..769]);
    println!("initializing merkle tree success");

    //generating random commitment
    // let mut rng = test_rng();
    // let left_input = Fp256::<ark_ed_on_bn254::FqParameters>::rand(&mut rng).into_repr().to_bytes_le();
    // let right_input = Fp256::<ark_ed_on_bn254::FqParameters>::rand(&mut rng).into_repr().to_bytes_le();
    let commit = vec![143, 120, 199, 24, 26, 175, 31, 125, 154, 127, 245, 235, 132, 57, 229, 4, 60, 255, 3, 234, 105, 16, 109, 207, 16, 139, 73, 235, 137, 17, 240, 2];//get_poseidon_ref_hash(&left_input[..], &right_input[..]);

    let mut i = 0;
    for (instruction_id) in &init_bytes11::INSERT_INSTRUCTION_ORDER_11 {
        //println!("instruction data {:?}", [vec![*instruction_id, 0u8], left_input.clone(), right_input.clone(), [i as u8].to_vec() ].concat());
        let instruction_data: Vec<u8> = [vec![*instruction_id, 0u8], commit.clone(), [i as u8].to_vec() ].concat();

        if i == 0 {
            let mut transaction = Transaction::new_with_payer(
                &[Instruction::new_with_bincode(
                    program_id,
                    &instruction_data,
                    vec![
                        AccountMeta::new(payer.pubkey(),true),
                        AccountMeta::new(hash_bytes_pubkey, false),
                        AccountMeta::new(merkle_tree_pubkey, false),
                        AccountMeta::new(hash_bytes_pubkey, false),
                    ],
                )],
                Some(&payer.pubkey()),
            );
            transaction.sign(&[&payer], recent_blockhash);

            banks_client.process_transaction(transaction).await.unwrap();
        } else {
            let mut transaction = Transaction::new_with_payer(
                &[Instruction::new_with_bincode(
                    program_id,
                    &instruction_data,
                    vec![
                        AccountMeta::new(payer.pubkey(),true),
                        AccountMeta::new(hash_bytes_pubkey, false),
                        AccountMeta::new(merkle_tree_pubkey, false),
                    ],
                )],
                Some(&payer.pubkey()),
            );
            transaction.sign(&[&payer], recent_blockhash);

            banks_client.process_transaction(transaction).await.unwrap();
        }
        // if i == 2 {
        //     break;
        // }
        i+=1;
    }
    let storage_account = banks_client
        .get_account(merkle_tree_pubkey)
        .await
        .expect("get_account").unwrap();


    let expected_root = [34, 123, 84, 166, 6, 76, 181, 15, 61, 255, 25, 97, 120, 213, 78, 22, 80, 142, 218, 61, 193, 159, 178, 196, 124, 93, 234, 104, 3, 59, 232, 16];
    //println!("storage_acc: {:?}", storage_account.data[700..(769+128)].to_vec());
    assert_eq!(expected_root, storage_account.data[769..(769+32)]);
    // let storage_account = banks_client
    //     .get_account(merkle_tree_pubkey)
    //     .await
    //     .expect("get_account").unwrap();
    // let mut unpacked_data = vec![0u8;121];
    //
    // unpacked_data = storage_account.data.clone();
    //
    // for i in 1..33 {
    //     print!("{}, ",unpacked_data[i]);
    // }
    // println!("Len data: {}", storage_account.data.len());
    //
    //
    //
    // assert_eq!(unpacked_data[1..33], poseidon_hash_ref);

    //let data = <PoseidonHashMemory as Pack>::unpack_from_slice(&unpacked_data).unwrap();

    // let storage_account = banks_client
    //     .get_packed_account_data::<PoseidonHashMemory>(merkle_tree_pubkey)
    //     .await
    //     .expect("get_packed_account_data");
    //println!("{:?}",unpacked_data[1..33]);
    // let storage_account = banks_client
    //     .get_packed_account_data::<Testing_Hardcoded_Params::PoseidonHashMemory>(merkle_tree_pubkey)
    //     .await
    //     .expect("get_packed_account_data");
    // //let data = Testing_Hardcoded_Params::PoseidonHashMemory::unpack(&storage_account.data).unwrap();

}
#[tokio::test]
async fn test_merkle_tree_fails()/*-> io::Result<()>*/ {
    let program_id = Pubkey::from_str("TransferLamports111111111111111111111111111").unwrap();

    let hash_bytes_pubkey = Pubkey::new_unique();
    let merkle_tree_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES);

    let mut program_test = ProgramTest::new(
        "Testing_Hardcoded_Params_devnet_new",
        program_id,
        processor!(process_instruction),
    );

    program_test.add_account(
        hash_bytes_pubkey,
        Account::new(10000000000, 233, &program_id),
    );

    program_test.add_account(
        merkle_tree_pubkey,
        Account::new(10000000000, 135057, &program_id),
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    //initialize MerkleTree account

    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &[240, 0],
            vec![
                AccountMeta::new(payer.pubkey(),true),
                AccountMeta::new(merkle_tree_pubkey, false),
            ],
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);

    banks_client.process_transaction(transaction).await.unwrap();

    let merkle_tree_data = banks_client
        .get_account(merkle_tree_pubkey)
        .await
        .expect("get_account").unwrap();
    //println!("merkletree: {:?}", merkle_tree_data);
    assert_eq!(init_bytes11::INIT_BYTES_MERKLE_TREE_11, merkle_tree_data.data[0..769]);
    println!("initializing merkle tree success");

    //generating random commitment
    // let mut rng = test_rng();
    // let left_input = Fp256::<ark_ed_on_bn254::FqParameters>::rand(&mut rng).into_repr().to_bytes_le();
    // let right_input = Fp256::<ark_ed_on_bn254::FqParameters>::rand(&mut rng).into_repr().to_bytes_le();
    let commit = vec![1, 120, 199, 24, 26, 175, 31, 125, 154, 127, 245, 235, 132, 57, 229, 4, 60, 255, 3, 234, 105, 16, 109, 207, 16, 139, 73, 235, 137, 17, 240, 2];//get_poseidon_ref_hash(&left_input[..], &right_input[..]);

    let mut i = 0;
    for (instruction_id) in &init_bytes11::INSERT_INSTRUCTION_ORDER_11 {
        //println!("instruction data {:?}", [vec![*instruction_id, 0u8], left_input.clone(), right_input.clone(), [i as u8].to_vec() ].concat());
        let instruction_data: Vec<u8> = [vec![*instruction_id, 0u8], commit.clone(), [i as u8].to_vec() ].concat();

        if i == 0 {
            let mut transaction = Transaction::new_with_payer(
                &[Instruction::new_with_bincode(
                    program_id,
                    &instruction_data,
                    vec![
                        AccountMeta::new(payer.pubkey(),true),
                        AccountMeta::new(hash_bytes_pubkey, false),
                        AccountMeta::new(merkle_tree_pubkey, false),
                        AccountMeta::new(hash_bytes_pubkey, false),
                    ],
                )],
                Some(&payer.pubkey()),
            );
            transaction.sign(&[&payer], recent_blockhash);

            banks_client.process_transaction(transaction).await.unwrap();
        } else {
            let mut transaction = Transaction::new_with_payer(
                &[Instruction::new_with_bincode(
                    program_id,
                    &instruction_data,
                    vec![
                        AccountMeta::new(payer.pubkey(),true),
                        AccountMeta::new(hash_bytes_pubkey, false),
                        AccountMeta::new(merkle_tree_pubkey, false),
                    ],
                )],
                Some(&payer.pubkey()),
            );
            transaction.sign(&[&payer], recent_blockhash);

            banks_client.process_transaction(transaction).await.unwrap();
        }
        // if i == 2 {
        //     break;
        // }
        i+=1;
    }
    let storage_account = banks_client
        .get_account(merkle_tree_pubkey)
        .await
        .expect("get_account").unwrap();


    let expected_root = [34, 123, 84, 166, 6, 76, 181, 15, 61, 255, 25, 97, 120, 213, 78, 22, 80, 142, 218, 61, 193, 159, 178, 196, 124, 93, 234, 104, 3, 59, 232, 16];
    assert!(expected_root != storage_account.data[769..(769+32)]);


}
