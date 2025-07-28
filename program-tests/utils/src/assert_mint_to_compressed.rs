use anchor_lang::prelude::borsh::BorshDeserialize;
use light_client::{
    indexer::{CompressedTokenAccount, Indexer},
    rpc::Rpc,
};
use light_compressed_token_sdk::instructions::derive_compressed_mint_from_spl_mint;
use light_ctoken_types::{instructions::mint_to_compressed::Recipient, state::CompressedMint};
use solana_sdk::pubkey::Pubkey;

pub async fn assert_mint_to_compressed<R: Rpc + Indexer>(
    rpc: &mut R,
    spl_mint_pda: Pubkey,
    recipients: &[Recipient],
    expected_total_supply: u64,
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
        let matching_accounts: Vec<_> = token_accounts
            .iter()
            .filter(|account| account.token.mint == spl_mint_pda)
            .collect();

        assert!(
            !matching_accounts.is_empty(),
            "Recipient {} should have at least one token account for mint {}",
            recipient_pubkey,
            spl_mint_pda
        );

        // Verify the recipient has the correct total amount for this mint
        let recipient_total: u64 = matching_accounts
            .iter()
            .map(|account| account.token.amount)
            .sum();

        assert!(
            recipient_total >= recipient.amount,
            "Recipient {} should have at least {} tokens, but has {}",
            recipient_pubkey,
            recipient.amount,
            recipient_total
        );

        // Verify token account properties
        for account in &matching_accounts {
            assert_eq!(
                account.token.mint, spl_mint_pda,
                "Token account should have correct mint"
            );
            assert_eq!(
                account.token.owner, recipient_pubkey,
                "Token account should have correct owner"
            );
            assert!(
                account.token.amount > 0,
                "Token account should have non-zero amount"
            );
        }

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
        .value;

    let updated_compressed_mint: CompressedMint = BorshDeserialize::deserialize(
        &mut updated_compressed_mint_account
            .data
            .unwrap()
            .data
            .as_slice(),
    )
    .expect("Failed to deserialize compressed mint");

    assert_eq!(
        updated_compressed_mint.supply, expected_total_supply,
        "Compressed mint supply should be updated to expected total supply"
    );

    assert_eq!(
        updated_compressed_mint.spl_mint,
        light_compressed_account::Pubkey::from(spl_mint_pda.to_bytes()),
        "Compressed mint should reference correct SPL mint PDA"
    );

    println!(" mint_to_compressed assertions passed:");
    println!("   - Recipients: {}", recipients.len());
    println!("   - Total minted: {}", total_minted);
    println!(
        "   - Updated mint supply: {}",
        updated_compressed_mint.supply
    );
    println!("   - Token accounts created: {}", all_token_accounts.len());

    all_token_accounts
}

pub async fn assert_mint_to_compressed_one<R: Rpc + Indexer>(
    rpc: &mut R,
    spl_mint_pda: Pubkey,
    recipient: Pubkey,
    expected_amount: u64,
    expected_total_supply: u64,
) -> light_client::indexer::CompressedTokenAccount {
    let recipients = vec![Recipient {
        recipient: recipient.into(),
        amount: expected_amount,
    }];

    let token_accounts =
        assert_mint_to_compressed(rpc, spl_mint_pda, &recipients, expected_total_supply).await;

    // Return the first token account for the recipient
    token_accounts
        .into_iter()
        .find(|account| account.token.owner == recipient && account.token.mint == spl_mint_pda)
        .expect("Should find exactly one matching token account for the recipient")
}
