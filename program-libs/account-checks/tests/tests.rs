#![cfg(all(feature = "solana", feature = "pinocchio"))]
/// Tests for all functions in checks.rs with both pinocchio and solana backends:
/// 1. account_info_init - 4 tests
///    - Solana: Success + Failure (already initialized)
///    - Pinocchio: Success + Failure (already initialized)
/// 2. check_account_info_mut - 4 tests
///    - Solana: Success + Failure (not writable)
///    - Pinocchio: Success + Failure (not writable)
/// 3. check_account_info_non_mut - 4 tests
///    - Solana: Success + Failure (is writable)
///    - Pinocchio: Success + Failure (is writable)
/// 4. check_non_mut - 4 tests
///    - Solana: Success + Failure (is writable)
///    - Pinocchio: Success + Failure (is writable)
/// 5. check_account_info - 6 tests
///    - Solana: Success + Failure (wrong owner) + Failure (wrong discriminator)
///    - Pinocchio: Success + Failure (wrong owner) + Failure (wrong discriminator)
/// 6. check_signer - 3 tests
///    - Solana: Failure (TestAccount always returns false for is_signer)
///    - Pinocchio: Success + Failure
/// 7. check_owner - 4 tests
///    - Solana: Success + Failure (wrong owner)
///    - Pinocchio: Success + Failure (wrong owner)
/// 8. check_program - 5 tests
///    - Solana: Failure (not executable) + Failure (wrong key)
///    - Pinocchio: Success + Failure (not executable) + Failure (wrong key)
/// 9. check_pda_seeds - 4 tests
///    - Solana: Success + Failure
///    - Pinocchio: Success + Failure (requires solana feature for fallback)
/// 10. check_pda_seeds_with_bump - 4 tests
///     - Solana: Success + Failure
///     - Pinocchio: Success + Failure (requires solana feature for fallback)
/// 11. check_account_balance_is_rent_exempt - 4 tests
///     - Solana: Success + Failure (wrong size)
///     - Pinocchio: Success + Failure (wrong size)
/// 12. set_discriminator - 2 tests
///     - Success + Failure (already initialized)
/// 13. check_discriminator - 3 tests
///     - Success + Failure (invalid discriminator) + Failure (too small)
/// 14. check_data_is_zeroed - 2 tests
///     - Success + Failure (not zeroed)
/// 15. get_min_rent_balance - 4 tests
///     - Solana: Success + Failure (sysvar error)
///     - Pinocchio: Success + Test fallback cases
use borsh::{BorshDeserialize, BorshSerialize};
use light_account_checks::{checks::*, discriminator::Discriminator, error::AccountError};

#[repr(C)]
#[derive(Debug, PartialEq, Copy, Clone, BorshSerialize, BorshDeserialize)]
pub struct TestStruct {
    pub data: u64,
}

impl Discriminator for TestStruct {
    const LIGHT_DISCRIMINATOR: [u8; 8] = [180, 4, 231, 26, 220, 144, 55, 168];
    const LIGHT_DISCRIMINATOR_SLICE: &[u8] = &Self::LIGHT_DISCRIMINATOR;
}

// Helper functions to create test accounts for both backends
#[cfg(feature = "solana")]
fn create_test_account_solana(
    key: solana_pubkey::Pubkey,
    owner: solana_pubkey::Pubkey,
    size: usize,
    writable: bool,
) -> light_account_checks::account_info::test_account_info::solana_program::TestAccount {
    let mut account =
        light_account_checks::account_info::test_account_info::solana_program::TestAccount::new(
            key, owner, size,
        );
    account.writable = writable;
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
    size: usize,
    writable: bool,
    signer: bool,
    executable: bool,
) -> pinocchio::account_info::AccountInfo {
    light_account_checks::account_info::test_account_info::pinocchio::get_account_info(
        key,
        owner,
        signer,
        writable,
        executable,
        vec![0u8; size],
    )
}

