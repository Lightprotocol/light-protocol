use light_client::rpc::Rpc;
use light_ctoken_types::{
    state::{CompressedToken, ZExtensionStruct, ZExtensionStructMut},
    COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
};
use light_program_test::LightProgramTest;
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use solana_sdk::{clock::Clock, pubkey::Pubkey};

pub async fn assert_claim(
    rpc: &mut LightProgramTest,
    token_account_pubkeys: &[Pubkey],
    pool_pda: Pubkey,
    rent_authority: Pubkey,
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

        // Parse pre-transaction token account data
        let (mut pre_compressed_token, _) =
            CompressedToken::zero_copy_at_mut(&mut pre_token_account.data)
                .expect("Failed to deserialize pre-transaction token account");

        // Find and extract pre-transaction compressible extension data
        let mut pre_last_claimed_slot = 0u64;
        let mut pre_base_lamports_balance = 0u64;
        let mut pre_rent_authority: Option<Pubkey> = None;
        let mut pre_rent_recipient: Option<Pubkey> = None;
        let mut not_claimed_was_none = false;

        if let Some(extensions) = pre_compressed_token.extensions.as_mut() {
            for extension in extensions {
                if let ZExtensionStructMut::Compressible(compressible_ext) = extension {
                    pre_last_claimed_slot = u64::from(*compressible_ext.last_claimed_slot);
                    pre_base_lamports_balance = u64::from(*compressible_ext.base_lamports_balance);
                    pre_rent_authority = compressible_ext
                        .rent_authority
                        .as_ref()
                        .map(|k| Pubkey::from(**k));
                    pre_rent_recipient = compressible_ext
                        .rent_recipient
                        .as_ref()
                        .map(|k| Pubkey::from(**k));
                    let lamports = compressible_ext.claim(
                        COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
                        rpc.pre_context.as_ref().unwrap().get_sysvar::<Clock>().slot,
                        pre_token_account.lamports,
                    );
                    not_claimed_was_none = lamports.is_none();
                    expected_lamports_claimed += lamports.unwrap_or_default();

                    break;
                }
            }
        } else {
            panic!("Token account should have compressible extension");
        }
        // Verify rent authority matches
        assert_eq!(
            pre_rent_authority,
            Some(rent_authority),
            "Rent authority should match the one in the extension"
        );

        // Verify rent recipient matches pool PDA
        assert_eq!(
            pre_rent_recipient,
            Some(pool_pda),
            "Rent recipient should match the pool PDA"
        );
        // Get post-transaction state
        let post_token_account = rpc
            .get_account(*token_account_pubkey)
            .await
            .expect("Failed to get post-transaction token account")
            .expect("Token account should still exist after claim");

        // Parse post-transaction token account data
        let (post_compressed_token, _) = CompressedToken::zero_copy_at(&post_token_account.data)
            .expect("Failed to deserialize post-transaction token account");

        // Find and extract post-transaction compressible extension data
        let mut post_last_claimed_slot = 0u64;
        let mut post_base_lamports_balance = 0u64;

        if let Some(extensions) = post_compressed_token.extensions.as_ref() {
            for extension in extensions {
                if let ZExtensionStruct::Compressible(compressible_ext) = extension {
                    post_last_claimed_slot = u64::from(*compressible_ext.last_claimed_slot);
                    println!("post_last_claimed_slot {}", post_last_claimed_slot);
                    post_base_lamports_balance = u64::from(*compressible_ext.base_lamports_balance);

                    break;
                }
            }
        } else {
            panic!("Token account should still have compressible extension after claim");
        }
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

        // Verify base_lamports_balance remains unchanged
        assert_eq!(
            post_base_lamports_balance, pre_base_lamports_balance,
            "base_lamports_balance should remain unchanged after claim"
        );
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
