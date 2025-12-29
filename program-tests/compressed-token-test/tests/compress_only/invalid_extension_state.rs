//! Tests for invalid extension state on Token-2022 mints.
//!
//! These tests verify that token pool creation fails when:
//! - TransferFeeConfig has non-zero fees
//! - TransferHook has non-nil program_id

use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use light_ctoken_interface::find_spl_interface_pda_with_index;
use light_ctoken_sdk::constants::CPI_AUTHORITY_PDA;
use light_program_test::{
    program_test::LightProgramTest, utils::assert::assert_rpc_error, ProgramTestConfig, Rpc,
};
use serial_test::serial;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, signature::Keypair, signer::Signer};
use spl_token_2022::{
    extension::{
        transfer_fee::instruction::initialize_transfer_fee_config,
        transfer_hook::instruction::initialize as initialize_transfer_hook, ExtensionType,
    },
    instruction::initialize_mint,
    state::Mint,
};

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
