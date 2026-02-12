//! Tests for invalid extension state on Token-2022 mints.
//!
//! These tests verify:
//! 1. Token pool creation FAILS when extension state is invalid
//! 2. Bypass operations SUCCEED even with invalid extension state:
//!    - CompressAndClose: Light Token → CompressedOnly
//!    - Decompress: CompressedOnly → Light Token
//!    - Light Token→SPL: Transfer from Light Token to SPL account

use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use light_client::indexer::Indexer;
use light_compressed_account::instruction_data::compressed_proof::ValidityProof;
use light_compressed_token_sdk::{
    compressed_token::{
        transfer2::{
            create_transfer2_instruction, Transfer2AccountsMetaConfig, Transfer2Config,
            Transfer2Inputs,
        },
        CTokenAccount2,
    },
    constants::CPI_AUTHORITY_PDA,
    spl_interface::find_spl_interface_pda_with_index as sdk_find_spl_interface_pda,
};
use light_program_test::{
    program_test::{LightProgramTest, TestRpc},
    utils::assert::assert_rpc_error,
    ProgramTestConfig, Rpc,
};
use light_test_utils::{
    actions::legacy::instructions::transfer2::{
        create_generic_transfer2_instruction, DecompressInput, Transfer2InstructionType,
    },
    mint_2022::{
        create_token_22_account, mint_spl_tokens_22, pause_mint, set_mint_transfer_fee,
        set_mint_transfer_hook,
    },
};
use light_token::instruction::{
    CompressibleParams, CreateTokenAccount, TransferFromSpl, TransferToSpl,
};
use light_token_interface::{
    find_spl_interface_pda_with_index,
    instructions::{
        extensions::{CompressedOnlyExtensionInstructionData, ExtensionInstructionData},
        transfer2::{Compression, MultiTokenTransferOutputData},
    },
    state::TokenDataVersion,
};
use serial_test::serial;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};
use spl_token_2022::{
    extension::{
        transfer_fee::instruction::initialize_transfer_fee_config,
        transfer_hook::instruction::initialize as initialize_transfer_hook, ExtensionType,
    },
    instruction::initialize_mint,
    state::Mint,
};

use super::shared::{setup_extensions_test, ExtensionsTestContext};

/// Expected error code for MintPaused
const MINT_PAUSED: u32 = 6127;

/// Expected error code for NonZeroTransferFeeNotSupported
const NON_ZERO_TRANSFER_FEE_NOT_SUPPORTED: u32 = 6129;

/// Expected error code for TransferHookNotSupported
const TRANSFER_HOOK_NOT_SUPPORTED: u32 = 6130;

/// Create a mint with non-zero transfer fee
async fn create_mint_with_non_zero_fee(rpc: &mut LightProgramTest, payer: &Keypair) -> Pubkey {
    use solana_system_interface::instruction as system_instruction;

    let mint_keypair = Keypair::new();
    let mint_pubkey = mint_keypair.pubkey();
    let authority = payer.pubkey();

    let extensions = [ExtensionType::TransferFeeConfig];
    let mint_len = ExtensionType::try_calculate_account_len::<Mint>(&extensions).unwrap();

    let rent = rpc
        .get_minimum_balance_for_rent_exemption(mint_len)
        .await
        .unwrap();

    // Create account
    let create_account_ix = system_instruction::create_account(
        &authority,
        &mint_pubkey,
        rent,
        mint_len as u64,
        &spl_token_2022::ID,
    );

    // Initialize transfer fee with NON-ZERO values
    let init_transfer_fee_ix = initialize_transfer_fee_config(
        &spl_token_2022::ID,
        &mint_pubkey,
        Some(&authority),
        Some(&authority),
        100,  // Non-zero transfer_fee_basis_points
        1000, // Non-zero maximum_fee
    )
    .unwrap();

    // Initialize mint
    let init_mint_ix = initialize_mint(
        &spl_token_2022::ID,
        &mint_pubkey,
        &authority,
        Some(&authority),
        9,
    )
    .unwrap();

    rpc.create_and_send_transaction(
        &[create_account_ix, init_transfer_fee_ix, init_mint_ix],
        &payer.pubkey(),
        &[payer, &mint_keypair],
    )
    .await
    .unwrap();

    mint_pubkey
}

