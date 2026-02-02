//! Integration tests for #[derive(LightProgram)] with all variant kinds.

use anchor_lang::{InstructionData, ToAccountMetas};
use light_client::interface::{
    get_create_accounts_proof, CreateAccountsProofInput, InitializeRentFreeConfig,
};
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

/// Setup helper: Creates a compressed mint using the ctoken SDK.
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

// =============================================================================
// 1. Create PDA
// =============================================================================

#[tokio::test]
async fn test_create_single_pda_derive() {
    use single_pda_derive_test::CreatePdaParams;

    let program_id = single_pda_derive_test::ID;
    let mut config =
        ProgramTestConfig::new_v2(true, Some(vec![("single_pda_derive_test", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    let (rent_sponsor, _) = derive_rent_sponsor_pda(&program_id);

    let (init_config_ix, config_pda) = InitializeRentFreeConfig::new(
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

    let owner = Keypair::new().pubkey();

    let (record_pda, _) =
        Pubkey::find_program_address(&[b"minimal_record", owner.as_ref()], &program_id);

    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::pda(record_pda)],
    )
    .await
    .unwrap();

    let accounts = single_pda_derive_test::accounts::CreatePda {
        fee_payer: payer.pubkey(),
        compression_config: config_pda,
        pda_rent_sponsor: rent_sponsor,
        record: record_pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = single_pda_derive_test::instruction::CreatePda {
        params: CreatePdaParams {
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
        .expect("CreatePda should succeed");

    let record_account = rpc
        .get_account(record_pda)
        .await
        .unwrap()
        .expect("Record PDA should exist on-chain");

    use single_pda_derive_test::MinimalRecord;
    let record: MinimalRecord =
        borsh::BorshDeserialize::deserialize(&mut &record_account.data[8..])
            .expect("Failed to deserialize MinimalRecord");

    assert_eq!(record.owner, owner, "Record owner should match");
    assert!(
        !record.compression_info.is_compressed(),
        "Record should be in decompressed state"
    );
}

// =============================================================================
// 2. Create ATA
// =============================================================================

#[tokio::test]
async fn test_create_ata_derive() {
    use single_pda_derive_test::CreateAtaParams;

    let program_id = single_pda_derive_test::ID;
    let mut config =
        ProgramTestConfig::new_v2(true, Some(vec![("single_pda_derive_test", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

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
        payer.pubkey(),
        9,
    )
    .await;

    let ata_owner = payer.pubkey();
    let (ata, ata_bump) = light_token::instruction::derive_token_ata(&ata_owner, &mint);

    // No PDA accounts for ATA-only instruction
    let proof_result = get_create_accounts_proof(&rpc, &program_id, vec![])
        .await
        .unwrap();

    let accounts = single_pda_derive_test::accounts::CreateAta {
        fee_payer: payer.pubkey(),
        ata_mint: mint,
        ata_owner,
        ata,
        light_token_config: LIGHT_TOKEN_CONFIG,
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = single_pda_derive_test::instruction::CreateAta {
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
        .expect("CreateAta should succeed");

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

    assert_eq!(token, expected_token, "ATA should match expected after creation");
}

// =============================================================================
// 3. Create Token Vault
// =============================================================================

#[tokio::test]
async fn test_create_token_vault_derive() {
    use single_pda_derive_test::{CreateTokenVaultParams, VAULT_AUTH_SEED, VAULT_SEED};

    let program_id = single_pda_derive_test::ID;
    let mut config =
        ProgramTestConfig::new_v2(true, Some(vec![("single_pda_derive_test", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

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
        payer.pubkey(),
        9,
    )
    .await;

    let (vault_authority, _auth_bump) =
        Pubkey::find_program_address(&[VAULT_AUTH_SEED], &program_id);
    let (vault, vault_bump) =
        Pubkey::find_program_address(&[VAULT_SEED, mint.as_ref()], &program_id);

    // No PDA accounts for token-only instruction
    let proof_result = get_create_accounts_proof(&rpc, &program_id, vec![])
        .await
        .unwrap();

    let accounts = single_pda_derive_test::accounts::CreateTokenVault {
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

    let instruction_data = single_pda_derive_test::instruction::CreateTokenVault {
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
        .expect("CreateTokenVault should succeed");

    let vault_account = rpc
        .get_account(vault)
        .await
        .unwrap()
        .expect("Token vault should exist on-chain");

    use light_token_interface::state::token::{AccountState, Token, ACCOUNT_TYPE_TOKEN_ACCOUNT};
    let token: Token = borsh::BorshDeserialize::deserialize(&mut &vault_account.data[..])
        .expect("Failed to deserialize Token");

    let expected_token = Token {
        mint: mint.to_bytes().into(),
        owner: vault_authority.to_bytes().into(),
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
        "Token vault should match expected after creation"
    );
}

// =============================================================================
// 4. Create Zero-Copy Record
// =============================================================================

#[tokio::test]
async fn test_create_zero_copy_record_derive() {
    use single_pda_derive_test::{CreateZeroCopyRecordParams, RECORD_SEED};

    let program_id = single_pda_derive_test::ID;
    let mut config =
        ProgramTestConfig::new_v2(true, Some(vec![("single_pda_derive_test", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    let (rent_sponsor, _) = derive_rent_sponsor_pda(&program_id);

    let (init_config_ix, config_pda) = InitializeRentFreeConfig::new(
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

    let owner = Keypair::new().pubkey();

    let (record_pda, _) =
        Pubkey::find_program_address(&[RECORD_SEED, owner.as_ref()], &program_id);

    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![CreateAccountsProofInput::pda(record_pda)],
    )
    .await
    .unwrap();

    let accounts = single_pda_derive_test::accounts::CreateZeroCopyRecord {
        fee_payer: payer.pubkey(),
        compression_config: config_pda,
        pda_rent_sponsor: rent_sponsor,
        record: record_pda,
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = single_pda_derive_test::instruction::CreateZeroCopyRecord {
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

    // Parse zero-copy data using bytemuck
    use single_pda_derive_test::ZeroCopyRecord;
    let discriminator_len = 8;
    let data = &record_account.data[discriminator_len..];
    let record: &ZeroCopyRecord = bytemuck::from_bytes(data);

    assert_eq!(record.owner, owner, "Record owner should match");
    assert_eq!(record.counter, 0, "Record counter should be 0");
}

// =============================================================================
// 5. Create Mint
// =============================================================================

#[tokio::test]
async fn test_create_mint_derive() {
    use single_pda_derive_test::{CreateMintParams, MINT_SIGNER_SEED_A};

    let program_id = single_pda_derive_test::ID;
    let mut config =
        ProgramTestConfig::new_v2(true, Some(vec![("single_pda_derive_test", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    let (rent_sponsor, _) = derive_rent_sponsor_pda(&program_id);

    let (init_config_ix, config_pda) = InitializeRentFreeConfig::new(
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

    let accounts = single_pda_derive_test::accounts::CreateMint {
        fee_payer: payer.pubkey(),
        authority: authority.pubkey(),
        mint_signer: mint_signer_pda,
        mint: mint_pda,
        compression_config: config_pda,
        light_token_config: LIGHT_TOKEN_CONFIG,
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        light_token_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = single_pda_derive_test::instruction::CreateMint {
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
}

// =============================================================================
// 6. Create Two Mints
// =============================================================================

#[tokio::test]
async fn test_create_two_mints_derive() {
    use single_pda_derive_test::{CreateTwoMintsParams, MINT_SIGNER_SEED_A, MINT_SIGNER_SEED_B};

    let program_id = single_pda_derive_test::ID;
    let mut config =
        ProgramTestConfig::new_v2(true, Some(vec![("single_pda_derive_test", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    let (rent_sponsor, _) = derive_rent_sponsor_pda(&program_id);

    let (init_config_ix, config_pda) = InitializeRentFreeConfig::new(
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

    let authority = Keypair::new();

    // Derive mint A
    let (mint_signer_a, mint_signer_bump_a) = Pubkey::find_program_address(
        &[MINT_SIGNER_SEED_A, authority.pubkey().as_ref()],
        &program_id,
    );
    let (mint_a_pda, _) = light_token::instruction::find_mint_address(&mint_signer_a);

    // Derive mint B
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

    let accounts = single_pda_derive_test::accounts::CreateTwoMints {
        fee_payer: payer.pubkey(),
        authority: authority.pubkey(),
        mint_signer_a,
        mint_a: mint_a_pda,
        mint_signer_b,
        mint_b: mint_b_pda,
        compression_config: config_pda,
        light_token_config: LIGHT_TOKEN_CONFIG,
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        light_token_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = single_pda_derive_test::instruction::CreateTwoMints {
        params: CreateTwoMintsParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            mint_signer_bump_a,
            mint_signer_bump_b,
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
        .expect("CreateTwoMints should succeed");

    // Verify mint A
    let mint_a_account = rpc
        .get_account(mint_a_pda)
        .await
        .unwrap()
        .expect("Mint A should exist on-chain");

    use light_token_interface::state::Mint;
    let mint_a: Mint = borsh::BorshDeserialize::deserialize(&mut &mint_a_account.data[..])
        .expect("Failed to deserialize Mint A");

    assert_eq!(mint_a.base.decimals, 9, "Mint A should have 9 decimals");
    assert_eq!(
        mint_a.base.mint_authority,
        Some(payer.pubkey().to_bytes().into()),
        "Mint A authority should be fee_payer"
    );

    // Verify mint B
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
        Some(payer.pubkey().to_bytes().into()),
        "Mint B authority should be fee_payer"
    );
}

// =============================================================================
// 7. Create All (combined)
// =============================================================================

#[tokio::test]
async fn test_create_all_derive() {
    use single_pda_derive_test::{
        CreateAllParams, MINT_SIGNER_SEED_A, MINT_SIGNER_SEED_B, RECORD_SEED, VAULT_AUTH_SEED,
        VAULT_SEED,
    };

    let program_id = single_pda_derive_test::ID;
    let mut config =
        ProgramTestConfig::new_v2(true, Some(vec![("single_pda_derive_test", program_id)]));
    config = config.with_light_protocol_events();

    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

    let (rent_sponsor, _) = derive_rent_sponsor_pda(&program_id);

    let (init_config_ix, config_pda) = InitializeRentFreeConfig::new(
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

    // Setup pre-existing mints for ATA and vault
    let (ata_mint, _) = setup_create_mint(&mut rpc, &payer, payer.pubkey(), 9).await;
    let (vault_mint, _) = setup_create_mint(&mut rpc, &payer, payer.pubkey(), 9).await;

    let owner = Keypair::new().pubkey();
    let authority = Keypair::new();

    // PDA
    let (record_pda, _) =
        Pubkey::find_program_address(&[b"minimal_record", owner.as_ref()], &program_id);

    // Zero-copy
    let (zc_record_pda, _) =
        Pubkey::find_program_address(&[RECORD_SEED, owner.as_ref()], &program_id);

    // ATA
    let ata_owner = payer.pubkey();
    let (ata, ata_bump) = light_token::instruction::derive_token_ata(&ata_owner, &ata_mint);

    // Token vault
    let (vault_authority, _) = Pubkey::find_program_address(&[VAULT_AUTH_SEED], &program_id);
    let (vault, vault_bump) =
        Pubkey::find_program_address(&[VAULT_SEED, vault_mint.as_ref()], &program_id);

    // Mint A
    let (mint_signer_a, mint_signer_bump_a) = Pubkey::find_program_address(
        &[MINT_SIGNER_SEED_A, authority.pubkey().as_ref()],
        &program_id,
    );
    let (mint_a_pda, _) = light_token::instruction::find_mint_address(&mint_signer_a);

    // Mint B
    let (mint_signer_b, mint_signer_bump_b) = Pubkey::find_program_address(
        &[MINT_SIGNER_SEED_B, authority.pubkey().as_ref()],
        &program_id,
    );
    let (mint_b_pda, _) = light_token::instruction::find_mint_address(&mint_signer_b);

    // Build proof inputs for all accounts
    let proof_result = get_create_accounts_proof(
        &rpc,
        &program_id,
        vec![
            CreateAccountsProofInput::pda(record_pda),
            CreateAccountsProofInput::pda(zc_record_pda),
            CreateAccountsProofInput::mint(mint_signer_a),
            CreateAccountsProofInput::mint(mint_signer_b),
        ],
    )
    .await
    .unwrap();

    let accounts = single_pda_derive_test::accounts::CreateAll {
        fee_payer: payer.pubkey(),
        compression_config: config_pda,
        pda_rent_sponsor: rent_sponsor,
        record: record_pda,
        zero_copy_record: zc_record_pda,
        ata_mint,
        ata_owner,
        ata,
        vault_mint,
        vault_authority,
        vault,
        authority: authority.pubkey(),
        mint_signer_a,
        mint_a: mint_a_pda,
        mint_signer_b,
        mint_b: mint_b_pda,
        light_token_config: LIGHT_TOKEN_CONFIG,
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = single_pda_derive_test::instruction::CreateAll {
        params: CreateAllParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            owner,
            ata_bump,
            vault_bump,
            mint_signer_bump_a,
            mint_signer_bump_b,
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
        .expect("CreateAll should succeed");

    // Verify PDA
    let record_account = rpc
        .get_account(record_pda)
        .await
        .unwrap()
        .expect("Record PDA should exist");
    use single_pda_derive_test::MinimalRecord;
    let record: MinimalRecord =
        borsh::BorshDeserialize::deserialize(&mut &record_account.data[8..])
            .expect("Failed to deserialize MinimalRecord");
    assert_eq!(record.owner, owner, "Record owner should match");

    // Verify zero-copy
    let zc_account = rpc
        .get_account(zc_record_pda)
        .await
        .unwrap()
        .expect("Zero-copy record should exist");
    use single_pda_derive_test::ZeroCopyRecord;
    let zc_record: &ZeroCopyRecord = bytemuck::from_bytes(&zc_account.data[8..]);
    assert_eq!(zc_record.owner, owner, "ZC record owner should match");
    assert_eq!(zc_record.counter, 0, "ZC record counter should be 0");

    // Verify ATA
    let ata_account = rpc
        .get_account(ata)
        .await
        .unwrap()
        .expect("ATA should exist");
    use light_token_interface::state::token::{AccountState, Token, ACCOUNT_TYPE_TOKEN_ACCOUNT};
    let ata_token: Token = borsh::BorshDeserialize::deserialize(&mut &ata_account.data[..])
        .expect("Failed to deserialize ATA Token");
    assert_eq!(
        ata_token.mint,
        ata_mint.to_bytes().into(),
        "ATA mint should match"
    );
    assert_eq!(
        ata_token.owner,
        ata_owner.to_bytes().into(),
        "ATA owner should match"
    );

    // Verify vault
    let vault_account = rpc
        .get_account(vault)
        .await
        .unwrap()
        .expect("Vault should exist");
    let vault_token: Token = borsh::BorshDeserialize::deserialize(&mut &vault_account.data[..])
        .expect("Failed to deserialize Vault Token");
    assert_eq!(
        vault_token.mint,
        vault_mint.to_bytes().into(),
        "Vault mint should match"
    );
    assert_eq!(
        vault_token.owner,
        vault_authority.to_bytes().into(),
        "Vault owner should match"
    );

    // Verify mint A
    let mint_a_account = rpc
        .get_account(mint_a_pda)
        .await
        .unwrap()
        .expect("Mint A should exist");
    use light_token_interface::state::Mint;
    let mint_a: Mint = borsh::BorshDeserialize::deserialize(&mut &mint_a_account.data[..])
        .expect("Failed to deserialize Mint A");
    assert_eq!(mint_a.base.decimals, 9, "Mint A should have 9 decimals");

    // Verify mint B
    let mint_b_account = rpc
        .get_account(mint_b_pda)
        .await
        .unwrap()
        .expect("Mint B should exist");
    let mint_b: Mint = borsh::BorshDeserialize::deserialize(&mut &mint_b_account.data[..])
        .expect("Failed to deserialize Mint B");
    assert_eq!(mint_b.base.decimals, 6, "Mint B should have 6 decimals");
}
