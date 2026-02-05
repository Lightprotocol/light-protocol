mod shared;

use light_client::interface::{create_load_instructions, get_create_accounts_proof, AccountSpec};
use light_compressible::rent::SLOTS_PER_EPOCH;
use light_program_test::{program_test::TestRpc, Rpc};
use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use light_token::instruction::{LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR};
use pinocchio_light_program_test::{
    ata::accounts::CreateAtaParams, discriminators, LightAccountVariant,
};
use solana_instruction::{AccountMeta, Instruction};
use solana_signer::Signer;

#[tokio::test]
async fn test_create_ata_derive() {
    let env = shared::setup_test_env().await;
    let mut rpc = env.rpc;
    let payer = env.payer;
    let program_id = env.program_id;

    let (mint, _mint_seed) = shared::setup_create_mint(&mut rpc, &payer, payer.pubkey(), 9).await;

    let ata_owner = payer.pubkey();
    let (ata, _ata_bump) = light_token::instruction::derive_token_ata(&ata_owner, &mint);

    let proof_result = get_create_accounts_proof(&rpc, &program_id, vec![])
        .await
        .unwrap();

    let params = CreateAtaParams {};

    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new_readonly(ata_owner, false),
        AccountMeta::new(ata, false),
        AccountMeta::new_readonly(LIGHT_TOKEN_CONFIG, false),
        AccountMeta::new(LIGHT_TOKEN_RENT_SPONSOR, false),
        AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID.into(), false),
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    let instruction = Instruction {
        program_id,
        accounts: [accounts, proof_result.remaining_accounts].concat(),
        data: shared::build_instruction_data(&discriminators::CREATE_ATA, &params),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
        .expect("CreateAta should succeed");

    // PHASE 1: Verify on-chain after creation
    let ata_account = rpc
        .get_account(ata)
        .await
        .unwrap()
        .expect("ATA should exist on-chain");

    use light_token_interface::state::token::{AccountState, Token, ACCOUNT_TYPE_TOKEN_ACCOUNT};
    let token: Token = borsh::BorshDeserialize::deserialize(&mut &ata_account.data[..])
        .expect("Failed to deserialize Token");

    let expected_token = Token {
        mint: mint.to_bytes().into(),
        owner: ata_owner.to_bytes().into(),
        amount: 0,
        delegate: None,
        state: AccountState::Initialized,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
        extensions: token.extensions.clone(),
    };

    assert_eq!(
        token, expected_token,
        "ATA should match expected after creation"
    );

    // PHASE 2: Warp to trigger auto-compression
    rpc.warp_slot_forward(SLOTS_PER_EPOCH * 30).await.unwrap();
    shared::assert_onchain_closed(&mut rpc, &ata, "ATA").await;

    // PHASE 3: Decompress via create_load_instructions
    let ata_interface = rpc
        .get_associated_token_account_interface(&ata_owner, &mint, None)
        .await
        .expect("failed to get ATA interface")
        .value
        .expect("ATA interface should exist");
    assert!(ata_interface.is_cold(), "ATA should be cold");

    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![AccountSpec::Ata(ata_interface)];

    let ixs = create_load_instructions(&specs, payer.pubkey(), env.config_pda, &rpc)
        .await
        .expect("create_load_instructions should succeed");

    rpc.create_and_send_transaction(&ixs, &payer.pubkey(), &[&payer])
        .await
        .expect("Decompression should succeed");

    // PHASE 4: Assert state preserved after decompression
    shared::assert_onchain_exists(&mut rpc, &ata, "ATA").await;

    let actual: Token = shared::parse_token(&rpc.get_account(ata).await.unwrap().unwrap().data);
    let expected = Token {
        mint: mint.to_bytes().into(),
        owner: ata_owner.to_bytes().into(),
        amount: 0,
        delegate: None,
        state: AccountState::Initialized,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
        extensions: actual.extensions.clone(),
    };
    assert_eq!(actual, expected, "ATA should match after decompression");
}