/// Create a mint with non-nil transfer hook program
async fn create_mint_with_non_nil_hook(rpc: &mut LightProgramTest, payer: &Keypair) -> Pubkey {
    use solana_system_interface::instruction as system_instruction;

    let mint_keypair = Keypair::new();
    let mint_pubkey = mint_keypair.pubkey();
    let authority = payer.pubkey();

    let extensions = [ExtensionType::TransferHook];
    let mint_len = ExtensionType::try_calculate_account_len::<Mint>(&extensions).unwrap();

    let rent = rpc
        .get_minimum_balance_for_rent_exemption(mint_len)
        .await
        .unwrap();

    // Create account
    let create_account_ix = system_instruction::create_account(
        &authority,
        &mint_pubkey,
        rent,
        mint_len as u64,
        &spl_token_2022::ID,
    );

    // Initialize transfer hook with NON-NIL program_id
    // Use a dummy program id (not nil/zero)
    let dummy_hook_program = Pubkey::new_unique();
    let init_transfer_hook_ix = initialize_transfer_hook(
        &spl_token_2022::ID,
        &mint_pubkey,
        Some(authority),
        Some(dummy_hook_program), // Non-nil program_id
    )
    .unwrap();

    // Initialize mint
    let init_mint_ix = initialize_mint(
        &spl_token_2022::ID,
        &mint_pubkey,
        &authority,
        Some(&authority),
        9,
    )
    .unwrap();

    rpc.create_and_send_transaction(
        &[create_account_ix, init_transfer_hook_ix, init_mint_ix],
        &payer.pubkey(),
        &[payer, &mint_keypair],
    )
    .await
    .unwrap();

    mint_pubkey
}

/// Helper to create a token pool instruction
fn create_token_pool_instruction(payer: Pubkey, mint: Pubkey, restricted: bool) -> Instruction {
    let (token_pool_pda, _) = find_spl_interface_pda_with_index(&mint, 0, restricted);

    let instruction_data = light_compressed_token::instruction::CreateTokenPool {};
    let accounts = light_compressed_token::accounts::CreateTokenPoolInstruction {
        fee_payer: payer,
        token_pool_pda,
        system_program: system_program::ID,
        mint,
        token_program: spl_token_2022::ID,
        cpi_authority_pda: CPI_AUTHORITY_PDA,
    };

    Instruction {
        program_id: light_compressed_token::ID,
        accounts: accounts.to_account_metas(Some(true)),
        data: instruction_data.data(),
    }
}

#[tokio::test]
#[serial]
async fn test_transfer_fee_not_zero() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create mint with non-zero transfer fee
    let mint_pubkey = create_mint_with_non_zero_fee(&mut rpc, &payer).await;

    // Try to create token pool - should fail with NonZeroTransferFeeNotSupported
    // TransferFeeConfig is a restricted extension, so use restricted=true for PDA derivation
    let create_pool_ix = create_token_pool_instruction(payer.pubkey(), mint_pubkey, true);

    let result = rpc
        .create_and_send_transaction(&[create_pool_ix], &payer.pubkey(), &[&payer])
        .await;

    assert_rpc_error(result, 0, NON_ZERO_TRANSFER_FEE_NOT_SUPPORTED).unwrap();
}

