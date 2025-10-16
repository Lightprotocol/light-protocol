use std::collections::HashMap;

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
    // First pass: Build expected SPL account states by accumulating all balance changes
    let mut expected_spl_accounts: HashMap<Pubkey, spl_token_2022::state::Account> = HashMap::new();

    for action in actions.iter() {
        match action {
            Transfer2InstructionType::Compress(compress_input) => {
                let pubkey = compress_input.solana_token_account;

                // Get or initialize the expected account state
                expected_spl_accounts.entry(pubkey).or_insert_with(|| {
                    let pre_account_data = rpc
                        .get_pre_transaction_account(&pubkey)
                        .expect("SPL token account should exist in pre-transaction context");

                    spl_token_2022::state::Account::unpack(&pre_account_data.data[..165])
                        .expect("Failed to unpack SPL token account")
                });

                // Decrement balance for compress
                expected_spl_accounts.get_mut(&pubkey).unwrap().amount -= compress_input.amount;
            }
            Transfer2InstructionType::Decompress(decompress_input) => {
                let pubkey = decompress_input.solana_token_account;

                // Get or initialize the expected account state
                expected_spl_accounts.entry(pubkey).or_insert_with(|| {
                    let pre_account_data = rpc
                        .get_pre_transaction_account(&pubkey)
                        .expect("SPL token account should exist in pre-transaction context");

                    spl_token_2022::state::Account::unpack(&pre_account_data.data)
                        .expect("Failed to unpack SPL token account")
                });

                // Increment balance for decompress
                expected_spl_accounts.get_mut(&pubkey).unwrap().amount += decompress_input.amount;
            }
            _ => {} // Other actions don't affect SPL accounts
        }
    }

    // Second pass: Assert compressed token accounts and other outcomes
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
                recipient_accounts.iter().for_each(|account| {
                    if account.account.data.as_ref().unwrap().discriminator == 4u64.to_be_bytes() {
                        assert_eq!(
                            account.account.data.as_ref().unwrap().data_hash,
                            account.token.hash_sha_flat().unwrap(),
                            "Invalid sha flat data hash {:?}",
                            account
                        );
                    }
                });

                // Use explicit change_amount if provided, otherwise calculate it
                let change_amount = transfer_input.change_amount.unwrap_or_else(|| {
                    // Sum all input amounts
                    let total_input: u64 = transfer_input
                        .compressed_token_account
                        .iter()
                        .map(|acc| acc.token.amount)
                        .sum();
                    total_input.saturating_sub(transfer_input.amount)
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
                        .unwrap_or_else(|| panic!("Should find change account with expected token data change_accounts: {:?} expected change account {:?}", change_accounts, expected_change_token));

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
                // Get mint from the source compressed token account
                let source_mint = decompress_input.compressed_token_account[0].token.mint;
                let source_owner = decompress_input.compressed_token_account[0].token.owner;

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
                    change_accounts.iter().for_each(|account| {
                        if account.account.data.as_ref().unwrap().discriminator
                            == 4u64.to_be_bytes()
                        {
                            assert_eq!(
                                account.account.data.as_ref().unwrap().data_hash,
                                account.token.hash_sha_flat().unwrap(),
                                "Invalid sha flat data hash {:?}",
                                account
                            );
                        }
                    });
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
                    change_accounts.iter().for_each(|account| {
                        if account.account.data.as_ref().unwrap().discriminator
                            == 4u64.to_be_bytes()
                        {
                            assert_eq!(
                                account.account.data.as_ref().unwrap().data_hash,
                                account.token.hash_sha_flat().unwrap(),
                                "Invalid sha flat data hash {:?}",
                                account
                            );
                        }
                    });
                }
            }

            Transfer2InstructionType::Compress(compress_input) => {
                // Verify recipient received compressed tokens
                let recipient_accounts = rpc
                    .indexer()
                    .unwrap()
                    .get_compressed_token_accounts_by_owner(&compress_input.to, None, None)
                    .await
                    .unwrap()
                    .value
                    .items;

                // Calculate expected amount including compressed inputs
                let compressed_input_amount = compress_input
                    .compressed_token_account
                    .as_ref()
                    .map(|accounts| accounts.iter().map(|a| a.token.amount).sum::<u64>())
                    .unwrap_or(0);

                let expected_recipient_token_data = light_sdk::token::TokenData {
                    mint: compress_input.mint,
                    owner: compress_input.to,
                    amount: compress_input.amount + compressed_input_amount,
                    delegate: None,
                    state: light_sdk::token::AccountState::Initialized,
                    tlv: None,
                };
                recipient_accounts.iter().for_each(|account| {
                    if account.account.data.as_ref().unwrap().discriminator == 4u64.to_be_bytes() {
                        assert_eq!(
                            account.account.data.as_ref().unwrap().data_hash,
                            account.token.hash_sha_flat().unwrap(),
                            "Invalid sha flat data hash {:?}",
                            account
                        );
                    }
                });
                // Find the compressed account that matches the expected amount
                // (there might be multiple accounts for the same owner/mint in complex transactions)
                let matching_account = recipient_accounts
                    .iter()
                    .find(|account| {
                        account.token.mint == expected_recipient_token_data.mint
                            && account.token.owner == expected_recipient_token_data.owner
                            && account.token.amount == expected_recipient_token_data.amount
                    })
                    .expect("Should find compressed account with expected amount");

                // Assert complete recipient compressed token account
                assert_eq!(
                    matching_account.token, expected_recipient_token_data,
                    "Compress recipient token account should match expected"
                );
                assert_eq!(
                    matching_account.account.owner.to_bytes(),
                    COMPRESSED_TOKEN_PROGRAM_ID,
                    "Compress recipient token account should match expected"
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

                // Check if compress_to_pubkey is set in the compressible extension
                use light_ctoken_types::state::{ctoken::CToken, ZExtensionStruct};
                use light_zero_copy::traits::ZeroCopyAt;

                let compress_to_pubkey = if pre_account_data.data.len() > 165 {
                    // Has extensions, check for compressible extension
                    let (ctoken, _) = CToken::zero_copy_at(&pre_account_data.data)
                        .expect("Failed to deserialize ctoken account");

                    if let Some(extensions) = ctoken.extensions.as_ref() {
                        extensions
                            .iter()
                            .find_map(|ext| match ext {
                                ZExtensionStruct::Compressible(comp) => {
                                    Some(comp.compress_to_pubkey == 1)
                                }
                                _ => None,
                            })
                            .unwrap_or(false)
                    } else {
                        false
                    }
                } else {
                    false
                };

                // Determine the expected owner in the compressed output
                let expected_owner = if compress_to_pubkey {
                    compress_and_close_input.solana_ctoken_account // Account pubkey becomes owner
                } else {
                    pre_token_account.owner // Original owner preserved
                };

                // Get the compressed token accounts by the expected owner
                let owner_accounts = rpc
                    .indexer()
                    .unwrap()
                    .get_compressed_token_accounts_by_owner(&expected_owner, None, None)
                    .await
                    .unwrap()
                    .value
                    .items;
                owner_accounts.iter().for_each(|account| {
                    if account.account.data.as_ref().unwrap().discriminator == 4u64.to_be_bytes() {
                        assert_eq!(
                            account.account.data.as_ref().unwrap().data_hash,
                            account.token.hash_sha_flat().unwrap(),
                            "Invalid sha flat data hash {:?}",
                            account
                        );
                    }
                });
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
                    compressed_account.token.owner,
                    expected_owner,
                    "CompressAndClose owner should be {} (compress_to_pubkey={})",
                    if compress_to_pubkey {
                        "account pubkey"
                    } else {
                        "original owner"
                    },
                    compress_to_pubkey
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

    // Third pass: Verify all SPL account final states against accumulated expected states
    for (pubkey, expected_account) in expected_spl_accounts.iter() {
        let actual_account_data = rpc
            .get_account(*pubkey)
            .await
            .expect("Failed to get SPL account")
            .expect("SPL account should exist");

        let actual_account =
            spl_token_2022::state::Account::unpack(&actual_account_data.data[..165])
                .expect("Failed to unpack SPL account");

        assert_eq!(
            actual_account, *expected_account,
            "SPL account {} final state should match expected state after all compress/decompress operations",
            pubkey
        );
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
