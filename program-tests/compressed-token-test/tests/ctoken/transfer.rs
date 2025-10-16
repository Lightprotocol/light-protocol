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
    context.token_account_keypair = source_keypair.insecure_clone();
    if let Some(epochs) = num_prepaid_epochs {
        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor,
            num_prepaid_epochs: epochs,
            lamports_per_write: Some(100),
            account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: payer_pubkey,
        };
        create_and_assert_token_account(&mut context, compressible_data, "source_account").await;
    } else {
        // Create non-compressible source account (165 bytes, no extension)
        create_non_compressible_token_account(&mut context, Some(&source_keypair)).await;
    }

    // Create destination token account
    context.token_account_keypair = destination_keypair.insecure_clone();
    if let Some(epochs) = num_prepaid_epochs {
        let compressible_data = CompressibleData {
            compression_authority: context.compression_authority,
            rent_sponsor,
            num_prepaid_epochs: epochs,
            lamports_per_write: Some(100),
            account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
            compress_to_pubkey: false,
            payer: payer_pubkey,
        };
        create_and_assert_token_account(&mut context, compressible_data, "destination_account")
            .await;
    } else {
        // Create non-compressible destination account (165 bytes, no extension)
        create_non_compressible_token_account(&mut context, Some(&destination_keypair)).await;
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
fn build_transfer_instruction(
    source: Pubkey,
    destination: Pubkey,
    amount: u64,
    authority: Pubkey,
) -> solana_sdk::instruction::Instruction {
    use anchor_lang::prelude::AccountMeta;
    use solana_sdk::instruction::Instruction;

    // Build instruction data: discriminator (3) + SPL Transfer data
    let mut data = vec![3]; // CTokenTransfer discriminator (first byte: 3)
    data.extend_from_slice(&amount.to_le_bytes()); // Amount as u64 little-endian

    // Build instruction
    Instruction {
        program_id: light_compressed_token::ID,
        accounts: vec![
            AccountMeta::new(source, false),
            AccountMeta::new(destination, false),
            AccountMeta::new(authority, true), // Authority must sign (also acts as payer for top-ups)
            AccountMeta::new_readonly(Pubkey::default(), false), // System program for lamport transfers during top-up
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
    // Create compressible accounts with 2 prepaid epochs (sufficient, no top-up needed)
    let (mut context, source, destination, _mint_amount, _source_keypair, _dest_keypair) =
        setup_transfer_test(Some(2), 1000).await.unwrap();

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
    // Create compressible accounts with 2 prepaid epochs
    let (mut context, source, destination, _mint_amount, _source_keypair, _dest_keypair) =
        setup_transfer_test(Some(2), 1000).await.unwrap();
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
    // Expected error: InsufficientFunds (error code 1)
    transfer_and_assert_fails(
        &mut context,
        source,
        destination,
        1500,
        &owner_keypair,
        "insufficient_balance_transfer",
        1, // InsufficientFunds
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
    // Expected error: AccountFrozen (error code 17)
    transfer_and_assert_fails(
        &mut context,
        source,
        destination,
        500,
        &owner_keypair,
        "frozen_source_transfer",
        17, // AccountFrozen
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
    // Expected error: AccountFrozen (error code 17)
    transfer_and_assert_fails(
        &mut context,
        source,
        destination,
        500,
        &owner_keypair,
        "frozen_destination_transfer",
        17, // AccountFrozen
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
    // Expected error: OwnerMismatch (error code 4)
    transfer_and_assert_fails(
        &mut context,
        source,
        destination,
        500,
        &wrong_authority,
        "wrong_authority_transfer",
        4, // OwnerMismatch
    )
    .await;
}

#[tokio::test]
async fn test_ctoken_transfer_mint_mismatch() {
    // Create source account with default mint
    let (mut context, source, _destination, _mint_amount, _source_keypair, _dest_keypair) =
        setup_transfer_test(None, 1000).await.unwrap();

    // Create destination account with a different mint
    let different_mint = Pubkey::new_unique();
    let original_mint = context.mint_pubkey;
    context.mint_pubkey = different_mint;

    let dest_keypair = Keypair::new();
    context.token_account_keypair = dest_keypair.insecure_clone();
    create_non_compressible_token_account(&mut context, Some(&dest_keypair)).await;
    let destination_with_different_mint = dest_keypair.pubkey();

    // Restore original mint for context
    context.mint_pubkey = original_mint;

    // Use the owner keypair as authority
    let owner_keypair = context.owner_keypair.insecure_clone();

    // Try to transfer between accounts with different mints
    // Expected error: MintMismatch (error code 3)
    transfer_and_assert_fails(
        &mut context,
        source,
        destination_with_different_mint,
        500,
        &owner_keypair,
        "mint_mismatch_transfer",
        3, // MintMismatch
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
    // Create source as compressible
    let mut context = setup_account_test().await.unwrap();
    let payer_pubkey = context.payer.pubkey();

    // Create compressible source account
    let source_keypair = Keypair::new();
    let source_pubkey = source_keypair.pubkey();
    context.token_account_keypair = source_keypair.insecure_clone();

    let compressible_data = CompressibleData {
        compression_authority: context.compression_authority,
        rent_sponsor: context.rent_sponsor,
        num_prepaid_epochs: 2,
        lamports_per_write: Some(100),
        account_version: light_ctoken_types::state::TokenDataVersion::ShaFlat,
        compress_to_pubkey: false,
        payer: payer_pubkey,
    };
    create_and_assert_token_account(&mut context, compressible_data, "source_account").await;

    // Create non-compressible destination account
    let destination_keypair = Keypair::new();
    let destination_pubkey = destination_keypair.pubkey();
    context.token_account_keypair = destination_keypair.insecure_clone();
    create_non_compressible_token_account(&mut context, Some(&destination_keypair)).await;

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

    // Fund owner to pay for top-up
    context
        .rpc
        .airdrop_lamports(&context.owner_keypair.pubkey(), 100_000_000)
        .await
        .unwrap();

    let owner_keypair = context.owner_keypair.insecure_clone();

    // Transfer from compressible to non-compressible (only source needs top-up)
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
