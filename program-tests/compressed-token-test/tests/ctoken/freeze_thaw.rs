//! Tests for CToken freeze and thaw instructions
//!
//! These tests verify that freeze and thaw instructions work correctly
//! for both basic mints and Token-2022 mints with extensions.

use borsh::{BorshDeserialize, BorshSerialize};
use light_ctoken_interface::{
    instructions::create_ctoken_account::CreateTokenAccountInstructionData,
    state::{
        AccountState, CToken, ExtensionStruct, PausableAccountExtension,
        PermanentDelegateAccountExtension, TokenDataVersion, TransferFeeAccountExtension,
        TransferHookAccountExtension,
    },
};
use light_ctoken_sdk::ctoken::{CompressibleParams, CreateCTokenAccount};
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_test_utils::{spl::create_mint_helper, Rpc, RpcError};
use serial_test::serial;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    signature::Keypair,
    signer::Signer,
    system_instruction::create_account,
};

use super::extensions::setup_extensions_test;

/// Helper to build a basic (non-compressible) CToken account initialization instruction
fn create_token_account(
    token_account: solana_sdk::pubkey::Pubkey,
    mint: solana_sdk::pubkey::Pubkey,
    owner: solana_sdk::pubkey::Pubkey,
) -> Result<Instruction, ProgramError> {
    let instruction_data = CreateTokenAccountInstructionData {
        owner: owner.to_bytes().into(),
        compressible_config: None,
    };

    let mut data = Vec::new();
    data.push(18u8); // CreateTokenAccount discriminator
    instruction_data
        .serialize(&mut data)
        .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;

    Ok(Instruction {
        program_id: light_compressed_token::ID,
        accounts: vec![
            AccountMeta::new(token_account, false),
            AccountMeta::new_readonly(mint, false),
        ],
        data,
    })
}

