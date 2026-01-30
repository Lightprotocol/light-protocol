//! Integration test for single ATA macro validation.

use anchor_lang::{InstructionData, ToAccountMetas};
use light_client::interface::{get_create_accounts_proof, InitializeRentFreeConfig};
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest},
    Indexer, ProgramTestConfig, Rpc,
};
use light_sdk::utils::derive_rent_sponsor_pda;
use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use light_token::instruction::{LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Setup helper: Creates a compressed mint directly using the ctoken SDK.
/// Returns (mint_pda, mint_seed_keypair)
async fn setup_create_mint(
    rpc: &mut (impl Rpc + Indexer),
    payer: &Keypair,
    mint_authority: Pubkey,
    decimals: u8,
) -> (Pubkey, Keypair) {
    use light_token::instruction::{CreateMint, CreateMintParams};

    let mint_seed = Keypair::new();
    let address_tree = rpc.get_address_tree_v2();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    let compression_address = light_token::instruction::derive_mint_compressed_address(
        &mint_seed.pubkey(),
        &address_tree.tree,
    );

    let (mint, bump) = light_token::instruction::find_mint_address(&mint_seed.pubkey());

    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![light_client::indexer::AddressWithTree {
                address: compression_address,
                tree: address_tree.tree,
            }],
            None,
        )
        .await
        .unwrap()
        .value;

    let params = CreateMintParams {
        decimals,
        address_merkle_tree_root_index: rpc_result.addresses[0].root_index,
        mint_authority,
        proof: rpc_result.proof.0.unwrap(),
        compression_address,
        mint,
        bump,
        freeze_authority: None,
        extensions: None,
        rent_payment: 16,
        write_top_up: 766,
    };

    let create_mint_builder = CreateMint::new(
        params,
        mint_seed.pubkey(),
        payer.pubkey(),
        address_tree.tree,
        output_queue,
    );
    let instruction = create_mint_builder.instruction().unwrap();

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer, &mint_seed])
        .await
        .unwrap();

    (mint, mint_seed)
}

/// Test creating a single ATA using the macro.
/// Validates that #[light_account(init, associated_token, ...)] works in isolation.
#[tokio::test]
async fn test_create_single_ata() {
    use single_ata_test::CreateAtaParams;

    let program_id = single_ata_test::ID;
    let mut config = ProgramTestConfig::new_v2(true, Some(vec![("single_ata_test", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    // Derive rent sponsor PDA for this program (not the light token program)
    let (rent_sponsor, _) = derive_rent_sponsor_pda(&program_id);

    let (init_config_ix, _config_pda) = InitializeRentFreeConfig::new(
        &program_id,
        &payer.pubkey(),
        &program_data_pda,
        rent_sponsor,
        payer.pubkey(),
    )
    .build();

    rpc.create_and_send_transaction(&[init_config_ix], &payer.pubkey(), &[&payer])
        .await
        .expect("Initialize config should succeed");

    // Setup mint first
    let (mint, _mint_seed) = setup_create_mint(
        &mut rpc,
        &payer,
        payer.pubkey(), // mint_authority
        9,              // decimals
    )
    .await;

    // The ATA owner will be the payer
    let ata_owner = payer.pubkey();

    // Derive the ATA address using Light Token SDK's derivation
    let (ata, ata_bump) = light_token::instruction::derive_token_ata(&ata_owner, &mint);

    // Get proof (no PDA accounts for ATA-only instruction)
    let proof_result = get_create_accounts_proof(&rpc, &program_id, vec![])
        .await
        .unwrap();

    // Build instruction
    let accounts = single_ata_test::accounts::CreateAta {
        fee_payer: payer.pubkey(),
        ata_mint: mint,
        ata_owner,
        ata,
        light_token_config: LIGHT_TOKEN_CONFIG,
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = single_ata_test::instruction::CreateAta {
        params: CreateAtaParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            ata_bump,
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
        .expect("CreateAta instruction should succeed");

    // Verify ATA exists on-chain
    let ata_account = rpc
        .get_account(ata)
        .await
        .unwrap()
        .expect("ATA should exist on-chain");

    // Parse and verify token data using full struct comparison
    use light_token_interface::state::token::{AccountState, Token, ACCOUNT_TYPE_TOKEN_ACCOUNT};
    let token: Token = borsh::BorshDeserialize::deserialize(&mut &ata_account.data[..])
        .expect("Failed to deserialize Token");

    // Build expected token for full comparison
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
        extensions: token.extensions.clone(), // Use actual extensions
    };

    assert_eq!(
        token, expected_token,
        "ATA should match expected after creation"
    );
}