#[tokio::test]
#[serial]
async fn test_transfer_hook_program_not_nil() {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None))
        .await
        .unwrap();
    let payer = rpc.get_payer().insecure_clone();

    // Create mint with non-nil hook program
    let mint_pubkey = create_mint_with_non_nil_hook(&mut rpc, &payer).await;

    // Try to create token pool - should fail with TransferHookNotSupported
    // TransferHook is a restricted extension, so use restricted=true for PDA derivation
    let create_pool_ix = create_token_pool_instruction(payer.pubkey(), mint_pubkey, true);

    let result = rpc
        .create_and_send_transaction(&[create_pool_ix], &payer.pubkey(), &[&payer])
        .await;

    assert_rpc_error(result, 0, TRANSFER_HOOK_NOT_SUPPORTED).unwrap();
}

// ============================================================================
// Bypass Tests: Operations that should SUCCEED with invalid extension state
//
// These tests verify that exiting compressed state bypasses extension checks:
// - CompressAndClose: Light Token → CompressedOnly
// - Decompress: CompressedOnly → Light Token
// - Light Token→SPL: Light Token account to SPL account
// ============================================================================

/// Helper: Create Light Token account with tokens and return context for bypass tests.
/// Uses zero-fee/nil-hook initially, then caller modifies state before testing.
async fn setup_ctoken_for_bypass_test(
    context: &mut ExtensionsTestContext,
) -> (Pubkey, Pubkey, Keypair, Keypair) {
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;

    // Create SPL source and mint tokens
    let spl_account =
        create_token_22_account(&mut context.rpc, &payer, &mint_pubkey, &payer.pubkey()).await;
    let mint_amount = 1_000_000_000u64;
    mint_spl_tokens_22(
        &mut context.rpc,
        &payer,
        &mint_pubkey,
        &spl_account,
        mint_amount,
    )
    .await;

    // Create owner and Light Token account with 0 prepaid epochs (immediately compressible)
    let owner = Keypair::new();
    let account_keypair = Keypair::new();
    let ctoken_account = account_keypair.pubkey();

    let create_ix =
        CreateTokenAccount::new(payer.pubkey(), ctoken_account, mint_pubkey, owner.pubkey())
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
                pre_pay_num_epochs: 0,
                lamports_per_write: Some(100),
                compress_to_account_pubkey: None,
                token_account_version: TokenDataVersion::ShaFlat,
                compression_only: true,
            })
            .instruction()
            .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[create_ix], &payer.pubkey(), &[&payer, &account_keypair])
        .await
        .unwrap();

    // Transfer SPL to Light Token using hot path
    let (spl_interface_pda, spl_interface_pda_bump) =
        sdk_find_spl_interface_pda(&mint_pubkey, 0, true);

    let transfer_ix = TransferFromSpl {
        amount: mint_amount,
        spl_interface_pda_bump,
        decimals: 9,
        source_spl_token_account: spl_account,
        destination: ctoken_account,
        authority: payer.pubkey(),
        mint: mint_pubkey,
        payer: payer.pubkey(),
        spl_interface_pda,
        spl_token_program: spl_token_2022::ID,
    }
    .instruction()
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &[&payer])
        .await
        .unwrap();

    (ctoken_account, spl_account, owner, account_keypair)
}

// ============================================================================
// Light Token→SPL Bypass Tests
// ============================================================================

/// Test that Light Token→SPL succeeds even with non-zero transfer fees.
/// This is a bypass operation because it's exiting compressed state.
#[tokio::test]
#[serial]
async fn test_ctoken_to_spl_bypasses_non_zero_fee() {
    let mut context = setup_extensions_test(&[ExtensionType::TransferFeeConfig])
        .await
        .unwrap();
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;

    // Setup Light Token with tokens (while extension state is valid)
    let (ctoken_account, _spl_source, owner, _) = setup_ctoken_for_bypass_test(&mut context).await;

    // Create destination SPL account
    let spl_dest =
        create_token_22_account(&mut context.rpc, &payer, &mint_pubkey, &payer.pubkey()).await;

    // Set non-zero transfer fees AFTER funding
    set_mint_transfer_fee(&mut context.rpc, &mint_pubkey, 100, 1000).await;

    // Light Token→SPL should SUCCEED (bypass)
    let (spl_interface_pda, spl_interface_pda_bump) =
        sdk_find_spl_interface_pda(&mint_pubkey, 0, true);

    let transfer_ix = TransferToSpl {
        source: ctoken_account,
        destination_spl_token_account: spl_dest,
        amount: 100_000_000,
        authority: owner.pubkey(),
        mint: mint_pubkey,
        payer: payer.pubkey(),
        spl_interface_pda,
        spl_interface_pda_bump,
        decimals: 9,
        spl_token_program: spl_token_2022::ID,
    }
    .instruction()
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &[&payer, &owner])
        .await
        .unwrap();

    println!("Light Token→SPL bypassed non-zero transfer fee check");
}