// 1. account_info_init tests - 4 tests total
#[test]
fn test_account_info_init() {
    // Solana success case
    #[cfg(feature = "solana")]
    {
        let key = create_pubkey();
        let owner = create_pubkey();
        let mut account = create_test_account_solana(key, owner, 16, true);
        assert!(account_info_init::<TestStruct, _>(&account.get_account_info()).is_ok());
    }

    // Solana failure case (already initialized)
    #[cfg(feature = "solana")]
    {
        let key = create_pubkey();
        let owner = create_pubkey();
        let mut account = create_test_account_solana(key, owner, 16, true);
        set_discriminator::<TestStruct>(&mut account.data).unwrap();
        assert_eq!(
            account_info_init::<TestStruct, _>(&account.get_account_info()),
            Err(AccountError::AlreadyInitialized)
        );
    }

    // Pinocchio success case
    #[cfg(feature = "pinocchio")]
    {
        let key = [1u8; 32];
        let owner = [2u8; 32];
        let account = create_test_account_pinocchio(key, owner, 16, true, false, false);
        assert!(account_info_init::<TestStruct, _>(&account).is_ok());
    }

    // Pinocchio failure case (already initialized)
    #[cfg(feature = "pinocchio")]
    {
        let key = [1u8; 32];
        let owner = [2u8; 32];
        let account = create_test_account_pinocchio(key, owner, 16, true, false, false);
        account_info_init::<TestStruct, _>(&account).unwrap();
        assert_eq!(
            account_info_init::<TestStruct, _>(&account),
            Err(AccountError::AlreadyInitialized)
        );
    }
}

// 2. check_account_info_mut tests - 4 tests total
#[test]
fn test_check_account_info_mut() {
    // Solana success case
    #[cfg(feature = "solana")]
    {
        let key = create_pubkey();
        let owner = create_pubkey();
        let mut account = create_test_account_solana(key, owner, 16, true);
        set_discriminator::<TestStruct>(&mut account.data).unwrap();
        assert!(check_account_info_mut::<TestStruct, _>(
            &owner.to_bytes(),
            &account.get_account_info()
        )
        .is_ok());
    }

    // Solana failure case (not writable)
    #[cfg(feature = "solana")]
    {
        let key = create_pubkey();
        let owner = create_pubkey();
        let mut account = create_test_account_solana(key, owner, 16, false);
        set_discriminator::<TestStruct>(&mut account.data).unwrap();
        assert_eq!(
            check_account_info_mut::<TestStruct, _>(&owner.to_bytes(), &account.get_account_info()),
            Err(AccountError::AccountNotMutable)
        );
    }

    // Pinocchio success case
    #[cfg(feature = "pinocchio")]
    {
        let key = [1u8; 32];
        let owner = [2u8; 32];
        let account = create_test_account_pinocchio(key, owner, 16, true, false, false);
        account_info_init::<TestStruct, _>(&account).unwrap();
        assert!(check_account_info_mut::<TestStruct, _>(&owner, &account).is_ok());
    }

    // Pinocchio failure case (not writable)
    #[cfg(feature = "pinocchio")]
    {
        let key = [1u8; 32];
        let owner = [2u8; 32];
        let account = create_test_account_pinocchio(key, owner, 16, false, false, false);
        account_info_init::<TestStruct, _>(&account).unwrap();
        assert_eq!(
            check_account_info_mut::<TestStruct, _>(&owner, &account),
            Err(AccountError::AccountNotMutable)
        );
    }
}

// 3. check_account_info_non_mut tests - 4 tests total
#[test]
fn test_check_account_info_non_mut() {
    // Solana success case
    #[cfg(feature = "solana")]
    {
        let key = create_pubkey();
        let owner = create_pubkey();
        let mut account = create_test_account_solana(key, owner, 16, false);
        set_discriminator::<TestStruct>(&mut account.data).unwrap();
        assert!(check_account_info_non_mut::<TestStruct, _>(
            &owner.to_bytes(),
            &account.get_account_info()
        )
        .is_ok());
    }

    // Solana failure case (is writable)
    #[cfg(feature = "solana")]
    {
        let key = create_pubkey();
        let owner = create_pubkey();
        let mut account = create_test_account_solana(key, owner, 16, true);
        set_discriminator::<TestStruct>(&mut account.data).unwrap();
        assert_eq!(
            check_account_info_non_mut::<TestStruct, _>(
                &owner.to_bytes(),
                &account.get_account_info()
            ),
            Err(AccountError::AccountMutable)
        );
    }

    // Pinocchio success case
    #[cfg(feature = "pinocchio")]
    {
        let key = [1u8; 32];
        let owner = [2u8; 32];
        let account = create_test_account_pinocchio(key, owner, 16, false, false, false);
        account_info_init::<TestStruct, _>(&account).unwrap();
        assert!(check_account_info_non_mut::<TestStruct, _>(&owner, &account).is_ok());
    }

    // Pinocchio failure case (is writable)
    #[cfg(feature = "pinocchio")]
    {
        let key = [1u8; 32];
        let owner = [2u8; 32];
        let account = create_test_account_pinocchio(key, owner, 16, true, false, false);
        account_info_init::<TestStruct, _>(&account).unwrap();
        assert_eq!(
            check_account_info_non_mut::<TestStruct, _>(&owner, &account),
            Err(AccountError::AccountMutable)
        );
    }
}

