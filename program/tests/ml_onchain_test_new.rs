// use crate::tokio::time::timeout;
// use ark_crypto_primitives::Error;
// use ark_ff::biginteger::BigInteger256;
// use ark_ff::{Fp256, FromBytes};
// use solana_program::program_pack::Pack;
// use Testing_Hardcoded_Params_devnet_new::{
//     poseidon_merkle_tree::mt_state::{HashBytes, MerkleTree, MERKLE_TREE_ACC_BYTES},
//     process_instruction,
//     utils::config,
//     Groth16_verifier::{
//         final_exponentiation::fe_ranges::*,
//         final_exponentiation::fe_state::{FinalExpBytes, INSTRUCTION_ORDER_VERIFIER_PART_2},
//         miller_loop::ml_state::*,
//         parsers::*,
//     },
// };

// use serde_json::{Result, Value};
// use {
//     solana_program::{
//         instruction::{AccountMeta, Instruction},
//         pubkey::Pubkey,
//     },
//     solana_program_test::*,
//     solana_sdk::{account::Account, signature::Signer, transaction::Transaction},
//     std::str::FromStr,
// };

// use ark_ed_on_bn254::Fq;
// use ark_ff::BigInteger;
// use solana_program_test::ProgramTestContext;
// use std::convert::TryInto;
// use std::{fs, time};
// mod fe_onchain_test;

// //mod tests::fe_onchain_test;
// //use crate::fe_onchain_test;
// mod mt_onchain_test;

// mod pi_onchain_test;
// // mod fe_offchain_test;
// // use crate::fe_offchain_test::tests::get_public_inputs_from_bytes_254;

// pub async fn create_and_start_program(
//     merkle_tree_init_bytes: Vec<u8>,
//     hash_bytes_init_bytes: Vec<u8>,
//     merkle_tree_pubkey: &Pubkey,
//     storage_account: &Pubkey,
//     program_id: &Pubkey,
//     signer_pubkey: &Pubkey,
// ) -> ProgramTestContext {
//     let mut program_test = ProgramTest::new(
//         "Testing_Hardcoded_Params_devnet_new",
//         *program_id,
//         processor!(process_instruction),
//     );
//     let mut merkle_tree = Account::new(10000000000, 16658, &program_id);

//     if merkle_tree_init_bytes.len() == 16658 {
//         merkle_tree.data = merkle_tree_init_bytes;
//     }
//     program_test.add_account(*merkle_tree_pubkey, merkle_tree);
//     let mut hash_byte = Account::new(10000000000, 3900, &program_id);

//     if hash_bytes_init_bytes.len() == 3900 {
//         hash_byte.data = hash_bytes_init_bytes;
//     }
//     program_test.add_account(*storage_account, hash_byte);

//     let mut program_context = program_test.start_with_context().await;
//     let mut transaction = solana_sdk::system_transaction::transfer(
//         &program_context.payer,
//         &signer_pubkey,
//         10000000000000,
//         program_context.last_blockhash,
//     );
//     transaction.sign(&[&program_context.payer], program_context.last_blockhash);
//     let res_request = program_context
//         .banks_client
//         .process_transaction(transaction)
//         .await;

//     program_context
// }

// pub async fn create_and_start_program_with_nullfier_pdas(
//     merkle_tree_init_bytes: Vec<u8>,
//     hash_bytes_init_bytes: Vec<u8>,
//     merkle_tree_pubkey: &Pubkey,
//     storage_account: &Pubkey,
//     two_leaves_pda_pubkey: &Pubkey,
//     nullifier_pubkeys: &Vec<Pubkey>,
//     program_id: &Pubkey,
//     signer_pubkey: &Pubkey,
// ) -> ProgramTestContext {
//     let mut program_test = ProgramTest::new(
//         "Testing_Hardcoded_Params_devnet_new",
//         *program_id,
//         processor!(process_instruction),
//     );
//     let mut merkle_tree = Account::new(10000000000, 16658, &program_id);

//     if merkle_tree_init_bytes.len() == 16658 {
//         merkle_tree.data = merkle_tree_init_bytes;
//     }

//     program_test.add_account(*merkle_tree_pubkey, merkle_tree);
//     let mut hash_byte = Account::new(10000000000, 3900, &program_id);

//     if hash_bytes_init_bytes.len() == 3900 {
//         hash_byte.data = hash_bytes_init_bytes;
//     }
//     program_test.add_account(*storage_account, hash_byte);
//     let mut two_leaves_pda_byte = Account::new(1100000000, 98, &program_id);

//     program_test.add_account(*two_leaves_pda_pubkey, two_leaves_pda_byte);

//     for pubkey in nullifier_pubkeys.iter() {
//         program_test.add_account(*pubkey, Account::new(10000000, 2, &program_id));
//     }