// Note: test_ctoken_to_spl_bypasses_non_nil_hook was removed because SPL Token-2022
// requires the transfer hook program to be present when doing transfers.
// The bypass only affects the compressed token program's internal checks,
// not SPL Token-2022's hook enforcement during the actual token transfer.

// ============================================================================
// CompressAndClose Bypass Tests
// ============================================================================

/// Test that CompressAndClose succeeds even with non-zero transfer fees.
/// This is a bypass operation because it preserves state in CompressedOnly.
#[tokio::test]
#[serial]
async fn test_compress_and_close_bypasses_non_zero_fee() {
    let mut context = setup_extensions_test(&[ExtensionType::TransferFeeConfig])
        .await
        .unwrap();
    let mint_pubkey = context.mint_pubkey;
    let owner = Keypair::new();

    // Setup Light Token with tokens
    let (ctoken_account, _spl_source, ctoken_owner, _) =
        setup_ctoken_for_bypass_test(&mut context).await;
    let _ = owner; // Use the owner from setup
    let owner = ctoken_owner;

    // Set non-zero transfer fees AFTER funding
    set_mint_transfer_fee(&mut context.rpc, &mint_pubkey, 100, 1000).await;

    // Warp epoch to trigger forester compression
    context.rpc.warp_epoch_forward(30).await.unwrap();

    // Assert the account has been compressed (closed)
    let account_after = context.rpc.get_account(ctoken_account).await.unwrap();
    assert!(
        account_after.is_none() || account_after.unwrap().lamports == 0,
        "Light Token account should be closed after compression"
    );

    // Get compressed accounts and verify
    let compressed_accounts = context
        .rpc
        .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(
        compressed_accounts.len(),
        1,
        "Should have exactly 1 compressed token account"
    );

    println!("CompressAndClose bypassed non-zero transfer fee check");
}

/// Test that CompressAndClose succeeds even with non-nil transfer hook.
/// This is a bypass operation because it preserves state in CompressedOnly.
#[tokio::test]
#[serial]
async fn test_compress_and_close_bypasses_non_nil_hook() {
    let mut context = setup_extensions_test(&[ExtensionType::TransferHook])
        .await
        .unwrap();
    let mint_pubkey = context.mint_pubkey;

    // Setup Light Token with tokens
    let (ctoken_account, _spl_source, owner, _) = setup_ctoken_for_bypass_test(&mut context).await;

    // Set non-nil transfer hook AFTER funding
    let dummy_hook_program = Pubkey::new_unique();
    set_mint_transfer_hook(&mut context.rpc, &mint_pubkey, dummy_hook_program).await;

    // Warp epoch to trigger forester compression
    context.rpc.warp_epoch_forward(30).await.unwrap();

    // Assert the account has been compressed (closed)
    let account_after = context.rpc.get_account(ctoken_account).await.unwrap();
    assert!(
        account_after.is_none() || account_after.unwrap().lamports == 0,
        "Light Token account should be closed after compression"
    );

    // Get compressed accounts and verify
    let compressed_accounts = context
        .rpc
        .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;

    assert_eq!(
        compressed_accounts.len(),
        1,
        "Should have exactly 1 compressed token account"
    );

    println!("CompressAndClose bypassed non-nil transfer hook check");
}

