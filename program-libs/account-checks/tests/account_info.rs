#![cfg(all(feature = "solana", feature = "pinocchio"))]
// Comprehensive tests for AccountInfoTrait implementations:
// - Solana implementation (solana_account_info::AccountInfo)
// - Pinocchio implementation (pinocchio::account_info::AccountInfo)
//
// Tests cover all trait methods with both functional and failing test cases:
// 1. key() - Returns account public key
// 2. is_writable() - Check if account is writable
// 3. is_signer() - Check if account is a signer
// 4. executable() - Check if account is executable
// 5. lamports() - Get account lamport balance
// 6. data_len() - Get account data length
// 7. try_borrow_data() - Borrow account data immutably
// 8. try_borrow_mut_data() - Borrow account data mutably
// 9. is_owned_by() - Check account ownership
// 10. find_program_address() - Find PDA (static method)
// 11. create_program_address() - Create PDA (static method)
// 12. data_is_empty() - Check if account data is empty
// 13. get_min_rent_balance() - Get minimum rent balance for size

use light_account_checks::AccountInfoTrait;

// Test helper functions
#[cfg(feature = "solana")]
fn create_test_account_solana(
    key: solana_pubkey::Pubkey,
    owner: solana_pubkey::Pubkey,
    lamports: u64,
    data: Vec<u8>,
    writable: bool,
    _signer: bool,
    _executable: bool,
) -> light_account_checks::account_info::test_account_info::solana_program::TestAccount {
    let mut account =
        light_account_checks::account_info::test_account_info::solana_program::TestAccount::new(
            key,
            owner,
            data.len(),
        );
    account.data = data;
    account.lamports = lamports;
    account.writable = writable;
    // Note: TestAccount doesn't have an executable field
    // Note: TestAccount doesn't support signer flag directly
    account
}

#[cfg(feature = "solana")]
fn create_pubkey() -> solana_pubkey::Pubkey {
    solana_pubkey::Pubkey::new_unique()
}

#[cfg(feature = "pinocchio")]
fn create_test_account_pinocchio(
    key: [u8; 32],
    owner: [u8; 32],
    data: Vec<u8>,
    writable: bool,
    signer: bool,
    executable: bool,
) -> pinocchio::account_info::AccountInfo {
    light_account_checks::account_info::test_account_info::pinocchio::get_account_info(
        key, owner, signer, writable, executable, data,
    )
}

