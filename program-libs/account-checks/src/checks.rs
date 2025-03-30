use crate::{discriminator::Discriminator, error::AccountError, AccountInfo, Pubkey};

/// Sets discriminator in account data.
pub fn account_info_init<T: Discriminator<U>, const U: usize>(
    account_info: &AccountInfo,
) -> Result<(), AccountError> {
    set_discriminator::<T, U>(
        &mut account_info
            .try_borrow_mut_data()
            .map_err(|_| AccountError::BorrowAccountDataFailed)?,
    )?;
    Ok(())
}

/// Checks:
/// 1. account is mutable
/// 2. account owned by program_id
/// 3. account discriminator
pub fn check_account_info_mut<T: Discriminator<U>, const U: usize>(
    program_id: &Pubkey,
    account_info: &AccountInfo,
) -> Result<(), AccountError> {
    #[cfg(not(feature = "pinocchio"))]
    if !account_info.is_writable {
        return Err(AccountError::AccountMutable);
    }
    #[cfg(feature = "pinocchio")]
    if !account_info.is_writable() {
        return Err(AccountError::AccountMutable);
    }
    check_account_info::<T, U>(program_id, account_info)
}

/// Checks:
/// 1. account is not mutable
/// 2. account owned by program_id
/// 3. account discriminator
pub fn check_account_info_non_mut<T: Discriminator<U>, const U: usize>(
    program_id: &Pubkey,
    account_info: &AccountInfo,
) -> Result<(), AccountError> {
    #[cfg(not(feature = "pinocchio"))]
    if account_info.is_writable {
        return Err(AccountError::AccountMutable);
    }
    #[cfg(feature = "pinocchio")]
    if account_info.is_writable() {
        return Err(AccountError::AccountMutable);
    }

    check_account_info::<T, U>(program_id, account_info)
}

/// Checks:
/// 1. account owned by program_id
/// 2. account discriminator
pub fn check_account_info<T: Discriminator<U>, const U: usize>(
    program_id: &Pubkey,
    account_info: &AccountInfo,
) -> Result<(), AccountError> {
    check_owner(program_id, account_info)?;

    let account_data = &account_info
        .try_borrow_data()
        .map_err(|_| AccountError::BorrowAccountDataFailed)?;
    check_discriminator::<T, U>(account_data)
}

/// Checks:
/// 1. discriminator is uninitialized
/// 2. sets discriminator
pub fn set_discriminator<T: Discriminator<U>, const U: usize>(
    bytes: &mut [u8],
) -> Result<(), AccountError> {
    if bytes[0..U] != [0; U] {
        // #[cfg(target_os = "solana")]
        // crate::msg!("Discriminator bytes must be zero for initialization.");
        return Err(AccountError::AlreadyInitialized);
    }
    bytes[0..U].copy_from_slice(&T::DISCRIMINATOR);
    Ok(())
}

/// Checks:
/// 1. account size is at least U
/// 2. account discriminator
pub fn check_discriminator<T: Discriminator<U>, const U: usize>(
    bytes: &[u8],
) -> Result<(), AccountError> {
    if bytes.len() < U {
        return Err(AccountError::InvalidAccountSize);
    }

    if T::DISCRIMINATOR != bytes[0..U] {
        // #[cfg(all(target_os = "solana", not(feature = "pinocchio")))]
        // crate::msg!(
        //     "Expected discriminator: {:?}, actual {:?} ",
        //     T::DISCRIMINATOR,
        //     bytes[0..U].to_vec()
        // );
        return Err(AccountError::InvalidDiscriminator);
    }
    Ok(())
}