/// Helper to build a freeze instruction
fn build_freeze_instruction(
    token_account: &solana_sdk::pubkey::Pubkey,
    mint: &solana_sdk::pubkey::Pubkey,
    freeze_authority: &solana_sdk::pubkey::Pubkey,
) -> Instruction {
    Instruction {
        program_id: light_compressed_token::ID,
        accounts: vec![
            AccountMeta::new(*token_account, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(*freeze_authority, true),
        ],
        data: vec![10], // CTokenFreezeAccount discriminator
    }
}

/// Helper to build a thaw instruction
fn build_thaw_instruction(
    token_account: &solana_sdk::pubkey::Pubkey,
    mint: &solana_sdk::pubkey::Pubkey,
    freeze_authority: &solana_sdk::pubkey::Pubkey,
) -> Instruction {
    Instruction {
        program_id: light_compressed_token::ID,
        accounts: vec![
            AccountMeta::new(*token_account, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(*freeze_authority, true),
        ],
        data: vec![11], // CTokenThawAccount discriminator
    }
}

/// Test freeze and thaw with a basic SPL Token mint (not Token-2022)
/// Uses create_mint_helper which creates a mint with freeze_authority = payer
#[tokio::test]
#[serial]
async fn test_freeze_thaw_with_basic_mint() -> Result<(), RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None)).await?;
    let payer = rpc.get_payer().insecure_clone();
    let owner = Keypair::new();

    // 1. Create SPL Token mint with freeze_authority = payer
    let mint_pubkey = create_mint_helper(&mut rpc, &payer).await;

    // 2. Create basic CToken account (no extensions, just 165 bytes)
    let token_account_keypair = Keypair::new();
    let token_account_pubkey = token_account_keypair.pubkey();

    let rent_exemption = rpc.get_minimum_balance_for_rent_exemption(165).await?;

    let create_account_ix = create_account(
        &payer.pubkey(),
        &token_account_pubkey,
        rent_exemption,
        165,
        &light_compressed_token::ID,
    );

    let mut initialize_account_ix =
        create_token_account(token_account_pubkey, mint_pubkey, owner.pubkey()).map_err(|e| {
            RpcError::AssertRpcError(format!("Failed to create token account instruction: {}", e))
        })?;
    initialize_account_ix.data.push(0); // Append version byte

    rpc.create_and_send_transaction(
        &[create_account_ix, initialize_account_ix],
        &payer.pubkey(),
        &[&payer, &token_account_keypair],
    )
    .await?;

    // Verify initial state is Initialized
    let account_data = rpc.get_account(token_account_pubkey).await?.unwrap();
    let ctoken_before =
        CToken::deserialize(&mut &account_data.data[..]).expect("Failed to deserialize CToken");
    assert_eq!(
        ctoken_before.state,
        AccountState::Initialized,
        "Initial state should be Initialized"
    );

    // 3. Freeze the account
    let freeze_ix = build_freeze_instruction(&token_account_pubkey, &mint_pubkey, &payer.pubkey());

    rpc.create_and_send_transaction(&[freeze_ix], &payer.pubkey(), &[&payer])
        .await?;

    // 4. Assert state is Frozen
    let account_data_frozen = rpc.get_account(token_account_pubkey).await?.unwrap();
    let ctoken_frozen = CToken::deserialize(&mut &account_data_frozen.data[..])
        .expect("Failed to deserialize CToken after freeze");

    let expected_frozen = CToken {
        mint: mint_pubkey.to_bytes().into(),
        owner: owner.pubkey().to_bytes().into(),
        amount: 0,
        delegate: None,
        state: AccountState::Frozen,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        extensions: None,
    };

    assert_eq!(
        ctoken_frozen, expected_frozen,
        "CToken account should be frozen with all fields preserved"
    );

    // 5. Thaw the account
    let thaw_ix = build_thaw_instruction(&token_account_pubkey, &mint_pubkey, &payer.pubkey());

    rpc.create_and_send_transaction(&[thaw_ix], &payer.pubkey(), &[&payer])
        .await?;

    // 6. Assert state is Initialized again
    let account_data_thawed = rpc.get_account(token_account_pubkey).await?.unwrap();
    let ctoken_thawed = CToken::deserialize(&mut &account_data_thawed.data[..])
        .expect("Failed to deserialize CToken after thaw");

    let expected_thawed = CToken {
        mint: mint_pubkey.to_bytes().into(),
        owner: owner.pubkey().to_bytes().into(),
        amount: 0,
        delegate: None,
        state: AccountState::Initialized,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        extensions: None,
    };

    assert_eq!(
        ctoken_thawed, expected_thawed,
        "CToken account should be thawed with all fields preserved"
    );

    println!("Successfully tested freeze and thaw with basic mint");
    Ok(())
}

