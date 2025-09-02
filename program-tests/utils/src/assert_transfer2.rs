use anchor_spl::token_2022::spl_token_2022;
use light_client::{indexer::Indexer, rpc::Rpc};
use light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID;
use light_token_client::instructions::transfer2::{
    CompressInput, DecompressInput, Transfer2InstructionType, TransferInput,
};
use solana_sdk::{program_pack::Pack, pubkey::Pubkey};

use crate::assert_decompressed_token_transfer::assert_compressible_for_account;

/// Comprehensive assertion for transfer2 operations that verifies all expected outcomes
/// based on the actions performed. This validates:
/// - Transfer recipients receive correct compressed token amounts
/// - Decompression creates correct SPL token amounts in target accounts
/// - Compression creates correct compressed tokens from SPL sources
/// - Delegate field preservation when delegate performs the transfer
pub async fn assert_transfer2_with_delegate<R: Rpc + Indexer>(
    rpc: &mut R,
    actions: Vec<Transfer2InstructionType<'_>>,
    pre_token_accounts: Vec<Option<spl_token_2022::state::Account>>,
    authority: Option<Pubkey>, // The actual signer (owner or delegate)
) {
    assert_eq!(
        actions.len(),
        pre_token_accounts.len(),
        "Actions and pre_token_accounts must have same length"
    );

    for (action, pre_account) in actions.iter().zip(pre_token_accounts.iter()) {
        match action {
            Transfer2InstructionType::Transfer(transfer_input) => {
                assert!(
                    pre_account.is_none(),
                    "Transfer actions should have None for pre_token_account"
                );
                // Get recipient's compressed token accounts
                let recipient_accounts = rpc
                    .indexer()
                    .unwrap()
                    .get_compressed_token_accounts_by_owner(&transfer_input.to, None, None)
                    .await
                    .unwrap()
                    .value
                    .items;

                // Get mint from the source compressed token account
                let source_mint = transfer_input.compressed_token_account[0].token.mint;
                let expected_recipient_token_data = light_sdk::token::TokenData {
                    mint: source_mint,
                    owner: transfer_input.to,
                    amount: transfer_input.amount,
                    delegate: None,
                    state: light_sdk::token::AccountState::Initialized,
                    tlv: None,
                };

                // Assert complete recipient token account
                assert_eq!(
                    recipient_accounts[0].token, expected_recipient_token_data,
                    "Transfer recipient token account should match expected"
                );
                assert_eq!(
                    recipient_accounts[0].account.owner.to_bytes(),
                    COMPRESSED_TOKEN_PROGRAM_ID,
                    "Transfer change token account should match expected"
                );
                // Get change account owner from source account and calculate change amount
                let source_owner = transfer_input.compressed_token_account[0].token.owner;
                let source_amount = transfer_input.compressed_token_account[0].token.amount;
                let source_delegate = transfer_input.compressed_token_account[0].token.delegate;
                let change_amount = source_amount - transfer_input.amount;

                // Assert change account if there should be change
                if change_amount > 0 {
                    let change_accounts = rpc
                        .indexer()
                        .unwrap()
                        .get_compressed_token_accounts_by_owner(&source_owner, None, None)
                        .await
                        .unwrap()
                        .value
                        .items;

                    // Determine if delegate should be preserved in change account
                    // If delegate is transferring (is_delegate_transfer = true), preserve the delegate
                    // If owner is transferring, clear the delegate
                    let expected_delegate =
                        if transfer_input.is_delegate_transfer && source_delegate.is_some() {
                            source_delegate // Preserve delegate if they are performing the transfer
                        } else {
                            None // No delegate to preserve
                        };

                    let expected_change_token = light_sdk::token::TokenData {
                        mint: source_mint,
                        owner: source_owner,
                        amount: change_amount,
                        delegate: expected_delegate,
                        state: light_sdk::token::AccountState::Initialized,
                        tlv: None,
                    };

                    // Find the change account that matches our expected token data
                    let matching_change_account = change_accounts
                        .iter()
                        .find(|acc| acc.token == expected_change_token)
                        .unwrap_or_else(|| panic!("Should find change account with expected token data change_accounts: {:?}", change_accounts));

                    // Assert complete change token account
                    assert_eq!(
                        matching_change_account.token, expected_change_token,
                        "Transfer change token account should match expected"
                    );
                    assert_eq!(
                        matching_change_account.account.owner.to_bytes(),
                        COMPRESSED_TOKEN_PROGRAM_ID,
                        "Transfer change token account should match expected"
                    );
                }
            }
            Transfer2InstructionType::Decompress(decompress_input) => {
                let pre_spl_account = pre_account
                    .as_ref()
                    .ok_or("Decompress actions require pre_token_account")
                    .unwrap();
                // Verify SPL token account received tokens
                let spl_account_data = rpc
                    .get_account(decompress_input.solana_token_account)
                    .await
                    .expect("Failed to get SPL token account")
                    .expect("SPL token account should exist");

                let actual_spl_token_account =
                    spl_token_2022::state::Account::unpack(&spl_account_data.data)
                        .expect("Failed to unpack SPL token account");

                // Get mint from the source compressed token account
                let source_mint = decompress_input.compressed_token_account[0].token.mint;
                let source_owner = decompress_input.compressed_token_account[0].token.owner;

                // Create expected SPL token account state
                let mut expected_spl_token_account = *pre_spl_account;
                expected_spl_token_account.amount += decompress_input.amount;

                // Assert complete SPL token account
                assert_eq!(
                    actual_spl_token_account, expected_spl_token_account,
                    "Decompressed SPL token account should match expected state"
                );

                // Assert change compressed token account if there should be change
                let source_amount = decompress_input.compressed_token_account[0].token.amount;
                let source_delegate = decompress_input.compressed_token_account[0].token.delegate;
                let change_amount = source_amount - decompress_input.amount;

                if change_amount > 0 {
                    let change_accounts = rpc
                        .indexer()
                        .unwrap()
                        .get_compressed_token_accounts_by_owner(&source_owner, None, None)
                        .await
                        .unwrap()
                        .value
                        .items;

                    // Determine if delegate should be preserved in change account
                    // Same logic as transfer: preserve if delegate is signer, clear if owner is signer
                    let expected_delegate = if let Some(auth) = authority {
                        if source_delegate == Some(auth) {
                            source_delegate // Preserve delegate if they are the signer
                        } else {
                            None // Clear delegate if owner is the signer
                        }
                    } else {
                        None // Default to None if no authority specified
                    };

                    let expected_change_token = light_sdk::token::TokenData {
                        mint: source_mint,
                        owner: source_owner,
                        amount: change_amount,
                        delegate: expected_delegate,
                        state: light_sdk::token::AccountState::Initialized,
                        tlv: None,
                    };

                    // Find the change account that matches our expected token data
                    let matching_change_account = change_accounts
                        .iter()
                        .find(|acc| acc.token == expected_change_token)
                        .expect("Should find change account with expected token data");

                    // Assert complete change token account
                    assert_eq!(
                        matching_change_account.token, expected_change_token,
                        "Decompress change token account should match expected"
                    );
                    assert_eq!(
                        matching_change_account.account.owner.to_bytes(),
                        COMPRESSED_TOKEN_PROGRAM_ID,
                        "Decompress change token account should match expected"
                    );
                }
            }

            Transfer2InstructionType::Approve(approve_input) => {
                assert!(
                    pre_account.is_none(),
                    "Approve actions should have None for pre_token_account"
                );
                let source_mint = approve_input.compressed_token_account[0].token.mint;
                let source_owner = approve_input.compressed_token_account[0].token.owner;

                // Calculate expected change amount
                let source_amount = approve_input
                    .compressed_token_account
                    .iter()
                    .map(|acc| acc.token.amount)
                    .sum::<u64>();
                let change_amount = source_amount - approve_input.delegate_amount;

                // Assert change account if there should be change
                if change_amount > 0 {
                    let change_accounts = rpc
                        .indexer()
                        .unwrap()
                        .get_compressed_token_accounts_by_owner(&source_owner, None, None)
                        .await
                        .unwrap()
                        .value
                        .items;

                    let expected_change_token = light_sdk::token::TokenData {
                        mint: source_mint,
                        owner: source_owner,
                        amount: change_amount,
                        delegate: Some(approve_input.delegate),
                        state: light_sdk::token::AccountState::Initialized,
                        tlv: None,
                    };

                    // Find the change account that matches our expected token data
                    let matching_change_account = change_accounts
                        .iter()
                        .find(|acc| acc.token == expected_change_token)
                        .unwrap_or_else(|| panic!("Should find change account with expected token data change_accounts: {:?}", change_accounts));

                    // Assert complete change token account
                    assert_eq!(
                        matching_change_account.token, expected_change_token,
                        "Transfer change token account should match expected"
                    );
                    assert_eq!(
                        matching_change_account.account.owner.to_bytes(),
                        COMPRESSED_TOKEN_PROGRAM_ID,
                        "Transfer change token account should match expected"
                    );
                }
            }

            Transfer2InstructionType::Compress(compress_input) => {
                let pre_spl_account = pre_account
                    .as_ref()
                    .ok_or("Compress actions require pre_token_account")
                    .unwrap();
                // Verify recipient received compressed tokens
                let recipient_accounts = rpc
                    .indexer()
                    .unwrap()
                    .get_compressed_token_accounts_by_owner(&compress_input.to, None, None)
                    .await
                    .unwrap()
                    .value
                    .items;

                let expected_recipient_token_data = light_sdk::token::TokenData {
                    mint: compress_input.mint,
                    owner: compress_input.to,
                    amount: compress_input.amount,
                    delegate: None,
                    state: light_sdk::token::AccountState::Initialized,
                    tlv: None,
                };

                // Assert complete recipient compressed token account
                assert_eq!(
                    recipient_accounts[0].token, expected_recipient_token_data,
                    "Compress recipient token account should match expected"
                );
                assert_eq!(
                    recipient_accounts[0].account.owner.to_bytes(),
                    COMPRESSED_TOKEN_PROGRAM_ID,
                    "Compress recipient token account should match expected"
                );

                // Verify SPL source account was reduced
                let spl_account_data = rpc
                    .get_account(compress_input.solana_token_account)
                    .await
                    .expect("Failed to get SPL source account")
                    .expect("SPL source account should exist");

                let actual_spl_token_account =
                    spl_token_2022::state::Account::unpack(&spl_account_data.data[..165])
                        .expect("Failed to unpack SPL source account");

                // Create expected SPL token account state (amount reduced by compression)
                let mut expected_spl_token_account = *pre_spl_account;
                expected_spl_token_account.amount -= compress_input.amount;

                // Assert complete SPL source account
                assert_eq!(
                    actual_spl_token_account, expected_spl_token_account,
                    "Compress SPL source account should match expected state"
                );
            }
            Transfer2InstructionType::CompressAndClose(compress_and_close_input) => {
                let pre_token_account =
                    pre_account.expect("CompressAndClose requires pre_token_account");

                // Get the compressed token accounts by owner
                let owner_accounts = rpc
                    .indexer()
                    .unwrap()
                    .get_compressed_token_accounts_by_owner(&pre_token_account.owner, None, None)
                    .await
                    .unwrap()
                    .value
                    .items;

                // Find the compressed account with the expected amount and mint
                let expected_amount = pre_token_account.amount;
                let expected_mint = pre_token_account.mint;

                // Verify exactly one compressed account was created for this mint
                let mint_accounts: Vec<_> = owner_accounts
                    .iter()
                    .filter(|acc| acc.token.mint == expected_mint)
                    .collect();

                assert_eq!(
                    mint_accounts.len(),
                    1,
                    "CompressAndClose should create exactly one compressed account for the mint"
                );

                let compressed_account = mint_accounts[0];

                // Verify the compressed account has the correct data
                assert_eq!(
                    compressed_account.token.amount, expected_amount,
                    "CompressAndClose compressed amount should match original balance"
                );
                assert_eq!(
                    compressed_account.token.owner, pre_token_account.owner,
                    "CompressAndClose owner should match original owner"
                );
                assert_eq!(
                    compressed_account.token.mint, expected_mint,
                    "CompressAndClose mint should match original mint"
                );
                assert_eq!(
                    compressed_account.token.delegate, None,
                    "CompressAndClose compressed account should have no delegate"
                );
                assert_eq!(
                    compressed_account.token.state,
                    light_sdk::token::AccountState::Initialized,
                    "CompressAndClose compressed account should be initialized"
                );
                assert_eq!(
                    compressed_account.token.tlv, None,
                    "CompressAndClose compressed account should have no TLV data"
                );

                // Verify compressed account metadata
                assert_eq!(
                    compressed_account.account.owner.to_bytes(),
                    COMPRESSED_TOKEN_PROGRAM_ID,
                    "CompressAndClose compressed account should be owned by compressed token program"
                );
                assert_eq!(
                    compressed_account.account.lamports, 0,
                    "CompressAndClose compressed account should have 0 lamports"
                );

                // Verify the source account is FULLY closed
                let spl_account_result = rpc
                    .get_account(compress_and_close_input.solana_ctoken_account)
                    .await
                    .expect("Failed to check closed account");

                if let Some(acc) = spl_account_result {
                    assert_eq!(
                        acc.lamports, 0,
                        "CompressAndClose source account should have 0 lamports after closing"
                    );
                    assert!(
                        acc.data.is_empty() || acc.data.iter().all(|&b| b == 0),
                        "CompressAndClose source account data should be cleared"
                    );
                    assert_eq!(
                        acc.owner, solana_sdk::system_program::ID,
                        "CompressAndClose source account owner should be System Program after closing"
                    );
                }
            }
        }
    }
}