// ============================================================================
// Decompress Bypass Tests
// ============================================================================

/// Test that Decompress succeeds even with non-zero transfer fees.
/// This is a bypass operation because it restores existing compressed state.
#[tokio::test]
#[serial]
async fn test_decompress_bypasses_non_zero_fee() {
    let mut context = setup_extensions_test(&[ExtensionType::TransferFeeConfig])
        .await
        .unwrap();
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;

    // Setup Light Token with tokens
    let (ctoken_account, _spl_source, owner, _) = setup_ctoken_for_bypass_test(&mut context).await;
    let mint_amount = 1_000_000_000u64;

    // Warp epoch to compress (while extension state is valid)
    context.rpc.warp_epoch_forward(30).await.unwrap();

    // Verify compressed
    let account_after = context.rpc.get_account(ctoken_account).await.unwrap();
    assert!(account_after.is_none() || account_after.unwrap().lamports == 0);

    // Get compressed account
    let compressed_accounts = context
        .rpc
        .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;
    assert_eq!(compressed_accounts.len(), 1);

    // Set non-zero transfer fees AFTER compression
    set_mint_transfer_fee(&mut context.rpc, &mint_pubkey, 100, 1000).await;

    // Create destination Light Token for decompress
    let dest_keypair = Keypair::new();
    let dest_account = dest_keypair.pubkey();

    let create_dest_ix =
        CreateTokenAccount::new(payer.pubkey(), dest_account, mint_pubkey, owner.pubkey())
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
            .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[create_dest_ix], &payer.pubkey(), &[&payer, &dest_keypair])
        .await
        .unwrap();

    // Decompress - should SUCCEED (bypass)
    let in_tlv = vec![vec![ExtensionInstructionData::CompressedOnly(
        CompressedOnlyExtensionInstructionData {
            delegated_amount: 0,
            withheld_transfer_fee: 0,
            is_frozen: false,
            compression_index: 0,
            is_ata: false,
            bump: 0,
            owner_index: 0,
        },
    )]];

    let decompress_ix = create_generic_transfer2_instruction(
        &mut context.rpc,
        vec![Transfer2InstructionType::Decompress(DecompressInput {
            compressed_token_account: vec![compressed_accounts[0].clone()],
            decompress_amount: mint_amount,
            solana_token_account: dest_account,
            amount: mint_amount,
            pool_index: None,
            decimals: 9,
            in_tlv: Some(in_tlv),
        })],
        payer.pubkey(),
        true,
    )
    .await
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[decompress_ix], &payer.pubkey(), &[&payer, &owner])
        .await
        .unwrap();

    println!("Decompress bypassed non-zero transfer fee check");
}

