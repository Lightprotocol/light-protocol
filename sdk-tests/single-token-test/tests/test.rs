//! Integration test for single token vault macro validation.

use anchor_lang::{InstructionData, ToAccountMetas};
use light_client::interface::{get_create_accounts_proof, InitializeRentFreeConfig};
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest},
    Indexer, ProgramTestConfig, Rpc,
};
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

/// Test creating a single token vault using the macro.
/// Validates that #[light_account(init, token, ...)] works in isolation.
#[tokio::test]
async fn test_create_single_token_vault() {
    use single_token_test::{CreateTokenVaultParams, VAULT_AUTH_SEED, VAULT_SEED};

    let program_id = single_token_test::ID;
    let mut config = ProgramTestConfig::new_v2(true, Some(vec![("single_token_test", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    let (init_config_ix, _config_pda) = InitializeRentFreeConfig::new(
        &program_id,
        &payer.pubkey(),
        &program_data_pda,
        LIGHT_TOKEN_RENT_SPONSOR,
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

    // Derive PDAs
    let (vault_authority, _auth_bump) =
        Pubkey::find_program_address(&[VAULT_AUTH_SEED], &program_id);
    let (vault, vault_bump) =
        Pubkey::find_program_address(&[VAULT_SEED, mint.as_ref()], &program_id);

    // Get proof (no PDA accounts for token-only instruction)
    let proof_result = get_create_accounts_proof(&rpc, &program_id, vec![])
        .await
        .unwrap();

    // Build instruction
    let accounts = single_token_test::accounts::CreateTokenVault {
        fee_payer: payer.pubkey(),
        mint,
        vault_authority,
        vault,
        light_token_config: LIGHT_TOKEN_CONFIG,
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = single_token_test::instruction::CreateTokenVault {
        params: CreateTokenVaultParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            vault_bump,
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
        .expect("CreateTokenVault instruction should succeed");

    // Verify token vault exists on-chain
    let vault_account = rpc
        .get_account(vault)
        .await
        .unwrap()
        .expect("Token vault should exist on-chain");

    // Parse and verify token data
    use light_token_interface::state::Token;
    let token: Token = borsh::BorshDeserialize::deserialize(&mut &vault_account.data[..])
        .expect("Failed to deserialize Token");

    // Verify owner (should be vault_authority PDA)
    assert_eq!(
        token.owner,
        vault_authority.to_bytes(),
        "Token vault owner should be vault_authority"
    );

    // Verify mint
    assert_eq!(token.mint, mint.to_bytes(), "Token vault mint should match");

    // Verify initial amount is 0
    assert_eq!(token.amount, 0, "Token vault amount should be 0 initially");
}
