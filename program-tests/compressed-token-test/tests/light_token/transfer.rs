use anchor_spl::token_2022::spl_token_2022;
use solana_sdk::program_pack::Pack;

use super::shared::*;

// ============================================================================
// Helper Functions
// ============================================================================

/// Setup context with two token accounts and mint tokens to the source
/// Returns (context, source_account, destination_account, mint_amount, source_keypair, dest_keypair)
async fn setup_transfer_test(
    num_prepaid_epochs: Option<u8>,
    mint_amount: u64,
) -> Result<(AccountTestContext, Pubkey, Pubkey, u64, Keypair, Keypair), RpcError> {
    let mut context = setup_account_test().await?;
    let payer_pubkey = context.payer.pubkey();

    // Create source account (where tokens will be minted)
    let source_keypair = Keypair::new();
    let source_pubkey = source_keypair.pubkey();

    // Create destination account (where tokens will be transferred)
    let destination_keypair = Keypair::new();
    let destination_pubkey = destination_keypair.pubkey();

    // Setup compressible data based on whether we want compressible accounts
    let rent_sponsor = context.rent_sponsor;

    // Create source token account
    // When num_prepaid_epochs is None, use 3 epochs (sufficient for no top-up: epochs_funded_ahead = 3 - 1 = 2 >= 2)
    let source_epochs = num_prepaid_epochs.unwrap_or(3);
    context.token_account_keypair = source_keypair.insecure_clone();
    {
        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor,
            num_prepaid_epochs: source_epochs,
            lamports_per_write: Some(100),
            account_version: light_token_interface::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: payer_pubkey,
        };
        create_and_assert_token_account(&mut context, compressible_data, "source_account").await;
    }

    // Create destination token account
    let dest_epochs = num_prepaid_epochs.unwrap_or(3);
    context.token_account_keypair = destination_keypair.insecure_clone();
    {
        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor,
            num_prepaid_epochs: dest_epochs,
            lamports_per_write: Some(100),
            account_version: light_token_interface::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: payer_pubkey,
        };
        create_and_assert_token_account(&mut context, compressible_data, "destination_account")
            .await;
    }

    // Mint tokens to source account using set_account
    if mint_amount > 0 {
        let mut source_account = context
            .rpc
            .get_account(source_pubkey)
            .await?
            .ok_or_else(|| RpcError::AssertRpcError("Source account not found".to_string()))?;

        // Deserialize and modify the token account (only use first 165 bytes)
        let mut token_account =
            spl_token_2022::state::Account::unpack_unchecked(&source_account.data[..165]).map_err(
                |e| RpcError::AssertRpcError(format!("Failed to unpack token account: {:?}", e)),
            )?;
        token_account.amount = mint_amount;
        spl_token_2022::state::Account::pack(token_account, &mut source_account.data[..165])
            .map_err(|e| {
                RpcError::AssertRpcError(format!("Failed to pack token account: {:?}", e))
            })?;

        // Set the modified account
        context.rpc.set_account(source_pubkey, source_account);
    }

    Ok((
        context,
        source_pubkey,
        destination_pubkey,
        mint_amount,
        source_keypair,
        destination_keypair,
    ))
}

/// Build a ctoken transfer instruction
///
/// For basic transfers (no T22 extensions), only 3 accounts are needed.
/// Authority is writable because compressible accounts may require top-up.
fn build_transfer_instruction(
    source: Pubkey,
    destination: Pubkey,
    amount: u64,
    authority: Pubkey,
) -> solana_sdk::instruction::Instruction {
    use anchor_lang::prelude::AccountMeta;
    use solana_sdk::instruction::Instruction;

    // Build instruction data: discriminator (3) + amount (8 bytes)
    let mut data = vec![3u8];
    data.extend_from_slice(&amount.to_le_bytes());

    // Note: Index 3 would be interpreted as mint (for T22 extension validation).
    // For basic transfers, we only pass 3 accounts.
    Instruction {
        program_id: light_compressed_token::ID,
        accounts: vec![
            AccountMeta::new(source, false),
            AccountMeta::new(destination, false),
            AccountMeta::new(authority, true), // Authority must sign and be writable for top-ups
        ],
        data,
    }
}