/// Test that Decompress succeeds even with non-nil transfer hook.
/// This is a bypass operation because it restores existing compressed state.
#[tokio::test]
#[serial]
async fn test_decompress_bypasses_non_nil_hook() {
    let mut context = setup_extensions_test(&[ExtensionType::TransferHook])
        .await
        .unwrap();
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;

    // Setup Light Token with tokens
    let (ctoken_account, _spl_source, owner, _) = setup_ctoken_for_bypass_test(&mut context).await;
    let mint_amount = 1_000_000_000u64;

    // Warp epoch to compress (while extension state is valid)
    context.rpc.warp_epoch_forward(30).await.unwrap();

    // Verify compressed
    let account_after = context.rpc.get_account(ctoken_account).await.unwrap();
    assert!(account_after.is_none() || account_after.unwrap().lamports == 0);

    // Get compressed account
    let compressed_accounts = context
        .rpc
        .get_compressed_token_accounts_by_owner(&owner.pubkey(), None, None)
        .await
        .unwrap()
        .value
        .items;
    assert_eq!(compressed_accounts.len(), 1);

    // Set non-nil transfer hook AFTER compression
    let dummy_hook_program = Pubkey::new_unique();
    set_mint_transfer_hook(&mut context.rpc, &mint_pubkey, dummy_hook_program).await;

    // Create destination Light Token for decompress
    let dest_keypair = Keypair::new();
    let dest_account = dest_keypair.pubkey();

    let create_dest_ix =
        CreateTokenAccount::new(payer.pubkey(), dest_account, mint_pubkey, owner.pubkey())
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
            .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[create_dest_ix], &payer.pubkey(), &[&payer, &dest_keypair])
        .await
        .unwrap();

    // Decompress - should SUCCEED (bypass)
    let in_tlv = vec![vec![ExtensionInstructionData::CompressedOnly(
        CompressedOnlyExtensionInstructionData {
            delegated_amount: 0,
            withheld_transfer_fee: 0,
            is_frozen: false,
            compression_index: 0,
            is_ata: false,
            bump: 0,
            owner_index: 0,
        },
    )]];

    let decompress_ix = create_generic_transfer2_instruction(
        &mut context.rpc,
        vec![Transfer2InstructionType::Decompress(DecompressInput {
            compressed_token_account: vec![compressed_accounts[0].clone()],
            decompress_amount: mint_amount,
            solana_token_account: dest_account,
            amount: mint_amount,
            pool_index: None,
            decimals: 9,
            in_tlv: Some(in_tlv),
        })],
        payer.pubkey(),
        true,
    )
    .await
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[decompress_ix], &payer.pubkey(), &[&payer, &owner])
        .await
        .unwrap();

    println!("Decompress bypassed non-nil transfer hook check");
}

// ============================================================================
// cToken-to-cToken Blocking Tests
//
// These tests verify that cToken-to-cToken transfers (Compress from cToken A +
// Decompress to cToken B with no compressed accounts) are BLOCKED when
// extension state is invalid (non-zero fees, paused, non-nil hook).
// ============================================================================

/// Build a cToken-to-cToken transfer instruction.
///
/// This constructs a transfer2 instruction with:
/// - Compress from source cToken (subtract tokens)
/// - Decompress to destination cToken (add tokens)
/// - No compressed accounts in either direction (hot path)
fn create_ctoken_to_ctoken_instruction(
    payer: Pubkey,
    source_ctoken: Pubkey,
    dest_ctoken: Pubkey,
    authority: Pubkey,
    mint: Pubkey,
    amount: u64,
) -> Instruction {
    let packed_accounts = vec![
        // Mint (index 0)
        AccountMeta::new_readonly(mint, false),
        // Source ctoken account (index 1) - writable
        AccountMeta::new(source_ctoken, false),
        // Authority for compression (index 2) - signer
        AccountMeta::new_readonly(authority, true),
        // Destination ctoken account (index 3) - writable
        AccountMeta::new(dest_ctoken, false),
        // System program (index 4) - needed for compressible account lamport top-ups
        AccountMeta::new_readonly(Pubkey::default(), false),
    ];

    let compress_from_source = CTokenAccount2 {
        inputs: vec![],
        output: MultiTokenTransferOutputData::default(),
        compression: Some(Compression::compress(
            amount, 0, // mint index
            1, // source ctoken index
            2, // authority index
        )),
        delegate_is_set: false,
        method_used: true,
    };

    let decompress_to_dest = CTokenAccount2 {
        inputs: vec![],
        output: MultiTokenTransferOutputData::default(),
        compression: Some(Compression::decompress(
            amount, 0, // mint index
            3, // destination ctoken index
        )),
        delegate_is_set: false,
        method_used: true,
    };

    let inputs = Transfer2Inputs {
        validity_proof: ValidityProof::new(None),
        transfer_config: Transfer2Config::default().filter_zero_amount_outputs(),
        meta_config: Transfer2AccountsMetaConfig::new_decompressed_accounts_only(
            payer,
            packed_accounts,
        ),
        in_lamports: None,
        out_lamports: None,
        token_accounts: vec![compress_from_source, decompress_to_dest],
        output_queue: 0,
        in_tlv: None,
    };

    create_transfer2_instruction(inputs).unwrap()
}

