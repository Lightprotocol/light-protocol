mod shared;

use light_client::interface::{
    create_load_instructions, get_create_accounts_proof, AccountInterface, AccountSpec,
    CreateAccountsProofInput,
};
use light_compressible::rent::SLOTS_PER_EPOCH;
use light_program_test::{program_test::TestRpc, Rpc};
use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use light_token::instruction::{LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR};
use pinocchio_light_program_test::{
    discriminators, two_mints::accounts::CreateTwoMintsParams, LightAccountVariant,
    MINT_SIGNER_SEED_A, MINT_SIGNER_SEED_B,
};
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

#[tokio::test]
async fn test_create_two_mints_derive() {
    let env = shared::setup_test_env().await;
    let mut rpc = env.rpc;
    let payer = env.payer;
    let program_id = env.program_id;

    let authority = Keypair::new();

    let (mint_signer_a, mint_signer_bump_a) = Pubkey::find_program_address(
        &[MINT_SIGNER_SEED_A, authority.pubkey().as_ref()],
        &program_id,
    );
    let (mint_a_pda, _) = light_token::instruction::find_mint_address(&mint_signer_a);

    let (mint_signer_b, mint_signer_bump_b) = Pubkey::find_program_address(
        &[MINT_SIGNER_SEED_B, authority.pubkey().as_ref()],
        &program_id,
    );
    let (mint_b_pda, _) = light_token::instruction::find_mint_address(&mint_signer_b);

    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![
            CreateAccountsProofInput::mint(mint_signer_a),
            CreateAccountsProofInput::mint(mint_signer_b),
        ],
    )
    .await
    .unwrap();

    let params = CreateTwoMintsParams {
        create_accounts_proof: proof_result.create_accounts_proof,
        mint_signer_bump_a,
        mint_signer_bump_b,
    };

    // Account order per two_mints/accounts.rs:
    // [0] payer (signer, writable)
    // [1] authority (signer)
    // [2] mint_signer_a
    // [3] mint_signer_b
    // [4] mint_a (writable)
    // [5] mint_b (writable)
    // [6] compressible_config (LIGHT_TOKEN_CONFIG)
    // [7] rent_sponsor (LIGHT_TOKEN_RENT_SPONSOR, writable)
    // [8] light_token_program
    // [9] cpi_authority
    // [10] system_program
    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(authority.pubkey(), true),
        AccountMeta::new_readonly(mint_signer_a, false),
        AccountMeta::new_readonly(mint_signer_b, false),
        AccountMeta::new(mint_a_pda, false),
        AccountMeta::new(mint_b_pda, false),
        AccountMeta::new_readonly(LIGHT_TOKEN_CONFIG, false),
        AccountMeta::new(LIGHT_TOKEN_RENT_SPONSOR, false),
        AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID.into(), false),
        AccountMeta::new_readonly(light_token_types::CPI_AUTHORITY_PDA.into(), false),
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    let instruction = Instruction {
        program_id,
        accounts: [accounts, proof_result.remaining_accounts].concat(),
        data: shared::build_instruction_data(&discriminators::CREATE_TWO_MINTS, &params),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &authority])
        .await
        .expect("CreateTwoMints should succeed");

    // PHASE 1: Verify on-chain after creation
    use light_token_interface::state::Mint;

    let mint_a_account = rpc
        .get_account(mint_a_pda)
        .await
        .unwrap()
        .expect("Mint A should exist on-chain");
    let mint_a: Mint = borsh::BorshDeserialize::deserialize(&mut &mint_a_account.data[..])
        .expect("Failed to deserialize Mint A");
    assert_eq!(mint_a.base.decimals, 9, "Mint A should have 9 decimals");
    assert_eq!(
        mint_a.base.mint_authority,
        Some(authority.pubkey().to_bytes().into()),
        "Mint A authority should be authority"
    );

    let mint_b_account = rpc
        .get_account(mint_b_pda)
        .await
        .unwrap()
        .expect("Mint B should exist on-chain");
    let mint_b: Mint = borsh::BorshDeserialize::deserialize(&mut &mint_b_account.data[..])
        .expect("Failed to deserialize Mint B");
    assert_eq!(mint_b.base.decimals, 6, "Mint B should have 6 decimals");
    assert_eq!(
        mint_b.base.mint_authority,
        Some(authority.pubkey().to_bytes().into()),
        "Mint B authority should be authority"
    );

    // PHASE 2: Warp to trigger auto-compression
    rpc.warp_slot_forward(SLOTS_PER_EPOCH * 30).await.unwrap();
    shared::assert_onchain_closed(&mut rpc, &mint_a_pda, "MintA").await;
    shared::assert_onchain_closed(&mut rpc, &mint_b_pda, "MintB").await;

    // PHASE 3: Decompress both mints via create_load_instructions
    let mint_a_interface = rpc
        .get_mint_interface(&mint_a_pda, None)
        .await
        .expect("failed to get mint A interface")
        .value
        .expect("mint A interface should exist");
    assert!(mint_a_interface.is_cold(), "Mint A should be cold");
    let mint_a_ai = AccountInterface::from(mint_a_interface);

    let mint_b_interface = rpc
        .get_mint_interface(&mint_b_pda, None)
        .await
        .expect("failed to get mint B interface")
        .value
        .expect("mint B interface should exist");
    assert!(mint_b_interface.is_cold(), "Mint B should be cold");
    let mint_b_ai = AccountInterface::from(mint_b_interface);

    let specs: Vec<AccountSpec<LightAccountVariant>> =
        vec![AccountSpec::Mint(mint_a_ai), AccountSpec::Mint(mint_b_ai)];

    let ixs = create_load_instructions(&specs, payer.pubkey(), env.config_pda, &rpc)
        .await
        .expect("create_load_instructions should succeed");

    rpc.create_and_send_transaction(&ixs, &payer.pubkey(), &[&payer])
        .await
        .expect("Decompression should succeed");

    // PHASE 4: Assert state preserved after decompression
    shared::assert_onchain_exists(&mut rpc, &mint_a_pda, "MintA").await;
    shared::assert_onchain_exists(&mut rpc, &mint_b_pda, "MintB").await;

    let actual_a: Mint = borsh::BorshDeserialize::deserialize(
        &mut &rpc.get_account(mint_a_pda).await.unwrap().unwrap().data[..],
    )
    .unwrap();
    assert_eq!(
        actual_a.base.decimals, 9,
        "Mint A decimals should be preserved"
    );
    assert_eq!(
        actual_a.base.mint_authority,
        Some(authority.pubkey().to_bytes().into()),
        "Mint A authority should be preserved"
    );

    let actual_b: Mint = borsh::BorshDeserialize::deserialize(
        &mut &rpc.get_account(mint_b_pda).await.unwrap().unwrap().data[..],
    )
    .unwrap();
    assert_eq!(
        actual_b.base.decimals, 6,
        "Mint B decimals should be preserved"
    );
    assert_eq!(
        actual_b.base.mint_authority,
        Some(authority.pubkey().to_bytes().into()),
        "Mint B authority should be preserved"
    );
}