// 4. check_non_mut tests - 4 tests total
#[test]
fn test_check_non_mut() {
    // Solana success case
    #[cfg(feature = "solana")]
    {
        let key = create_pubkey();
        let owner = create_pubkey();
        let mut account = create_test_account_solana(key, owner, 16, false);
        assert!(check_non_mut(&account.get_account_info()).is_ok());
    }

    // Solana failure case (is writable)
    #[cfg(feature = "solana")]
    {
        let key = create_pubkey();
        let owner = create_pubkey();
        let mut account = create_test_account_solana(key, owner, 16, true);
        assert_eq!(
            check_non_mut(&account.get_account_info()),
            Err(AccountError::AccountMutable)
        );
    }

    // Pinocchio success case
    #[cfg(feature = "pinocchio")]
    {
        let key = [1u8; 32];
        let owner = [2u8; 32];
        let account = create_test_account_pinocchio(key, owner, 16, false, false, false);
        assert!(check_non_mut(&account).is_ok());
    }

    // Pinocchio failure case (is writable)
    #[cfg(feature = "pinocchio")]
    {
        let key = [1u8; 32];
        let owner = [2u8; 32];
        let account = create_test_account_pinocchio(key, owner, 16, true, false, false);
        assert_eq!(check_non_mut(&account), Err(AccountError::AccountMutable));
    }
}

// 4.5. check_mut tests - 4 tests total
#[test]
fn test_check_mut() {
    // Solana success case
    #[cfg(feature = "solana")]
    {
        let key = create_pubkey();
        let owner = create_pubkey();
        let mut account = create_test_account_solana(key, owner, 16, true);
        assert!(check_mut(&account.get_account_info()).is_ok());
    }

    // Solana failure case (not writable)
    #[cfg(feature = "solana")]
    {
        let key = create_pubkey();
        let owner = create_pubkey();
        let mut account = create_test_account_solana(key, owner, 16, false);
        assert_eq!(
            check_mut(&account.get_account_info()),
            Err(AccountError::AccountNotMutable)
        );
    }

    // Pinocchio success case
    #[cfg(feature = "pinocchio")]
    {
        let key = [1u8; 32];
        let owner = [2u8; 32];
        let account = create_test_account_pinocchio(key, owner, 16, true, false, false);
        assert!(check_mut(&account).is_ok());
    }

    // Pinocchio failure case (not writable)
    #[cfg(feature = "pinocchio")]
    {
        let key = [1u8; 32];
        let owner = [2u8; 32];
        let account = create_test_account_pinocchio(key, owner, 16, false, false, false);
        assert_eq!(check_mut(&account), Err(AccountError::AccountNotMutable));
    }
}

