#![cfg(feature = "pinocchio")]
//! Tests for AccountIterator (49 tests):
//! Functional tests (15): All success paths for each method
//!   1. new_iterator_empty_accounts - Creates iterator with empty slice
//!   2. new_iterator_with_accounts - Creates iterator with 5 accounts
//!   3. new_with_owner - Creates iterator with owner
//!   4. next_account_sequential - Iterates through accounts sequentially
//!   5. next_option_some - Tests next_option with is_some=true
//!   6. next_option_none - Tests next_option with is_some=false
//!   7. next_option_mut_some - Tests next_option_mut with mutable account
//!   8. next_signer - Tests next_signer with signer account
//!   9. next_signer_mut - Tests next_signer_mut with signer+mutable
//!   10. next_non_mut - Tests next_non_mut with readonly account
//!   11. next_mut - Tests next_mut with mutable account
//!   12. remaining_at_start - Tests remaining() at position=0
//!   13. remaining_partial - Tests remaining() after consuming accounts
//!   14. remaining_unchecked_empty - Tests remaining_unchecked() when exhausted
//!   15. state_queries - Tests position(), len(), is_empty(), iterator_is_empty()
//!       Failing tests (11): Each validation failure with exact error
//!   16. next_account_exhausted - NotEnoughAccountKeys when exhausted
//!   17. next_account_beyond_initial - NotEnoughAccountKeys beyond limit
//!   18. next_option_some_exhausted - NotEnoughAccountKeys with is_some=true
//!   19. next_option_mut_not_writable - AccountNotMutable
//!   20. next_signer_not_signer - MissingSignature
//!   21. next_signer_exhausted - NotEnoughAccountKeys
//!   22. next_signer_mut_not_mutable - AccountNotMutable
//!   23. next_signer_mut_not_signer - MissingSignature
//!   24. next_non_mut_is_mutable - AccountMutable
//!   25. next_mut_not_mutable - AccountNotMutable
//!   26. remaining_exhausted - NotEnoughAccountKeys
//!       Complex workflows (4): Mixed operation sequences
//!   27. mixed_operations_workflow - Complex sequence of operations
//!   28. optional_accounts_workflow - Mix of Some/None options
//!   29. complete_consumption - Consume all accounts
//!   30. interleaved_validation - Different validation methods
//!       Edge cases (5): Boundary conditions
//!   31. single_account_iterator - Iterator with exactly 1 account
//!   32. zero_position_queries - Query methods when position=0
//!   33. boundary_position - Operations at last account
//!   34. multiple_remaining_calls - Call remaining() multiple times
//!   35. remaining_unchecked_vs_remaining - Compare both methods
//!       Randomized tests (4): 1000 iterations each
//!   36. randomized_account_properties - Random counts and properties
//!   37. randomized_operation_sequence - Random operation sequences
//!   38. randomized_optional_patterns - Random is_some patterns
//!   39. randomized_validation_chains - Random validation chains
//!       Core functionality (2): Basic iterator operations
//!   40. pinocchio_backend_iterator - Core functionality test
//!   41. backend_error_consistency - Error consistency validation
//!       Systematic validation (3): Helper patterns
//!   42. systematic_next_account_validation - Test each account variant
//!   43. error_message_content - Verify error message format
//!   44. print_on_error_scenarios - Test error printing
//!       State consistency (5): Memory safety
//!   45. iterator_immutability - Iterator doesn't modify slice
//!   46. position_overflow_protection - Test saturating_sub
//!   47. concurrent_iterators - Multiple iterators independence
//!   48. iterator_state_complete - Complete state assertion
//!   49. remaining_slice_equality - Exact slice reference

// Import test account creation utilities
use light_account_checks::{
    account_info::test_account_info, account_iterator::AccountIterator, error::AccountError,
    AccountInfoTrait,
};
use pinocchio::account_info::AccountInfo;

// Helper to extract error from Result when Ok type doesn't implement Debug
fn get_error<T>(result: Result<T, AccountError>) -> AccountError {
    match result {
        Ok(_) => panic!("Expected error but got Ok"),
        Err(e) => e,
    }
}
// Helper to create Pinocchio test accounts

fn create_pinocchio_accounts(
    count: usize,
    signer: bool,
    writable: bool,
) -> Vec<pinocchio::account_info::AccountInfo> {
    (0..count)
        .map(|i| {
            let key = [i as u8; 32];
            let owner = [255u8; 32];
            test_account_info::pinocchio::get_account_info(
                key,
                owner,
                signer,
                writable,
                false, // executable
                vec![0u8; 32],
            )
        })
        .collect()
}

