mod shared;

use light_account::LightDiscriminator;
use light_client::interface::{
    create_load_instructions, get_create_accounts_proof, AccountSpec, CreateAccountsProofInput,
    PdaSpec,
};
use light_compressible::rent::SLOTS_PER_EPOCH;
use light_program_test::{program_test::TestRpc, Rpc};
use pinocchio_light_program_test::{
    discriminators, one_byte_pda::accounts::CreateOneByteRecordParams, LightAccountVariant,
    OneByteRecord, OneByteRecordSeeds,
};
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

#[tokio::test]
async fn test_create_compress_decompress_one_byte_record() {
    let env = shared::setup_test_env().await;
    let mut rpc = env.rpc;
    let payer = env.payer;
    let program_id = env.program_id;

    let owner = Keypair::new().pubkey();

    let (record_pda, _) =
        Pubkey::find_program_address(&[b"one_byte_record", owner.as_ref()], &program_id);

    // PHASE 1: Create
    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::pda(record_pda)],
    )
    .await
    .unwrap();

    let params = CreateOneByteRecordParams {
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
        data: shared::build_instruction_data(&discriminators::CREATE_ONE_BYTE_RECORD, &params),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .expect("CreateOneByteRecord should succeed");

    // Verify on-chain state after creation
    let record_account = rpc
        .get_account(record_pda)
        .await
        .unwrap()
        .expect("OneByteRecord PDA should exist on-chain");

    assert_eq!(
        &record_account.data[..OneByteRecord::LIGHT_DISCRIMINATOR_SLICE.len()],
        OneByteRecord::LIGHT_DISCRIMINATOR_SLICE,
        "First byte(s) should match OneByteRecord discriminator"
    );

    let record: OneByteRecord =
        borsh::BorshDeserialize::deserialize(&mut &record_account.data[1..])
            .expect("Failed to deserialize OneByteRecord");

    assert_eq!(
        record.owner,
        owner.to_bytes(),
        "Owner should match after creation"
    );

    // PHASE 2: Warp to trigger auto-compression
    rpc.warp_slot_forward(SLOTS_PER_EPOCH * 30).await.unwrap();
    shared::assert_onchain_closed(&mut rpc, &record_pda, "OneByteRecord").await;

    // PHASE 3: Decompress via create_load_instructions
    let account_interface = rpc
        .get_account_interface(&record_pda, None)
        .await
        .expect("failed to get OneByteRecord interface")
        .value
        .expect("OneByteRecord interface should exist");
    assert!(account_interface.is_cold(), "OneByteRecord should be cold");

    // The indexer returns: [8-byte LIGHT_DISCRIMINATOR] + [borsh(OneByteRecord)]
    let data: OneByteRecord =
        borsh::BorshDeserialize::deserialize(&mut &account_interface.account.data[8..])
            .expect("Failed to parse OneByteRecord from interface");
    assert_eq!(
        data.owner,
        owner.to_bytes(),
        "Owner should match in compressed state"
    );

    let variant = LightAccountVariant::OneByteRecord {
        seeds: OneByteRecordSeeds {
            owner: owner.to_bytes(),
        },
        data,
    };

    let spec = PdaSpec::new(account_interface, variant, program_id);
    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![AccountSpec::Pda(spec)];

    let ixs = create_load_instructions(&specs, payer.pubkey(), env.config_pda, &rpc)
        .await
        .expect("create_load_instructions should succeed");

    rpc.create_and_send_transaction(&ixs, &payer.pubkey(), &[&payer])
        .await
        .expect("Decompression should succeed");

    // PHASE 4: Verify state preserved after decompression
    shared::assert_onchain_exists(&mut rpc, &record_pda, "OneByteRecord").await;

    let account = rpc.get_account(record_pda).await.unwrap().unwrap();
    assert_eq!(
        &account.data[..OneByteRecord::LIGHT_DISCRIMINATOR_SLICE.len()],
        OneByteRecord::LIGHT_DISCRIMINATOR_SLICE,
        "First byte(s) should match OneByteRecord discriminator after decompression"
    );

    let decompressed: OneByteRecord = borsh::BorshDeserialize::deserialize(&mut &account.data[1..])
        .expect("Failed to deserialize decompressed OneByteRecord");
    assert_eq!(
        decompressed.owner,
        owner.to_bytes(),
        "Owner should match after decompression"
    );
}