// 5. check_account_info tests - 6 tests total
#[test]
fn test_check_account_info() {
    // Solana success case
    #[cfg(feature = "solana")]
    {
        let key = create_pubkey();
        let owner = create_pubkey();
        let mut account = create_test_account_solana(key, owner, 16, true);
        set_discriminator::<TestStruct>(&mut account.data).unwrap();
        assert!(check_account_info::<TestStruct, _>(
            &owner.to_bytes(),
            &account.get_account_info()
        )
        .is_ok());
    }

    // Solana failure case (wrong owner)
    #[cfg(feature = "solana")]
    {
        let key = create_pubkey();
        let owner = create_pubkey();
        let wrong_owner = create_pubkey();
        let mut account = create_test_account_solana(key, owner, 16, true);
        set_discriminator::<TestStruct>(&mut account.data).unwrap();
        assert_eq!(
            check_account_info::<TestStruct, _>(
                &wrong_owner.to_bytes(),
                &account.get_account_info()
            ),
            Err(AccountError::AccountOwnedByWrongProgram)
        );
    }

    // Solana failure case (wrong discriminator)
    #[cfg(feature = "solana")]
    {
        let key = create_pubkey();
        let owner = create_pubkey();
        let mut account = create_test_account_solana(key, owner, 16, true);
        assert_eq!(
            check_account_info::<TestStruct, _>(&owner.to_bytes(), &account.get_account_info()),
            Err(AccountError::InvalidDiscriminator)
        );
    }

    // Pinocchio success case
    #[cfg(feature = "pinocchio")]
    {
        let key = [1u8; 32];
        let owner = [2u8; 32];
        let account = create_test_account_pinocchio(key, owner, 16, true, false, false);
        account_info_init::<TestStruct, _>(&account).unwrap();
        assert!(check_account_info::<TestStruct, _>(&owner, &account).is_ok());
    }

    // Pinocchio failure case (wrong owner)
    #[cfg(feature = "pinocchio")]
    {
        let key = [1u8; 32];
        let owner = [2u8; 32];
        let wrong_owner = [3u8; 32];
        let account = create_test_account_pinocchio(key, owner, 16, true, false, false);
        account_info_init::<TestStruct, _>(&account).unwrap();
        assert_eq!(
            check_account_info::<TestStruct, _>(&wrong_owner, &account),
            Err(AccountError::AccountOwnedByWrongProgram)
        );
    }

    // Pinocchio failure case (wrong discriminator)
    #[cfg(feature = "pinocchio")]
    {
        let key = [1u8; 32];
        let owner = [2u8; 32];
        let account = create_test_account_pinocchio(key, owner, 16, true, false, false);
        assert_eq!(
            check_account_info::<TestStruct, _>(&owner, &account),
            Err(AccountError::InvalidDiscriminator)
        );
    }
}

// 6. check_signer tests - 3 tests total
#[test]
fn test_check_signer() {
    // Solana failure case (TestAccount always returns false for is_signer)
    #[cfg(feature = "solana")]
    {
        let key = create_pubkey();
        let owner = create_pubkey();
        let mut account = create_test_account_solana(key, owner, 16, true);
        assert_eq!(
            check_signer(&account.get_account_info()),
            Err(AccountError::InvalidSigner)
        );
    }

    // Pinocchio success case
    #[cfg(feature = "pinocchio")]
    {
        let key = [1u8; 32];
        let owner = [2u8; 32];
        let account = create_test_account_pinocchio(key, owner, 16, true, true, false);
        assert!(check_signer(&account).is_ok());
    }

    // Pinocchio failure case
    #[cfg(feature = "pinocchio")]
    {
        let key = [1u8; 32];
        let owner = [2u8; 32];
        let account = create_test_account_pinocchio(key, owner, 16, true, false, false);
        assert_eq!(check_signer(&account), Err(AccountError::InvalidSigner));
    }
}

// 7. check_owner tests - 4 tests total
#[test]
fn test_check_owner() {
    // Solana success case
    #[cfg(feature = "solana")]
    {
        let key = create_pubkey();
        let owner = create_pubkey();
        let mut account = create_test_account_solana(key, owner, 16, true);
        assert!(check_owner(&owner.to_bytes(), &account.get_account_info()).is_ok());
    }

    // Solana failure case (wrong owner)
    #[cfg(feature = "solana")]
    {
        let key = create_pubkey();
        let owner = create_pubkey();
        let wrong_owner = create_pubkey();
        let mut account = create_test_account_solana(key, owner, 16, true);
        assert_eq!(
            check_owner(&wrong_owner.to_bytes(), &account.get_account_info()),
            Err(AccountError::AccountOwnedByWrongProgram)
        );
    }

    // Pinocchio success case
    #[cfg(feature = "pinocchio")]
    {
        let key = [1u8; 32];
        let owner = [2u8; 32];
        let account = create_test_account_pinocchio(key, owner, 16, true, false, false);
        assert!(check_owner(&owner, &account).is_ok());
    }

    // Pinocchio failure case (wrong owner)
    #[cfg(feature = "pinocchio")]
    {
        let key = [1u8; 32];
        let owner = [2u8; 32];
        let wrong_owner = [3u8; 32];
        let account = create_test_account_pinocchio(key, owner, 16, true, false, false);
        assert_eq!(
            check_owner(&wrong_owner, &account),
            Err(AccountError::AccountOwnedByWrongProgram)
        );
    }
}

