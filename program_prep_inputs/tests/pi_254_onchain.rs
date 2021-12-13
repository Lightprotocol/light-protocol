use crate::tokio::time::timeout;
use ark_ff::biginteger::BigInteger256;
use ark_ff::Fp256;
use std::time;
use Prepare_Inputs::pi_254_parsers::parse_x_group_affine_from_bytes_254;
use {
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
    },
    solana_program_test::*,
    solana_sdk::{account::Account, signature::Signer, transaction::Transaction},
    std::str::FromStr,
    Prepare_Inputs::process_instruction,
};

async fn create_and_start_program(
    account_init_bytes: Vec<u8>,
    storage_pubkey: Pubkey,
    program_id: Pubkey,
) -> ProgramTestContext {
    let mut program_test = ProgramTest::new(
        "Prepare_Inputs",
        program_id,
        processor!(process_instruction),
    );

    // Initializes acc based on x state and y pubkey
    let mut account_storage = Account::new(10000000000, 4972, &program_id);
    account_storage.data = account_init_bytes;
    program_test.add_account(storage_pubkey, account_storage);

    let tmp = program_test.start_with_context().await;
    println!("program started w/ context");
    tmp
}

// TODO: execute prepare_inputs lib call before with the same inputs as onchain, write g_ic to
// file, and read the value here for an assert. Need to make it compile with ark-groth16 before.
#[tokio::test]
async fn test_pi_254_onchain() {
    // Creates program, accounts, setup.
    let program_id = Pubkey::from_str("TransferLamports111111111551111111111111111").unwrap();

    let init_bytes_storage: [u8; 4972] = [0; 4972];
    let storage_pubkey = Pubkey::new_unique();

    let mut program_context =
        create_and_start_program(init_bytes_storage.to_vec(), storage_pubkey, program_id).await;

    // Executes first ix with input data, this would come from client.
    let inputs_bytes: Vec<u8> = vec![
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
    println!("inputs bytes length: {:?}", inputs_bytes.len());
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
    let mut unpacked_data = vec![0u8; 4972];
    unpacked_data = storage_account.data.clone();

    // x_1_range: 252..316.
    // Keep in mind that g_ic_reference_value is based on running groth16.prepare_inputs() with 7 hardcoded inputs.
    let g_ic_projective = parse_x_group_affine_from_bytes_254(&unpacked_data[252..316].to_vec());
    let g_ic_reference_value =
        ark_ec::short_weierstrass_jacobian::GroupProjective::<ark_bn254::g1::Parameters>::new(
            Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
                471223499747275859,
                7569467397224972520,
                7773394081695017935,
                2286200356768260157,
            ])), // Cost: 31
            Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
                3547148427379123761,
                953159675554207994,
                12994789713999071316,
                608791868936298975,
            ])), // Cost: 31
            Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
                18420557557863729053,
                2004103336708983265,
                11245578246982574736,
                2207358309629838870,
            ])), // Cost: 31
        );
    assert_eq!(
        g_ic_projective, g_ic_reference_value,
        "different g_ic projective than libray implementation with the same inputs"
    );
}
