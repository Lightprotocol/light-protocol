use light_client::rpc::Rpc;
use light_ctoken_interface::state::{
    CToken, CompressedMint, ACCOUNT_TYPE_MINT, ACCOUNT_TYPE_TOKEN_ACCOUNT,
};
use light_program_test::LightProgramTest;
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use solana_sdk::{clock::Clock, pubkey::Pubkey};

/// Determines account type from account data.
/// - If account is exactly 165 bytes: CToken (legacy size without extensions)
/// - If account is > 165 bytes: read byte 165 for discriminator
/// - If account is < 165 bytes: invalid (returns None)
fn determine_account_type(data: &[u8]) -> Option<u8> {
    const ACCOUNT_TYPE_OFFSET: usize = 165;

    match data.len().cmp(&ACCOUNT_TYPE_OFFSET) {
        std::cmp::Ordering::Less => None,
        std::cmp::Ordering::Equal => Some(ACCOUNT_TYPE_TOKEN_ACCOUNT),
        std::cmp::Ordering::Greater => Some(data[ACCOUNT_TYPE_OFFSET]),
    }
}

/// Helper struct to hold extracted compression info for assertions
struct CompressionAssertData {
    last_claimed_slot: u64,
    compression_authority: Pubkey,
    rent_sponsor: Pubkey,
    claimable_lamports: Option<u64>,
    claim_failed: bool,
}

/// Extract compression info from pre-transaction account data (mutable, computes claim)
fn extract_pre_compression_mut(
    data: &mut [u8],
    account_size: u64,
    current_slot: u64,
    account_lamports: u64,
    base_lamports: u64,
    pubkey: &Pubkey,
) -> CompressionAssertData {
    let account_type = determine_account_type(data)
        .unwrap_or_else(|| panic!("Failed to determine account type for {}", pubkey));

    match account_type {
        ACCOUNT_TYPE_TOKEN_ACCOUNT => {
            let (mut ctoken, _) = CToken::zero_copy_at_mut(data)
                .unwrap_or_else(|e| panic!("Failed to parse ctoken account {}: {:?}", pubkey, e));
            let compressible = ctoken
                .get_compressible_extension_mut()
                .unwrap_or_else(|| panic!("CToken {} should have Compressible extension", pubkey));
            let compression = &mut compressible.info;
            let last_claimed_slot = u64::from(compression.last_claimed_slot);
            let compression_authority = Pubkey::from(compression.compression_authority);
            let rent_sponsor = Pubkey::from(compression.rent_sponsor);
            let lamports_result =
                compression.claim(account_size, current_slot, account_lamports, base_lamports);
            let claim_failed = lamports_result.is_err();
            let claimable_lamports = lamports_result.ok().flatten();
            CompressionAssertData {
                last_claimed_slot,
                compression_authority,
                rent_sponsor,
                claimable_lamports,
                claim_failed,
            }
        }
        ACCOUNT_TYPE_MINT => {
            let (mut cmint, _) = CompressedMint::zero_copy_at_mut(data)
                .unwrap_or_else(|e| panic!("Failed to parse cmint account {}: {:?}", pubkey, e));
            let compression = &mut cmint.base.compression;
            let last_claimed_slot = u64::from(compression.last_claimed_slot);
            let compression_authority = Pubkey::from(compression.compression_authority);
            let rent_sponsor = Pubkey::from(compression.rent_sponsor);
            let lamports_result =
                compression.claim(account_size, current_slot, account_lamports, base_lamports);
            let claim_failed = lamports_result.is_err();
            let claimable_lamports = lamports_result.ok().flatten();
            CompressionAssertData {
                last_claimed_slot,
                compression_authority,
                rent_sponsor,
                claimable_lamports,
                claim_failed,
            }
        }
        _ => panic!("Unknown account type {} for {}", account_type, pubkey),
    }
}

/// Extract post-transaction compression info (immutable)
fn extract_post_compression(data: &[u8], pubkey: &Pubkey) -> u64 {
    let account_type = determine_account_type(data)
        .unwrap_or_else(|| panic!("Failed to determine account type for {}", pubkey));

    match account_type {
        ACCOUNT_TYPE_TOKEN_ACCOUNT => {
            let (ctoken, _) = CToken::zero_copy_at(data)
                .unwrap_or_else(|e| panic!("Failed to parse ctoken account {}: {:?}", pubkey, e));
            let compressible = ctoken
                .get_compressible_extension()
                .unwrap_or_else(|| panic!("CToken {} should have Compressible extension", pubkey));
            u64::from(compressible.info.last_claimed_slot)
        }
        ACCOUNT_TYPE_MINT => {
            let (cmint, _) = CompressedMint::zero_copy_at(data)
                .unwrap_or_else(|e| panic!("Failed to parse cmint account {}: {:?}", pubkey, e));
            u64::from(cmint.base.compression.last_claimed_slot)
        }
        _ => panic!("Unknown account type {} for {}", account_type, pubkey),
    }
}

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
        // Must have > 165 bytes to include account_type discriminator
        assert!(
            pre_token_account.data.len() > 165,
            "Account must have > 165 bytes for CToken/CMint"
        );
        // Get account size and lamports before parsing (to avoid borrow conflicts)
        let account_size = pre_token_account.data.len() as u64;
        let account_lamports = pre_token_account.lamports;
        let current_slot = rpc.pre_context.as_ref().unwrap().get_sysvar::<Clock>().slot;
        let base_lamports = rpc
            .get_minimum_balance_for_rent_exemption(account_size as usize)
            .await
            .unwrap();

        // Extract compression info (handles both CToken and CMint)
        let pre_data = extract_pre_compression_mut(
            &mut pre_token_account.data,
            account_size,
            current_slot,
            account_lamports,
            base_lamports,
            token_account_pubkey,
        );

        if let Some(lamports) = pre_data.claimable_lamports {
            expected_lamports_claimed += lamports;
        }

        // Verify rent authority matches
        assert_eq!(
            pre_data.compression_authority, compression_authority,
            "Rent authority should match the one in the compression info"
        );

        // Verify rent recipient matches pool PDA
        assert_eq!(
            pre_data.rent_sponsor, pool_pda,
            "Rent recipient should match the pool PDA"
        );

        // Get post-transaction state
        let post_token_account = rpc
            .get_account(*token_account_pubkey)
            .await
            .expect("Failed to get post-transaction account")
            .expect("Account should still exist after claim");

        // Extract post-transaction compression info
        let post_last_claimed_slot =
            extract_post_compression(&post_token_account.data, token_account_pubkey);

        println!("post_last_claimed_slot {}", post_last_claimed_slot);
        if !pre_data.claim_failed {
            // Verify last_claimed_slot was updated
            assert!(
                post_last_claimed_slot > pre_data.last_claimed_slot,
                "last_claimed_slot should be updated to a higher slot {} {}",
                post_last_claimed_slot,
                pre_data.last_claimed_slot
            );
        } else {
            assert_eq!(
                post_last_claimed_slot, pre_data.last_claimed_slot,
                "last_claimed_slot should not be updated to a higher slot {} {}",
                post_last_claimed_slot, pre_data.last_claimed_slot
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