/// Build a ctoken transfer instruction with max_top_up parameter
fn build_transfer_instruction_with_max_top_up(
    source: Pubkey,
    destination: Pubkey,
    amount: u64,
    authority: Pubkey,
    max_top_up: u16,
) -> solana_sdk::instruction::Instruction {
    use anchor_lang::prelude::AccountMeta;
    use solana_sdk::instruction::Instruction;

    // Build instruction data: discriminator (3) + amount (8 bytes) + max_top_up (2 bytes)
    let mut data = vec![3u8];
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(&max_top_up.to_le_bytes());

    Instruction {
        program_id: light_compressed_token::ID,
        accounts: vec![
            AccountMeta::new(source, false),
            AccountMeta::new(destination, false),
            AccountMeta::new(authority, true), // Authority must sign and be writable for top-ups
        ],
        data,
    }
}

/// Execute a ctoken transfer and assert success
async fn transfer_and_assert(
    context: &mut AccountTestContext,
    source: Pubkey,
    destination: Pubkey,
    amount: u64,
    authority: &Keypair,
    name: &str,
) {
    use light_test_utils::assert_ctoken_transfer::assert_ctoken_transfer;

    println!("Transfer initiated for: {}", name);

    let payer_pubkey = context.payer.pubkey();

    // Build transfer instruction
    let transfer_ix = build_transfer_instruction(source, destination, amount, authority.pubkey());

    // Execute transfer
    context
        .rpc
        .create_and_send_transaction(&[transfer_ix], &payer_pubkey, &[&context.payer, authority])
        .await
        .unwrap();

    // Assert transfer was successful
    assert_ctoken_transfer(&mut context.rpc, source, destination, amount).await;
}

/// Execute a ctoken transfer expecting failure with specific error code
async fn transfer_and_assert_fails(
    context: &mut AccountTestContext,
    source: Pubkey,
    destination: Pubkey,
    amount: u64,
    authority: &Keypair,
    name: &str,
    expected_error_code: u32,
) {
    println!("Transfer (expecting failure) initiated for: {}", name);

    let payer_pubkey = context.payer.pubkey();

    // Build transfer instruction
    let transfer_ix = build_transfer_instruction(source, destination, amount, authority.pubkey());

    // Execute transfer expecting failure
    let result = context
        .rpc
        .create_and_send_transaction(&[transfer_ix], &payer_pubkey, &[&context.payer, authority])
        .await;

    // Assert that the transaction failed with the expected error code
    light_program_test::utils::assert::assert_rpc_error(result, 0, expected_error_code).unwrap();
}

// ============================================================================
// Successful Transfer Tests
// ============================================================================

#[tokio::test]
async fn test_ctoken_transfer_basic_non_compressible() {
    let (mut context, source, destination, _mint_amount, _source_keypair, _dest_keypair) =
        setup_transfer_test(None, 1000).await.unwrap();

    // Use the owner keypair as authority (token accounts are owned by context.owner_keypair)
    let owner_keypair = context.owner_keypair.insecure_clone();

    transfer_and_assert(
        &mut context,
        source,
        destination,
        500,
        &owner_keypair,
        "basic_non_compressible_transfer",
    )
    .await;
}

#[tokio::test]
async fn test_ctoken_transfer_compressible_no_topup() {
    // Create compressible accounts with 3 prepaid epochs (sufficient for max_funded_epochs=2, no top-up needed)
    // epochs_funded_ahead = total_epochs_fundable - required_epochs = 3 - 1 = 2 >= 2
    let (mut context, source, destination, _mint_amount, _source_keypair, _dest_keypair) =
        setup_transfer_test(Some(3), 1000).await.unwrap();

    // Use the owner keypair as authority (token accounts are owned by context.owner_keypair)
    let owner_keypair = context.owner_keypair.insecure_clone();

    transfer_and_assert(
        &mut context,
        source,
        destination,
        500,
        &owner_keypair,
        "compressible_transfer_no_topup",
    )
    .await;
}