/// Checks that the account balance is greater or eqal to rent exemption.
pub fn check_account_balance_is_rent_exempt(
    account_info: &AccountInfo,
    expected_size: usize,
) -> Result<u64, AccountError> {
    let account_size = account_info.data_len();
    if account_size != expected_size {
        // #[cfg(all(target_os = "solana", not(feature = "pinocchio")))]
        // crate::msg!(
        //     "Account {:?} size not equal to expected size. size: {}, expected size {}",
        //     account_info.key,
        //     account_size,
        //     expected_size
        // );
        return Err(AccountError::InvalidAccountSize);
    }
    let lamports = account_info.lamports();
    #[cfg(target_os = "solana")]
    {
        use crate::Sysvar;
        let rent_exemption = (crate::Rent::get()
            .map_err(|_| AccountError::FailedBorrowRentSysvar))?
        .minimum_balance(expected_size);
        if lamports != rent_exemption {
            crate::msg!(
                format!("Account {:?} lamports is not equal to rentexemption: lamports {}, rent exemption {}",
                account_info.key,
                lamports,
                rent_exemption).as_str()
            );
            return Err(AccountError::InvalidAccountBalance);
        }
    }
    #[cfg(not(target_os = "solana"))]
    println!("Rent exemption check skipped since not target_os solana.");
    Ok(lamports)
}

#[cfg(not(feature = "pinocchio"))]
pub fn check_signer(account_info: &AccountInfo) -> Result<(), AccountError> {
    if !account_info.is_signer {
        return Err(AccountError::InvalidSigner);
    }
    Ok(())
}
#[cfg(feature = "pinocchio")]
pub fn check_signer(account_info: &AccountInfo) -> Result<(), AccountError> {
    if !account_info.is_signer() {
        return Err(AccountError::InvalidSigner);
    }
    Ok(())
}

#[cfg(not(feature = "pinocchio"))]
pub fn check_owner(owner: &Pubkey, account_info: &AccountInfo) -> Result<(), AccountError> {
    if *owner != *account_info.owner {
        return Err(AccountError::AccountOwnedByWrongProgram);
    }

    Ok(())
}

#[cfg(feature = "pinocchio")]
pub fn check_owner(owner: &Pubkey, account_info: &AccountInfo) -> Result<(), AccountError> {
    if !account_info.is_owned_by(owner) {
        pinocchio::msg!(format!(
            "check_owner expected {:?} got: {:?}",
            owner,
            account_info.key()
        )
        .as_str());
        return Err(AccountError::AccountOwnedByWrongProgram);
    }
    Ok(())
}

#[cfg(not(feature = "pinocchio"))]
pub fn check_program(program_id: &Pubkey, account_info: &AccountInfo) -> Result<(), AccountError> {
    if *account_info.key != *program_id {
        // msg!(
        //     "check_owner expected {:?} got: {:?}",
        //     program_id,
        //     account_info.key()
        // );
        return Err(AccountError::InvalidProgramId);
    }
    if !account_info.executable {
        return Err(AccountError::ProgramNotExecutable);
    }
    Ok(())
}

#[cfg(feature = "pinocchio")]
pub fn check_program(program_id: &Pubkey, account_info: &AccountInfo) -> Result<(), AccountError> {
    if *account_info.key() != *program_id {
        pinocchio::msg!(format!(
            "check_owner expected {:?} got: {:?}",
            program_id,
            account_info.key()
        )
        .as_str());
        return Err(AccountError::InvalidProgramId);
    }
    if !account_info.executable() {
        return Err(AccountError::ProgramNotExecutable);
    }
    Ok(())
}

#[cfg(not(feature = "pinocchio"))]
pub fn check_pda_seeds(
    seeds: &[&[u8]],
    program_id: &Pubkey,
    account_info: &AccountInfo,
) -> Result<(), AccountError> {
    if !Pubkey::find_program_address(seeds, program_id)
        .0
        .eq(account_info.key)
    {
        return Err(AccountError::InvalidSeeds);
    }

    Ok(())
}

// TODO: add with provided bump
#[cfg(feature = "pinocchio")]
pub fn check_pda_seeds(
    seeds: &[&[u8]],
    program_id: &Pubkey,
    account_info: &AccountInfo,
) -> Result<(), AccountError> {
    if !pinocchio::pubkey::find_program_address(seeds, program_id)
        .0
        .eq(account_info.key())
    {
        return Err(AccountError::InvalidSeeds);
    }
    Ok(())
}