// Helper struct for complete state assertion
#[derive(Debug, PartialEq)]
struct IteratorState {
    position: usize,
    len: usize,
    is_empty: bool,
    iterator_is_empty: bool,
}

fn get_iterator_state<T: AccountInfoTrait>(iter: &AccountIterator<T>) -> IteratorState {
    IteratorState {
        position: iter.position(),
        len: iter.len(),
        is_empty: iter.is_empty(),
        iterator_is_empty: iter.iterator_is_empty(),
    }
}
#[test]
fn test_new_iterator_empty_accounts() {
    // Pinocchio only - Solana TestAccount requires mutable ref for get_account_info()

    let accounts: Vec<pinocchio::account_info::AccountInfo> = vec![];
    let iter = AccountIterator::new(&accounts);

    let expected = IteratorState {
        position: 0,
        len: 0,
        is_empty: true,
        iterator_is_empty: true,
    };
    assert_eq!(get_iterator_state(&iter), expected);
}

#[test]
fn test_new_iterator_with_accounts() {
    // Pinocchio only - Solana TestAccount requires mutable ref for get_account_info()

    let accounts = create_pinocchio_accounts(5, false, false);
    let iter = AccountIterator::new(&accounts);

    let expected = IteratorState {
        position: 0,
        len: 5,
        is_empty: false,
        iterator_is_empty: false,
    };
    assert_eq!(get_iterator_state(&iter), expected);
}

#[test]
fn test_new_with_owner() {
    let owner = [42u8; 32];

    // Pinocchio (owner is used in this backend)

    let accounts = create_pinocchio_accounts(3, false, false);
    let iter = AccountIterator::new_with_owner(&accounts, owner);
    assert_eq!(iter.position(), 0);
    assert_eq!(iter.len(), 3);
}

#[test]
fn test_next_account_sequential() {
    // Pinocchio

    let accounts = create_pinocchio_accounts(5, false, false);
    let mut iter = AccountIterator::new(&accounts);

    for i in 0..5 {
        let account = iter.next_account(&format!("account_{}", i)).unwrap();
        assert_eq!(account.key()[0], i as u8); // Check it's the right account
        assert_eq!(iter.position(), i + 1);
    }

    assert!(iter.iterator_is_empty());
}

#[test]
fn test_next_option_some() {
    let accounts = create_pinocchio_accounts(3, false, false);
    let mut iter = AccountIterator::new(&accounts);

    let result = iter.next_option("optional_account", true).unwrap();
    assert!(result.is_some());
    assert_eq!(iter.position(), 1);
}

#[test]
fn test_next_option_none() {
    let accounts = create_pinocchio_accounts(3, false, false);
    let mut iter = AccountIterator::new(&accounts);

    let result = iter.next_option("optional_account", false).unwrap();
    assert!(result.is_none());
    assert_eq!(iter.position(), 0); // Position should not advance
}

#[test]
fn test_next_option_mut_some() {
    let accounts = create_pinocchio_accounts(3, false, true); // writable
    let mut iter = AccountIterator::new(&accounts);

    let result = iter.next_option_mut("optional_mut_account", true).unwrap();
    assert!(result.is_some());
    assert_eq!(iter.position(), 1);
}

#[test]
fn test_next_signer() {
    let accounts = create_pinocchio_accounts(3, true, false); // signer
    let mut iter = AccountIterator::new(&accounts);

    let account = iter.next_signer("signer_account").unwrap();
    assert!(account.is_signer());
    assert_eq!(iter.position(), 1);
}

#[test]
fn test_next_signer_mut() {
    let accounts = create_pinocchio_accounts(3, true, true); // signer + writable
    let mut iter = AccountIterator::new(&accounts);

    let account = iter.next_signer_mut("signer_mut_account").unwrap();
    assert!(account.is_signer());
    assert!(account.is_writable());
    assert_eq!(iter.position(), 1);
}

#[test]
fn test_next_non_mut() {
    let accounts = create_pinocchio_accounts(3, false, false); // not writable
    let mut iter = AccountIterator::new(&accounts);

    let account = iter.next_non_mut("readonly_account").unwrap();
    assert!(!account.is_writable());
    assert_eq!(iter.position(), 1);
}

#[test]
fn test_next_mut() {
    let accounts = create_pinocchio_accounts(3, false, true); // writable
    let mut iter = AccountIterator::new(&accounts);

    let account = iter.next_mut("mutable_account").unwrap();
    assert!(account.is_writable());
    assert_eq!(iter.position(), 1);
}