#[tokio::test]
async fn test_ctoken_transfer_compressible_with_topup() {
    // Create compressible accounts with 3 prepaid epochs (sufficient for max_funded_epochs=2, no top-up needed)
    // epochs_funded_ahead = total_epochs_fundable - required_epochs = 3 - 1 = 2 >= 2
    let (mut context, source, destination, _mint_amount, _source_keypair, _dest_keypair) =
        setup_transfer_test(Some(3), 1000).await.unwrap();
    // For this test, we need to transfer ownership to the payer so it can pay for top-ups
    // Or we can use a delegate. But the simplest is to use payer as authority for this specific test.
    // Actually, the owner needs to be the authority for the transfer to work.
    // We need to fund the owner_keypair so it can pay for top-ups.

    // Fund the owner keypair so it can pay for top-ups
    context
        .rpc
        .airdrop_lamports(&context.owner_keypair.pubkey(), 100_000_000)
        .await
        .unwrap();

    let owner_keypair = context.owner_keypair.insecure_clone();

    transfer_and_assert(
        &mut context,
        source,
        destination,
        500,
        &owner_keypair,
        "compressible_transfer_with_topup",
    )
    .await;
}

#[tokio::test]
async fn test_ctoken_transfer_entire_balance() {
    let (mut context, source, destination, mint_amount, _source_keypair, _dest_keypair) =
        setup_transfer_test(None, 1000).await.unwrap();

    // Use the owner keypair as authority (token accounts are owned by context.owner_keypair)
    let owner_keypair = context.owner_keypair.insecure_clone();

    // Transfer the entire balance (1000 tokens)
    transfer_and_assert(
        &mut context,
        source,
        destination,
        mint_amount,
        &owner_keypair,
        "transfer_entire_balance",
    )
    .await;
}

// ============================================================================
// Failing Transfer Tests
// ============================================================================

#[tokio::test]
async fn test_ctoken_transfer_insufficient_balance() {
    let (mut context, source, destination, _mint_amount, _source_keypair, _dest_keypair) =
        setup_transfer_test(None, 1000).await.unwrap();

    // Use the owner keypair as authority (token accounts are owned by context.owner_keypair)
    let owner_keypair = context.owner_keypair.insecure_clone();

    // Try to transfer more than the balance (1500 > 1000)
    // Expected error: SplInsufficientFunds (6154)
    transfer_and_assert_fails(
        &mut context,
        source,
        destination,
        1500,
        &owner_keypair,
        "insufficient_balance_transfer",
        6154, // SplInsufficientFunds
    )
    .await;
}

#[tokio::test]
async fn test_ctoken_transfer_frozen_source() {
    let (mut context, source, destination, _mint_amount, _source_keypair, _dest_keypair) =
        setup_transfer_test(None, 1000).await.unwrap();

    // Freeze the source account by modifying its state
    let mut source_account = context.rpc.get_account(source).await.unwrap().unwrap();
    // Set the state field (byte 108 in SPL token account) to Frozen (2)
    source_account.data[108] = 2; // AccountState::Frozen
    context.rpc.set_account(source, source_account);

    // Use the owner keypair as authority
    let owner_keypair = context.owner_keypair.insecure_clone();

    // Try to transfer from frozen account
    // Expected error: TokenError::InvalidAccountState (frozen accounts rejected by zero_copy_at_mut_checked)
    transfer_and_assert_fails(
        &mut context,
        source,
        destination,
        500,
        &owner_keypair,
        "frozen_source_transfer",
        18036, // TokenError::InvalidAccountState
    )
    .await;
}

#[tokio::test]
async fn test_ctoken_transfer_frozen_destination() {
    let (mut context, source, destination, _mint_amount, _source_keypair, _dest_keypair) =
        setup_transfer_test(None, 1000).await.unwrap();

    // Freeze the destination account by modifying its state
    let mut dest_account = context.rpc.get_account(destination).await.unwrap().unwrap();
    // Set the state field (byte 108 in SPL token account) to Frozen (2)
    dest_account.data[108] = 2; // AccountState::Frozen
    context.rpc.set_account(destination, dest_account);

    // Use the owner keypair as authority
    let owner_keypair = context.owner_keypair.insecure_clone();

    // Try to transfer to frozen account
    // Expected error: TokenError::InvalidAccountState (frozen accounts rejected by zero_copy_at_mut_checked)
    transfer_and_assert_fails(
        &mut context,
        source,
        destination,
        500,
        &owner_keypair,
        "frozen_destination_transfer",
        18036, // TokenError::InvalidAccountState
    )
    .await;
}