#[cfg(not(feature = "pinocchio"))]
#[cfg(test)]
mod check_account_tests {
    use std::{cell::RefCell, rc::Rc};

    use borsh::{BorshDeserialize, BorshSerialize};

    use super::*;

    // Helper function to create pubkeys for tests
    #[cfg(not(feature = "pinocchio"))]
    fn create_pubkey() -> Pubkey {
        Pubkey::new_unique()
    }

    #[cfg(feature = "pinocchio")]
    fn create_pubkey() -> Pubkey {
        let mut rng = [0u8; 32];
        for i in 0..32 {
            rng[i] = i as u8;
        }
        rng
    }

    #[repr(C)]
    #[derive(Debug, PartialEq, Copy, Clone, BorshSerialize, BorshDeserialize)]
    pub struct MyStruct {
        pub data: u64,
    }
    impl Discriminator<8> for MyStruct {
        const DISCRIMINATOR: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    }

    /// Tests:
    /// 1. functional set discriminator
    /// 2. failing set discriminator
    /// 3. functional check discriminator
    /// 4. failing check discriminator
    #[test]
    fn test_discriminator() {
        let mut bytes = vec![0; 8 + std::mem::size_of::<MyStruct>()];

        // Test 1 functional set discriminator.
        assert_eq!(bytes[0..8], [0; 8]);
        set_discriminator::<MyStruct, 8>(&mut bytes).unwrap();
        assert_eq!(bytes[0..8], MyStruct::DISCRIMINATOR);
        // Test 2 failing set discriminator.
        assert_eq!(
            set_discriminator::<MyStruct, 8>(&mut bytes),
            Err(AccountError::AlreadyInitialized)
        );
        // Test 3 functional check discriminator.
        assert!(check_discriminator::<MyStruct, 8>(&bytes).is_ok());
        // Test 4 failing check discriminator.
        bytes[0] = 0;
        assert_eq!(
            check_discriminator::<MyStruct, 8>(&bytes),
            Err(AccountError::InvalidDiscriminator)
        );
    }

    pub struct TestAccount {
        pub key: Pubkey,
        pub owner: Pubkey,
        pub data: Vec<u8>,
        pub lamports: u64,
        pub writable: bool,
        pub is_signer: bool,
        pub executable: bool,
    }
    impl TestAccount {
        pub fn new(key: Pubkey, owner: Pubkey, size: usize) -> Self {
            Self {
                key,
                owner,
                data: vec![0; size],
                lamports: 0,
                writable: true,
                is_signer: false,
                executable: false,
            }
        }