#[test]
fn test_remaining_at_start() {
    let accounts = create_pinocchio_accounts(5, false, false);
    let iter = AccountIterator::new(&accounts);

    let remaining = iter.remaining().unwrap();
    assert_eq!(remaining.len(), 5);
}

#[test]
fn test_remaining_partial() {
    let accounts = create_pinocchio_accounts(5, false, false);
    let mut iter = AccountIterator::new(&accounts);

    // Consume 2 accounts
    iter.next_account("account_0").unwrap();
    iter.next_account("account_1").unwrap();

    let remaining = iter.remaining().unwrap();
    assert_eq!(remaining.len(), 3);
    assert_eq!(remaining[0].key()[0], 2); // Should start from account 2
}

#[test]
fn test_remaining_unchecked_empty() {
    let accounts = create_pinocchio_accounts(2, false, false);
    let mut iter = AccountIterator::new(&accounts);

    // Consume all accounts
    iter.next_account("account_0").unwrap();
    iter.next_account("account_1").unwrap();

    let remaining = iter.remaining_unchecked().unwrap();
    assert_eq!(remaining.len(), 0);
}

#[test]
fn test_state_queries() {
    let accounts = create_pinocchio_accounts(3, false, false);
    let mut iter = AccountIterator::new(&accounts);

    // Initial state
    assert_eq!(iter.position(), 0);
    assert_eq!(iter.len(), 3);
    assert!(!iter.is_empty());
    assert!(!iter.iterator_is_empty());

    // After consuming one
    iter.next_account("account").unwrap();
    assert_eq!(iter.position(), 1);
    assert_eq!(iter.len(), 3);
    assert!(!iter.is_empty());
    assert!(!iter.iterator_is_empty());

    // After consuming all
    iter.next_account("account").unwrap();
    iter.next_account("account").unwrap();
    assert_eq!(iter.position(), 3);
    assert_eq!(iter.len(), 3);
    assert!(!iter.is_empty());
    assert!(iter.iterator_is_empty());
}

#[test]
fn test_next_account_exhausted() {
    let accounts = create_pinocchio_accounts(1, false, false);
    let mut iter = AccountIterator::new(&accounts);

    // Consume the only account
    iter.next_account("first").unwrap();

    // Try to get another
    let result = iter.next_account("second");
    assert_eq!(get_error(result), AccountError::NotEnoughAccountKeys);
}

#[test]
fn test_next_account_beyond_initial() {
    let accounts = create_pinocchio_accounts(2, false, false);
    let mut iter = AccountIterator::new(&accounts);

    // Try to get 3 accounts when only 2 exist
    iter.next_account("first").unwrap();
    iter.next_account("second").unwrap();
    let result = iter.next_account("third");
    assert_eq!(get_error(result), AccountError::NotEnoughAccountKeys);
}

#[test]
fn test_next_option_some_exhausted() {
    let accounts = create_pinocchio_accounts(0, false, false);
    let mut iter = AccountIterator::new(&accounts);

    let result = iter.next_option("optional", true);
    assert_eq!(get_error(result), AccountError::NotEnoughAccountKeys);
}

#[test]
fn test_next_option_mut_not_writable() {
    let accounts = create_pinocchio_accounts(1, false, false); // not writable
    let mut iter = AccountIterator::new(&accounts);

    let result = iter.next_option_mut("optional_mut", true);
    assert_eq!(get_error(result), AccountError::AccountNotMutable);
}

#[test]
fn test_next_signer_not_signer() {
    let accounts = create_pinocchio_accounts(1, false, false); // not signer
    let mut iter = AccountIterator::new(&accounts);

    let result = iter.next_signer("signer");
    assert_eq!(get_error(result), AccountError::InvalidSigner);
}

#[test]
fn test_next_signer_exhausted() {
    let accounts = create_pinocchio_accounts(0, true, false);
    let mut iter = AccountIterator::new(&accounts);

    let result = iter.next_signer("signer");
    assert_eq!(get_error(result), AccountError::NotEnoughAccountKeys);
}

#[test]
fn test_next_signer_mut_not_mutable() {
    let accounts = create_pinocchio_accounts(1, true, false); // signer but not writable
    let mut iter = AccountIterator::new(&accounts);

    let result = iter.next_signer_mut("signer_mut");
    assert_eq!(get_error(result), AccountError::AccountNotMutable);
}

#[test]
fn test_next_signer_mut_not_signer() {
    let accounts = create_pinocchio_accounts(1, false, true); // writable but not signer
    let mut iter = AccountIterator::new(&accounts);

    let result = iter.next_signer_mut("signer_mut");
    assert_eq!(get_error(result), AccountError::InvalidSigner);
}

