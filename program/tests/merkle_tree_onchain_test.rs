use {
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
    },
    solana_program_test::*,
    solana_sdk::{account::Account, signature::Signer, transaction::Transaction, msg},
    Testing_Hardcoded_Params_devnet_new::{process_instruction, state_merkle_tree::{MerkleTree,HashBytes,MERKLE_TREE_ACC_BYTES}, init_bytes18},
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
use solana_program_test::ProgramTestContext;
use crate::tokio::time::timeout;
use std::{thread, time};

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

async fn create_and_start_program(
        merkle_tree_init_bytes: Vec<u8>,
        hash_bytes_init_bytes: Vec<u8>,
        merkle_tree_pubkey: &Pubkey,
        hash_bytes_pubkey: &Pubkey,
        two_leaves_pda_pubkey: &Pubkey,
        program_id: &Pubkey,
        signer_pubkey: &Pubkey
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
    program_test.add_account(
        *merkle_tree_pubkey,
        merkle_tree,
    );
    let mut hash_byte = Account::new(10000000000, 297, &program_id);

    if hash_bytes_init_bytes.len() == 297 {

        hash_byte.data = hash_bytes_init_bytes;
    }
    program_test.add_account(
        *hash_bytes_pubkey,
        hash_byte,
    );
    let mut two_leaves_pda_byte = Account::new(10000000000, 98, &program_id);

    // if two_leaves_pda_bytes_init_bytes.len() == 98 {
    //
    //     two_leaves_pda_byte.data = two_leaves_pda_bytes_init_bytes;
    // }
    program_test.add_account(
        *two_leaves_pda_pubkey,
        two_leaves_pda_byte,
    );

    let mut program_context = program_test.start_with_context().await;
    let mut transaction = solana_sdk::system_transaction::transfer(&program_context.payer, &signer_pubkey, 10000000000000, program_context.last_blockhash);
    transaction.sign(&[&program_context.payer], program_context.last_blockhash);
    let res_request = program_context.banks_client.process_transaction(transaction).await;

    program_context

}