/// Backwards compatibility wrapper for assert_transfer2_with_delegate
/// Uses None for authority (assumes owner is signer)
pub async fn assert_transfer2<R: Rpc + Indexer>(
    rpc: &mut R,
    actions: Vec<Transfer2InstructionType<'_>>,
    pre_token_accounts: Vec<Option<spl_token_2022::state::Account>>,
) {
    assert_transfer2_with_delegate(rpc, actions, pre_token_accounts, None).await;
}

/// Assert transfer operation that transfers compressed tokens to a new recipient
pub async fn assert_transfer2_transfer<R: Rpc + Indexer>(
    rpc: &mut R,
    transfer_input: TransferInput<'_>,
) {
    assert_transfer2(
        rpc,
        vec![Transfer2InstructionType::Transfer(transfer_input)],
        vec![None],
    )
    .await;
}

/// Assert decompress operation that converts compressed tokens to SPL tokens
pub async fn assert_transfer2_decompress<R: Rpc + Indexer>(
    rpc: &mut R,
    decompress_input: DecompressInput<'_>,
    pre_spl_token_account: spl_token_2022::state::Account,
) {
    assert_transfer2(
        rpc,
        vec![Transfer2InstructionType::Decompress(decompress_input)],
        vec![Some(pre_spl_token_account)],
    )
    .await;
}