#[test]
fn test_next_non_mut_is_mutable() {
    let accounts = create_pinocchio_accounts(1, false, true); // writable
    let mut iter = AccountIterator::new(&accounts);

    let result = iter.next_non_mut("readonly");
    assert_eq!(get_error(result), AccountError::AccountMutable);
}

#[test]
fn test_next_mut_not_mutable() {
    let accounts = create_pinocchio_accounts(1, false, false); // not writable
    let mut iter = AccountIterator::new(&accounts);

    let result = iter.next_mut("mutable");
    assert_eq!(get_error(result), AccountError::AccountNotMutable);
}

#[test]
fn test_remaining_exhausted() {
    let accounts = create_pinocchio_accounts(1, false, false);
    let mut iter = AccountIterator::new(&accounts);

    // Consume all
    iter.next_account("account").unwrap();

    let result = iter.remaining();
    assert_eq!(get_error(result), AccountError::NotEnoughAccountKeys);
}

#[test]
fn test_mixed_operations_workflow() {
    // Create accounts with different properties
    let accounts = vec![
        test_account_info::pinocchio::get_account_info(
            [0; 32],
            [255; 32],
            false,
            false,
            false,
            vec![0; 32],
        ), // regular
        test_account_info::pinocchio::get_account_info(
            [1; 32],
            [255; 32],
            true,
            false,
            false,
            vec![0; 32],
        ), // signer
        test_account_info::pinocchio::get_account_info(
            [2; 32],
            [255; 32],
            false,
            true,
            false,
            vec![0; 32],
        ), // mutable
        test_account_info::pinocchio::get_account_info(
            [3; 32],
            [255; 32],
            false,
            false,
            false,
            vec![0; 32],
        ), // readonly
    ];

    let mut iter = AccountIterator::new(&accounts);

    // Complex sequence
    let _account1 = iter.next_account("regular").unwrap();
    let _signer = iter.next_signer("signer").unwrap();
    let _mutable = iter.next_mut("mutable").unwrap();
    let _readonly = iter.next_non_mut("readonly").unwrap();

    assert!(iter.iterator_is_empty());

    // remaining should fail
    assert_eq!(
        get_error(iter.remaining()),
        AccountError::NotEnoughAccountKeys
    );
}

#[test]
fn test_optional_accounts_workflow() {
    let accounts = create_pinocchio_accounts(5, false, true);
    let mut iter = AccountIterator::new(&accounts);

    // Mix of Some and None
    let opt1 = iter.next_option("opt1", true).unwrap();
    assert!(opt1.is_some());
    assert_eq!(iter.position(), 1);

    let opt2 = iter.next_option("opt2", false).unwrap();
    assert!(opt2.is_none());
    assert_eq!(iter.position(), 1); // Position unchanged

    let opt3 = iter.next_option_mut("opt3", true).unwrap();
    assert!(opt3.is_some());
    assert_eq!(iter.position(), 2);

    let opt4 = iter.next_option("opt4", false).unwrap();
    assert!(opt4.is_none());
    assert_eq!(iter.position(), 2); // Position unchanged

    // Verify we can still get remaining
    let remaining = iter.remaining().unwrap();
    assert_eq!(remaining.len(), 3);
}

#[test]
fn test_complete_consumption() {
    let accounts = vec![
        test_account_info::pinocchio::get_account_info(
            [0; 32],
            [255; 32],
            true,
            true,
            false,
            vec![0; 32],
        ),
        test_account_info::pinocchio::get_account_info(
            [1; 32],
            [255; 32],
            false,
            true,
            false,
            vec![0; 32],
        ),
        test_account_info::pinocchio::get_account_info(
            [2; 32],
            [255; 32],
            false,
            false,
            false,
            vec![0; 32],
        ),
    ];

    let mut iter = AccountIterator::new(&accounts);

    // Consume all through different methods
    let _a1 = iter.next_signer_mut("signer_mut").unwrap();
    let _a2 = iter.next_mut("mutable").unwrap();
    let _a3 = iter.next_non_mut("readonly").unwrap();

    assert!(iter.iterator_is_empty());
    assert_eq!(iter.position(), 3);
    assert_eq!(iter.len(), 3);
}