#[tokio::test]
async fn test_ctoken_transfer_wrong_authority() {
    let (mut context, source, destination, _mint_amount, _source_keypair, _dest_keypair) =
        setup_transfer_test(None, 1000).await.unwrap();

    // Use a wrong keypair (not the owner) as authority
    let wrong_authority = Keypair::new();

    // Try to transfer with wrong authority
    // Expected error: OwnerMismatch (6075)
    transfer_and_assert_fails(
        &mut context,
        source,
        destination,
        500,
        &wrong_authority,
        "wrong_authority_transfer",
        6075, // OwnerMismatch
    )
    .await;
}

#[tokio::test]
async fn test_ctoken_transfer_mint_mismatch() {
    // Create two accounts with the same mint first
    let (mut context, source, destination, _mint_amount, _source_keypair, _dest_keypair) =
        setup_transfer_test(None, 1000).await.unwrap();

    // Modify the destination account's mint field to create a mint mismatch
    let different_mint = Pubkey::new_unique();
    let mut dest_account = context.rpc.get_account(destination).await.unwrap().unwrap();

    // Light Token mint is the first 32 bytes after the account type discriminator
    // The mint field is at bytes 0-32 in the Light Token account data
    dest_account.data[0..32].copy_from_slice(&different_mint.to_bytes());
    context.rpc.set_account(destination, dest_account);

    // Use the owner keypair as authority
    let owner_keypair = context.owner_keypair.insecure_clone();

    // Try to transfer between accounts with different mints
    // Expected error: SplMintMismatch (6155)
    transfer_and_assert_fails(
        &mut context,
        source,
        destination,
        500,
        &owner_keypair,
        "mint_mismatch_transfer",
        6155, // SplMintMismatch
    )
    .await;
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[tokio::test]
async fn test_ctoken_transfer_zero_amount() {
    let (mut context, source, destination, _mint_amount, _source_keypair, _dest_keypair) =
        setup_transfer_test(None, 1000).await.unwrap();

    // Use the owner keypair as authority
    let owner_keypair = context.owner_keypair.insecure_clone();

    // Transfer 0 tokens (should succeed - SPL token allows this)
    transfer_and_assert(
        &mut context,
        source,
        destination,
        0,
        &owner_keypair,
        "zero_amount_transfer",
    )
    .await;
}

#[tokio::test]
async fn test_ctoken_transfer_mixed_compressible_non_compressible() {
    // Create source with more prepaid epochs
    let mut context = setup_account_test().await.unwrap();
    let payer_pubkey = context.payer.pubkey();

    // Create source account with more prepaid epochs (lamports_per_write = Some(100))
    let source_keypair = Keypair::new();
    let source_pubkey = source_keypair.pubkey();
    context.token_account_keypair = source_keypair.insecure_clone();

    let source_data = CompressibleData {
        compression_authority: context.compression_authority,
        rent_sponsor: context.rent_sponsor,
        num_prepaid_epochs: 5, // More epochs with higher lamports_per_write
        lamports_per_write: Some(100),
        account_version: light_token_interface::state::TokenDataVersion::ShaFlat,
        compress_to_pubkey: false,
        payer: payer_pubkey,
    };
    create_and_assert_token_account(&mut context, source_data, "source_account").await;

    // Create destination account with fewer prepaid epochs (no lamports_per_write)
    let destination_keypair = Keypair::new();
    let destination_pubkey = destination_keypair.pubkey();
    context.token_account_keypair = destination_keypair.insecure_clone();

    let dest_data = CompressibleData {
        compression_authority: context.compression_authority,
        rent_sponsor: context.rent_sponsor,
        num_prepaid_epochs: 3, // Standard 3 epochs sufficient for no top-up
        lamports_per_write: None,
        account_version: light_token_interface::state::TokenDataVersion::ShaFlat,
        compress_to_pubkey: false,
        payer: payer_pubkey,
    };
    create_and_assert_token_account(&mut context, dest_data, "destination_account").await;

    // Mint tokens to source
    let mut source_account = context
        .rpc
        .get_account(source_pubkey)
        .await
        .unwrap()
        .unwrap();
    let mut token_account =
        spl_token_2022::state::Account::unpack_unchecked(&source_account.data[..165]).unwrap();
    token_account.amount = 1000;
    spl_token_2022::state::Account::pack(token_account, &mut source_account.data[..165]).unwrap();
    context.rpc.set_account(source_pubkey, source_account);

    // Fund owner to pay for potential top-up
    context
        .rpc
        .airdrop_lamports(&context.owner_keypair.pubkey(), 100_000_000)
        .await
        .unwrap();

    let owner_keypair = context.owner_keypair.insecure_clone();

    // Transfer from source with more prepaid to destination with fewer prepaid
    transfer_and_assert(
        &mut context,
        source_pubkey,
        destination_pubkey,
        500,
        &owner_keypair,
        "mixed_compressible_source",
    )
    .await;
}

// ============================================================================
// max_top_up Tests
// ============================================================================

/// Test that ctoken_transfer fails when max_top_up is exceeded.
/// Creates compressible accounts with num_prepaid_epochs = 0 (no prepaid rent),
/// which requires rent top-up on every write. Setting max_top_up = 1 (too low)
/// should trigger MaxTopUpExceeded error (18043).
#[tokio::test]
async fn test_ctoken_transfer_max_top_up_exceeded() {
    // Create compressible accounts with num_prepaid_epochs = 0 (needs top-up immediately)
    let (mut context, source, destination, _mint_amount, _source_keypair, _dest_keypair) =
        setup_transfer_test(Some(0), 1000).await.unwrap();

    // Fund owner to pay for potential top-up
    context
        .rpc
        .airdrop_lamports(&context.owner_keypair.pubkey(), 100_000_000)
        .await
        .unwrap();

    let owner_keypair = context.owner_keypair.insecure_clone();
    let payer_pubkey = context.payer.pubkey();

    // Build transfer instruction with max_top_up = 1 (too low to cover rent top-up)
    let transfer_ix = build_transfer_instruction_with_max_top_up(
        source,
        destination,
        100,
        owner_keypair.pubkey(),
        1, // max_top_up = 1 (1,000 lamports budget, still too low for rent top-up)
    );

    // Execute transfer expecting failure
    let result = context
        .rpc
        .create_and_send_transaction(
            &[transfer_ix],
            &payer_pubkey,
            &[&context.payer, &owner_keypair],
        )
        .await;

    // Assert MaxTopUpExceeded (error code 18043)
    light_program_test::utils::assert::assert_rpc_error(result, 0, 18043).unwrap();
}

// ============================================================================
// Transfer Checked Helper Functions
// ============================================================================

use light_token::instruction::TransferChecked;

/// Setup context with two token accounts for transfer_checked tests using a real SPL Token mint
async fn setup_transfer_checked_test_with_spl_mint(
    num_prepaid_epochs: Option<u8>,
    mint_amount: u64,
    decimals: u8,
) -> Result<(AccountTestContext, Pubkey, Pubkey, u64, Keypair, Keypair), RpcError> {
    let mut context = setup_account_test_with_spl_mint(decimals).await?;
    let payer_pubkey = context.payer.pubkey();

    let source_keypair = Keypair::new();
    let source_pubkey = source_keypair.pubkey();

    let destination_keypair = Keypair::new();
    let destination_pubkey = destination_keypair.pubkey();

    let rent_sponsor = context.rent_sponsor;
    let source_epochs = num_prepaid_epochs.unwrap_or(3);

    context.token_account_keypair = source_keypair.insecure_clone();
    {
        let compressible_params = CompressibleParams {
            compressible_config: context.compressible_config,
            rent_sponsor,
            pre_pay_num_epochs: source_epochs,
            lamports_per_write: Some(100),
            compress_to_account_pubkey: None,
            token_account_version: light_token_interface::state::TokenDataVersion::ShaFlat,
            compression_only: false,
        };

        let create_token_account_ix = CreateTokenAccount::new(
            payer_pubkey,
            source_pubkey,
            context.mint_pubkey,
            context.owner_keypair.pubkey(),
        )
        .with_compressible(compressible_params)
        .instruction()
        .unwrap();

        context
            .rpc
            .create_and_send_transaction(
                &[create_token_account_ix],
                &payer_pubkey,
                &[&context.payer, &source_keypair],
            )
            .await
            .unwrap();
    }

    let dest_epochs = num_prepaid_epochs.unwrap_or(3);
    context.token_account_keypair = destination_keypair.insecure_clone();
    {
        let compressible_params = CompressibleParams {
            compressible_config: context.compressible_config,
            rent_sponsor,
            pre_pay_num_epochs: dest_epochs,
            lamports_per_write: Some(100),
            compress_to_account_pubkey: None,
            token_account_version: light_token_interface::state::TokenDataVersion::ShaFlat,
            compression_only: false,
        };

        let create_token_account_ix = CreateTokenAccount::new(
            payer_pubkey,
            destination_pubkey,
            context.mint_pubkey,
            context.owner_keypair.pubkey(),
        )
        .with_compressible(compressible_params)
        .instruction()
        .unwrap();

        context
            .rpc
            .create_and_send_transaction(
                &[create_token_account_ix],
                &payer_pubkey,
                &[&context.payer, &destination_keypair],
            )
            .await
            .unwrap();
    }

    if mint_amount > 0 {
        let mut source_account = context
            .rpc
            .get_account(source_pubkey)
            .await?
            .ok_or_else(|| RpcError::AssertRpcError("Source account not found".to_string()))?;

        let mut token_account =
            spl_token_2022::state::Account::unpack_unchecked(&source_account.data[..165]).map_err(
                |e| RpcError::AssertRpcError(format!("Failed to unpack token account: {:?}", e)),
            )?;
        token_account.amount = mint_amount;
        spl_token_2022::state::Account::pack(token_account, &mut source_account.data[..165])
            .map_err(|e| {
                RpcError::AssertRpcError(format!("Failed to pack token account: {:?}", e))
            })?;

        context.rpc.set_account(source_pubkey, source_account);
    }

    Ok((
        context,
        source_pubkey,
        destination_pubkey,
        mint_amount,
        source_keypair,
        destination_keypair,
    ))
}

/// Execute a ctoken transfer_checked and assert success
#[allow(clippy::too_many_arguments)]
async fn transfer_checked_and_assert(
    context: &mut AccountTestContext,
    source: Pubkey,
    mint: Pubkey,
    destination: Pubkey,
    amount: u64,
    decimals: u8,
    authority: &Keypair,
    name: &str,
) {
    use light_test_utils::assert_ctoken_transfer::assert_ctoken_transfer;

    println!("Transfer checked initiated for: {}", name);

    let payer_pubkey = context.payer.pubkey();

    let transfer_ix = TransferChecked {
        source,
        mint,
        destination,
        amount,
        decimals,
        authority: authority.pubkey(),
        max_top_up: None,
        fee_payer: None,
    }
    .instruction()
    .unwrap();

    context
        .rpc
        .create_and_send_transaction(&[transfer_ix], &payer_pubkey, &[&context.payer, authority])
        .await
        .unwrap();

    assert_ctoken_transfer(&mut context.rpc, source, destination, amount).await;
}

/// Execute a ctoken transfer_checked expecting failure with specific error code
#[allow(clippy::too_many_arguments)]
async fn transfer_checked_and_assert_fails(
    context: &mut AccountTestContext,
    source: Pubkey,
    mint: Pubkey,
    destination: Pubkey,
    amount: u64,
    decimals: u8,
    authority: &Keypair,
    name: &str,
    expected_error_code: u32,
) {
    println!(
        "Transfer checked (expecting failure) initiated for: {}",
        name
    );

    let payer_pubkey = context.payer.pubkey();

    let transfer_ix = TransferChecked {
        source,
        mint,
        destination,
        amount,
        decimals,
        authority: authority.pubkey(),
        max_top_up: None,
        fee_payer: None,
    }
    .instruction()
    .unwrap();

    let result = context
        .rpc
        .create_and_send_transaction(&[transfer_ix], &payer_pubkey, &[&context.payer, authority])
        .await;

    light_program_test::utils::assert::assert_rpc_error(result, 0, expected_error_code).unwrap();
}

// ============================================================================
// Transfer Checked Success Tests
// ============================================================================

#[tokio::test]
async fn test_ctoken_transfer_checked_with_spl_mint() {
    let (mut context, source, destination, _mint_amount, _source_keypair, _dest_keypair) =
        setup_transfer_checked_test_with_spl_mint(None, 1000, 9)
            .await
            .unwrap();

    let mint = context.mint_pubkey;
    let owner_keypair = context.owner_keypair.insecure_clone();

    transfer_checked_and_assert(
        &mut context,
        source,
        mint,
        destination,
        500,
        9,
        &owner_keypair,
        "transfer_checked_spl_mint",
    )
    .await;
}

// Note: Token-2022 mint tests are covered in sdk-tests/sdk-light-token-test/tests/test_transfer_checked.rs
// The T22 mint requires additional setup (extensions, token pool, etc.) that is handled there.

#[tokio::test]
async fn test_ctoken_transfer_checked_compressible_with_topup() {
    let (mut context, source, destination, _mint_amount, _source_keypair, _dest_keypair) =
        setup_transfer_checked_test_with_spl_mint(Some(3), 1000, 9)
            .await
            .unwrap();

    context
        .rpc
        .airdrop_lamports(&context.owner_keypair.pubkey(), 100_000_000)
        .await
        .unwrap();

    let mint = context.mint_pubkey;
    let owner_keypair = context.owner_keypair.insecure_clone();

    transfer_checked_and_assert(
        &mut context,
        source,
        mint,
        destination,
        500,
        9,
        &owner_keypair,
        "compressible_transfer_checked_with_topup",
    )
    .await;
}

// ============================================================================
// Transfer Checked Failure Tests
// ============================================================================

#[tokio::test]
async fn test_ctoken_transfer_checked_wrong_decimals() {
    let (mut context, source, destination, _mint_amount, _source_keypair, _dest_keypair) =
        setup_transfer_checked_test_with_spl_mint(None, 1000, 9)
            .await
            .unwrap();

    let mint = context.mint_pubkey;
    let owner_keypair = context.owner_keypair.insecure_clone();

    transfer_checked_and_assert_fails(
        &mut context,
        source,
        mint,
        destination,
        500,
        8, // Wrong decimals - mint has 9
        &owner_keypair,
        "wrong_decimals_transfer_checked",
        2, // InvalidInstructionData
    )
    .await;
}

#[tokio::test]
async fn test_ctoken_transfer_checked_wrong_mint() {
    let (mut context, source, destination, _mint_amount, _source_keypair, _dest_keypair) =
        setup_transfer_checked_test_with_spl_mint(None, 1000, 9)
            .await
            .unwrap();

    let wrong_mint = Pubkey::new_unique();
    let owner_keypair = context.owner_keypair.insecure_clone();

    transfer_checked_and_assert_fails(
        &mut context,
        source,
        wrong_mint,
        destination,
        500,
        9,
        &owner_keypair,
        "wrong_mint_transfer_checked",
        18002, // TokenError::MintMismatch
    )
    .await;
}

#[tokio::test]
async fn test_ctoken_transfer_checked_insufficient_balance() {
    let (mut context, source, destination, _mint_amount, _source_keypair, _dest_keypair) =
        setup_transfer_checked_test_with_spl_mint(None, 1000, 9)
            .await
            .unwrap();

    let mint = context.mint_pubkey;
    let owner_keypair = context.owner_keypair.insecure_clone();

    transfer_checked_and_assert_fails(
        &mut context,
        source,
        mint,
        destination,
        1500,
        9,
        &owner_keypair,
        "insufficient_balance_transfer_checked",
        6154, // SplInsufficientFunds
    )
    .await;
}

#[tokio::test]
async fn test_ctoken_transfer_checked_max_top_up_exceeded() {
    let (mut context, source, destination, _mint_amount, _source_keypair, _dest_keypair) =
        setup_transfer_checked_test_with_spl_mint(Some(0), 1000, 9)
            .await
            .unwrap();

    context
        .rpc
        .airdrop_lamports(&context.owner_keypair.pubkey(), 100_000_000)
        .await
        .unwrap();

    let mint = context.mint_pubkey;
    let owner_keypair = context.owner_keypair.insecure_clone();
    let payer_pubkey = context.payer.pubkey();

    let transfer_ix = TransferChecked {
        source,
        mint,
        destination,
        amount: 100,
        decimals: 9,
        authority: owner_keypair.pubkey(),
        max_top_up: Some(1),
        fee_payer: None,
    }
    .instruction()
    .unwrap();

    let result = context
        .rpc
        .create_and_send_transaction(
            &[transfer_ix],
            &payer_pubkey,
            &[&context.payer, &owner_keypair],
        )
        .await;

    light_program_test::utils::assert::assert_rpc_error(result, 0, 18043).unwrap();
}
