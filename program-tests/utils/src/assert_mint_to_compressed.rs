use anchor_lang::prelude::borsh::BorshDeserialize;
use anchor_spl::token_2022::spl_token_2022;
use light_client::{
    indexer::{CompressedTokenAccount, Indexer},
    rpc::Rpc,
};
use light_compressed_token::instructions::create_token_pool::find_token_pool_pda_with_index;
use light_compressed_token_sdk::instructions::derive_compressed_mint_from_spl_mint;
use light_ctoken_types::{
    instructions::mint_action::Recipient, state::CompressedMint, COMPRESSED_TOKEN_PROGRAM_ID,
};
use solana_sdk::{program_pack::Pack, pubkey::Pubkey};

pub async fn assert_mint_to_compressed<R: Rpc + Indexer>(
    rpc: &mut R,
    spl_mint_pda: Pubkey,
    recipients: &[Recipient],
    pre_token_pool_account: Option<spl_token_2022::state::Account>,
    pre_compressed_mint: CompressedMint,
    pre_spl_mint: Option<spl_token_2022::state::Mint>,
) -> Vec<CompressedTokenAccount> {
    // Derive compressed mint address from SPL mint PDA (same as instruction)
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_mint_address =
        derive_compressed_mint_from_spl_mint(&spl_mint_pda, &address_tree_pubkey);
    // Verify each recipient received their tokens
    let mut all_token_accounts = Vec::new();
    let mut total_minted = 0u64;

    for recipient in recipients {
        let recipient_pubkey = Pubkey::from(recipient.recipient);

        // Get compressed token accounts for this recipient
        let token_accounts = rpc
            .get_compressed_token_accounts_by_owner(&recipient_pubkey, None, None)
            .await
            .expect("Failed to get compressed token accounts")
            .value
            .items;

        // Find the token account for this specific mint
        let matching_account = token_accounts
            .iter()
            .find(|account| {
                account.token.mint == spl_mint_pda && account.token.amount == recipient.amount
            })
            .unwrap_or_else(|| {
                panic!(
                    "Recipient {} should have a token account with {} tokens for mint {}",
                    recipient_pubkey, recipient.amount, spl_mint_pda
                )
            });

        // Create expected token data
        let expected_token_data = light_sdk::token::TokenData {
            mint: spl_mint_pda,
            owner: recipient_pubkey,
            amount: recipient.amount,
            delegate: None,
            state: light_sdk::token::AccountState::Initialized,
            tlv: None,
        };

        // Assert complete token account matches expected
        assert_eq!(
            matching_account.token, expected_token_data,
            "Recipient token account should match expected"
        );
        assert_eq!(
            matching_account.account.owner.to_bytes(),
            COMPRESSED_TOKEN_PROGRAM_ID,
            "Recipient token account should have correct program owner"
        );

        // Add to total minted amount
        total_minted += recipient.amount;

        // Collect all token accounts for return
        all_token_accounts.extend(token_accounts);
    }

    // Verify the compressed mint supply was updated correctly
    let updated_compressed_mint_account = rpc
        .get_compressed_account(compressed_mint_address, None)
        .await
        .expect("Failed to get compressed mint account")
        .value
        .expect("Compressed mint account not found");

    let actual_compressed_mint: CompressedMint = BorshDeserialize::deserialize(
        &mut updated_compressed_mint_account
            .data
            .unwrap()
            .data
            .as_slice(),
    )
    .expect("Failed to deserialize compressed mint");

    // Create expected compressed mint by mutating the pre-mint
    let mut expected_compressed_mint = pre_compressed_mint;
    expected_compressed_mint.base.supply += total_minted;

    assert_eq!(
        actual_compressed_mint, expected_compressed_mint,
        "Compressed mint should match expected state after mint"
    );

    // If mint is decompressed and pre_token_pool_account is provided, validate SPL mint and token pool
    if actual_compressed_mint.metadata.spl_mint_initialized {
        if let Some(pre_pool_account) = pre_token_pool_account {
            // Validate SPL mint supply
            let spl_mint_data = rpc
                .get_account(spl_mint_pda)
                .await
                .expect("Failed to get SPL mint account")
                .expect("SPL mint should exist when decompressed");

            let actual_spl_mint = spl_token_2022::state::Mint::unpack(&spl_mint_data.data)
                .expect("Failed to unpack SPL mint data");

            // Validate SPL mint using mutation pattern if pre_spl_mint is provided
            if let Some(pre_spl_mint_account) = pre_spl_mint {
                let mut expected_spl_mint = pre_spl_mint_account;
                expected_spl_mint.supply += total_minted;

                assert_eq!(
                    actual_spl_mint, expected_spl_mint,
                    "SPL mint should match expected state after mint"
                );
            } else {
                // Fallback validation if no pre_spl_mint provided
                assert_eq!(
                    actual_spl_mint.supply, total_minted,
                    "SPL mint supply should be updated to expected total supply when decompressed"
                );
            }

            // Validate token pool balance increase
            let (token_pool_pda, _) = find_token_pool_pda_with_index(&spl_mint_pda, 0);
            let token_pool_data = rpc
                .get_account(token_pool_pda)
                .await
                .expect("Failed to get token pool account")
                .expect("Token pool should exist when decompressed");

            let actual_token_pool = spl_token_2022::state::Account::unpack(&token_pool_data.data)
                .expect("Failed to unpack token pool data");

            // Create expected token pool account by mutating the pre-account
            let mut expected_token_pool = pre_pool_account;
            expected_token_pool.amount += total_minted;

            assert_eq!(
                actual_token_pool, expected_token_pool,
                "Token pool should match expected state after mint"
            );
        }
    }

    all_token_accounts
}

pub async fn assert_mint_to_compressed_one<R: Rpc + Indexer>(
    rpc: &mut R,
    spl_mint_pda: Pubkey,
    recipient: Pubkey,
    expected_amount: u64,
    pre_token_pool_account: Option<spl_token_2022::state::Account>,
    pre_compressed_mint: CompressedMint,
    pre_spl_mint: Option<spl_token_2022::state::Mint>,
) -> light_client::indexer::CompressedTokenAccount {
    let recipients = vec![Recipient {
        recipient: recipient.into(),
        amount: expected_amount,
    }];

    let token_accounts = assert_mint_to_compressed(
        rpc,
        spl_mint_pda,
        &recipients,
        pre_token_pool_account,
        pre_compressed_mint,
        pre_spl_mint,
    )
    .await;

    // Return the first token account for the recipient
    token_accounts
        .into_iter()
        .find(|account| account.token.owner == recipient && account.token.mint == spl_mint_pda)
        .expect("Should find exactly one matching token account for the recipient")
}