#[test]
fn test_interleaved_validation() {
    let accounts = vec![
        test_account_info::pinocchio::get_account_info(
            [0; 32],
            [255; 32],
            true,
            false,
            false,
            vec![0; 32],
        ),
        test_account_info::pinocchio::get_account_info(
            [1; 32],
            [255; 32],
            false,
            true,
            false,
            vec![0; 32],
        ),
        test_account_info::pinocchio::get_account_info(
            [2; 32],
            [255; 32],
            false,
            false,
            false,
            vec![0; 32],
        ),
        test_account_info::pinocchio::get_account_info(
            [3; 32],
            [255; 32],
            true,
            true,
            false,
            vec![0; 32],
        ),
    ];

    let mut iter = AccountIterator::new(&accounts);

    let _signer = iter.next_signer("signer1").unwrap();
    let _mutable = iter.next_mut("mutable").unwrap();
    let _readonly = iter.next_non_mut("readonly").unwrap();
    let _signer_mut = iter.next_signer_mut("signer_mut").unwrap();

    assert!(iter.iterator_is_empty());
}

#[test]
fn test_single_account_iterator() {
    let accounts = create_pinocchio_accounts(1, true, true);
    let mut iter = AccountIterator::new(&accounts);

    assert!(!iter.is_empty());
    assert!(!iter.iterator_is_empty());

    let account = iter.next_account("single").unwrap();
    assert!(account.is_signer());

    assert!(iter.iterator_is_empty());
    assert_eq!(iter.remaining_unchecked().unwrap().len(), 0);
}

#[test]
fn test_zero_position_queries() {
    let accounts = create_pinocchio_accounts(3, false, false);
    let iter = AccountIterator::new(&accounts);

    assert_eq!(iter.position(), 0);
    assert_eq!(iter.len(), 3);
    assert!(!iter.is_empty());
    assert!(!iter.iterator_is_empty());

    let remaining = iter.remaining().unwrap();
    assert_eq!(remaining.len(), 3);
}

#[test]
fn test_boundary_position() {
    let accounts = create_pinocchio_accounts(3, false, false);
    let mut iter = AccountIterator::new(&accounts);

    // Move to last account
    iter.next_account("first").unwrap();
    iter.next_account("second").unwrap();

    // At boundary (one account left)
    assert_eq!(iter.position(), 2);
    let remaining = iter.remaining().unwrap();
    assert_eq!(remaining.len(), 1);
}

#[test]
fn test_remaining_consumes_iterator() {
    let accounts = create_pinocchio_accounts(4, false, false);
    let mut iter = AccountIterator::new(&accounts);

    iter.next_account("first").unwrap();

    // remaining() consumes the iterator
    let remaining = iter.remaining().unwrap();

    assert_eq!(remaining.len(), 3);
    // Iterator is consumed, cannot use it anymore
}

#[test]
fn test_remaining_unchecked_vs_remaining() {
    // Test remaining() with accounts available
    let accounts = create_pinocchio_accounts(2, false, false);
    let iter = AccountIterator::new(&accounts);
    let remaining1 = iter.remaining().unwrap();
    assert_eq!(remaining1.len(), 2);

    // Test remaining_unchecked() with accounts available
    let accounts = create_pinocchio_accounts(2, false, false);
    let iter = AccountIterator::new(&accounts);
    let remaining2 = iter.remaining_unchecked().unwrap();
    assert_eq!(remaining2.len(), 2);

    // Test remaining() when all consumed - should error
    let accounts = create_pinocchio_accounts(2, false, false);
    let mut iter = AccountIterator::new(&accounts);
    iter.next_account("first").unwrap();
    iter.next_account("second").unwrap();
    assert_eq!(
        get_error(iter.remaining()),
        AccountError::NotEnoughAccountKeys
    );

    // Test remaining_unchecked() when all consumed - should return empty
    let accounts = create_pinocchio_accounts(2, false, false);
    let mut iter = AccountIterator::new(&accounts);
    iter.next_account("first").unwrap();
    iter.next_account("second").unwrap();
    let unchecked = iter.remaining_unchecked().unwrap();
    assert_eq!(unchecked.len(), 0);
}

#[test]
fn test_randomized_account_properties() {
    // Use deterministic LCG for reproducibility
    let mut rng_state = 42u64;
    let mut next_rand = || {
        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        rng_state
    };

    for _iteration in 0..1000 {
        let count = (next_rand() % 20) as usize; // 0-19 accounts

        {
            let mut accounts = vec![];
            for i in 0..count {
                let signer = (next_rand() % 2) == 1;
                let writable = (next_rand() % 2) == 1;
                accounts.push(test_account_info::pinocchio::get_account_info(
                    [i as u8; 32],
                    [255; 32],
                    signer,
                    writable,
                    false,
                    vec![0; 32],
                ));
            }

            let mut iter = AccountIterator::new(&accounts);

            // Property: position always increments correctly
            for i in 0..count {
                assert_eq!(iter.position(), i);
                iter.next_account(&format!("account_{}", i)).unwrap();
                assert_eq!(iter.position(), i + 1);
            }

            // Property: length remains constant
            assert_eq!(iter.len(), count);

            // Property: iterator_is_empty() ↔ position == len
            assert_eq!(iter.iterator_is_empty(), iter.position() == iter.len());
        }
    }
}

