#![cfg(feature = "test-sbf")]

use light_program_test::{program_test::LightProgramTest, ProgramTestConfig, Rpc};
use pinocchio_nostd_test::test_helpers::get_program_id;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, signature::Signer};

#[tokio::test]
async fn test_nostd_basic() {
    let config = ProgramTestConfig::new_v2(
        false,
        Some(vec![(
            "pinocchio_nostd_test",
            Pubkey::new_from_array(get_program_id()),
        )]),
    );
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create a basic instruction
    let instruction_data = vec![0u8]; // InstructionType::TestBasic
    let ix = Instruction {
        program_id: Pubkey::new_from_array(get_program_id()),
        accounts: vec![],
        data: instruction_data,
    };

    // Execute the instruction
    let result = rpc
        .create_and_send_transaction(&[ix], &payer.pubkey(), &[&payer])
        .await;

    assert!(result.is_ok(), "Transaction should succeed");
}