/// Assert compress operation that converts SPL or solana decompressed ctokens to compressed tokens
pub async fn assert_transfer2_compress<R: Rpc + Indexer>(
    rpc: &mut R,
    compress_input: CompressInput<'_>,
    pre_spl_token_account: spl_token_2022::state::Account,
    pre_spl_account_data: &[u8],
) {
    // Get current slot for compressible extension assertion
    let current_slot = rpc.get_slot().await.unwrap();

    assert_transfer2(
        rpc,
        vec![Transfer2InstructionType::Compress(compress_input.clone())],
        vec![Some(pre_spl_token_account)],
    )
    .await;

    // Get the account data after compression to check compressible extensions
    let spl_account_data_after = rpc
        .get_account(compress_input.solana_token_account)
        .await
        .expect("Failed to get SPL token account after compression")
        .expect("SPL token account should exist after compression");

    // Assert compressible extension was updated if it exists
    assert_compressible_for_account(
        "SPL source account",
        pre_spl_account_data,
        &spl_account_data_after.data,
        current_slot,
    );
}

/// Assert compress_and_close operation that compresses all tokens and closes the account
pub async fn assert_transfer2_compress_and_close<R: Rpc + Indexer>(
    rpc: &mut R,
    compress_and_close_input: light_token_client::instructions::transfer2::CompressAndCloseInput,
    pre_spl_token_account: spl_token_2022::state::Account,
    pre_spl_account_data: &[u8],
    initial_recipient_lamports: u64,
) {
    use light_ctoken_types::state::{CompressedToken, ZExtensionStruct};
    use light_zero_copy::traits::ZeroCopyAt;

    use crate::assert_close_token_account::assert_close_token_account;

    // Parse to extract rent_recipient from compressible extension
    let (compressed_token, _) = CompressedToken::zero_copy_at(pre_spl_account_data)
        .expect("Failed to parse compressed token account");

    let rent_recipient = compressed_token
        .extensions
        .as_ref()
        .and_then(|extensions| {
            extensions.iter().find_map(|ext| {
                if let ZExtensionStruct::Compressible(comp) = ext {
                    Some(Pubkey::from(comp.rent_recipient.to_bytes()))
                } else {
                    None
                }
            })
        })
        .expect("Should have compressible extension with rent_recipient");

    // Use the existing assert_transfer2 for CompressAndClose validation
    assert_transfer2(
        rpc,
        vec![Transfer2InstructionType::CompressAndClose(
            compress_and_close_input.clone(),
        )],
        vec![Some(pre_spl_token_account)],
    )
    .await;

    // Use the existing assert_close_token_account for exact rent validation
    assert_close_token_account(
        rpc,
        compress_and_close_input.solana_ctoken_account,
        Some(pre_spl_account_data),
        rent_recipient,
        initial_recipient_lamports,
    )
    .await;
}