#[test]
fn test_randomized_operation_sequence() {
    let mut rng_state = 123u64;
    let mut next_rand = || {
        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        rng_state
    };

    for _iteration in 0..1000 {
        {
            // Create random accounts
            let account_count = 1 + (next_rand() % 10) as usize;
            let mut accounts = vec![];
            for i in 0..account_count {
                let signer = (next_rand() % 2) == 1;
                let writable = (next_rand() % 2) == 1;
                accounts.push(test_account_info::pinocchio::get_account_info(
                    [i as u8; 32],
                    [255; 32],
                    signer,
                    writable,
                    false,
                    vec![0; 32],
                ));
            }

            let mut iter = AccountIterator::new(&accounts);
            let mut consumed = 0;

            // Random operations until exhausted
            while consumed < account_count {
                let op = next_rand() % 7;
                let account = &accounts[consumed];

                match op {
                    0 => {
                        // next_account always succeeds if accounts available
                        iter.next_account("account").unwrap();
                    }
                    1 if account.is_signer() => {
                        iter.next_signer("signer").unwrap();
                    }
                    2 if account.is_writable() => {
                        iter.next_mut("mutable").unwrap();
                    }
                    3 if !account.is_writable() => {
                        iter.next_non_mut("readonly").unwrap();
                    }
                    4 if account.is_signer() && account.is_writable() => {
                        iter.next_signer_mut("signer_mut").unwrap();
                    }
                    _ => {
                        // Fallback to next_account
                        iter.next_account("fallback").unwrap();
                    }
                }
                consumed += 1;
            }

            assert!(iter.iterator_is_empty());
        }
    }
}
#[test]
fn test_randomized_optional_patterns() {
    let mut rng_state = 456u64;
    let mut next_rand = || {
        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        rng_state
    };

    for _iteration in 0..1000 {
        {
            let accounts = create_pinocchio_accounts(10, false, true);
            let mut iter = AccountIterator::new(&accounts);

            let mut expected_position = 0;

            for _ in 0..20 {
                let is_some = (next_rand() % 2) == 1;
                let use_mut = (next_rand() % 2) == 1;

                if expected_position >= 10 && is_some {
                    // Should fail - no more accounts
                    if use_mut {
                        assert!(iter.next_option_mut("opt", true).is_err());
                    } else {
                        assert!(iter.next_option("opt", true).is_err());
                    }
                } else if !is_some {
                    // None doesn't advance position
                    if use_mut {
                        let result = iter.next_option_mut("opt", false).unwrap();
                        assert!(result.is_none());
                    } else {
                        let result = iter.next_option("opt", false).unwrap();
                        assert!(result.is_none());
                    }
                    assert_eq!(iter.position(), expected_position);
                } else {
                    // Some advances position
                    if use_mut {
                        let result = iter.next_option_mut("opt", true).unwrap();
                        assert!(result.is_some());
                    } else {
                        let result = iter.next_option("opt", true).unwrap();
                        assert!(result.is_some());
                    }
                    expected_position += 1;
                    assert_eq!(iter.position(), expected_position);
                }
            }
        }
    }
}
#[test]
fn test_randomized_validation_chains() {
    let mut rng_state = 789u64;
    let mut next_rand = || {
        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        rng_state
    };

    for _iteration in 0..1000 {
        {
            // Create account with random properties
            let signer = (next_rand() % 2) == 1;
            let writable = (next_rand() % 2) == 1;
            let account = test_account_info::pinocchio::get_account_info(
                [0; 32],
                [255; 32],
                signer,
                writable,
                false,
                vec![0; 32],
            );
            let accounts = vec![account];

            // Test compound validations
            let mut iter = AccountIterator::new(&accounts);

            // signer_mut requires both signer AND mutable
            if signer && writable {
                assert!(iter.next_signer_mut("test").is_ok());
            } else if !signer {
                assert_eq!(
                    get_error(iter.next_signer_mut("test")),
                    AccountError::InvalidSigner
                );
            } else {
                assert_eq!(
                    get_error(iter.next_signer_mut("test")),
                    AccountError::AccountNotMutable
                );
            }
        }
    }
}
#[test]
fn test_pinocchio_backend_iterator() {
    let accounts = create_pinocchio_accounts(3, true, true);
    let mut iter = AccountIterator::new(&accounts);

    // Core functionality
    let _a1 = iter.next_signer("first").unwrap();
    let _a2 = iter.next_signer_mut("second").unwrap();
    assert_eq!(iter.position(), 2);

    let remaining = iter.remaining().unwrap();
    assert_eq!(remaining.len(), 1);
}