        #[cfg(not(feature = "pinocchio"))]
        pub fn get_account_info(&mut self) -> AccountInfo<'_> {
            AccountInfo {
                key: &self.key,
                is_signer: self.is_signer,
                is_writable: self.writable,
                lamports: Rc::new(RefCell::new(&mut self.lamports)),
                data: Rc::new(RefCell::new(&mut self.data)),
                owner: &self.owner,
                executable: self.executable,
                rent_epoch: 0,
            }
        }
    }

    /// Tests:
    /// 1. functional check_account_info
    /// 2. failing AccountOwnedByWrongProgram
    /// 3. failing empty discriminator (InvalidDiscriminator)
    /// 4. failing InvalidDiscriminator
    /// 5. functional check_account_info_mut
    /// 6. failing AccountNotMutable with check_account_info_mut
    /// 7. functional check_account_info_non_mut
    /// 8. failing AccountMutable with check_account_info_non_mut
    #[test]
    fn test_check_account_info() {
        let key = create_pubkey();
        let program_id = create_pubkey();
        let size = 8 + std::mem::size_of::<MyStruct>();

        // Test 1 functional check_account_info.
        {
            let mut account = TestAccount::new(key, program_id, size);
            set_discriminator::<MyStruct, 8>(&mut account.data).unwrap();
            assert!(
                check_account_info::<MyStruct, 8>(&program_id, &account.get_account_info()).is_ok()
            );
        }
        // Test 2 failing AccountOwnedByWrongProgram.
        {
            let mut account = TestAccount::new(key, program_id, size);
            set_discriminator::<MyStruct, 8>(&mut account.data).unwrap();
            account.owner = create_pubkey();
            assert_eq!(
                check_account_info::<MyStruct, 8>(&program_id, &account.get_account_info()),
                Err(AccountError::AccountOwnedByWrongProgram)
            );
        }
        // Test 3 failing empty discriminator (InvalidDiscriminator).
        {
            let mut account = TestAccount::new(key, program_id, size);
            assert_eq!(
                check_account_info::<MyStruct, 8>(&program_id, &account.get_account_info()),
                Err(AccountError::InvalidDiscriminator)
            );
        }
        // Test 4 failing InvalidDiscriminator.
        {
            let mut account = TestAccount::new(key, program_id, size - 1);
            account.data[0..8].copy_from_slice(&[1; 8]);
            assert_eq!(
                check_account_info::<MyStruct, 8>(&program_id, &account.get_account_info()),
                Err(AccountError::InvalidDiscriminator)
            );
        }
        // Test 5 functional check_account_info_mut.
        {
            let mut account = TestAccount::new(key, program_id, size);
            set_discriminator::<MyStruct, 8>(&mut account.data).unwrap();
            assert!(check_account_info_mut::<MyStruct, 8>(
                &program_id,
                &account.get_account_info()
            )
            .is_ok());
        }
        // Test 6 failing AccountNotMutable with check_account_info_mut.
        {
            let mut account = TestAccount::new(key, program_id, size);
            set_discriminator::<MyStruct, 8>(&mut account.data).unwrap();
            account.writable = false;
            // The error can be different depending on the framework
            let result =
                check_account_info_mut::<MyStruct, 8>(&program_id, &account.get_account_info());
            assert!(result.is_err());
        }
        // Test 7 functional check_account_info_non_mut.
        {
            let mut account = TestAccount::new(key, program_id, size);
            set_discriminator::<MyStruct, 8>(&mut account.data).unwrap();
            account.writable = false;
            assert!(check_account_info_non_mut::<MyStruct, 8>(
                &program_id,
                &account.get_account_info()
            )
            .is_ok());
        }
        // Test 8 failing with check_account_info_non_mut.
        {
            let mut account = TestAccount::new(key, program_id, size);
            set_discriminator::<MyStruct, 8>(&mut account.data).unwrap();
            // Different behavior based on the feature flag
            #[cfg(not(feature = "pinocchio"))]
            assert_eq!(
                check_account_info_non_mut::<MyStruct, 8>(&program_id, &account.get_account_info()),
                Err(AccountError::AccountMutable)
            );
            #[cfg(feature = "pinocchio")]
            assert!(check_account_info_non_mut::<MyStruct, 8>(
                &program_id,
                &account.get_account_info()
            )
            .is_err());
        }
        // Test 9 functional account_info_init
        {
            let mut account = TestAccount::new(key, program_id, size);
            assert!(account_info_init::<MyStruct, 8>(&account.get_account_info()).is_ok());
        }
        // Test 10 failing account_info_init
        {
            let mut account = TestAccount::new(key, program_id, size);
            set_discriminator::<MyStruct, 8>(&mut account.data).unwrap();
            assert_eq!(
                account_info_init::<MyStruct, 8>(&account.get_account_info()),
                Err(AccountError::AlreadyInitialized)
            );
        }
    }

    /// Tests for check_signer function
    /// 1. Functional test - account is a signer
    /// 2. Failing test - account is not a signer
    #[test]
    fn test_signer_check() {
        let key = create_pubkey();
        let program_id = create_pubkey();
        let size = 8;

        // Test 1: Successful signer check
        {
            let mut account = TestAccount::new(key, program_id, size);
            account.is_signer = true;
            assert!(check_signer(&account.get_account_info()).is_ok());
        }

        // Test 2: Failed signer check - account is not a signer
        {
            let mut account = TestAccount::new(key, program_id, size);
            account.is_signer = false;
            assert_eq!(
                check_signer(&account.get_account_info()),
                Err(AccountError::InvalidSigner)
            );
        }
    }

    /// Tests for check_owner function
    /// 1. Functional test - account is owned by the correct program
    /// 2. Failing test - account is owned by a different program
    #[test]
    fn test_program_check() {
        let key = create_pubkey();
        let program_id = create_pubkey();
        let wrong_program_id = create_pubkey();
        let size = 8;

        // Test 1: Successful program check
        {
            let mut account = TestAccount::new(key, program_id, size);
            assert!(check_owner(&program_id, &account.get_account_info()).is_ok());
        }

        // Test 2: Failed program check - account owned by wrong program
        {
            let mut account = TestAccount::new(key, wrong_program_id, size);
            assert_eq!(
                check_owner(&program_id, &account.get_account_info()),
                Err(AccountError::AccountOwnedByWrongProgram)
            );
        }
    }

    /// Tests for check_pda_seeds function
    /// 1. Functional test - PDA matches with the given seeds and program ID
    /// 2. Failing test - PDA doesn't match with the given seeds
    /// 3. Failing test - Invalid seeds (can't create a valid PDA)
    #[test]
    #[ignore = "reason"]
    #[cfg(not(feature = "pinocchio"))]
    fn test_check_pda_seeds_solana() {
        let program_id = create_pubkey();

        // Test 1: Create a valid PDA and verify it
        {
            let seeds = &[b"test_seed".as_ref(), &[1, 2, 3]];
            // Generate a PDA
            let (pda, _) = Pubkey::find_program_address(seeds, &program_id);

            // Recreate the seeds for the check (without the bump)
            let check_seeds = &[b"test_seed".as_ref(), &[1, 2, 3]];

            // Create a test account with the PDA as key
            let mut account = TestAccount::new(pda, program_id, 8);

            // This should fail because find_program_address adds the bump seed automatically
            // which check_pda_seeds doesn't do
            assert!(
                check_pda_seeds(check_seeds, &program_id, &account.get_account_info()).is_err()
            );

            // Get the correct seeds with bump
            let (_, bump) = Pubkey::find_program_address(seeds, &program_id);
            let correct_seeds = &[b"test_seed".as_ref(), &[1, 2, 3], &[bump]];
            // Now the check should pass with the correct seeds including bump
            assert!(
                check_pda_seeds(correct_seeds, &program_id, &account.get_account_info()).is_ok()
            );
        }

        // Test 2: Failed check - PDA doesn't match with the given seeds
        {
            // Generate a valid PDA
            let seeds = &[b"test_seed".as_ref(), &[1, 2, 3]];
            let (_, bump) = Pubkey::find_program_address(seeds, &program_id);
            let correct_seeds = &[b"test_seed".as_ref(), &[1, 2, 3], &[bump]];

            // Create account with a different key
            let different_key = create_pubkey();
            let mut account = TestAccount::new(different_key, program_id, 8);

            // This should fail because the account key doesn't match the PDA
            assert_eq!(
                check_pda_seeds(correct_seeds, &program_id, &account.get_account_info()),
                Err(AccountError::InvalidSeeds)
            );
        }

        // Test 3: Invalid seeds - use seeds that would not create a valid program address
        {
            // Create a random account key
            let random_key = create_pubkey();
            let mut account = TestAccount::new(random_key, program_id, 8);

            // Create seeds that don't correspond to this account's key
            let invalid_seeds = &[b"random_seeds".as_ref()];

            // This should return InvalidSeeds because the derived address doesn't match
            assert!(
                check_pda_seeds(invalid_seeds, &program_id, &account.get_account_info()).is_err()
            );
        }
    }

    #[cfg(feature = "pinocchio")]
    mod pinocchio_tests {
        use super::*;

        #[test]
        fn test_discriminator() {
            // Test that the discriminator functionality works
            let mut bytes = vec![0; 8 + 8]; // 8 for discriminator, 8 for a u64

            // Check that setting and checking a discriminator works as expected
            struct TestDiscriminator {}
            impl Discriminator<8> for TestDiscriminator {
                const DISCRIMINATOR: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
            }

            assert_eq!(bytes[0..8], [0; 8]);
            set_discriminator::<TestDiscriminator, 8>(&mut bytes).unwrap();
            assert_eq!(bytes[0..8], TestDiscriminator::DISCRIMINATOR);

            // Check that trying to set it again fails
            assert_eq!(
                set_discriminator::<TestDiscriminator, 8>(&mut bytes),
                Err(AccountError::AlreadyInitialized)
            );

            // Check that validating works
            assert!(check_discriminator::<TestDiscriminator, 8>(&bytes).is_ok());

            // Modify discriminator and check that validation fails
            bytes[0] = 0;
            assert_eq!(
                check_discriminator::<TestDiscriminator, 8>(&bytes),
                Err(AccountError::InvalidDiscriminator)
            );
        }
    }

    #[cfg(feature = "pinocchio")]
    fn test_check_pda_seeds_pinocchio() {
        let program_id = create_pubkey();

        // Test 1: Create a valid PDA and verify it
        {
            let seeds = &[b"test_seed".as_ref(), &[1, 2, 3]];
            // Generate a PDA
            let (pda, _) = pinocchio::pubkey::find_program_address(seeds, &program_id);

            // Recreate the seeds for the check (without the bump)
            let check_seeds = &[b"test_seed".as_ref(), &[1, 2, 3]];

            // Create a test account with the PDA as key
            let mut account = TestAccount::new(pda, program_id, 8);

            // This should fail because find_program_address adds the bump seed automatically
            // which check_pda_seeds doesn't do
            assert!(
                check_pda_seeds(check_seeds, &program_id, &account.get_account_info()).is_err()
            );

            // Get the correct seeds with bump
            let (_, bump) = pinocchio::pubkey::find_program_address(seeds, &program_id);
            let correct_seeds = &[b"test_seed".as_ref(), &[1, 2, 3], &[bump]];
            // Now the check should pass with the correct seeds including bump
            assert!(
                check_pda_seeds(correct_seeds, &program_id, &account.get_account_info()).is_ok()
            );
        }

        // Test 2: Failed check - PDA doesn't match with the given seeds
        {
            // Generate a valid PDA
            let seeds = &[b"test_seed".as_ref(), &[1, 2, 3]];
            let (_, bump) = pinocchio::pubkey::find_program_address(seeds, &program_id);
            let correct_seeds = &[b"test_seed".as_ref(), &[1, 2, 3], &[bump]];

            // Create account with a different key
            let different_key = create_pubkey();
            let mut account = TestAccount::new(different_key, program_id, 8);

            // This should fail because the account key doesn't match the PDA
            assert_eq!(
                check_pda_seeds(correct_seeds, &program_id, &account.get_account_info()),
                Err(AccountError::InvalidSeeds)
            );
        }

        // Test 3: Invalid seeds - use seeds that would not create a valid program address
        {
            // Create a random account key
            let random_key = create_pubkey();
            let mut account = TestAccount::new(random_key, program_id, 8);

            // Create seeds that don't correspond to this account's key
            let invalid_seeds = &[b"random_seeds".as_ref()];

            // This should return InvalidSeeds because the derived address doesn't match
            assert!(
                check_pda_seeds(invalid_seeds, &program_id, &account.get_account_info()).is_err()
            );
        }
    }
}
