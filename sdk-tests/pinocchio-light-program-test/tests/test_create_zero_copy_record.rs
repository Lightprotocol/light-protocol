mod shared;

use light_client::interface::{
    create_load_instructions, get_create_accounts_proof, AccountInterfaceExt, AccountSpec,
    CreateAccountsProofInput, PdaSpec,
};
use light_compressible::rent::SLOTS_PER_EPOCH;
use light_program_test::{program_test::TestRpc, Rpc};
use pinocchio_light_program_test::{
    account_loader::accounts::CreateZeroCopyRecordParams, discriminators, LightAccountVariant,
    ZeroCopyRecord, ZeroCopyRecordSeeds, RECORD_SEED,
};
use solana_instruction::{AccountMeta, Instruction};
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

    let params = CreateZeroCopyRecordParams {
        create_accounts_proof: proof_result.create_accounts_proof,
        owner: owner.to_bytes(),
    };

    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(env.config_pda, false),
        AccountMeta::new(env.rent_sponsor, false),
        AccountMeta::new(record_pda, false),
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    let instruction = Instruction {
        program_id,
        accounts: [accounts, proof_result.remaining_accounts].concat(),
        data: shared::build_instruction_data(&discriminators::CREATE_ZERO_COPY_RECORD, &params),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .expect("CreateZeroCopyRecord should succeed");

    // PHASE 1: Verify on-chain after creation
    let record_account = rpc
        .get_account(record_pda)
        .await
        .unwrap()
        .expect("Record PDA should exist on-chain");

    let discriminator_len = 8;
    let data = &record_account.data[discriminator_len..];
    let record: &ZeroCopyRecord = bytemuck::from_bytes(data);

    assert_eq!(record.owner, owner.to_bytes(), "Record owner should match");
    assert_eq!(record.counter, 0, "Record counter should be 0");

    // PHASE 2: Warp to trigger auto-compression
    rpc.warp_slot_forward(SLOTS_PER_EPOCH * 30).await.unwrap();
    shared::assert_onchain_closed(&mut rpc, &record_pda, "ZeroCopyRecord").await;

    // PHASE 3: Decompress via create_load_instructions
    let account_interface = rpc
        .get_account_interface(&record_pda, &program_id)
        .await
        .expect("failed to get ZeroCopyRecord interface");
    assert!(account_interface.is_cold(), "ZeroCopyRecord should be cold");

    let zc_data: ZeroCopyRecord =
        borsh::BorshDeserialize::deserialize(&mut &account_interface.account.data[8..])
            .expect("Failed to parse ZeroCopyRecord from interface");
    let variant = LightAccountVariant::ZeroCopyRecord {
        seeds: ZeroCopyRecordSeeds {
            owner: owner.to_bytes(),
        },
        data: zc_data,
    };

    let spec = PdaSpec::new(account_interface, variant, program_id);
    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![AccountSpec::Pda(spec)];

    let ixs = create_load_instructions(&specs, payer.pubkey(), env.config_pda, &rpc)
        .await
        .expect("create_load_instructions should succeed");

    rpc.create_and_send_transaction(&ixs, &payer.pubkey(), &[&payer])
        .await
        .expect("Decompression should succeed");

    // PHASE 4: Assert state preserved after decompression
    shared::assert_onchain_exists(&mut rpc, &record_pda, "ZeroCopyRecord").await;

    let account = rpc.get_account(record_pda).await.unwrap().unwrap();
    let actual: &ZeroCopyRecord = bytemuck::from_bytes(&account.data[8..]);
    let expected = ZeroCopyRecord {
        compression_info: shared::expected_compression_info(&actual.compression_info),
        owner: owner.to_bytes(),
        counter: 0,
    };
    assert_eq!(
        *actual, expected,
        "ZeroCopyRecord should match after decompression"
    );
}
