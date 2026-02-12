mod shared;

use anchor_lang::{InstructionData, ToAccountMetas};
use anchor_semi_manual_test::{CreateMintParams, MINT_SIGNER_SEED_A};
use light_client::interface::{
    create_load_instructions, get_create_accounts_proof, AccountSpec, CreateAccountsProofInput,
};
use light_compressible::rent::SLOTS_PER_EPOCH;
use light_program_test::{program_test::TestRpc, Rpc};
use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use light_token::instruction::{LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR};
use solana_instruction::Instruction;
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

    let accounts = anchor_semi_manual_test::accounts::CreateMint {
        fee_payer: payer.pubkey(),
        authority: authority.pubkey(),
        mint_signer: mint_signer_pda,
        mint: mint_pda,
        compression_config: env.config_pda,
        light_token_config: LIGHT_TOKEN_CONFIG,
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        light_token_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = anchor_semi_manual_test::instruction::CreateMint {
        params: CreateMintParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            mint_signer_bump,
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
        Some(payer.pubkey().to_bytes().into()),
        "Mint authority should be fee_payer"
    );

    // PHASE 2: Warp to trigger auto-compression
    rpc.warp_slot_forward(SLOTS_PER_EPOCH * 30).await.unwrap();
    shared::assert_onchain_closed(&mut rpc, &mint_pda, "Mint").await;

    // PHASE 3: Decompress via create_load_instructions
    use anchor_semi_manual_test::LightAccountVariant;

    let mint_interface = rpc
        .get_account_interface(&mint_pda, None)
        .await
        .expect("failed to get mint interface")
        .value
        .expect("mint interface should exist");
    assert!(mint_interface.is_cold(), "Mint should be cold");

    let specs: Vec<AccountSpec<LightAccountVariant>> = vec![AccountSpec::Mint(mint_interface)];

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
        Some(payer.pubkey().to_bytes().into()),
        "Mint authority should be preserved"
    );
}
