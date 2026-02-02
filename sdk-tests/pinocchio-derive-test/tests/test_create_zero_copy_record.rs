mod shared;

use anchor_lang::{InstructionData, ToAccountMetas};
use light_client::interface::{get_create_accounts_proof, CreateAccountsProofInput};
use light_program_test::Rpc;
use pinocchio_derive_test::{CreateZeroCopyRecordParams, RECORD_SEED};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

#[tokio::test]
async fn test_create_zero_copy_record_derive() {
    let env = shared::setup_test_env().await;
    let mut rpc = env.rpc;
    let payer = env.payer;
    let program_id = env.program_id;

    let owner = Keypair::new().pubkey();

    let (record_pda, _) = Pubkey::find_program_address(&[RECORD_SEED, owner.as_ref()], &program_id);

    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::pda(record_pda)],
    )
    .await
    .unwrap();

    let accounts = pinocchio_derive_test::accounts::CreateZeroCopyRecord {
        fee_payer: payer.pubkey(),
        compression_config: env.config_pda,
        pda_rent_sponsor: env.rent_sponsor,
        record: record_pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = pinocchio_derive_test::instruction::CreateZeroCopyRecord {
        params: CreateZeroCopyRecordParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
        },
    };

    let instruction = Instruction {
        program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .expect("CreateZeroCopyRecord should succeed");

    let record_account = rpc
        .get_account(record_pda)
        .await
        .unwrap()
        .expect("Record PDA should exist on-chain");

    use pinocchio_derive_test::ZeroCopyRecord;
    let discriminator_len = 8;
    let data = &record_account.data[discriminator_len..];
    let record: &ZeroCopyRecord = bytemuck::from_bytes(data);

    assert_eq!(record.owner, owner, "Record owner should match");
    assert_eq!(record.counter, 0, "Record counter should be 0");
}
