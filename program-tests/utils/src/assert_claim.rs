use light_client::rpc::Rpc;
use light_ctoken_interface::{state::CToken, BASE_TOKEN_ACCOUNT_SIZE};
use light_program_test::LightProgramTest;
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use solana_sdk::{clock::Clock, pubkey::Pubkey};

pub async fn assert_claim(
    rpc: &mut LightProgramTest,
    token_account_pubkeys: &[Pubkey],
    pool_pda: Pubkey,
    compression_authority: Pubkey,
) {
    let pre_pool_lamports = rpc
        .get_pre_transaction_account(&pool_pda)
        .map(|acc| acc.lamports)
        .unwrap_or_else(|| {
            panic!("Pool PDA should exist in pre-transaction context");
        });
    let mut expected_lamports_claimed = 0;
    for token_account_pubkey in token_account_pubkeys {
        // Get pre-transaction state for all relevant accounts
        let mut pre_token_account = rpc
            .get_pre_transaction_account(token_account_pubkey)
            .expect("Token account should exist in pre-transaction context");
        assert!(
            pre_token_account.data.len() >= BASE_TOKEN_ACCOUNT_SIZE as usize,
            "Token account should have at least BASE_TOKEN_ACCOUNT_SIZE bytes"
        );
        // Get account size and lamports before parsing (to avoid borrow conflicts)
        let account_size = pre_token_account.data.len() as u64;
        let account_lamports = pre_token_account.lamports;
        let current_slot = rpc.pre_context.as_ref().unwrap().get_sysvar::<Clock>().slot;
        let base_lamports = rpc
            .get_minimum_balance_for_rent_exemption(account_size as usize)
            .await
            .unwrap();

        // Parse pre-transaction token account data
        let (mut pre_compressed_token, _) = CToken::zero_copy_at_mut(&mut pre_token_account.data)
            .expect("Failed to deserialize pre-transaction token account");

        // Get compression info from meta.compression
        let compression = &mut pre_compressed_token.compression;
        let pre_last_claimed_slot = u64::from(compression.last_claimed_slot);

        let pre_compression_authority = Pubkey::from(compression.compression_authority);
        let pre_rent_sponsor = Pubkey::from(compression.rent_sponsor);

        let lamports_result =
            compression.claim(account_size, current_slot, account_lamports, base_lamports);
        let not_claimed_was_none = lamports_result.is_err();
        if let Ok(Some(lamports)) = lamports_result {
            expected_lamports_claimed += lamports;
        }
        // Verify rent authority matches
        assert_eq!(
            pre_compression_authority, compression_authority,
            "Rent authority should match the one in the compression info"
        );

        // Verify rent recipient matches pool PDA
        assert_eq!(
            pre_rent_sponsor, pool_pda,
            "Rent recipient should match the pool PDA"
        );
        // Get post-transaction state
        let post_token_account = rpc
            .get_account(*token_account_pubkey)
            .await
            .expect("Failed to get post-transaction token account")
            .expect("Token account should still exist after claim");

        // Parse post-transaction token account data
        let (post_compressed_token, _) = CToken::zero_copy_at(&post_token_account.data)
            .expect("Failed to deserialize post-transaction token account");

        // Get post-transaction compression info from meta.compression
        let post_compression = &post_compressed_token.compression;
        let post_last_claimed_slot = u64::from(post_compression.last_claimed_slot);
        println!("post_last_claimed_slot {}", post_last_claimed_slot);
        if !not_claimed_was_none {
            // Verify last_claimed_slot was updated
            assert!(
                post_last_claimed_slot > pre_last_claimed_slot,
                "last_claimed_slot should be updated to a higher slot {} {}",
                post_last_claimed_slot,
                pre_last_claimed_slot
            );
        } else {
            assert_eq!(
                post_last_claimed_slot, pre_last_claimed_slot,
                "last_claimed_slot should not be updated to a higher slot {} {}",
                post_last_claimed_slot, pre_last_claimed_slot
            );
        }
    }
    let post_pool_lamports = rpc
        .get_account(pool_pda)
        .await
        .expect("Failed to get post-transaction pool account")
        .expect("Pool PDA should exist after claim")
        .lamports;

    assert_eq!(
        post_pool_lamports,
        pre_pool_lamports + expected_lamports_claimed,
        "Pool PDA lamports should increase by claimed amount"
    );
}