// 8. check_program tests - 5 tests total
#[test]
fn test_check_program() {
    // Solana failure case (not executable)
    #[cfg(feature = "solana")]
    {
        let program_id = create_pubkey();
        let owner = create_pubkey();
        let mut account = create_test_account_solana(program_id, owner, 16, true);
        assert_eq!(
            check_program(&program_id.to_bytes(), &account.get_account_info()),
            Err(AccountError::ProgramNotExecutable)
        );
    }

    // Solana failure case (wrong key)
    #[cfg(feature = "solana")]
    {
        let program_id = create_pubkey();
        let different_key = create_pubkey();
        let owner = create_pubkey();
        let mut account = create_test_account_solana(different_key, owner, 16, true);
        assert_eq!(
            check_program(&program_id.to_bytes(), &account.get_account_info()),
            Err(AccountError::InvalidProgramId)
        );
    }

    // Pinocchio success case
    #[cfg(feature = "pinocchio")]
    {
        let program_id = [1u8; 32];
        let owner = [2u8; 32];
        let account = create_test_account_pinocchio(program_id, owner, 16, true, false, true);
        assert!(check_program(&program_id, &account).is_ok());
    }

    // Pinocchio failure case (not executable)
    #[cfg(feature = "pinocchio")]
    {
        let program_id = [1u8; 32];
        let owner = [2u8; 32];
        let account = create_test_account_pinocchio(program_id, owner, 16, true, false, false);
        assert_eq!(
            check_program(&program_id, &account),
            Err(AccountError::ProgramNotExecutable)
        );
    }

    // Pinocchio failure case (wrong key)
    #[cfg(feature = "pinocchio")]
    {
        let program_id = [1u8; 32];
        let different_key = [2u8; 32];
        let owner = [3u8; 32];
        let account = create_test_account_pinocchio(different_key, owner, 16, true, false, true);
        assert_eq!(
            check_program(&program_id, &account),
            Err(AccountError::InvalidProgramId)
        );
    }
}

// 9. check_pda_seeds tests - 4 tests total
#[test]
fn test_check_pda_seeds() {
    // Solana success case
    #[cfg(feature = "solana")]
    {
        use solana_account_info::AccountInfo;

        let program_id = create_pubkey();
        let seeds = &[b"test_seed".as_ref()];
        let (pda, _) = solana_pubkey::Pubkey::find_program_address(seeds, &program_id);
        let mut account = create_test_account_solana(pda, program_id, 16, true);

        assert!(check_pda_seeds::<AccountInfo>(
            seeds,
            &program_id.to_bytes(),
            &account.get_account_info()
        )
        .is_ok());
    }

    // Solana failure case
    #[cfg(feature = "solana")]
    {
        use solana_account_info::AccountInfo;

        let program_id = create_pubkey();
        let wrong_key = create_pubkey();
        let seeds = &[b"test_seed".as_ref()];
        let mut account = create_test_account_solana(wrong_key, program_id, 16, true);

        assert_eq!(
            check_pda_seeds::<AccountInfo>(
                seeds,
                &program_id.to_bytes(),
                &account.get_account_info()
            ),
            Err(AccountError::InvalidSeeds)
        );
    }

    // Pinocchio success case (requires solana feature for fallback)
    #[cfg(all(feature = "pinocchio", feature = "solana"))]
    {
        let program_id = [1u8; 32];
        let seeds = &[b"test_seed".as_ref()];
        let (pda, _) = solana_pubkey::Pubkey::find_program_address(
            seeds,
            &solana_pubkey::Pubkey::from(program_id),
        );
        let account =
            create_test_account_pinocchio(pda.to_bytes(), program_id, 16, true, false, false);

        assert!(check_pda_seeds(seeds, &program_id, &account).is_ok());
    }

    // Pinocchio failure case (requires solana feature for fallback)
    #[cfg(all(feature = "pinocchio", feature = "solana"))]
    {
        let program_id = [1u8; 32];
        let wrong_key = [2u8; 32];
        let seeds = &[b"test_seed".as_ref()];
        let account = create_test_account_pinocchio(wrong_key, program_id, 16, true, false, false);

        assert_eq!(
            check_pda_seeds(seeds, &program_id, &account),
            Err(AccountError::InvalidSeeds)
        );
    }
}

