use anchor_spl::token_2022::spl_token_2022;
use light_client::{indexer::Indexer, rpc::Rpc};
use light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID;
use light_program_test::LightProgramTest;
use light_token_client::instructions::transfer2::{
    CompressInput, DecompressInput, Transfer2InstructionType, TransferInput,
};
use solana_sdk::{program_pack::Pack, pubkey::Pubkey};

use crate::{
    assert_close_token_account::assert_close_token_account,
    assert_ctoken_transfer::assert_compressible_for_account,
};

/// Comprehensive assertion for transfer2 operations that verifies all expected outcomes
/// based on the actions performed. This validates:
/// - Transfer recipients receive correct compressed token amounts
/// - Decompression creates correct SPL token amounts in target accounts
/// - Compression creates correct compressed tokens from SPL sources
/// - Delegate field preservation when delegate performs the transfer
pub async fn assert_transfer2_with_delegate(
    rpc: &mut LightProgramTest,
    actions: Vec<Transfer2InstructionType>,
    authority: Option<Pubkey>, // The actual signer (owner or delegate)
) {
    for action in actions.iter() {
        match action {
            Transfer2InstructionType::Transfer(transfer_input) => {
                // Get recipient's compressed token accounts
                let recipient_accounts = rpc
                    .indexer()
                    .unwrap()
                    .get_compressed_token_accounts_by_owner(&transfer_input.to, None, None)
                    .await
                    .unwrap()
                    .value
                    .items;
                let source_mint = if let Some(mint) = transfer_input.mint {
                    mint
                } else if !transfer_input.compressed_token_account.is_empty() {
                    transfer_input.compressed_token_account[0].token.mint
                } else {
                    panic!("Transfer input must have either mint or compressed_token_account");
                };

                // Get mint from the source compressed token account
                let expected_recipient_token_data = light_sdk::token::TokenData {
                    mint: source_mint,
                    owner: transfer_input.to,
                    amount: transfer_input.amount,
                    delegate: None,
                    state: light_sdk::token::AccountState::Initialized,
                    tlv: None,
                };

                // Assert complete recipient token account
                assert!(
                    recipient_accounts
                        .iter()
                        .any(|account| account.token == expected_recipient_token_data),
                    "Transfer recipient token account should match expected"
                );
                assert!(
                    recipient_accounts
                        .iter()
                        .any(|account| account.account.owner.to_bytes()
                            == COMPRESSED_TOKEN_PROGRAM_ID),
                    "Transfer change token account should match expected"
                );

                // Use explicit change_amount if provided, otherwise calculate it
                let change_amount = transfer_input.change_amount.unwrap_or_else(|| {
                    transfer_input.compressed_token_account[0].token.amount - transfer_input.amount
                });

                // Assert change account if there should be change
                if change_amount > 0 {
                    // Get change account owner from source account and calculate change amount
                    let source_owner = transfer_input.compressed_token_account[0].token.owner;
                    let source_delegate = transfer_input.compressed_token_account[0].token.delegate;
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
                // Get pre-transaction SPL account from cache
                let pre_account_data = rpc
                    .get_pre_transaction_account(&decompress_input.solana_token_account)
                    .expect("SPL token account should exist in pre-transaction context");

                use spl_token_2022::state::Account as SplTokenAccount;
                let pre_spl_account = SplTokenAccount::unpack(&pre_account_data.data)
                    .expect("Failed to unpack SPL token account");
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
                let mut expected_spl_token_account = pre_spl_account;
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
                // Get pre-transaction SPL account from cache
                let pre_account_data = rpc
                    .get_pre_transaction_account(&compress_input.solana_token_account)
                    .expect("SPL token account should exist in pre-transaction context");

                use spl_token_2022::state::Account as SplTokenAccount;
                let pre_spl_account = SplTokenAccount::unpack(&pre_account_data.data[..165])
                    .expect("Failed to unpack SPL token account");
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
                let mut expected_spl_token_account = pre_spl_account;
                expected_spl_token_account.amount -= compress_input.amount;

                // Assert complete SPL source account
                assert_eq!(
                    actual_spl_token_account, expected_spl_token_account,
                    "Compress SPL source account should match expected state"
                );
            }
            Transfer2InstructionType::CompressAndClose(compress_and_close_input) => {
                // Get pre-transaction account from cache
                let pre_account_data = rpc
                    .get_pre_transaction_account(&compress_and_close_input.solana_ctoken_account)
                    .expect("Token account should exist in pre-transaction context");

                use spl_token_2022::state::Account as SplTokenAccount;
                let pre_token_account = SplTokenAccount::unpack(&pre_account_data.data[..165])
                    .expect("Failed to unpack SPL token account");

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
pub async fn assert_transfer2(rpc: &mut LightProgramTest, actions: Vec<Transfer2InstructionType>) {
    assert_transfer2_with_delegate(rpc, actions, None).await;
}

/// Assert transfer operation that transfers compressed tokens to a new recipient
pub async fn assert_transfer2_transfer(rpc: &mut LightProgramTest, transfer_input: TransferInput) {
    assert_transfer2(
        rpc,
        vec![Transfer2InstructionType::Transfer(transfer_input)],
    )
    .await;
}

/// Assert decompress operation that converts compressed tokens to SPL tokens
pub async fn assert_transfer2_decompress(
    rpc: &mut LightProgramTest,
    decompress_input: DecompressInput,
) {
    assert_transfer2(
        rpc,
        vec![Transfer2InstructionType::Decompress(decompress_input)],
    )
    .await;
}

/// Assert compress operation that converts SPL or solana decompressed ctokens to compressed tokens
pub async fn assert_transfer2_compress(rpc: &mut LightProgramTest, compress_input: CompressInput) {
    assert_transfer2(
        rpc,
        vec![Transfer2InstructionType::Compress(compress_input.clone())],
    )
    .await;

    // Assert compressible extension was updated if it exists
    assert_compressible_for_account(
        rpc,
        "SPL source account",
        compress_input.solana_token_account,
    )
    .await;
}

/// Assert compress_and_close operation that compresses all tokens and closes the account
/// Automatically retrieves pre-state from the cached context
pub async fn assert_transfer2_compress_and_close(
    rpc: &mut LightProgramTest,
    compress_and_close_input: light_token_client::instructions::transfer2::CompressAndCloseInput,
) {
    // Get the destination account
    let destination_pubkey = compress_and_close_input
        .destination
        .unwrap_or(compress_and_close_input.authority);

    // Use the existing assert_transfer2 for CompressAndClose validation
    assert_transfer2(
        rpc,
        vec![Transfer2InstructionType::CompressAndClose(
            compress_and_close_input.clone(),
        )],
    )
    .await;

    // Use the existing assert_close_token_account for exact rent validation
    // This now includes the compression incentive check for rent authority closes
    assert_close_token_account(
        rpc,
        compress_and_close_input.solana_ctoken_account,
        compress_and_close_input.authority,
        destination_pubkey,
    )
    .await;

    // Verify the account is closed
    let token_account_info = rpc
        .get_account(compress_and_close_input.solana_ctoken_account)
        .await
        .unwrap();
    assert!(token_account_info.is_none() || token_account_info.unwrap().data.is_empty());
}