//     let mut program_context = program_test.start_with_context().await;
//     let mut transaction = solana_sdk::system_transaction::transfer(
//         &program_context.payer,
//         &signer_pubkey,
//         10000000000000,
//         program_context.last_blockhash,
//     );
//     transaction.sign(&[&program_context.payer], program_context.last_blockhash);
//     let res_request = program_context
//         .banks_client
//         .process_transaction(transaction)
//         .await;

//     program_context
// }

// #[tokio::test]
// async fn miller_loop_onchain_test() {
//     // Executes first ix: [0]
//     let i_data0: Vec<u8> = vec![0; 2];
//     let mut transaction = Transaction::new_with_payer(
//         &[Instruction::new_with_bincode(
//             program_id,
//             &[vec![1, 1], i_data0].concat(),
//             vec![
//                 AccountMeta::new(signer_pubkey, true),
//                 AccountMeta::new(storage_pubkey, false),
//             ],
//         )],
//         Some(&signer_pubkey),
//     );
//     transaction.sign(&[&signer_keypair], program_context.last_blockhash);
//     program_context
//         .banks_client
//         .process_transaction(transaction)
//         .await
//         .unwrap();

//     // Executes second ix: [1]
//     // Parses proof_a and proof_c bytes ()
//     let i_data: Vec<u8> = vec![0]; //[proof_a_bytes, proof_c_bytes].concat(); // 128 b
//     let mut transaction = Transaction::new_with_payer(
//         &[Instruction::new_with_bincode(
//             program_id,
//             &[vec![1, 1], i_data].concat(), // 129++
//             vec![
//                 AccountMeta::new(signer_pubkey, true),
//                 AccountMeta::new(storage_pubkey, false),
//             ],
//         )],
//         Some(&signer_pubkey),
//     );
//     transaction.sign(&[&signer_keypair], program_context.last_blockhash);
//     program_context
//         .banks_client
//         .process_transaction(transaction)
//         .await
//         .unwrap();

//     // Executes third ix [2]
//     // Parses proof_b_bytes (2..194) // 128 b
//     let i_data_2: Vec<u8> = vec![0]; //proof_b_bytes[..].to_vec();
//     let mut transaction = Transaction::new_with_payer(
//         &[Instruction::new_with_bincode(
//             program_id,
//             &[vec![1, 1], i_data_2].concat(),
//             vec![
//                 AccountMeta::new(signer_pubkey, true),
//                 AccountMeta::new(storage_pubkey, false),
//             ],
//         )],
//         Some(&signer_pubkey),
//     );
//     transaction.sign(&[&signer_keypair], program_context.last_blockhash);
//     program_context
//         .banks_client
//         .process_transaction(transaction)
//         .await
//         .unwrap();

//     let storage_account = program_context
//         .banks_client
//         .get_account(storage_pubkey)
//         .await
//         .expect("get_account")
//         .unwrap();
//     let account_data = ML254Bytes::unpack(&storage_account.data.clone()).unwrap();
//     println!("init state f_range: {:?}", account_data.f_range);
//     println!("init state P1x: {:?}", account_data.p_1_x_range);
//     println!("init state P1y: {:?}", account_data.p_1_y_range);

//     println!("init state P2x: {:?}", account_data.p_2_x_range);
//     println!("init state P2y: {:?}", account_data.p_2_y_range);

//     println!("init state P3x: {:?}", account_data.p_3_x_range);
//     println!("init state P3y: {:?}", account_data.p_3_y_range);

//     println!("init state PROOFB: {:?}", account_data.proof_b);
//     //assert_eq!(true, false);
//     // Executes 1973 following ix.
//     println!("xxxxx");
//     let mut i = 0usize;
//     for _id in 3..431usize {
//         // 3..612 @merging helpers and add step
//         // 3..639 @14,15,16 merged
//         // 3..693 11,12,13 merged
//         // 3..821 @3,4 merged
//         // 3..884 @d1-d5 merged
//         // 3..1157 @d2-d5 merged
//         // 3..1976
//         let mut success = false;
//         let mut retries_left = 2;
//         while retries_left > 0 && success != true {
//             let mut transaction = Transaction::new_with_payer(
//                 &[Instruction::new_with_bincode(
//                     program_id,
//                     &[vec![1, 1], usize::to_le_bytes(i).to_vec()].concat(),
//                     vec![
//                         AccountMeta::new(signer_pubkey, true),
//                         AccountMeta::new(storage_pubkey, false),
//                     ],
//                 )],
//                 Some(&signer_pubkey),
//             );
//             transaction.sign(&[&signer_keypair], program_context.last_blockhash);
//             let res_request = timeout(
//                 time::Duration::from_millis(500),
//                 program_context
//                     .banks_client
//                     .process_transaction(transaction),
//             )
//             .await;
//             match res_request {
//                 Ok(_) => success = true,
//                 Err(_e) => {
//                     println!("retries_left {}", retries_left);
//                     retries_left -= 1;
//                     let storage_account = program_context
//                         .banks_client
//                         .get_account(storage_pubkey)
//                         .await
//                         .expect("get_account")
//                         .unwrap();
//                     // program_context = create_and_start_program(
//                     //     storage_account.data.to_vec(),
//                     //     storage_pubkey,
//                     //     pi_bytes_pubkey,
//                     //     program_id,
//                     // )
//                     // .await;
//                     program_context = create_and_start_program(
//                         merkle_tree_account.data.to_vec(),
//                         storage_account.data.to_vec(),
//                         &merkle_tree_pubkey,
//                         &storage_pubkey,
//                         &program_id,
//                         &signer_pubkey,
//                     )
//                     .await;
//                 }
//             }
//         }
//         i += 1;
//     }