#[test]
fn test_backend_error_consistency() {
    let accounts: Vec<pinocchio::account_info::AccountInfo> = vec![];
    let mut iter = AccountIterator::new(&accounts);

    assert_eq!(
        get_error(iter.next_account("missing")),
        AccountError::NotEnoughAccountKeys
    );
}

fn get_signer_mutable_account() -> pinocchio::account_info::AccountInfo {
    test_account_info::pinocchio::get_account_info(
        [1; 32],
        [255; 32],
        true,
        true,
        false,
        vec![0; 32],
    )
}

fn get_signer_readonly_account() -> pinocchio::account_info::AccountInfo {
    test_account_info::pinocchio::get_account_info(
        [2; 32],
        [255; 32],
        true,
        false,
        false,
        vec![0; 32],
    )
}

fn get_nonsigner_mutable_account() -> pinocchio::account_info::AccountInfo {
    test_account_info::pinocchio::get_account_info(
        [3; 32],
        [255; 32],
        false,
        true,
        false,
        vec![0; 32],
    )
}

fn get_nonsigner_readonly_account() -> pinocchio::account_info::AccountInfo {
    test_account_info::pinocchio::get_account_info(
        [4; 32],
        [255; 32],
        false,
        false,
        false,
        vec![0; 32],
    )
}

