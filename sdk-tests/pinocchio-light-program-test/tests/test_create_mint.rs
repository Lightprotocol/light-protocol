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
    discriminators, mint::accounts::CreateMintParams, LightAccountVariant, MINT_SIGNER_SEED_A,
};
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

#[tokio::test]
async fn test_create_mint_derive() {
    let env = shared::setup_test_env().await;
    let mut rpc = env.rpc;
    let payer = env.payer;
    let program_id = env.program_id;

    let authority = Keypair::new();

    let (mint_signer_pda, mint_signer_bump) = Pubkey::find_program_address(
        &[MINT_SIGNER_SEED_A, authority.pubkey().as_ref()],
        &program_id,
    );

    let (mint_pda, _) = light_token::instruction::find_mint_address(&mint_signer_pda);

    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::mint(mint_signer_pda)],
    )
    .await
    .unwrap();

    let params = CreateMintParams {
        create_accounts_proof: proof_result.create_accounts_proof,
        mint_signer_bump,
    };

    // Account order per mint/accounts.rs:
    // [0] payer (signer, writable)
    // [1] authority (signer)
    // [2] mint_signer
    // [3] mint (writable)
    // [4] compressible_config (LIGHT_TOKEN_CONFIG)
    // [5] rent_sponsor (LIGHT_TOKEN_RENT_SPONSOR, writable)
    // [6] light_token_program
    // [7] cpi_authority
    // [8] system_program
    let accounts = vec![
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(authority.pubkey(), true),
        AccountMeta::new_readonly(mint_signer_pda, false),
        AccountMeta::new(mint_pda, false),
        AccountMeta::new_readonly(LIGHT_TOKEN_CONFIG, false),
        AccountMeta::new(LIGHT_TOKEN_RENT_SPONSOR, false),
        AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID.into(), false),
        AccountMeta::new_readonly(light_token_types::CPI_AUTHORITY_PDA.into(), false),
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
    ];

    let instruction = Instruction {
        program_id,
        accounts: [accounts, proof_result.remaining_accounts].concat(),
        data: shared::build_instruction_data(&discriminators::CREATE_MINT, &params),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer, &authority])
        .await
        .expect("CreateMint should succeed");

    // PHASE 1: Verify on-chain after creation
    let mint_account = rpc
        .get_account(mint_pda)
        .await
        .unwrap()
        .expect("Mint should exist on-chain");

    use light_token_interface::state::Mint;
    let mint: Mint = borsh::BorshDeserialize::deserialize(&mut &mint_account.data[..])
        .expect("Failed to deserialize Mint");

    assert_eq!(mint.base.decimals, 9, "Mint should have 9 decimals");
    assert_eq!(
        mint.base.mint_authority,
        Some(authority.pubkey().to_bytes().into()),
        "Mint authority should be authority"
    );

    // PHASE 2: Warp to trigger auto-compression
    rpc.warp_slot_forward(SLOTS_PER_EPOCH * 30).await.unwrap();
    shared::assert_onchain_closed(&mut rpc, &mint_pda, "Mint").await;

    // PHASE 3: Decompress via create_load_instructions
    let mint_interface = rpc
        .get_mint_interface(&mint_pda, None)
        .await
        .expect("failed to get mint interface")
        .value
        .expect("mint interface should exist");
    assert!(mint_interface.is_cold(), "Mint should be cold");
    let mint_account_interface = AccountInterface::from(mint_interface);

    let specs: Vec<AccountSpec<LightAccountVariant>> =
        vec![AccountSpec::Mint(mint_account_interface)];

    let ixs = create_load_instructions(&specs, payer.pubkey(), env.config_pda, &rpc)
        .await
        .expect("create_load_instructions should succeed");

    rpc.create_and_send_transaction(&ixs, &payer.pubkey(), &[&payer])
        .await
        .expect("Decompression should succeed");

    // PHASE 4: Assert state preserved after decompression
    shared::assert_onchain_exists(&mut rpc, &mint_pda, "Mint").await;

    let actual: Mint = borsh::BorshDeserialize::deserialize(
        &mut &rpc.get_account(mint_pda).await.unwrap().unwrap().data[..],
    )
    .unwrap();
    assert_eq!(actual.base.decimals, 9, "Mint decimals should be preserved");
    assert_eq!(
        actual.base.mint_authority,
        Some(authority.pubkey().to_bytes().into()),
        "Mint authority should be preserved"
    );
}