// 10. check_pda_seeds_with_bump tests - 4 tests total
#[test]
fn test_check_pda_seeds_with_bump() {
    // Solana success case
    #[cfg(feature = "solana")]
    {
        use solana_account_info::AccountInfo;

        let program_id = create_pubkey();
        let base_seeds = &[b"test_seed".as_ref()];
        let (pda, bump) = solana_pubkey::Pubkey::find_program_address(base_seeds, &program_id);
        let seeds_with_bump = &[b"test_seed".as_ref(), &[bump]];
        let mut account = create_test_account_solana(pda, program_id, 16, true);

        assert!(check_pda_seeds_with_bump::<AccountInfo>(
            seeds_with_bump,
            &program_id.to_bytes(),
            &account.get_account_info()
        )
        .is_ok());
    }

    // Solana failure case
    #[cfg(feature = "solana")]
    {
        use solana_account_info::AccountInfo;

        let program_id = create_pubkey();
        let wrong_key = create_pubkey();
        let base_seeds = &[b"test_seed".as_ref()];
        let (_, bump) = solana_pubkey::Pubkey::find_program_address(base_seeds, &program_id);
        let seeds_with_bump = &[b"test_seed".as_ref(), &[bump]];
        let mut account = create_test_account_solana(wrong_key, program_id, 16, true);

        assert_eq!(
            check_pda_seeds_with_bump::<AccountInfo>(
                seeds_with_bump,
                &program_id.to_bytes(),
                &account.get_account_info()
            ),
            Err(AccountError::InvalidSeeds)
        );
    }

    // Pinocchio success case (requires solana feature for fallback)
    #[cfg(all(feature = "pinocchio", feature = "solana"))]
    {
        let program_id = [1u8; 32];
        let base_seeds = &[b"test_seed".as_ref()];
        let (pda, bump) = solana_pubkey::Pubkey::find_program_address(
            base_seeds,
            &solana_pubkey::Pubkey::from(program_id),
        );
        let seeds_with_bump = &[b"test_seed".as_ref(), &[bump]];
        let account =
            create_test_account_pinocchio(pda.to_bytes(), program_id, 16, true, false, false);

        assert!(check_pda_seeds_with_bump(seeds_with_bump, &program_id, &account).is_ok());
    }

    // Pinocchio failure case (requires solana feature for fallback)
    #[cfg(all(feature = "pinocchio", feature = "solana"))]
    {
        let program_id = [1u8; 32];
        let wrong_key = [2u8; 32];
        let base_seeds = &[b"test_seed".as_ref()];
        let (_, bump) = solana_pubkey::Pubkey::find_program_address(
            base_seeds,
            &solana_pubkey::Pubkey::from(program_id),
        );
        let seeds_with_bump = &[b"test_seed".as_ref(), &[bump]];
        let account = create_test_account_pinocchio(wrong_key, program_id, 16, true, false, false);

        assert_eq!(
            check_pda_seeds_with_bump(seeds_with_bump, &program_id, &account),
            Err(AccountError::InvalidSeeds)
        );
    }
}

// 11. check_account_balance_is_rent_exempt tests - 4 tests total
#[test]
fn test_check_account_balance_is_rent_exempt() {
    // Solana success case
    #[cfg(feature = "solana")]
    {
        let key = create_pubkey();
        let owner = create_pubkey();
        let expected_size = 16;
        let mut account = create_test_account_solana(key, owner, expected_size, true);
        account.lamports = 1000000; // High lamports to ensure rent exemption

        assert!(
            check_account_balance_is_rent_exempt(&account.get_account_info(), expected_size)
                .is_ok()
        );
    }

    // Solana failure case (wrong size)
    #[cfg(feature = "solana")]
    {
        let key = create_pubkey();
        let owner = create_pubkey();
        let actual_size = 16;
        let expected_size = 32;
        let mut account = create_test_account_solana(key, owner, actual_size, true);

        assert_eq!(
            check_account_balance_is_rent_exempt(&account.get_account_info(), expected_size),
            Err(AccountError::InvalidAccountSize)
        );
    }

    // Pinocchio success case
    #[cfg(feature = "pinocchio")]
    {
        let key = [1u8; 32];
        let owner = [2u8; 32];
        let expected_size = 16;
        let account = create_test_account_pinocchio(key, owner, expected_size, true, false, false);

        assert!(check_account_balance_is_rent_exempt(&account, expected_size).is_ok());
    }

    // Pinocchio failure case (wrong size)
    #[cfg(feature = "pinocchio")]
    {
        let key = [1u8; 32];
        let owner = [2u8; 32];
        let actual_size = 16;
        let expected_size = 32;
        let account = create_test_account_pinocchio(key, owner, actual_size, true, false, false);

        assert_eq!(
            check_account_balance_is_rent_exempt(&account, expected_size),
            Err(AccountError::InvalidAccountSize)
        );
    }
}