/// Helper: Set up source cToken (with tokens) and an empty destination cToken for
/// cToken-to-cToken transfer tests. Extension state is still valid at this point.
/// Returns (context, source_ctoken, dest_ctoken, owner).
async fn setup_ctoken_to_ctoken_test(
    extensions: &[ExtensionType],
) -> (ExtensionsTestContext, Pubkey, Pubkey, Keypair) {
    let mut context = setup_extensions_test(extensions).await.unwrap();
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;

    let (source_ctoken, _spl_source, owner, _) = setup_ctoken_for_bypass_test(&mut context).await;

    let dest_keypair = Keypair::new();
    let dest_ctoken = dest_keypair.pubkey();
    let create_dest_ix =
        CreateTokenAccount::new(payer.pubkey(), dest_ctoken, mint_pubkey, owner.pubkey())
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
            .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[create_dest_ix], &payer.pubkey(), &[&payer, &dest_keypair])
        .await
        .unwrap();

    (context, source_ctoken, dest_ctoken, owner)
}

/// Test that cToken-to-cToken transfer is blocked when the mint has non-zero transfer fees.
#[tokio::test]
#[serial]
async fn test_ctoken_to_ctoken_blocked_by_non_zero_fee() {
    let (mut context, source_ctoken, dest_ctoken, owner) =
        setup_ctoken_to_ctoken_test(&[ExtensionType::TransferFeeConfig]).await;
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;

    set_mint_transfer_fee(&mut context.rpc, &mint_pubkey, 100, 1000).await;

    let transfer_ix = create_ctoken_to_ctoken_instruction(
        payer.pubkey(),
        source_ctoken,
        dest_ctoken,
        owner.pubkey(),
        mint_pubkey,
        100_000_000,
    );

    let result = context
        .rpc
        .create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    assert_rpc_error(result, 0, NON_ZERO_TRANSFER_FEE_NOT_SUPPORTED).unwrap();
}

/// Test that cToken-to-cToken transfer is blocked when the mint is paused.
#[tokio::test]
#[serial]
async fn test_ctoken_to_ctoken_blocked_by_pause() {
    let (mut context, source_ctoken, dest_ctoken, owner) =
        setup_ctoken_to_ctoken_test(&[ExtensionType::Pausable]).await;
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;

    pause_mint(&mut context.rpc, &mint_pubkey).await;

    let transfer_ix = create_ctoken_to_ctoken_instruction(
        payer.pubkey(),
        source_ctoken,
        dest_ctoken,
        owner.pubkey(),
        mint_pubkey,
        100_000_000,
    );

    let result = context
        .rpc
        .create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    assert_rpc_error(result, 0, MINT_PAUSED).unwrap();
}

/// Test that cToken-to-cToken transfer is blocked when the mint has a non-nil transfer hook.
#[tokio::test]
#[serial]
async fn test_ctoken_to_ctoken_blocked_by_non_nil_hook() {
    let (mut context, source_ctoken, dest_ctoken, owner) =
        setup_ctoken_to_ctoken_test(&[ExtensionType::TransferHook]).await;
    let payer = context.payer.insecure_clone();
    let mint_pubkey = context.mint_pubkey;

    let dummy_hook_program = Pubkey::new_unique();
    set_mint_transfer_hook(&mut context.rpc, &mint_pubkey, dummy_hook_program).await;

    let transfer_ix = create_ctoken_to_ctoken_instruction(
        payer.pubkey(),
        source_ctoken,
        dest_ctoken,
        owner.pubkey(),
        mint_pubkey,
        100_000_000,
    );

    let result = context
        .rpc
        .create_and_send_transaction(&[transfer_ix], &payer.pubkey(), &[&payer, &owner])
        .await;

    assert_rpc_error(result, 0, TRANSFER_HOOK_NOT_SUPPORTED).unwrap();
}