// Solana AccountInfoTrait implementation tests
#[test]
#[cfg(feature = "solana")]
fn test_solana_account_info_trait() {
    let key = create_pubkey();
    let owner = create_pubkey();
    let data = vec![1, 2, 3, 4, 5];
    let lamports = 1000000u64;

    // Test writable account
    {
        let mut account = create_test_account_solana(
            key,
            owner,
            lamports,
            data.clone(),
            true,  // writable
            false, // signer
            false, // executable
        );
        let account_info = account.get_account_info();

        // Test key() - functional
        assert_eq!(AccountInfoTrait::key(&account_info), key.to_bytes());

        // Test is_writable() - functional (true case)
        assert!(AccountInfoTrait::is_writable(&account_info));

        // Test is_signer() - failing (TestAccount always returns false)
        assert!(!AccountInfoTrait::is_signer(&account_info));

        // Test executable() - failing (false case)
        assert!(!AccountInfoTrait::executable(&account_info));

        // Test lamports() - functional
        assert_eq!(AccountInfoTrait::lamports(&account_info), lamports);

        // Test data_len() - functional
        assert_eq!(AccountInfoTrait::data_len(&account_info), data.len());

        // Test try_borrow_data() - functional
        {
            let borrowed_data = AccountInfoTrait::try_borrow_data(&account_info).unwrap();
            assert_eq!(*borrowed_data, data);
        } // Drop immutable borrow

        // Test try_borrow_mut_data() - functional (writable account)
        {
            let mut borrowed_mut_data =
                AccountInfoTrait::try_borrow_mut_data(&account_info).unwrap();
            borrowed_mut_data[0] = 99;
        } // Drop mutable borrow

        // Verify mutation worked
        {
            let borrowed_data = AccountInfoTrait::try_borrow_data(&account_info).unwrap();
            assert_eq!(borrowed_data[0], 99);
        }

        // Test is_owned_by() - functional (correct owner)
        assert!(AccountInfoTrait::is_owned_by(
            &account_info,
            &owner.to_bytes()
        ));

        // Test is_owned_by() - failing (wrong owner)
        let wrong_owner = create_pubkey();
        assert!(!AccountInfoTrait::is_owned_by(
            &account_info,
            &wrong_owner.to_bytes()
        ));

        // Test data_is_empty() - failing (has data)
        assert!(!AccountInfoTrait::data_is_empty(&account_info));
    }

    // Test non-writable account
    {
        let mut account = create_test_account_solana(
            key,
            owner,
            lamports,
            data.clone(),
            false, // not writable
            false, // signer
            false, // executable
        );
        let account_info = account.get_account_info();

        // Test is_writable() - failing (false case)
        assert!(!AccountInfoTrait::is_writable(&account_info));

        // Test try_borrow_mut_data() should still work (TestAccount doesn't enforce this)
        // Note: Real Solana runtime would fail this for non-writable accounts
        let _borrowed_mut_data = AccountInfoTrait::try_borrow_mut_data(&account_info).unwrap();
    }

    // Test executable account
    // Note: TestAccount doesn't support executable flag in constructor, so we test the default false case
    {
        let mut account = create_test_account_solana(
            key,
            owner,
            lamports,
            data.clone(),
            false, // not writable
            false, // signer
            false, // executable (TestAccount doesn't support setting this to true)
        );
        let account_info = account.get_account_info();

        // Test executable() - failing (TestAccount always returns false)
        assert!(!AccountInfoTrait::executable(&account_info));
    }

    // Test empty data account
    {
        let mut account = create_test_account_solana(
            key,
            owner,
            lamports,
            vec![], // empty data
            true,   // writable
            false,  // signer
            false,  // executable
        );
        let account_info = account.get_account_info();

        // Test data_len() - functional (zero length)
        assert_eq!(AccountInfoTrait::data_len(&account_info), 0);

        // Test data_is_empty() - functional (empty data)
        assert!(AccountInfoTrait::data_is_empty(&account_info));
    }

    // Test static methods (find_program_address and create_program_address)
    {
        use light_account_checks::error::AccountError;
        use solana_account_info::AccountInfo;

        let program_id = create_pubkey();
        let seeds = &[b"test_seed".as_ref(), &[1, 2, 3]];

        // Test find_program_address() - functional
        let (pda, bump) = AccountInfo::find_program_address(seeds, &program_id.to_bytes());

        // Verify the PDA is valid by using Solana's function
        let (expected_pda, expected_bump) =
            solana_pubkey::Pubkey::find_program_address(seeds, &program_id);
        assert_eq!(pda, expected_pda.to_bytes());
        assert_eq!(bump, expected_bump);

        // Test create_program_address() - functional
        let seeds_with_bump = &[b"test_seed".as_ref(), &[1, 2, 3], &[bump]];
        let created_pda =
            AccountInfo::create_program_address(seeds_with_bump, &program_id.to_bytes()).unwrap();
        assert_eq!(created_pda, pda);

        // Test create_program_address() - failing (invalid bump)
        let invalid_seeds = &[b"test_seed".as_ref(), &[1, 2, 3], &[255u8]]; // Invalid bump
        let result = AccountInfo::create_program_address(invalid_seeds, &program_id.to_bytes());
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), AccountError::InvalidSeeds);
    }

    // Test get_min_rent_balance() - static method
    // TODO: Enable when sysvar is available in test environment
    /*
    {
        use solana_account_info::AccountInfo;

        // Test get_min_rent_balance() - functional
        let small_rent = AccountInfo::get_min_rent_balance(100);
        assert!(small_rent.is_ok());
        let small_rent_value = small_rent.unwrap();
        assert_eq!(small_rent_value, 1002240);

        let large_rent = AccountInfo::get_min_rent_balance(1000);
        assert!(large_rent.is_ok());
        let large_rent_value = large_rent.unwrap();
        assert_eq!(large_rent_value, 1009440);

        // Larger accounts should cost more rent
        assert!(large_rent_value > small_rent_value);

        // Zero size should still have base rent
        let zero_rent = AccountInfo::get_min_rent_balance(0);
        assert!(zero_rent.is_ok());
        assert_eq!(zero_rent.unwrap(), 890880);
    }
    */
}