// Additional tests for functions that work with raw bytes (not requiring AccountInfo)
#[test]
fn test_set_discriminator() {
    let mut bytes = vec![0; 16];
    assert!(set_discriminator::<TestStruct>(&mut bytes).is_ok());
    assert_eq!(bytes[0..8], TestStruct::LIGHT_DISCRIMINATOR);

    // Test failure case (already initialized)
    assert_eq!(
        set_discriminator::<TestStruct>(&mut bytes),
        Err(AccountError::AlreadyInitialized)
    );
}

#[test]
fn test_check_discriminator() {
    let mut bytes = vec![0; 16];
    set_discriminator::<TestStruct>(&mut bytes).unwrap();
    assert!(check_discriminator::<TestStruct>(&bytes).is_ok());

    // Test failure case (invalid discriminator)
    let bytes = vec![0; 16]; // No discriminator set
    assert_eq!(
        check_discriminator::<TestStruct>(&bytes),
        Err(AccountError::InvalidDiscriminator)
    );

    // Test failure case (too small)
    let bytes = vec![0; 4]; // Too small for discriminator
    assert_eq!(
        check_discriminator::<TestStruct>(&bytes),
        Err(AccountError::InvalidAccountSize)
    );
}

#[test]
fn test_check_data_is_zeroed() {
    let zeroed_data = [0u8; 32];
    assert!(check_data_is_zeroed::<8>(zeroed_data.as_slice()).is_ok());

    // Test failure case (not zeroed)
    let mut not_zeroed_data = [0u8; 32];
    not_zeroed_data[7] = 1;
    assert_eq!(
        check_data_is_zeroed::<8>(not_zeroed_data.as_slice()),
        Err(AccountError::AccountNotZeroed)
    );
}

// 15. get_min_rent_balance tests - 4 tests total
// TODO: Enable when sysvar is available in test environment
/*
#[test]
fn test_get_min_rent_balance() {
    // Solana success case - get rent for different sizes
    #[cfg(feature = "solana")]
    {
        use solana_account_info::AccountInfo;

        // Test with common account sizes
        let small_rent = AccountInfo::get_min_rent_balance(100);
        let large_rent = AccountInfo::get_min_rent_balance(1000);

        // Rent for larger accounts should be higher than smaller accounts
        assert!(small_rent.is_ok());
        assert!(large_rent.is_ok());
        assert!(large_rent.unwrap() > small_rent.unwrap());
        assert_eq!(small_rent.unwrap(), 1002240);
        assert_eq!(large_rent.unwrap(), 1009440);

        // Test with zero size
        let zero_rent = AccountInfo::get_min_rent_balance(0);
        assert!(zero_rent.is_ok());
        assert_eq!(zero_rent.unwrap(), 890880);
    }

    // Pinocchio success case - test different feature configurations
    #[cfg(feature = "pinocchio")]
    {
        use pinocchio::account_info::AccountInfo;

        // Test with common account sizes
        let small_rent = AccountInfo::get_min_rent_balance(100);
        let large_rent = AccountInfo::get_min_rent_balance(1000);

        // Should succeed under any feature configuration
        assert!(small_rent.is_ok());
        assert!(large_rent.is_ok());

        #[cfg(feature = "solana")]
        {
            // When solana feature is available, should behave like solana
            assert!(large_rent.unwrap() > small_rent.unwrap());
            assert_eq!(small_rent.unwrap(), 1002240);
            assert_eq!(large_rent.unwrap(), 1009440);
        }

        #[cfg(not(feature = "solana"))]
        {
            // When solana feature is not available, should return 0 for testing
            assert_eq!(small_rent.unwrap(), 0);
            assert_eq!(large_rent.unwrap(), 0);
        }
    }
}
*/