#[tokio::test]
async fn test_merkle_tree_correct()/*-> io::Result<()>*/ {
    let program_id = Pubkey::from_str("TransferLamports111111111111111111111111111").unwrap();

    let hash_bytes_pubkey = Pubkey::new_unique();
    let two_leaves_pda_pubkey = Pubkey::new_unique();

    println!("HashBytes {:?}", hash_bytes_pubkey);
    let merkle_tree_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES);


    let signer_keypair = solana_sdk::signer::keypair::Keypair::new();

    let signer_pubkey = signer_keypair.pubkey();

    //let (mut program_context.banks_client, signer_keypair, program_context.last_blockhash) = program_test.start().await;
    let mut program_context = create_and_start_program(vec![0], vec![0], &merkle_tree_pubkey, &hash_bytes_pubkey, &two_leaves_pda_pubkey, &program_id, &signer_pubkey).await;

    //initialize MerkleTree account

    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &[vec![240u8, 0u8], usize::to_le_bytes(1000).to_vec()].concat(),
            vec![
                AccountMeta::new(signer_keypair.pubkey(),true),
                AccountMeta::new(merkle_tree_pubkey, false),
            ],
        )],
        Some(&signer_keypair.pubkey()),
    );
    transaction.sign(&[&signer_keypair], program_context.last_blockhash);

    program_context.banks_client.process_transaction(transaction).await.unwrap();

    let merkle_tree_data = program_context.banks_client
        .get_account(merkle_tree_pubkey)
        .await
        .expect("get_account").unwrap();
    //println!("merkletree: {:?}", merkle_tree_data);
    assert_eq!(init_bytes18::INIT_BYTES_MERKLE_TREE_18, merkle_tree_data.data[0..641]);
    //assert_eq!(true, false);
    println!("initializing merkle tree success");

    //generating random commitment
    // let mut rng = test_rng();
    // let left_input = Fp256::<ark_ed_on_bn254::FqParameters>::rand(&mut rng).into_repr().to_bytes_le();
    // let right_input = Fp256::<ark_ed_on_bn254::FqParameters>::rand(&mut rng).into_repr().to_bytes_le();
    let commit = vec![143, 120, 199, 24, 26, 175, 31, 125, 154, 127, 245, 235, 132, 57, 229, 4, 60, 255, 3, 234, 105, 16, 109, 207, 16, 139, 73, 235, 137, 17, 240, 2];//get_poseidon_ref_hash(&left_input[..], &right_input[..]);

    let mut i = 0;
    for (instruction_id) in &init_bytes18::INSERT_INSTRUCTION_ORDER_18 {
        //println!("instruction data {:?}", [vec![*instruction_id, 0u8], left_input.clone(), right_input.clone(), [i as u8].to_vec() ].concat());
        let instruction_data: Vec<u8> = [vec![*instruction_id, 0u8], commit.clone(), commit.clone(), [i as u8].to_vec() ].concat();

        if i == 0 {
            let mut transaction = Transaction::new_with_payer(
                &[Instruction::new_with_bincode(
                    program_id,
                    &instruction_data,
                    vec![
                        AccountMeta::new(signer_keypair.pubkey(),true),
                        AccountMeta::new(hash_bytes_pubkey, false),
                        AccountMeta::new(merkle_tree_pubkey, false),
                        AccountMeta::new(hash_bytes_pubkey, false),
                    ],
                )],
                Some(&signer_keypair.pubkey()),
            );
            transaction.sign(&[&signer_keypair], program_context.last_blockhash);

            program_context.banks_client.process_transaction(transaction).await.unwrap();
        } else if i == init_bytes18::INSERT_INSTRUCTION_ORDER_18.len()-1 {
            println!("Last tx ------------------------------");
            let mut transaction = Transaction::new_with_payer(
                &[Instruction::new_with_bincode(
                    program_id,
                    &instruction_data,
                    vec![
                        AccountMeta::new(signer_keypair.pubkey(),true),
                        AccountMeta::new(hash_bytes_pubkey, false),
                        AccountMeta::new(merkle_tree_pubkey, false),
                        AccountMeta::new(two_leaves_pda_pubkey, false),
                    ],
                )],
                Some(&signer_keypair.pubkey()),
            );
            transaction.sign(&[&signer_keypair], program_context.last_blockhash);

            program_context.banks_client.process_transaction(transaction).await.unwrap();
        } else {
            let mut success = false;
            let mut retries_left = 2;
            while(retries_left > 0 && success != true ) {
                let mut transaction = Transaction::new_with_payer(
                    &[Instruction::new_with_bincode(
                        program_id,
                        &instruction_data,
                        vec![
                            AccountMeta::new(signer_keypair.pubkey(),true),
                            AccountMeta::new(hash_bytes_pubkey, false),
                            AccountMeta::new(merkle_tree_pubkey, false),
                        ],
                    )],
                    Some(&signer_keypair.pubkey()),
                );
                transaction.sign(&[&signer_keypair], program_context.last_blockhash);

                //program_context.banks_client.process_transaction(transaction).await.unwrap();

                let res_request = timeout(time::Duration::from_millis(500), program_context.banks_client.process_transaction(transaction)).await;
                //let ten_millis = time::Duration::from_millis(400);

                //thread::sleep(ten_millis);
                //println!("res: {:?}", res_request);
                match res_request {
                    Ok(_) => success = true,
                    Err(e) => {

                        println!("retries_left {}", retries_left);
                        retries_left -=1;
                        let merkle_tree_account = program_context.banks_client
                            .get_account(merkle_tree_pubkey)
                            .await
                            .expect("get_account").unwrap();
                        let hash_bytes_account = program_context.banks_client
                            .get_account(hash_bytes_pubkey)
                            .await
                            .expect("get_account").unwrap();
                        //println!("data: {:?}", storage_account.data);
                        //let old_payer = signer_keypair;
                        program_context = create_and_start_program(
                            merkle_tree_account.data.to_vec(),
                            hash_bytes_account.data.to_vec(),
                            &merkle_tree_pubkey,
                            &hash_bytes_pubkey,
                            &two_leaves_pda_pubkey,
                            &program_id,
                            &signer_pubkey
                        ).await;
                        //assert_eq!(signer_keypair, old_payer);
                        let merkle_tree_account_new = program_context.banks_client
                            .get_account(merkle_tree_pubkey)
                            .await
                            .expect("get_account").unwrap();
                        let hash_bytes_account_new = program_context.banks_client
                            .get_account(hash_bytes_pubkey)
                            .await
                            .expect("get_account").unwrap();
                        assert_eq!(merkle_tree_account_new.data, merkle_tree_account.data);
                        assert_eq!(hash_bytes_account_new.data, hash_bytes_account.data);

                    },
                }
            }

        }
        println!("Instruction index {}", i);
        i+=1;
    }
    let storage_account = program_context.banks_client
        .get_account(merkle_tree_pubkey)
        .await
        .expect("get_account").unwrap();


    let expected_root = [247, 16, 124, 67, 44, 62, 195, 226, 182, 62, 41, 237, 78, 64, 195, 249, 67, 169, 200, 24, 158, 153, 57, 144, 24, 245, 131, 44, 127, 129, 44, 10];
    //println!("storage_acc: {:?}", storage_account.data[700..(769+128)].to_vec());
    assert_eq!(expected_root, storage_account.data[609 +32..(609+64)]);

    let storage_account = program_context.banks_client
        .get_account(two_leaves_pda_pubkey)
        .await
        .expect("get_account").unwrap();

    assert_eq!(1, storage_account.data[0]);
    assert_eq!(4, storage_account.data[1]);

    assert_eq!(commit, storage_account.data[2..34]);
    assert_eq!(commit, storage_account.data[34..66]);
    assert_eq!(MERKLE_TREE_ACC_BYTES, storage_account.data[66..98]);

    println!("pda_account_data = : {:?}", storage_account.data);
    // let storage_account = program_context.banks_client
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

    // let storage_account = program_context.banks_client
    //     .get_packed_account_data::<PoseidonHashMemory>(merkle_tree_pubkey)
    //     .await
    //     .expect("get_packed_account_data");
    //println!("{:?}",unpacked_data[1..33]);
    // let storage_account = program_context.banks_client
    //     .get_packed_account_data::<Testing_Hardcoded_Params::PoseidonHashMemory>(merkle_tree_pubkey)
    //     .await
    //     .expect("get_packed_account_data");
    // //let data = Testing_Hardcoded_Params::PoseidonHashMemory::unpack(&storage_account.data).unwrap();

}
// #[tokio::test]
// async fn test_merkle_tree_fails()/*-> io::Result<()>*/ {
//     let program_id = Pubkey::from_str("TransferLamports111111111111111111111111111").unwrap();
//
//     let hash_bytes_pubkey = Pubkey::new_unique();
//     let merkle_tree_pubkey = Pubkey::new(&MERKLE_TREE_ACC_BYTES);
//
//     let mut program_test = ProgramTest::new(
//         "Testing_Hardcoded_Params_devnet_new",
//         program_id,
//         processor!(process_instruction),
//     );
//
//     program_test.add_account(
//         hash_bytes_pubkey,
//         Account::new(10000000000, 233, &program_id),
//     );
//
//     program_test.add_account(
//         merkle_tree_pubkey,
//         Account::new(10000000000, 16657, &program_id),
//     );
//
//     let (mut program_context.banks_client, signer_keypair, program_context.last_blockhash) = program_test.start().await;
//
//     //initialize MerkleTree account
//
//     let mut transaction = Transaction::new_with_payer(
//         &[Instruction::new_with_bincode(
//             program_id,
//             &[240, 0],
//             vec![
//                 AccountMeta::new(signer_keypair.pubkey(),true),
//                 AccountMeta::new(merkle_tree_pubkey, false),
//             ],
//         )],
//         Some(&signer_keypair.pubkey()),
//     );
//     transaction.sign(&[&signer_keypair], program_context.last_blockhash);
//
//     program_context.banks_client.process_transaction(transaction).await.unwrap();
//
//     let merkle_tree_data = program_context.banks_client
//         .get_account(merkle_tree_pubkey)
//         .await
//         .expect("get_account").unwrap();
//     //println!("merkletree: {:?}", merkle_tree_data);
//     assert_eq!(init_bytes18::INIT_BYTES_MERKLE_TREE_11, merkle_tree_data.data[0..769]);
//     println!("initializing merkle tree success");
//
//     //generating random commitment
//     // let mut rng = test_rng();
//     // let left_input = Fp256::<ark_ed_on_bn254::FqParameters>::rand(&mut rng).into_repr().to_bytes_le();
//     // let right_input = Fp256::<ark_ed_on_bn254::FqParameters>::rand(&mut rng).into_repr().to_bytes_le();
//     let commit = vec![1, 120, 199, 24, 26, 175, 31, 125, 154, 127, 245, 235, 132, 57, 229, 4, 60, 255, 3, 234, 105, 16, 109, 207, 16, 139, 73, 235, 137, 17, 240, 2];//get_poseidon_ref_hash(&left_input[..], &right_input[..]);
//
//     let mut i = 0;
//     for (instruction_id) in &init_bytes18::INIT_BYTES_MERKLE_TREE_18 {
//         //println!("instruction data {:?}", [vec![*instruction_id, 0u8], left_input.clone(), right_input.clone(), [i as u8].to_vec() ].concat());
//         let instruction_data: Vec<u8> = [vec![*instruction_id, 0u8], commit.clone(), [i as u8].to_vec() ].concat();
//
//         if i == 0 {
//             let mut transaction = Transaction::new_with_payer(
//                 &[Instruction::new_with_bincode(
//                     program_id,
//                     &instruction_data,
//                     vec![
//                         AccountMeta::new(signer_keypair.pubkey(),true),
//                         AccountMeta::new(hash_bytes_pubkey, false),
//                         AccountMeta::new(merkle_tree_pubkey, false),
//                         AccountMeta::new(hash_bytes_pubkey, false),
//                     ],
//                 )],
//                 Some(&signer_keypair.pubkey()),
//             );
//             transaction.sign(&[&signer_keypair], program_context.last_blockhash);
//
//             program_context.banks_client.process_transaction(transaction).await.unwrap();
//         } else {
//             let mut transaction = Transaction::new_with_payer(
//                 &[Instruction::new_with_bincode(
//                     program_id,
//                     &instruction_data,
//                     vec![
//                         AccountMeta::new(signer_keypair.pubkey(),true),
//                         AccountMeta::new(hash_bytes_pubkey, false),
//                         AccountMeta::new(merkle_tree_pubkey, false),
//                     ],
//                 )],
//                 Some(&signer_keypair.pubkey()),
//             );
//             transaction.sign(&[&signer_keypair], program_context.last_blockhash);
//
//             program_context.banks_client.process_transaction(transaction).await.unwrap();
//         }
//
//         i+=1;
//     }
//     let storage_account = program_context.banks_client
//         .get_account(merkle_tree_pubkey)
//         .await
//         .expect("get_account").unwrap();
//
//
//     let expected_root = [34, 123, 84, 166, 6, 76, 181, 15, 61, 255, 25, 97, 120, 213, 78, 22, 80, 142, 218, 61, 193, 159, 178, 196, 124, 93, 234, 104, 3, 59, 232, 16];
//     assert!(expected_root != storage_account.data[769..(769+32)]);
//
//
// }
