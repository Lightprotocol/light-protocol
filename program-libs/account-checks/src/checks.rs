use solana_program::{account_info::AccountInfo, pubkey::Pubkey};
#[cfg(target_os = "solana")]
use solana_program::{rent::Rent, sysvar::Sysvar};

use crate::{discriminator::Discriminator, error::AccountError};

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
    if !account_info.is_writable {
        return Err(AccountError::AccountNotMutable);
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
    if account_info.is_writable {
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
    if *program_id != *account_info.owner {
        return Err(AccountError::AccountOwnedByWrongProgram);
    }

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
        #[cfg(target_os = "solana")]
        solana_program::msg!("Discriminator bytes must be zero for initialization.");
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
        #[cfg(target_os = "solana")]
        solana_program::msg!(
            "Expected discriminator: {:?}, actual {:?} ",
            T::DISCRIMINATOR,
            bytes[0..U].to_vec()
        );
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
        #[cfg(target_os = "solana")]
        solana_program::msg!(
            "Account {:?} size not equal to expected size. size: {}, expected size {}",
            account_info.key,
            account_size,
            expected_size
        );
        return Err(AccountError::InvalidAccountSize);
    }
    let lamports = account_info.lamports();
    #[cfg(target_os = "solana")]
    {
        let rent_exemption = (Rent::get().map_err(|_| AccountError::FailedBorrowRentSysvar))?
            .minimum_balance(expected_size);
        if lamports < rent_exemption {
            solana_program::msg!(
            "Account {:?} lamports is less than rentexemption: lamports {}, rent exemption {}",
            account_info.key,
            lamports,
            rent_exemption
        );
            return Err(AccountError::InvalidAccountBalance);
        }
    }
    #[cfg(not(target_os = "solana"))]
    println!("Rent exemption check skipped since not target_os solana.");
    Ok(lamports)
}

#[cfg(test)]
mod check_account_tests {
    use std::{cell::RefCell, rc::Rc};

    use borsh::{BorshDeserialize, BorshSerialize};

    use super::*;

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
    }
    impl TestAccount {
        pub fn new(key: Pubkey, owner: Pubkey, size: usize) -> Self {
            Self {
                key,
                owner,
                data: vec![0; size],
                lamports: 0,
                writable: true,
            }
        }

        pub fn get_account_info(&mut self) -> AccountInfo<'_> {
            AccountInfo {
                key: &self.key,
                is_signer: false,
                is_writable: self.writable,
                lamports: Rc::new(RefCell::new(&mut self.lamports)),
                data: Rc::new(RefCell::new(&mut self.data)),
                owner: &self.owner,
                executable: false,
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
        let key = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();
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
            account.owner = Pubkey::new_unique();
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
            assert_eq!(
                check_account_info_mut::<MyStruct, 8>(&program_id, &account.get_account_info()),
                Err(AccountError::AccountNotMutable)
            );
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
        // Test 8 failing AccountMutable with check_account_info_non_mut.
        {
            let mut account = TestAccount::new(key, program_id, size);
            set_discriminator::<MyStruct, 8>(&mut account.data).unwrap();
            assert_eq!(
                check_account_info_non_mut::<MyStruct, 8>(&program_id, &account.get_account_info()),
                Err(AccountError::AccountMutable)
            );
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
}
