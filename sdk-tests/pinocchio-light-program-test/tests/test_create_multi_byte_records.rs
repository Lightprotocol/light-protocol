mod shared;

use light_account::LightDiscriminator;
use light_client::interface::{
    create_load_instructions, get_create_accounts_proof, AccountSpec, CreateAccountsProofInput,
    PdaSpec,
};
use light_compressible::rent::SLOTS_PER_EPOCH;
use light_program_test::{program_test::TestRpc, Rpc};
use pinocchio_light_program_test::{
    discriminators, multi_byte_pda::accounts::CreateMultiByteRecordsParams, FiveByteRecord,
    FiveByteRecordSeeds, FourByteRecord, FourByteRecordSeeds, LightAccountVariant, SevenByteRecord,
    SevenByteRecordSeeds, SixByteRecord, SixByteRecordSeeds, ThreeByteRecord, ThreeByteRecordSeeds,
    TwoByteRecord, TwoByteRecordSeeds,
};
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

#[tokio::test]
async fn test_create_compress_decompress_multi_byte_records() {
    let env = shared::setup_test_env().await;
    let mut rpc = env.rpc;
    let payer = env.payer;
    let program_id = env.program_id;

    let owner = Keypair::new().pubkey();

    let (two_byte_pda, _) =
        Pubkey::find_program_address(&[b"two_byte_record", owner.as_ref()], &program_id);
    let (three_byte_pda, _) =
        Pubkey::find_program_address(&[b"three_byte_record", owner.as_ref()], &program_id);
    let (four_byte_pda, _) =
        Pubkey::find_program_address(&[b"four_byte_record", owner.as_ref()], &program_id);
    let (five_byte_pda, _) =
        Pubkey::find_program_address(&[b"five_byte_record", owner.as_ref()], &program_id);
    let (six_byte_pda, _) =
        Pubkey::find_program_address(&[b"six_byte_record", owner.as_ref()], &program_id);
    let (seven_byte_pda, _) =
        Pubkey::find_program_address(&[b"seven_byte_record", owner.as_ref()], &program_id);

    // PHASE 1: Create all 6 accounts
    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![
            CreateAccountsProofInput::pda(two_byte_pda),
            CreateAccountsProofInput::pda(three_byte_pda),
            CreateAccountsProofInput::pda(four_byte_pda),
            CreateAccountsProofInput::pda(five_byte_pda),
            CreateAccountsProofInput::pda(six_byte_pda),
            CreateAccountsProofInput::pda(seven_byte_pda),
        ],
    )
    .await
    .unwrap();

    let params = CreateMultiByteRecordsParams {
        create_accounts_proof: proof_result.create_accounts_proof,
        owner: owner.to_bytes(),
    };

    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(env.config_pda, false),
        AccountMeta::new(env.rent_sponsor, false),
        AccountMeta::new(two_byte_pda, false),
        AccountMeta::new(three_byte_pda, false),
        AccountMeta::new(four_byte_pda, false),
        AccountMeta::new(five_byte_pda, false),
        AccountMeta::new(six_byte_pda, false),
        AccountMeta::new(seven_byte_pda, false),
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    let instruction = Instruction {
        program_id,
        accounts: [accounts, proof_result.remaining_accounts].concat(),
        data: shared::build_instruction_data(&discriminators::CREATE_MULTI_BYTE_RECORDS, &params),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .expect("CreateMultiByteRecords should succeed");

    // Verify all 6 PDAs on-chain after creation
    let two_byte_account = rpc
        .get_account(two_byte_pda)
        .await
        .unwrap()
        .expect("TwoByteRecord PDA should exist");
    let disc_len = TwoByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
    assert_eq!(
        &two_byte_account.data[..disc_len],
        TwoByteRecord::LIGHT_DISCRIMINATOR_SLICE,
        "TwoByteRecord discriminator should match"
    );
    let actual_two: TwoByteRecord =
        borsh::BorshDeserialize::deserialize(&mut &two_byte_account.data[disc_len..])
            .expect("Failed to deserialize TwoByteRecord");
    assert_eq!(
        actual_two,
        TwoByteRecord {
            compression_info: shared::expected_compression_info(&actual_two.compression_info),
            owner: owner.to_bytes(),
        },
        "TwoByteRecord should match after creation"
    );

    let three_byte_account = rpc
        .get_account(three_byte_pda)
        .await
        .unwrap()
        .expect("ThreeByteRecord PDA should exist");
    let disc_len = ThreeByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
    assert_eq!(
        &three_byte_account.data[..disc_len],
        ThreeByteRecord::LIGHT_DISCRIMINATOR_SLICE,
        "ThreeByteRecord discriminator should match"
    );
    let actual_three: ThreeByteRecord =
        borsh::BorshDeserialize::deserialize(&mut &three_byte_account.data[disc_len..])
            .expect("Failed to deserialize ThreeByteRecord");
    assert_eq!(
        actual_three,
        ThreeByteRecord {
            compression_info: shared::expected_compression_info(&actual_three.compression_info),
            owner: owner.to_bytes(),
        },
        "ThreeByteRecord should match after creation"
    );

    let four_byte_account = rpc
        .get_account(four_byte_pda)
        .await
        .unwrap()
        .expect("FourByteRecord PDA should exist");
    let disc_len = FourByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
    assert_eq!(
        &four_byte_account.data[..disc_len],
        FourByteRecord::LIGHT_DISCRIMINATOR_SLICE,
        "FourByteRecord discriminator should match"
    );
    let actual_four: FourByteRecord =
        borsh::BorshDeserialize::deserialize(&mut &four_byte_account.data[disc_len..])
            .expect("Failed to deserialize FourByteRecord");
    assert_eq!(
        actual_four,
        FourByteRecord {
            compression_info: shared::expected_compression_info(&actual_four.compression_info),
            owner: owner.to_bytes(),
        },
        "FourByteRecord should match after creation"
    );

    let five_byte_account = rpc
        .get_account(five_byte_pda)
        .await
        .unwrap()
        .expect("FiveByteRecord PDA should exist");
    let disc_len = FiveByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
    assert_eq!(
        &five_byte_account.data[..disc_len],
        FiveByteRecord::LIGHT_DISCRIMINATOR_SLICE,
        "FiveByteRecord discriminator should match"
    );
    let actual_five: FiveByteRecord =
        borsh::BorshDeserialize::deserialize(&mut &five_byte_account.data[disc_len..])
            .expect("Failed to deserialize FiveByteRecord");
    assert_eq!(
        actual_five,
        FiveByteRecord {
            compression_info: shared::expected_compression_info(&actual_five.compression_info),
            owner: owner.to_bytes(),
        },
        "FiveByteRecord should match after creation"
    );

    let six_byte_account = rpc
        .get_account(six_byte_pda)
        .await
        .unwrap()
        .expect("SixByteRecord PDA should exist");
    let disc_len = SixByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
    assert_eq!(
        &six_byte_account.data[..disc_len],
        SixByteRecord::LIGHT_DISCRIMINATOR_SLICE,
        "SixByteRecord discriminator should match"
    );
    let actual_six: SixByteRecord =
        borsh::BorshDeserialize::deserialize(&mut &six_byte_account.data[disc_len..])
            .expect("Failed to deserialize SixByteRecord");
    assert_eq!(
        actual_six,
        SixByteRecord {
            compression_info: shared::expected_compression_info(&actual_six.compression_info),
            owner: owner.to_bytes(),
        },
        "SixByteRecord should match after creation"
    );

    let seven_byte_account = rpc
        .get_account(seven_byte_pda)
        .await
        .unwrap()
        .expect("SevenByteRecord PDA should exist");
    let disc_len = SevenByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
    assert_eq!(
        &seven_byte_account.data[..disc_len],
        SevenByteRecord::LIGHT_DISCRIMINATOR_SLICE,
        "SevenByteRecord discriminator should match"
    );
    let actual_seven: SevenByteRecord =
        borsh::BorshDeserialize::deserialize(&mut &seven_byte_account.data[disc_len..])
            .expect("Failed to deserialize SevenByteRecord");
    assert_eq!(
        actual_seven,
        SevenByteRecord {
            compression_info: shared::expected_compression_info(&actual_seven.compression_info),
            owner: owner.to_bytes(),
        },
        "SevenByteRecord should match after creation"
    );

    // PHASE 2: Warp to trigger auto-compression
    rpc.warp_slot_forward(SLOTS_PER_EPOCH * 30).await.unwrap();
    shared::assert_onchain_closed(&mut rpc, &two_byte_pda, "TwoByteRecord").await;
    shared::assert_onchain_closed(&mut rpc, &three_byte_pda, "ThreeByteRecord").await;
    shared::assert_onchain_closed(&mut rpc, &four_byte_pda, "FourByteRecord").await;
    shared::assert_onchain_closed(&mut rpc, &five_byte_pda, "FiveByteRecord").await;
    shared::assert_onchain_closed(&mut rpc, &six_byte_pda, "SixByteRecord").await;
    shared::assert_onchain_closed(&mut rpc, &seven_byte_pda, "SevenByteRecord").await;

    // PHASE 3: Decompress via create_load_instructions
    let two_byte_iface = rpc
        .get_account_interface(&two_byte_pda, None)
        .await
        .expect("failed to get TwoByteRecord interface")
        .value
        .expect("TwoByteRecord interface should exist");
    assert!(two_byte_iface.is_cold(), "TwoByteRecord should be cold");
    let two_byte_data: TwoByteRecord =
        borsh::BorshDeserialize::deserialize(&mut &two_byte_iface.account.data[8..])
            .expect("Failed to parse TwoByteRecord from interface");
    let two_byte_variant = LightAccountVariant::TwoByteRecord {
        seeds: TwoByteRecordSeeds {
            owner: owner.to_bytes(),
        },
        data: two_byte_data,
    };
    let two_byte_spec = PdaSpec::new(two_byte_iface, two_byte_variant, program_id);

    let three_byte_iface = rpc
        .get_account_interface(&three_byte_pda, None)
        .await
        .expect("failed to get ThreeByteRecord interface")
        .value
        .expect("ThreeByteRecord interface should exist");
    assert!(three_byte_iface.is_cold(), "ThreeByteRecord should be cold");
    let three_byte_data: ThreeByteRecord =
        borsh::BorshDeserialize::deserialize(&mut &three_byte_iface.account.data[8..])
            .expect("Failed to parse ThreeByteRecord from interface");
    let three_byte_variant = LightAccountVariant::ThreeByteRecord {
        seeds: ThreeByteRecordSeeds {
            owner: owner.to_bytes(),
        },
        data: three_byte_data,
    };
    let three_byte_spec = PdaSpec::new(three_byte_iface, three_byte_variant, program_id);

    let four_byte_iface = rpc
        .get_account_interface(&four_byte_pda, None)
        .await
        .expect("failed to get FourByteRecord interface")
        .value
        .expect("FourByteRecord interface should exist");
    assert!(four_byte_iface.is_cold(), "FourByteRecord should be cold");
    let four_byte_data: FourByteRecord =
        borsh::BorshDeserialize::deserialize(&mut &four_byte_iface.account.data[8..])
            .expect("Failed to parse FourByteRecord from interface");
    let four_byte_variant = LightAccountVariant::FourByteRecord {
        seeds: FourByteRecordSeeds {
            owner: owner.to_bytes(),
        },
        data: four_byte_data,
    };
    let four_byte_spec = PdaSpec::new(four_byte_iface, four_byte_variant, program_id);

    let five_byte_iface = rpc
        .get_account_interface(&five_byte_pda, None)
        .await
        .expect("failed to get FiveByteRecord interface")
        .value
        .expect("FiveByteRecord interface should exist");
    assert!(five_byte_iface.is_cold(), "FiveByteRecord should be cold");
    let five_byte_data: FiveByteRecord =
        borsh::BorshDeserialize::deserialize(&mut &five_byte_iface.account.data[8..])
            .expect("Failed to parse FiveByteRecord from interface");
    let five_byte_variant = LightAccountVariant::FiveByteRecord {
        seeds: FiveByteRecordSeeds {
            owner: owner.to_bytes(),
        },
        data: five_byte_data,
    };
    let five_byte_spec = PdaSpec::new(five_byte_iface, five_byte_variant, program_id);

    let six_byte_iface = rpc
        .get_account_interface(&six_byte_pda, None)
        .await
        .expect("failed to get SixByteRecord interface")
        .value
        .expect("SixByteRecord interface should exist");
    assert!(six_byte_iface.is_cold(), "SixByteRecord should be cold");
    let six_byte_data: SixByteRecord =
        borsh::BorshDeserialize::deserialize(&mut &six_byte_iface.account.data[8..])
            .expect("Failed to parse SixByteRecord from interface");
    let six_byte_variant = LightAccountVariant::SixByteRecord {
        seeds: SixByteRecordSeeds {
            owner: owner.to_bytes(),
        },
        data: six_byte_data,
    };
    let six_byte_spec = PdaSpec::new(six_byte_iface, six_byte_variant, program_id);

    let seven_byte_iface = rpc
        .get_account_interface(&seven_byte_pda, None)
        .await
        .expect("failed to get SevenByteRecord interface")
        .value
        .expect("SevenByteRecord interface should exist");
    assert!(seven_byte_iface.is_cold(), "SevenByteRecord should be cold");
    let seven_byte_data: SevenByteRecord =
        borsh::BorshDeserialize::deserialize(&mut &seven_byte_iface.account.data[8..])
            .expect("Failed to parse SevenByteRecord from interface");
    let seven_byte_variant = LightAccountVariant::SevenByteRecord {
        seeds: SevenByteRecordSeeds {
            owner: owner.to_bytes(),
        },
        data: seven_byte_data,
    };
    let seven_byte_spec = PdaSpec::new(seven_byte_iface, seven_byte_variant, program_id);

    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![
        AccountSpec::Pda(two_byte_spec),
        AccountSpec::Pda(three_byte_spec),
        AccountSpec::Pda(four_byte_spec),
        AccountSpec::Pda(five_byte_spec),
        AccountSpec::Pda(six_byte_spec),
        AccountSpec::Pda(seven_byte_spec),
    ];

    let ixs = create_load_instructions(&specs, payer.pubkey(), env.config_pda, &rpc)
        .await
        .expect("create_load_instructions should succeed");

    rpc.create_and_send_transaction(&ixs, &payer.pubkey(), &[&payer])
        .await
        .expect("Decompression should succeed");

    // PHASE 4: Verify state preserved after decompression
    shared::assert_onchain_exists(&mut rpc, &two_byte_pda, "TwoByteRecord").await;
    shared::assert_onchain_exists(&mut rpc, &three_byte_pda, "ThreeByteRecord").await;
    shared::assert_onchain_exists(&mut rpc, &four_byte_pda, "FourByteRecord").await;
    shared::assert_onchain_exists(&mut rpc, &five_byte_pda, "FiveByteRecord").await;
    shared::assert_onchain_exists(&mut rpc, &six_byte_pda, "SixByteRecord").await;
    shared::assert_onchain_exists(&mut rpc, &seven_byte_pda, "SevenByteRecord").await;

    let account = rpc.get_account(two_byte_pda).await.unwrap().unwrap();
    let disc_len = TwoByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
    assert_eq!(
        &account.data[..disc_len],
        TwoByteRecord::LIGHT_DISCRIMINATOR_SLICE,
        "TwoByteRecord discriminator should match after decompression"
    );
    let actual: TwoByteRecord =
        borsh::BorshDeserialize::deserialize(&mut &account.data[disc_len..]).unwrap();
    assert_eq!(
        actual,
        TwoByteRecord {
            compression_info: shared::expected_compression_info(&actual.compression_info),
            owner: owner.to_bytes(),
        },
        "TwoByteRecord should match after decompression"
    );

    let account = rpc.get_account(three_byte_pda).await.unwrap().unwrap();
    let disc_len = ThreeByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
    assert_eq!(
        &account.data[..disc_len],
        ThreeByteRecord::LIGHT_DISCRIMINATOR_SLICE,
        "ThreeByteRecord discriminator should match after decompression"
    );
    let actual: ThreeByteRecord =
        borsh::BorshDeserialize::deserialize(&mut &account.data[disc_len..]).unwrap();
    assert_eq!(
        actual,
        ThreeByteRecord {
            compression_info: shared::expected_compression_info(&actual.compression_info),
            owner: owner.to_bytes(),
        },
        "ThreeByteRecord should match after decompression"
    );

    let account = rpc.get_account(four_byte_pda).await.unwrap().unwrap();
    let disc_len = FourByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
    assert_eq!(
        &account.data[..disc_len],
        FourByteRecord::LIGHT_DISCRIMINATOR_SLICE,
        "FourByteRecord discriminator should match after decompression"
    );
    let actual: FourByteRecord =
        borsh::BorshDeserialize::deserialize(&mut &account.data[disc_len..]).unwrap();
    assert_eq!(
        actual,
        FourByteRecord {
            compression_info: shared::expected_compression_info(&actual.compression_info),
            owner: owner.to_bytes(),
        },
        "FourByteRecord should match after decompression"
    );

    let account = rpc.get_account(five_byte_pda).await.unwrap().unwrap();
    let disc_len = FiveByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
    assert_eq!(
        &account.data[..disc_len],
        FiveByteRecord::LIGHT_DISCRIMINATOR_SLICE,
        "FiveByteRecord discriminator should match after decompression"
    );
    let actual: FiveByteRecord =
        borsh::BorshDeserialize::deserialize(&mut &account.data[disc_len..]).unwrap();
    assert_eq!(
        actual,
        FiveByteRecord {
            compression_info: shared::expected_compression_info(&actual.compression_info),
            owner: owner.to_bytes(),
        },
        "FiveByteRecord should match after decompression"
    );

    let account = rpc.get_account(six_byte_pda).await.unwrap().unwrap();
    let disc_len = SixByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
    assert_eq!(
        &account.data[..disc_len],
        SixByteRecord::LIGHT_DISCRIMINATOR_SLICE,
        "SixByteRecord discriminator should match after decompression"
    );
    let actual: SixByteRecord =
        borsh::BorshDeserialize::deserialize(&mut &account.data[disc_len..]).unwrap();
    assert_eq!(
        actual,
        SixByteRecord {
            compression_info: shared::expected_compression_info(&actual.compression_info),
            owner: owner.to_bytes(),
        },
        "SixByteRecord should match after decompression"
    );

    let account = rpc.get_account(seven_byte_pda).await.unwrap().unwrap();
    let disc_len = SevenByteRecord::LIGHT_DISCRIMINATOR_SLICE.len();
    assert_eq!(
        &account.data[..disc_len],
        SevenByteRecord::LIGHT_DISCRIMINATOR_SLICE,
        "SevenByteRecord discriminator should match after decompression"
    );
    let actual: SevenByteRecord =
        borsh::BorshDeserialize::deserialize(&mut &account.data[disc_len..]).unwrap();
    assert_eq!(
        actual,
        SevenByteRecord {
            compression_info: shared::expected_compression_info(&actual.compression_info),
            owner: owner.to_bytes(),
        },
        "SevenByteRecord should match after decompression"
    );
}