/// Test freeze and thaw with a Token-2022 mint that has all extensions
/// Verifies that extensions are preserved through freeze/thaw cycle
#[tokio::test]
#[serial]
async fn test_freeze_thaw_with_extensions() -> Result<(), RpcError> {
    let mut context = setup_extensions_test().await?;
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;
    let owner = Keypair::new();

    // 1. Create compressible CToken account with all extensions
    let account_keypair = Keypair::new();
    let account_pubkey = account_keypair.pubkey();

    let create_ix =
        CreateCTokenAccount::new(payer.pubkey(), account_pubkey, mint_pubkey, owner.pubkey())
            .with_compressible(CompressibleParams {
                compressible_config: context
                    .rpc
                    .test_accounts
                    .funding_pool_config
                    .compressible_config_pda,
                rent_sponsor: context
                    .rpc
                    .test_accounts
                    .funding_pool_config
                    .rent_sponsor_pda,
                pre_pay_num_epochs: 2,
                lamports_per_write: Some(100),
                compress_to_account_pubkey: None,
                token_account_version: TokenDataVersion::ShaFlat,
                compression_only: true,
            })
            .instruction()
            .map_err(|e| {
                RpcError::AssertRpcError(format!("Failed to create instruction: {}", e))
            })?;

    context
        .rpc
        .create_and_send_transaction(&[create_ix], &payer.pubkey(), &[&payer, &account_keypair])
        .await?;

    // Verify account was created with correct size (274 bytes with all extensions)
    let account_data_initial = context.rpc.get_account(account_pubkey).await?.unwrap();
    assert_eq!(
        account_data_initial.data.len(),
        274,
        "CToken account should be 274 bytes with all extensions"
    );

    // Deserialize and verify initial state
    let ctoken_initial = CToken::deserialize(&mut &account_data_initial.data[..])
        .expect("Failed to deserialize CToken");
    assert_eq!(
        ctoken_initial.state,
        AccountState::Initialized,
        "Initial state should be Initialized"
    );

    // Extract CompressionInfo (contains runtime values we need to preserve in expected)
    let compression_info = ctoken_initial
        .extensions
        .as_ref()
        .and_then(|exts| {
            exts.iter().find_map(|e| match e {
                ExtensionStruct::Compressible(info) => Some(*info),
                _ => None,
            })
        })
        .expect("Should have Compressible extension");

    // 2. Freeze the account
    let freeze_ix = build_freeze_instruction(&account_pubkey, &mint_pubkey, &payer.pubkey());

    context
        .rpc
        .create_and_send_transaction(&[freeze_ix], &payer.pubkey(), &[&payer])
        .await?;

    // 3. Assert state is Frozen with all extensions preserved
    let account_data_frozen = context.rpc.get_account(account_pubkey).await?.unwrap();
    let ctoken_frozen = CToken::deserialize(&mut &account_data_frozen.data[..])
        .expect("Failed to deserialize CToken after freeze");

    let expected_frozen = CToken {
        mint: mint_pubkey.to_bytes().into(),
        owner: owner.pubkey().to_bytes().into(),
        amount: 0,
        delegate: None,
        state: AccountState::Frozen,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        extensions: Some(vec![
            ExtensionStruct::Compressible(compression_info),
            ExtensionStruct::PausableAccount(PausableAccountExtension),
            ExtensionStruct::PermanentDelegateAccount(PermanentDelegateAccountExtension),
            ExtensionStruct::TransferFeeAccount(TransferFeeAccountExtension { withheld_amount: 0 }),
            ExtensionStruct::TransferHookAccount(TransferHookAccountExtension { transferring: 0 }),
        ]),
    };

    assert_eq!(
        ctoken_frozen, expected_frozen,
        "Frozen CToken should have state=Frozen with all 5 extensions preserved"
    );

    // 4. Thaw the account
    let thaw_ix = build_thaw_instruction(&account_pubkey, &mint_pubkey, &payer.pubkey());

    context
        .rpc
        .create_and_send_transaction(&[thaw_ix], &payer.pubkey(), &[&payer])
        .await?;

    // 5. Assert state is Initialized again with all extensions preserved
    let account_data_thawed = context.rpc.get_account(account_pubkey).await?.unwrap();
    let ctoken_thawed = CToken::deserialize(&mut &account_data_thawed.data[..])
        .expect("Failed to deserialize CToken after thaw");

    let expected_thawed = CToken {
        mint: mint_pubkey.to_bytes().into(),
        owner: owner.pubkey().to_bytes().into(),
        amount: 0,
        delegate: None,
        state: AccountState::Initialized,
        is_native: None,
        delegated_amount: 0,
        close_authority: None,
        extensions: Some(vec![
            ExtensionStruct::Compressible(compression_info),
            ExtensionStruct::PausableAccount(PausableAccountExtension),
            ExtensionStruct::PermanentDelegateAccount(PermanentDelegateAccountExtension),
            ExtensionStruct::TransferFeeAccount(TransferFeeAccountExtension { withheld_amount: 0 }),
            ExtensionStruct::TransferHookAccount(TransferHookAccountExtension { transferring: 0 }),
        ]),
    };

    assert_eq!(
        ctoken_thawed, expected_thawed,
        "Thawed CToken should have state=Initialized with all 5 extensions preserved"
    );

    println!("Successfully tested freeze and thaw with Token-2022 extensions");
    Ok(())
}