//     // Compute the affine value from this and compare to the (hardcoded) value that's returned from
//     // prepare_inputs lib call/reference.
//     let storage_account = program_context
//         .banks_client
//         .get_account(storage_pubkey)
//         .await
//         .expect("get_account")
//         .unwrap();
//     let account_data = ML254Bytes::unpack(&storage_account.data.clone()).unwrap();

//     // = ark_groth16-miller_output reference
//     let reference_f = [
//         41, 164, 125, 219, 237, 181, 202, 195, 98, 55, 97, 232, 35, 147, 153, 23, 164, 70, 211,
//         144, 151, 9, 219, 197, 234, 13, 164, 242, 67, 59, 148, 5, 132, 108, 82, 161, 228, 167, 20,
//         24, 207, 201, 203, 25, 249, 125, 54, 96, 182, 231, 150, 215, 149, 43, 216, 0, 36, 166, 232,
//         13, 126, 3, 53, 0, 174, 209, 16, 242, 177, 143, 60, 247, 181, 65, 132, 142, 14, 231, 170,
//         52, 3, 34, 70, 49, 210, 158, 211, 173, 165, 155, 219, 80, 225, 32, 64, 8, 65, 139, 16, 138,
//         240, 218, 36, 220, 8, 100, 236, 141, 1, 223, 60, 59, 24, 38, 90, 254, 47, 91, 205, 228,
//         169, 103, 178, 30, 124, 141, 43, 9, 83, 155, 75, 140, 209, 26, 2, 250, 250, 20, 185, 78,
//         53, 54, 68, 178, 88, 78, 246, 132, 97, 167, 124, 253, 96, 26, 213, 99, 157, 155, 40, 9, 60,
//         139, 112, 126, 230, 195, 217, 125, 68, 169, 208, 149, 175, 33, 226, 17, 47, 132, 8, 154,
//         237, 156, 34, 97, 55, 129, 155, 64, 202, 54, 161, 19, 24, 1, 208, 104, 140, 149, 25, 229,
//         96, 239, 202, 24, 235, 221, 133, 137, 30, 226, 62, 112, 26, 58, 1, 85, 207, 182, 41, 213,
//         42, 72, 139, 41, 108, 152, 252, 164, 121, 76, 17, 62, 147, 226, 220, 79, 236, 132, 109,
//         130, 163, 209, 203, 14, 144, 180, 25, 216, 234, 198, 199, 74, 48, 62, 57, 0, 206, 138, 12,
//         130, 25, 12, 187, 216, 86, 208, 84, 198, 58, 204, 6, 161, 93, 63, 68, 121, 173, 129, 255,
//         249, 47, 42, 218, 214, 129, 29, 136, 7, 213, 160, 139, 148, 58, 6, 191, 11, 161, 114, 56,
//         174, 224, 86, 243, 103, 166, 151, 107, 36, 205, 170, 206, 196, 248, 251, 147, 91, 3, 136,
//         208, 36, 3, 51, 84, 102, 139, 252, 193, 9, 172, 113, 116, 50, 242, 70, 26, 115, 166, 252,
//         204, 163, 149, 78, 13, 255, 235, 222, 174, 120, 182, 178, 186, 22, 169, 153, 73, 48, 242,
//         139, 120, 98, 33, 101, 204, 204, 169, 57, 249, 168, 45, 197, 126, 105, 54, 187, 35, 241,
//         253, 4, 33, 70, 246, 206, 32, 17,
//     ];
//     // assert_eq!(
//     //     account_data.f_range, reference_f,
//     //     "onchain f result != reference f (hardcoded from lib call)"
//     // );
//     println!("onchain test success");
// }
