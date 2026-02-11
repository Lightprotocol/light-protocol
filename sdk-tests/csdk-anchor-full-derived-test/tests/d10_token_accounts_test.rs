//! Integration tests for D10 token account macro features.
//!
//! Tests #[light_account(init, token, ...)] and #[light_account(init, associated_token, ...)]
//! automatic code generation for creating compressed token accounts.

mod shared;

use anchor_lang::{InstructionData, ToAccountMetas};
use csdk_anchor_full_derived_test::d10_token_accounts::{
    D10SingleAtaMarkonlyParams, D10SingleAtaParams, D10SingleVaultParams,
    D10_SINGLE_VAULT_AUTH_SEED, D10_SINGLE_VAULT_SEED,
};
use light_client::interface::{get_create_accounts_proof, InitializeRentFreeConfig};
use light_program_test::{
    program_test::{setup_mock_program_data, LightProgramTest},
    ProgramTestConfig, Rpc,
};
use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use light_token::instruction::{LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Test context for D10 token account tests
struct D10TestContext {
    rpc: LightProgramTest,
    payer: Keypair,
    #[allow(dead_code)]
    config_pda: Pubkey,
    program_id: Pubkey,
}

impl D10TestContext {
    async fn new() -> Self {
        let program_id = csdk_anchor_full_derived_test::ID;
        let mut config = ProgramTestConfig::new_v2(
            true,
            Some(vec![("csdk_anchor_full_derived_test", program_id)]),
        );
        config = config.with_light_protocol_events();

        let mut rpc = LightProgramTest::new(config).await.unwrap();
        let payer = rpc.get_payer().insecure_clone();

        let program_data_pda = setup_mock_program_data(&mut rpc, &payer, &program_id);

        let (init_config_ix, config_pda) = InitializeRentFreeConfig::new(
            &program_id,
            &payer.pubkey(),
            &program_data_pda,
            csdk_anchor_full_derived_test::program_rent_sponsor(),
            payer.pubkey(),
        )
        .build();

        rpc.create_and_send_transaction(&[init_config_ix], &payer.pubkey(), &[&payer])
            .await
            .expect("Initialize config should succeed");

        Self {
            rpc,
            payer,
            config_pda,
            program_id,
        }
    }

    /// Setup a mint for token-based tests.
    /// Returns (mint_pubkey, compression_address, ata_pubkeys, mint_seed_keypair)
    async fn setup_mint(&mut self) -> (Pubkey, [u8; 32], Vec<Pubkey>, Keypair) {
        shared::setup_create_mint(
            &mut self.rpc,
            &self.payer,
            self.payer.pubkey(), // mint_authority
            9,                   // decimals
            vec![],              // no recipients initially
        )
        .await
    }
}

/// Tests D10SingleVault: #[light_account(init, token, ...)] automatic code generation.
/// The macro should generate CreateTokenAccountCpi in LightFinalize.
#[tokio::test]
async fn test_d10_single_vault() {
    let mut ctx = D10TestContext::new().await;

    // Setup mint
    let (mint, _compression_addr, _atas, _mint_seed) = ctx.setup_mint().await;

    // Derive PDAs
    let (d10_vault_authority, _auth_bump) =
        Pubkey::find_program_address(&[D10_SINGLE_VAULT_AUTH_SEED], &ctx.program_id);
    let (d10_single_vault, vault_bump) =
        Pubkey::find_program_address(&[D10_SINGLE_VAULT_SEED, mint.as_ref()], &ctx.program_id);

    // Get proof (no PDA accounts for token-only instruction)
    let proof_result = get_create_accounts_proof(&ctx.rpc, &ctx.program_id, vec![])
        .await
        .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D10SingleVault {
        fee_payer: ctx.payer.pubkey(),
        d10_mint: mint,
        d10_vault_authority,
        d10_single_vault,
        light_token_config: LIGHT_TOKEN_CONFIG,
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_cpi_authority: light_token_types::CPI_AUTHORITY_PDA.into(),
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D10SingleVault {
        params: D10SingleVaultParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            vault_bump,
        },
    };

    let instruction = Instruction {
        program_id: ctx.program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    ctx.rpc
        .create_and_send_transaction(&[instruction], &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .expect("D10SingleVault instruction should succeed");

    // Verify token vault exists on-chain
    shared::assert_onchain_exists(&mut ctx.rpc, &d10_single_vault, "d10_single_vault").await;

    // Full-struct Token assertion for vault after creation
    {
        use light_token_interface::state::token::{
            AccountState, Token, ACCOUNT_TYPE_TOKEN_ACCOUNT,
        };
        let vault_account = ctx
            .rpc
            .get_account(d10_single_vault)
            .await
            .unwrap()
            .unwrap();
        let vault_data: Token =
            borsh::BorshDeserialize::deserialize(&mut &vault_account.data[..]).unwrap();
        let expected_vault = Token {
            mint: mint.into(),
            owner: d10_vault_authority.into(),
            amount: 0,
            delegate: None,
            state: AccountState::Initialized,
            is_native: None,
            delegated_amount: 0,
            close_authority: None,
            account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
            extensions: vault_data.extensions.clone(),
        };
        assert_eq!(
            vault_data, expected_vault,
            "d10_single_vault should match after creation"
        );
    }
}

/// Tests D10SingleAta: #[light_account(init, associated_token, ...)] automatic code generation.
/// The macro should generate create_associated_token_account_idempotent in LightFinalize.
#[tokio::test]
async fn test_d10_single_ata() {
    let mut ctx = D10TestContext::new().await;

    // Setup mint
    let (mint, _compression_addr, _atas, _mint_seed) = ctx.setup_mint().await;

    // The ATA owner will be the payer
    let ata_owner = ctx.payer.pubkey();

    // Derive the ATA address using Light Token SDK's derivation
    let (d10_single_ata, ata_bump) = light_token::instruction::derive_token_ata(&ata_owner, &mint);

    // Get proof (no PDA accounts for ATA-only instruction)
    let proof_result = get_create_accounts_proof(&ctx.rpc, &ctx.program_id, vec![])
        .await
        .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D10SingleAta {
        fee_payer: ctx.payer.pubkey(),
        d10_ata_mint: mint,
        d10_ata_owner: ata_owner,
        d10_single_ata,
        light_token_config: LIGHT_TOKEN_CONFIG,
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D10SingleAta {
        params: D10SingleAtaParams {
            create_accounts_proof: proof_result.create_accounts_proof,
            ata_bump,
        },
    };

    let instruction = Instruction {
        program_id: ctx.program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    ctx.rpc
        .create_and_send_transaction(&[instruction], &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .expect("D10SingleAta instruction should succeed");

    // Verify ATA exists on-chain
    shared::assert_onchain_exists(&mut ctx.rpc, &d10_single_ata, "d10_single_ata").await;

    // Full-struct Token assertion for ATA after creation
    {
        use light_token_interface::state::token::{
            AccountState, Token, ACCOUNT_TYPE_TOKEN_ACCOUNT,
        };
        let ata_account = ctx.rpc.get_account(d10_single_ata).await.unwrap().unwrap();
        let ata_data: Token =
            borsh::BorshDeserialize::deserialize(&mut &ata_account.data[..]).unwrap();
        let expected_ata = Token {
            mint: mint.into(),
            owner: ata_owner.into(),
            amount: 0,
            delegate: None,
            state: AccountState::Initialized,
            is_native: None,
            delegated_amount: 0,
            close_authority: None,
            account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
            extensions: ata_data.extensions.clone(),
        };
        assert_eq!(
            ata_data, expected_ata,
            "d10_single_ata should match after creation"
        );
    }
}

/// Tests idempotent ATA creation.
/// Creating the same ATA twice should succeed (idempotent).
#[tokio::test]
async fn test_d10_single_ata_idempotent_creation() {
    let mut ctx = D10TestContext::new().await;

    // Setup mint
    let (mint, _compression_addr, _atas, _mint_seed) = ctx.setup_mint().await;

    // The ATA owner will be the payer
    let ata_owner = ctx.payer.pubkey();

    // Derive the ATA address
    let (d10_single_ata, ata_bump) = light_token::instruction::derive_token_ata(&ata_owner, &mint);

    // Get proof for first creation
    let proof_result = get_create_accounts_proof(&ctx.rpc, &ctx.program_id, vec![])
        .await
        .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D10SingleAta {
        fee_payer: ctx.payer.pubkey(),
        d10_ata_mint: mint,
        d10_ata_owner: ata_owner,
        d10_single_ata,
        light_token_config: LIGHT_TOKEN_CONFIG,
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D10SingleAta {
        params: D10SingleAtaParams {
            create_accounts_proof: proof_result.create_accounts_proof.clone(),
            ata_bump,
        },
    };

    let instruction = Instruction {
        program_id: ctx.program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts.clone(),
        ]
        .concat(),
        data: instruction_data.data(),
    };

    // First creation should succeed
    ctx.rpc
        .create_and_send_transaction(&[instruction], &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .expect("First D10SingleAta creation should succeed");

    // Verify ATA exists on-chain
    shared::assert_onchain_exists(&mut ctx.rpc, &d10_single_ata, "d10_single_ata").await;

    // Get balance after first creation
    let ata_account_1 = ctx.rpc.get_account(d10_single_ata).await.unwrap().unwrap();
    let balance_after_first = ata_account_1.lamports;

    // Get fresh proof for second creation
    let proof_result_2 = get_create_accounts_proof(&ctx.rpc, &ctx.program_id, vec![])
        .await
        .unwrap();

    let accounts_2 = csdk_anchor_full_derived_test::accounts::D10SingleAta {
        fee_payer: ctx.payer.pubkey(),
        d10_ata_mint: mint,
        d10_ata_owner: ata_owner,
        d10_single_ata,
        light_token_config: LIGHT_TOKEN_CONFIG,
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data_2 = csdk_anchor_full_derived_test::instruction::D10SingleAta {
        params: D10SingleAtaParams {
            create_accounts_proof: proof_result_2.create_accounts_proof,
            ata_bump,
        },
    };

    let instruction_2 = Instruction {
        program_id: ctx.program_id,
        accounts: [
            accounts_2.to_account_metas(None),
            proof_result_2.remaining_accounts,
        ]
        .concat(),
        data: instruction_data_2.data(),
    };

    // Second creation should also succeed (idempotent)
    ctx.rpc
        .create_and_send_transaction(&[instruction_2], &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .expect("Second D10SingleAta creation should succeed (idempotent)");

    // Verify ATA still exists with same balance
    let ata_account_2 = ctx.rpc.get_account(d10_single_ata).await.unwrap().unwrap();
    assert_eq!(
        ata_account_2.lamports, balance_after_first,
        "ATA balance should be unchanged after idempotent second creation"
    );
}

/// Tests D10SingleAtaMarkonly: #[light_account(associated_token::...)] mark-only mode.
///
/// This tests the mark-only ATA pattern where:
/// - The macro generates no-op LightPreInit/LightFinalize implementations
/// - User manually calls CreateTokenAtaCpi in the instruction handler
/// - No custom seed structs needed - ATA addresses are derived deterministically from (authority, mint)
///
/// For decompression, ATAs use the standard derivation rather than custom seed structs.
/// The forester can re-create an ATA by calling CreateTokenAtaCpi.idempotent() with
/// the same authority and mint, which will recreate the account at the deterministic address.
#[tokio::test]
async fn test_d10_single_ata_markonly() {
    let mut ctx = D10TestContext::new().await;

    // Setup mint
    let (mint, _compression_addr, _atas, _mint_seed) = ctx.setup_mint().await;

    // The ATA owner will be a different keypair (not the payer)
    let ata_owner = Keypair::new().pubkey();

    // Derive the ATA address using Light Token SDK's derivation
    let (d10_markonly_ata, ata_bump) =
        light_token::instruction::derive_token_ata(&ata_owner, &mint);

    // Get proof (no PDA accounts for ATA-only instruction)
    let proof_result = get_create_accounts_proof(&ctx.rpc, &ctx.program_id, vec![])
        .await
        .unwrap();

    // Build instruction
    let accounts = csdk_anchor_full_derived_test::accounts::D10SingleAtaMarkonly {
        fee_payer: ctx.payer.pubkey(),
        d10_markonly_ata_mint: mint,
        d10_markonly_ata_owner: ata_owner,
        d10_markonly_ata,
        light_token_config: LIGHT_TOKEN_CONFIG,
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D10SingleAtaMarkonly {
        params: D10SingleAtaMarkonlyParams { ata_bump },
    };

    let instruction = Instruction {
        program_id: ctx.program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    ctx.rpc
        .create_and_send_transaction(&[instruction], &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .expect("D10SingleAtaMarkonly instruction should succeed");

    // Verify ATA exists on-chain
    shared::assert_onchain_exists(&mut ctx.rpc, &d10_markonly_ata, "d10_markonly_ata").await;
}

/// Tests mark-only ATA compression and decompression lifecycle.
///
/// Verifies that:
/// 1. ATA is created via manual CreateTokenAtaCpi
/// 2. ATA is auto-compressed by forester after time warp
/// 3. ATA can be decompressed using create_load_instructions with AccountSpec::Ata
#[tokio::test]
async fn test_d10_single_ata_markonly_lifecycle() {
    use csdk_anchor_full_derived_test::csdk_anchor_full_derived_test::LightAccountVariant;
    use light_client::interface::{create_load_instructions, AccountSpec};
    use light_compressible::rent::SLOTS_PER_EPOCH;
    use light_program_test::program_test::TestRpc;

    let mut ctx = D10TestContext::new().await;

    // Setup mint
    let (mint, _compression_addr, _atas, _mint_seed) = ctx.setup_mint().await;

    // The ATA owner will be a keypair we control (needed for decompression signing)
    let ata_owner_keypair = Keypair::new();
    let ata_owner = ata_owner_keypair.pubkey();

    // Derive the ATA address
    let (d10_markonly_ata, ata_bump) =
        light_token::instruction::derive_token_ata(&ata_owner, &mint);

    // PHASE 1: Create ATA
    let proof_result = get_create_accounts_proof(&ctx.rpc, &ctx.program_id, vec![])
        .await
        .unwrap();

    let accounts = csdk_anchor_full_derived_test::accounts::D10SingleAtaMarkonly {
        fee_payer: ctx.payer.pubkey(),
        d10_markonly_ata_mint: mint,
        d10_markonly_ata_owner: ata_owner,
        d10_markonly_ata,
        light_token_config: LIGHT_TOKEN_CONFIG,
        light_token_rent_sponsor: LIGHT_TOKEN_RENT_SPONSOR,
        light_token_program: LIGHT_TOKEN_PROGRAM_ID.into(),
        system_program: solana_sdk::system_program::ID,
    };

    let instruction_data = csdk_anchor_full_derived_test::instruction::D10SingleAtaMarkonly {
        params: D10SingleAtaMarkonlyParams { ata_bump },
    };

    let instruction = Instruction {
        program_id: ctx.program_id,
        accounts: [
            accounts.to_account_metas(None),
            proof_result.remaining_accounts,
        ]
        .concat(),
        data: instruction_data.data(),
    };

    ctx.rpc
        .create_and_send_transaction(&[instruction], &ctx.payer.pubkey(), &[&ctx.payer])
        .await
        .expect("D10SingleAtaMarkonly creation should succeed");

    // Verify ATA exists
    shared::assert_onchain_exists(&mut ctx.rpc, &d10_markonly_ata, "d10_markonly_ata").await;

    // Full-struct Token assertion for ATA after creation
    {
        use light_token_interface::state::token::{
            AccountState, Token, ACCOUNT_TYPE_TOKEN_ACCOUNT,
        };
        let ata_account = ctx
            .rpc
            .get_account(d10_markonly_ata)
            .await
            .unwrap()
            .unwrap();
        let ata_data: Token =
            borsh::BorshDeserialize::deserialize(&mut &ata_account.data[..]).unwrap();
        let expected_ata = Token {
            mint: mint.into(),
            owner: ata_owner.into(),
            amount: 0,
            delegate: None,
            state: AccountState::Initialized,
            is_native: None,
            delegated_amount: 0,
            close_authority: None,
            account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
            extensions: ata_data.extensions.clone(),
        };
        assert_eq!(
            ata_data, expected_ata,
            "d10_markonly_ata should match after creation"
        );
    }

    // PHASE 2: Warp time to trigger forester auto-compression
    ctx.rpc
        .warp_slot_forward(SLOTS_PER_EPOCH * 30)
        .await
        .unwrap();

    // Verify ATA is compressed (closed on-chain)
    shared::assert_onchain_closed(&mut ctx.rpc, &d10_markonly_ata, "d10_markonly_ata").await;

    // PHASE 3: Decompress ATA using create_load_instructions
    // ATAs use get_associated_token_account_interface which fetches the compressed token data
    let ata_interface = ctx
        .rpc
        .get_associated_token_account_interface(&ata_owner, &mint, None)
        .await
        .expect("get_associated_token_account_interface should succeed")
        .value
        .expect("ata interface should exist");
    assert!(
        ata_interface.is_cold(),
        "ATA should be cold after compression"
    );

    // Build AccountSpec for ATA decompression
    let specs: Vec<AccountSpec<LightAccountVariant>> =
        vec![AccountSpec::Ata(Box::new(ata_interface))];

    // Create decompression instructions
    let decompress_instructions =
        create_load_instructions(&specs, ctx.payer.pubkey(), ctx.config_pda, &ctx.rpc)
            .await
            .expect("create_load_instructions should succeed");

    // Execute decompression (ATA owner must sign for decompression)
    ctx.rpc
        .create_and_send_transaction(
            &decompress_instructions,
            &ctx.payer.pubkey(),
            &[&ctx.payer, &ata_owner_keypair],
        )
        .await
        .expect("ATA decompression should succeed");

    // PHASE 4: Verify ATA is back on-chain
    shared::assert_onchain_exists(&mut ctx.rpc, &d10_markonly_ata, "d10_markonly_ata").await;

    // Full-struct Token assertion for ATA after decompression
    {
        use light_token_interface::state::token::{
            AccountState, Token, ACCOUNT_TYPE_TOKEN_ACCOUNT,
        };
        let ata_account = ctx
            .rpc
            .get_account(d10_markonly_ata)
            .await
            .unwrap()
            .unwrap();
        let ata_data: Token =
            borsh::BorshDeserialize::deserialize(&mut &ata_account.data[..]).unwrap();
        let expected_ata = Token {
            mint: mint.into(),
            owner: ata_owner.into(),
            amount: 0,
            delegate: None,
            state: AccountState::Initialized,
            is_native: None,
            delegated_amount: 0,
            close_authority: None,
            account_type: ACCOUNT_TYPE_TOKEN_ACCOUNT,
            extensions: ata_data.extensions.clone(),
        };
        assert_eq!(
            ata_data, expected_ata,
            "d10_markonly_ata should match after decompression"
        );
    }
}