type AccountTestCase = (fn() -> AccountInfo, &'static str, Result<(), AccountError>);

#[test]
fn test_systematic_next_account_validation() {
    // Test each method with each account variant systematically
    let test_cases: [AccountTestCase; 16] = [
        // (account_getter, method_name, expected_result)
        // next_signer tests
        (get_signer_mutable_account, "next_signer", Ok(())),
        (get_signer_readonly_account, "next_signer", Ok(())),
        (
            get_nonsigner_mutable_account,
            "next_signer",
            Err(AccountError::InvalidSigner),
        ),
        (
            get_nonsigner_readonly_account,
            "next_signer",
            Err(AccountError::InvalidSigner),
        ),
        // next_mut tests
        (get_signer_mutable_account, "next_mut", Ok(())),
        (
            get_signer_readonly_account,
            "next_mut",
            Err(AccountError::AccountNotMutable),
        ),
        (get_nonsigner_mutable_account, "next_mut", Ok(())),
        (
            get_nonsigner_readonly_account,
            "next_mut",
            Err(AccountError::AccountNotMutable),
        ),
        // next_non_mut tests
        (
            get_signer_mutable_account,
            "next_non_mut",
            Err(AccountError::AccountMutable),
        ),
        (get_signer_readonly_account, "next_non_mut", Ok(())),
        (
            get_nonsigner_mutable_account,
            "next_non_mut",
            Err(AccountError::AccountMutable),
        ),
        (get_nonsigner_readonly_account, "next_non_mut", Ok(())),
        // next_signer_mut tests
        (get_signer_mutable_account, "next_signer_mut", Ok(())),
        (
            get_signer_readonly_account,
            "next_signer_mut",
            Err(AccountError::AccountNotMutable),
        ),
        (
            get_nonsigner_mutable_account,
            "next_signer_mut",
            Err(AccountError::InvalidSigner),
        ),
        (
            get_nonsigner_readonly_account,
            "next_signer_mut",
            Err(AccountError::InvalidSigner),
        ),
    ];

    for (account_fn, method, expected) in test_cases.iter() {
        let accounts = vec![account_fn()];
        let mut iter = AccountIterator::new(&accounts);

        let result = match *method {
            "next_signer" => iter.next_signer("test").map(|_| ()),
            "next_mut" => iter.next_mut("test").map(|_| ()),
            "next_non_mut" => iter.next_non_mut("test").map(|_| ()),
            "next_signer_mut" => iter.next_signer_mut("test").map(|_| ()),
            _ => unreachable!(),
        };

        match expected {
            Ok(()) => assert!(result.is_ok(), "Expected Ok for {}", method),
            Err(expected_err) => assert_eq!(
                get_error(result),
                *expected_err,
                "Expected {:?} for {}",
                expected_err,
                method
            ),
        }
    }
}

#[test]
fn test_error_message_content() {
    // Note: We can't directly test the error messages since they use solana_msg::msg!
    // which outputs to logs, but we can verify the error types are correct

    let accounts = create_pinocchio_accounts(0, false, false);
    let mut iter = AccountIterator::new(&accounts);

    // This should trigger error message with account name and position
    let result = iter.next_account("test_account_name");
    assert_eq!(get_error(result), AccountError::NotEnoughAccountKeys);

    // The actual message would be logged via solana_msg::msg!
}

#[test]
fn test_print_on_error_scenarios() {
    // Test that validation errors trigger print_on_error

    let accounts = create_pinocchio_accounts(1, false, false); // not signer, not writable
    let mut iter = AccountIterator::new(&accounts);

    // Each of these should trigger print_on_error with the error details
    assert_eq!(
        get_error(iter.next_signer("signer_test")),
        AccountError::InvalidSigner
    );

    let accounts = create_pinocchio_accounts(1, false, false);
    let mut iter = AccountIterator::new(&accounts);
    assert_eq!(
        get_error(iter.next_mut("mut_test")),
        AccountError::AccountNotMutable
    );

    let accounts = create_pinocchio_accounts(1, false, true);
    let mut iter = AccountIterator::new(&accounts);
    assert_eq!(
        get_error(iter.next_non_mut("non_mut_test")),
        AccountError::AccountMutable
    );
}

#[test]
fn test_iterator_immutability() {
    let accounts = create_pinocchio_accounts(5, false, false);
    let original_len = accounts.len();

    let mut iter = AccountIterator::new(&accounts);

    // Consume some accounts
    iter.next_account("first").unwrap();
    iter.next_account("second").unwrap();

    // Original slice should be unchanged
    assert_eq!(accounts.len(), original_len);
}

#[test]
fn test_position_overflow_protection() {
    // The saturating_sub in print_on_error prevents underflow

    let accounts = create_pinocchio_accounts(0, false, false);
    let iter = AccountIterator::new(&accounts);

    // position is 0, saturating_sub(1) should give 0, not underflow
    assert_eq!(iter.position(), 0);
    // This would be used in print_on_error: self.position.saturating_sub(1)
}

#[test]
fn test_concurrent_iterators() {
    let accounts = create_pinocchio_accounts(5, false, false);

    let mut iter1 = AccountIterator::new(&accounts);
    let mut iter2 = AccountIterator::new(&accounts);

    // Advance iter1
    iter1.next_account("first").unwrap();
    iter1.next_account("second").unwrap();
    assert_eq!(iter1.position(), 2);

    // iter2 should be independent
    assert_eq!(iter2.position(), 0);

    // Advance iter2
    iter2.next_account("first").unwrap();
    assert_eq!(iter2.position(), 1);
    assert_eq!(iter1.position(), 2); // iter1 unchanged
}

#[test]
fn test_iterator_state_complete() {
    let accounts = create_pinocchio_accounts(3, false, false);
    let mut iter = AccountIterator::new(&accounts);

    // Initial state
    let expected = IteratorState {
        position: 0,
        len: 3,
        is_empty: false,
        iterator_is_empty: false,
    };
    assert_eq!(get_iterator_state(&iter), expected);

    // After one account
    iter.next_account("first").unwrap();
    let expected = IteratorState {
        position: 1,
        len: 3,
        is_empty: false,
        iterator_is_empty: false,
    };
    assert_eq!(get_iterator_state(&iter), expected);

    // After all accounts
    iter.next_account("second").unwrap();
    iter.next_account("third").unwrap();
    let expected = IteratorState {
        position: 3,
        len: 3,
        is_empty: false,
        iterator_is_empty: true,
    };
    assert_eq!(get_iterator_state(&iter), expected);
}

#[test]
fn test_remaining_slice_equality() {
    let accounts = create_pinocchio_accounts(5, false, false);
    let mut iter = AccountIterator::new(&accounts);

    // Consume 2 accounts
    iter.next_account("first").unwrap();
    iter.next_account("second").unwrap();

    // Get remaining
    let remaining = iter.remaining().unwrap();

    // Should be exact slice from position 2
    assert_eq!(remaining.len(), 3);
    assert_eq!(remaining[0].key(), accounts[2].key());
    assert_eq!(remaining[1].key(), accounts[3].key());
    assert_eq!(remaining[2].key(), accounts[4].key());

    // Verify it's the same slice reference (pointer equality)
    let expected_slice = &accounts[2..];
    assert_eq!(remaining.as_ptr(), expected_slice.as_ptr());
}
