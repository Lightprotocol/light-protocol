//! Greedy descending account selection algorithm.
//!
//! Selects the minimum number of compressed token accounts to satisfy a target amount.

use crate::{error::KoraLightError, types::CompressedTokenAccountInput};

/// Maximum number of compressed accounts per transaction.
pub const MAX_INPUT_ACCOUNTS: usize = 8;

/// Select compressed token accounts to satisfy the given amount.
///
/// Uses a greedy descending algorithm: sorts by amount (largest first),
/// then selects accounts until the cumulative amount meets or exceeds
/// the target. Returns up to `MAX_INPUT_ACCOUNTS` (8) accounts.
///
/// If `target_amount` is 0, returns an empty vec.
/// If total available balance is insufficient, returns an error.
pub fn select_input_accounts(
    accounts: &[CompressedTokenAccountInput],
    target_amount: u64,
) -> Result<Vec<CompressedTokenAccountInput>, KoraLightError> {
    if target_amount == 0 {
        return Ok(Vec::new());
    }

    if accounts.is_empty() {
        return Err(KoraLightError::NoCompressedAccounts);
    }

    // Sort by amount descending (largest first)
    let mut sorted: Vec<&CompressedTokenAccountInput> = accounts.iter().collect();
    sorted.sort_by(|a, b| b.amount.cmp(&a.amount));

    // Greedy selection: take accounts until we have enough
    let mut accumulated: u64 = 0;
    let mut count_needed: usize = 0;

    for acc in &sorted {
        count_needed += 1;
        accumulated = accumulated
            .checked_add(acc.amount)
            .ok_or(KoraLightError::ArithmeticOverflow)?;
        if accumulated >= target_amount {
            break;
        }
    }

    // Check if we have enough
    if accumulated < target_amount {
        return Err(KoraLightError::InsufficientBalance {
            needed: target_amount,
            available: accumulated,
        });
    }

    // Clamp to MAX_INPUT_ACCOUNTS
    let select_count = count_needed.min(MAX_INPUT_ACCOUNTS).min(sorted.len());

    // If we had to clamp, verify the top accounts still satisfy the target
    if count_needed > MAX_INPUT_ACCOUNTS {
        let top_sum: u64 = sorted[..select_count]
            .iter()
            .try_fold(0u64, |acc, a| acc.checked_add(a.amount))
            .ok_or(KoraLightError::ArithmeticOverflow)?;
        if top_sum < target_amount {
            return Err(KoraLightError::InsufficientBalance {
                needed: target_amount,
                available: top_sum,
            });
        }
    }

    Ok(sorted[..select_count]
        .iter()
        .map(|a| (*a).clone())
        .collect())
}

#[cfg(test)]
mod tests {
    use solana_pubkey::Pubkey;

    use super::*;

    fn make_account(amount: u64) -> CompressedTokenAccountInput {
        CompressedTokenAccountInput {
            hash: [0u8; 32],
            tree: Pubkey::default(),
            queue: Pubkey::default(),
            amount,
            leaf_index: 0,
            prove_by_index: false,
            root_index: 0,
            version: 0,
            owner: Pubkey::default(),
            mint: Pubkey::default(),
            delegate: None,
        }
    }

    #[test]
    fn test_select_exact_amount() {
        let accounts = vec![make_account(500), make_account(300), make_account(200)];
        let selected = select_input_accounts(&accounts, 500).unwrap();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].amount, 500);
    }

    #[test]
    fn test_select_multiple_accounts() {
        let accounts = vec![make_account(300), make_account(200), make_account(100)];
        let selected = select_input_accounts(&accounts, 450).unwrap();
        assert_eq!(selected.len(), 2);
        // Should pick largest first
        assert_eq!(selected[0].amount, 300);
        assert_eq!(selected[1].amount, 200);
    }

    #[test]
    fn test_select_all_accounts() {
        let accounts = vec![make_account(100), make_account(100), make_account(100)];
        let selected = select_input_accounts(&accounts, 300).unwrap();
        assert_eq!(selected.len(), 3);
        let total: u64 = selected.iter().map(|a| a.amount).sum();
        assert_eq!(total, 300);
    }

    #[test]
    fn test_select_insufficient_balance() {
        let accounts = vec![make_account(100), make_account(50)];
        let result = select_input_accounts(&accounts, 200);
        assert!(matches!(
            result,
            Err(KoraLightError::InsufficientBalance { .. })
        ));
    }

    #[test]
    fn test_select_zero_amount() {
        let accounts = vec![make_account(100)];
        let selected = select_input_accounts(&accounts, 0).unwrap();
        assert!(selected.is_empty());
    }

    #[test]
    fn test_select_empty_accounts() {
        let result = select_input_accounts(&[], 100);
        assert!(matches!(result, Err(KoraLightError::NoCompressedAccounts)));
    }

    #[test]
    fn test_select_respects_max_limit() {
        // 10 accounts of 100 each, target 900: top 8 = 800 < 900 → InsufficientBalance
        let accounts: Vec<_> = (0..10).map(|_| make_account(100)).collect();
        let result = select_input_accounts(&accounts, 900);
        assert!(matches!(
            result,
            Err(KoraLightError::InsufficientBalance {
                needed: 900,
                available: 800,
            })
        ));
    }

    #[test]
    fn test_select_max_limit_sufficient() {
        // 10 accounts of 100 each, target 800: top 8 = 800 >= 800 → success
        let accounts: Vec<_> = (0..10).map(|_| make_account(100)).collect();
        let selected = select_input_accounts(&accounts, 800).unwrap();
        assert_eq!(selected.len(), MAX_INPUT_ACCOUNTS);
        let total: u64 = selected.iter().map(|a| a.amount).sum();
        assert_eq!(total, 800);
    }

    #[test]
    fn test_select_greedy_descending() {
        let accounts = vec![
            make_account(10),
            make_account(1000),
            make_account(50),
            make_account(500),
        ];
        let selected = select_input_accounts(&accounts, 1200).unwrap();
        // Should pick 1000 + 500 = 1500 >= 1200
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].amount, 1000);
        assert_eq!(selected[1].amount, 500);
    }
}
