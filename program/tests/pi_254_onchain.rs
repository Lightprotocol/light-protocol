use crate::tokio::time::timeout;
use ark_ff::biginteger::BigInteger256;
use ark_ff::{Fp256, ToBytes};
use ark_bn254;
use std::time;
use Testing_Hardcoded_Params_devnet_new::pi_254_parsers::parse_x_group_affine_from_bytes_254;
use {
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey
    },
    solana_program_test::*,
    solana_sdk::{account::Account, signature::Signer, transaction::Transaction},
    std::str::FromStr,
    Testing_Hardcoded_Params_devnet_new::process_instruction,
};
use ark_crypto_primitives::{Error};
// use ark_groth16::prepare_verifying_key;
// use ark_groth16::{verify_proof, prepare_inputs, verify_proof_with_prepared_inputs};
mod verifier_final_exp_test;
use crate::verifier_final_exp_test::tests::get_public_inputs_from_bytes_254;

async fn create_and_start_program(
    account_init_bytes: Vec<u8>,
    storage_pubkey: Pubkey,
    program_id: Pubkey,
) -> ProgramTestContext {
    let mut program_test = ProgramTest::new(
        "Testing_Hardcoded_Params_devnet_new",
        program_id,
        processor!(process_instruction),
    );

    // Initializes acc based on x state and y pubkey
    let mut account_storage = Account::new(10000000000, 3900, &program_id);
    account_storage.data = account_init_bytes;
    program_test.add_account(storage_pubkey, account_storage);

    let tmp = program_test.start_with_context().await;
    println!("program started w/ context");
    tmp
}

