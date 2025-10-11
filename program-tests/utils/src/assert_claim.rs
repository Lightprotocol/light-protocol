use light_client::rpc::Rpc;
use light_ctoken_types::{
    state::{CToken, ZExtensionStruct, ZExtensionStructMut},
    COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
};
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
        assert_eq!(
            pre_token_account.data.len(),
            COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize
        );
        // Parse pre-transaction token account data
        let (mut pre_compressed_token, _) = CToken::zero_copy_at_mut(&mut pre_token_account.data)
            .expect("Failed to deserialize pre-transaction token account");

        // Find and extract pre-transaction compressible extension data
        let mut pre_last_claimed_slot = 0u64;
        let mut pre_compression_authority: Option<Pubkey> = None;
        let mut pre_rent_sponsor: Option<Pubkey> = None;
        let mut not_claimed_was_none = false;

        if let Some(extensions) = pre_compressed_token.extensions.as_mut() {
            for extension in extensions {
                if let ZExtensionStructMut::Compressible(compressible_ext) = extension {
                    pre_last_claimed_slot = u64::from(compressible_ext.last_claimed_slot);
                    // Check if compression_authority is set (non-zero)
                    pre_compression_authority =
                        if compressible_ext.compression_authority != [0u8; 32] {
                            Some(Pubkey::from(compressible_ext.compression_authority))
                        } else {
                            None
                        };
                    // Check if rent_sponsor is set (non-zero)
                    pre_rent_sponsor = if compressible_ext.rent_sponsor != [0u8; 32] {
                        Some(Pubkey::from(compressible_ext.rent_sponsor))
                    } else {
                        None
                    };
                    let current_slot = rpc.pre_context.as_ref().unwrap().get_sysvar::<Clock>().slot;
                    let base_lamports = rpc
                        .get_minimum_balance_for_rent_exemption(
                            COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize,
                        )
                        .await
                        .unwrap();
                    let lamports_result = compressible_ext.claim(
                        COMPRESSIBLE_TOKEN_ACCOUNT_SIZE,
                        current_slot,
                        pre_token_account.lamports,
                        base_lamports,
                    );
                    not_claimed_was_none = lamports_result.is_err();
                    if let Ok(Some(lamports)) = lamports_result {
                        expected_lamports_claimed += lamports;
                    }

                    break;
                }
            }
        } else {
            panic!("Token account should have compressible extension");
        }
        // Verify rent authority matches
        assert_eq!(
            pre_compression_authority,
            Some(compression_authority),
            "Rent authority should match the one in the extension"
        );

        // Verify rent recipient matches pool PDA
        assert_eq!(
            pre_rent_sponsor,
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
        let (post_compressed_token, _) = CToken::zero_copy_at(&post_token_account.data)
            .expect("Failed to deserialize post-transaction token account");

        // Find and extract post-transaction compressible extension data
        let mut post_last_claimed_slot = 0u64;

        if let Some(extensions) = post_compressed_token.extensions.as_ref() {
            for extension in extensions {
                if let ZExtensionStruct::Compressible(compressible_ext) = extension {
                    post_last_claimed_slot = u64::from(compressible_ext.last_claimed_slot);
                    println!("post_last_claimed_slot {}", post_last_claimed_slot);

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