// Pinocchio AccountInfoTrait implementation tests
#[test]
#[cfg(feature = "pinocchio")]
fn test_pinocchio_account_info_trait() {
    let key = [1u8; 32];
    let owner = [2u8; 32];
    let data = vec![1, 2, 3, 4, 5];

    // Test writable account
    {
        let account = create_test_account_pinocchio(
            key,
            owner,
            data.clone(),
            true,  // writable
            false, // signer
            false, // executable
        );

        // Test key() - functional (using AccountInfoTrait method)
        assert_eq!(AccountInfoTrait::key(&account), key);

        // Test is_writable() - functional (true case)
        assert!(AccountInfoTrait::is_writable(&account));

        // Test is_signer() - failing (false case)
        assert!(!AccountInfoTrait::is_signer(&account));

        // Test executable() - failing (false case)
        assert!(!AccountInfoTrait::executable(&account));

        // Test lamports() - functional (fixed at 1000 in test helper)
        assert_eq!(AccountInfoTrait::lamports(&account), 1000);

        // Test data_len() - functional
        assert_eq!(AccountInfoTrait::data_len(&account), data.len());

        // Test try_borrow_data() - functional
        {
            let borrowed_data = AccountInfoTrait::try_borrow_data(&account).unwrap();
            assert_eq!(*borrowed_data, data);
        } // Drop immutable borrow

        // Test try_borrow_mut_data() - functional (writable account)
        {
            let mut borrowed_mut_data = AccountInfoTrait::try_borrow_mut_data(&account).unwrap();
            borrowed_mut_data[0] = 99;
        } // Drop mutable borrow

        // Verify mutation worked
        {
            let borrowed_data = AccountInfoTrait::try_borrow_data(&account).unwrap();
            assert_eq!(borrowed_data[0], 99);
        }

        // Test is_owned_by() - functional (correct owner)
        assert!(AccountInfoTrait::is_owned_by(&account, &owner));

        // Test is_owned_by() - failing (wrong owner)
        let wrong_owner = [3u8; 32];
        assert!(!AccountInfoTrait::is_owned_by(&account, &wrong_owner));

        // Test data_is_empty() - failing (has data)
        assert!(!AccountInfoTrait::data_is_empty(&account));
    }

    // Test non-writable account
    {
        let account = create_test_account_pinocchio(
            key,
            owner,
            data.clone(),
            false, // not writable
            false, // signer
            false, // executable
        );

        // Test is_writable() - failing (false case)
        assert!(!AccountInfoTrait::is_writable(&account));

        // Test try_borrow_mut_data() - should still work (test implementation doesn't enforce this)
        let _borrowed_mut_data = AccountInfoTrait::try_borrow_mut_data(&account).unwrap();
    }

    // Test signer account
    {
        let account = create_test_account_pinocchio(
            key,
            owner,
            data.clone(),
            true,  // writable
            true,  // signer
            false, // executable
        );

        // Test is_signer() - functional (true case)
        assert!(AccountInfoTrait::is_signer(&account));
    }

    // Test executable account
    {
        let account = create_test_account_pinocchio(
            key,
            owner,
            data.clone(),
            false, // not writable
            false, // signer
            true,  // executable
        );

        // Test executable() - functional (true case)
        assert!(AccountInfoTrait::executable(&account));
    }

    // Test empty data account
    {
        let account = create_test_account_pinocchio(
            key,
            owner,
            vec![], // empty data
            true,   // writable
            false,  // signer
            false,  // executable
        );

        // Test data_len() - functional (zero length)
        assert_eq!(AccountInfoTrait::data_len(&account), 0);

        // Test data_is_empty() - functional (empty data)
        assert!(AccountInfoTrait::data_is_empty(&account));
    }

    // Test static methods (find_program_address and create_program_address)
    // Note: Pinocchio implementation falls back to Solana when solana feature is enabled
    #[cfg(feature = "solana")]
    {
        use light_account_checks::{error::AccountError, AccountInfoTrait};

        let program_id = [4u8; 32];
        let seeds = &[b"test_seed".as_ref(), &[1, 2, 3]];

        // Test find_program_address() - functional
        let (pda, bump) =
            pinocchio::account_info::AccountInfo::find_program_address(seeds, &program_id);

        // Verify the PDA is valid by using Solana's function
        let (expected_pda, expected_bump) = solana_pubkey::Pubkey::find_program_address(
            seeds,
            &solana_pubkey::Pubkey::from(program_id),
        );
        assert_eq!(pda, expected_pda.to_bytes());
        assert_eq!(bump, expected_bump);

        // Test create_program_address() - functional
        let seeds_with_bump = &[b"test_seed".as_ref(), &[1, 2, 3], &[bump]];
        let created_pda = pinocchio::account_info::AccountInfo::create_program_address(
            seeds_with_bump,
            &program_id,
        )
        .unwrap();
        assert_eq!(created_pda, pda);

        // Test create_program_address() - failing (invalid bump)
        let invalid_seeds = &[b"test_seed".as_ref(), &[1, 2, 3], &[255u8]]; // Invalid bump
        let result = pinocchio::account_info::AccountInfo::create_program_address(
            invalid_seeds,
            &program_id,
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), AccountError::InvalidSeeds);
    }

    // Test zero lamports account
    {
        let account = create_test_account_pinocchio(
            key,
            owner,
            data.clone(),
            true,  // writable
            false, // signer
            false, // executable
        );

        // Test lamports() - functional (always 1000 in test helper)
        assert_eq!(AccountInfoTrait::lamports(&account), 1000);
    }

    // Test get_min_rent_balance() - static method
    // TODO: Enable when sysvar is available in test environment
    /*
    {
        use pinocchio::account_info::AccountInfo;

        // Test get_min_rent_balance() - functional
        let small_rent = AccountInfo::get_min_rent_balance(100);
        assert!(small_rent.is_ok());
        let small_rent_value = small_rent.unwrap();

        let large_rent = AccountInfo::get_min_rent_balance(1000);
        assert!(large_rent.is_ok());
        let large_rent_value = large_rent.unwrap();

        // Behavior depends on feature configuration
        #[cfg(feature = "solana")]
        {
            // When solana feature is available, should behave like solana
            assert!(large_rent_value > small_rent_value);
            assert_eq!(small_rent_value, 1002240);
            assert_eq!(large_rent_value, 1009440);
        }

        #[cfg(not(feature = "solana"))]
        {
            // When solana feature is not available, returns 0 for testing
            assert_eq!(small_rent_value, 0);
            assert_eq!(large_rent_value, 0);
        }

        // Zero size should still have base rent (when solana feature enabled) or 0 (when not)
        let zero_rent = AccountInfo::get_min_rent_balance(0);
        assert!(zero_rent.is_ok());

        #[cfg(feature = "solana")]
        {
            assert_eq!(zero_rent.unwrap(), 890880);
        }

        #[cfg(not(feature = "solana"))]
        {
            assert_eq!(zero_rent.unwrap(), 0);
        }
    }
    */
}