// TODO: execute prepare_inputs lib call before with the same inputs as onchain, write g_ic to
// file, and read the value here for an assert. Need to make it compile with ark-groth16 before.
#[tokio::test]
async fn test_pi_254_onchain() -> Result<(), Error>{

    // let pvk_unprepped = verifier_final_exp_test::tests::verifier_final_exp_test::get_pvk_from_bytes_254()?;
    // let pvk = prepare_verifying_key(&pvk_unprepped);
    // let proof = verifier_final_exp_test::tests::verifier_final_exp_test::get_proof_from_bytes_254()?;
    //
    // let public_inputs = verifier_final_exp_test::tests::verifier_final_exp_test::get_public_inputs_from_bytes_254()?;
    //
    // let prepared_inputs = prepare_inputs(&pvk, &public_inputs).unwrap();
    //
    // println!("prepared_inputs: {:?}", prepared_inputs);
    // assert_eq!(true, false);
    // Creates program, accounts, setup.
    let public_inputs = get_public_inputs_from_bytes_254()?;
    let mut input_bytes_from_file = vec![0u8;224];
    for i in 0..7 {

        <Fp256<ark_bn254::FrParameters> as ToBytes>::write(&public_inputs[i], &mut input_bytes_from_file[(i * 32)..(i * 32 + 32)]); // i 0..48

    }
    //println!("input bytes: {:?}", input_bytes_from_file);
    let program_id = Pubkey::from_str("TransferLamports111111111551111111111111111").unwrap();

    let init_bytes_storage: [u8; 3900] = [0; 3900];
    let storage_pubkey = Pubkey::new_unique();

    let mut program_context =
        create_and_start_program(init_bytes_storage.to_vec(), storage_pubkey, program_id).await;


    // Executes first ix with input data, this would come from client.
    let inputs_bytes: Vec<u8> = vec![
        40, 3, 89,73,181,223,102,213,65,254,19,15,156,236,156,28,242,244,137,6,141,198,148,190,214,144,232,66,205,181,194,5,202,36,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,177,80,240,13,221,245,120,254,37,1,128,209,15,117,127,48,128,212,60,0,139,4,61,76,148,132,161,75,84,166,205,39,52,242,73,174,72,47,8,123,19,170,193,153,185,200,250,205,155,186,207,101,19,135,243,148,46,174,54,70,192,214,240,29,66,73,129,124,71,219,147,101,210,32,61,169,93,102,2,121,61,214,146,36,73,175,30,191,82,205,197,197,28,204,51,18, 49,111,114,83,155,28,206,135,243,18,244,157,59,217,100,227,113,62,168,167,211,92,221,133,29,12,187,219,16,97,59,12,160,198,101,107,12,153,155,83,44,166,226,22,139,237,186,192,170,237,48,183,223,198,243,73,14,161,131,26,151,8,45,1
    ];
    let inputs_bytes_old: Vec<u8> = vec![
        40, 3, 139, 101, 98, 198, 106, 26, 157, 253, 217, 85, 208, 20, 62, 194, 7, 229, 230, 196,
        195, 91, 112, 106, 227, 5, 89, 90, 68, 176, 218, 172, 23, 34, 1, 0, 63, 128, 161, 110, 190,
        67, 145, 112, 185, 121, 72, 232, 51, 40, 93, 88, 129, 129, 182, 69, 80, 184, 41, 160, 49,
        225, 114, 78, 100, 48, 224, 137, 70, 92, 255, 138, 142, 119, 60, 162, 100, 218, 34, 199,
        20, 246, 167, 35, 235, 134, 225, 54, 67, 209, 246, 194, 128, 223, 27, 115, 112, 25, 13,
        113, 159, 110, 133, 81, 26, 27, 23, 26, 184, 1, 175, 109, 99, 85, 188, 45, 119, 213, 233,
        137, 186, 52, 25, 2, 52, 160, 2, 122, 107, 18, 62, 183, 110, 221, 22, 145, 254, 220, 22,
        239, 208, 169, 202, 190, 70, 169, 206, 157, 185, 145, 226, 81, 196, 182, 29, 125, 181, 119,
        242, 71, 107, 10, 167, 4, 10, 212, 160, 90, 85, 209, 147, 16, 119, 99, 254, 93, 143, 137,
        91, 121, 198, 246, 245, 79, 190, 201, 63, 229, 250, 134, 157, 180, 3, 12, 228, 236, 174,
        112, 138, 244, 188, 161, 144, 60, 210, 99, 115, 64, 69, 63, 35, 176, 250, 189, 20, 28, 23,
        2, 19, 94, 196, 88, 14, 51, 12, 21,
    ];
    println!("inputs bytes length: {} {}", inputs_bytes.len(), inputs_bytes.len());
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &[inputs_bytes],
            vec![
                AccountMeta::new(program_context.payer.pubkey(), true),
                AccountMeta::new(storage_pubkey, false),
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
                        AccountMeta::new(program_context.payer.pubkey(), true),
                        AccountMeta::new(storage_pubkey, false),
                    ],
                )],
                Some(&program_context.payer.pubkey()),
            );
            transaction.sign(&[&program_context.payer], program_context.last_blockhash);
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
                    program_context = create_and_start_program(
                        storage_account.data.to_vec(),
                        storage_pubkey,
                        program_id,
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

    // x_1_range: 252..316.
    // Keep in mind that g_ic_reference_value is based on running groth16.prepare_inputs() with 7 hardcoded inputs.
    let g_ic_projective = parse_x_group_affine_from_bytes_254(&unpacked_data[252..316].to_vec());
    let g_ic_reference_value =
        ark_ec::short_weierstrass_jacobian::GroupProjective::<ark_bn254::g1::Parameters>::new(
            Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
                4558364185828577028, 2968328072905581441, 15831331149718564992, 1208602698044891702
            ])), // Cost: 31
            Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
                15482105712819104980, 10686255431817088435, 17716373216643709577, 264028719181254570
            ])), // Cost: 31
            Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
                13014122463130548586, 16367981906331090583, 13731940001588685782, 2029626530375041604
            ])), // Cost: 31
        );
    assert_eq!(
        g_ic_projective, g_ic_reference_value,
        "different g_ic projective than libray implementation with the same inputs"
    );
    Ok(())
}
