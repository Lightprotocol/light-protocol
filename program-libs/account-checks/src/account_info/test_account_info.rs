#![cfg(feature = "test-only")]

#[cfg(feature = "pinocchio")]
pub mod pinocchio {
    extern crate std;
    use std::vec::Vec;

    use pinocchio::{account_info::AccountInfo, instruction::Account, pubkey::Pubkey};
    use rand::{prelude::Rng, thread_rng};

    pub fn pubkey_unique() -> Pubkey {
        let mut rng = thread_rng();
        rng.gen::<[u8; 32]>()
    }

    pub fn get_account_info(
        address: Pubkey,
        owner: Pubkey,
        is_signer: bool,
        is_writable: bool,
        is_executable: bool,
        data: Vec<u8>,
    ) -> AccountInfo {
        // The Account struct has fields for flags, pubkeys, lamports, etc
        // Correct size for an Account struct - it's larger than what was originally used
        let account_size = 88; // Total size of Account struct

        // Allocate memory for Account + data
        let mut raw_data = vec![0u8; account_size + data.len()];

        // Set the boolean flags - use 1 for true as the AccountInfo implementation checks for non-zero
        // IMPORTANT: borrow_state needs to be 0xFF (all bits set) to indicate unborrowed state
        raw_data[0] = 0xFF; // borrow_state - all bits set means unborrowed
        raw_data[1] = if is_signer { 1 } else { 0 }; // is_signer
        raw_data[2] = if is_writable { 1 } else { 0 }; // is_writable
        raw_data[3] = if is_executable { 1 } else { 0 }; // executable

        // resize_delta at offset 4 (changed from original_data_len in pinocchio 0.9)
        raw_data[4..8].copy_from_slice(&0i32.to_le_bytes());

        // key at offset 8
        raw_data[8..40].copy_from_slice(address.as_ref());

        // owner at offset 40
        raw_data[40..72].copy_from_slice(owner.as_ref());

        // lamports at offset 72
        raw_data[72..80].copy_from_slice(&1000u64.to_le_bytes());

        // data_len at offset 80 - this is crucial to fix the issue
        raw_data[80..88].copy_from_slice(&(data.len() as u64).to_le_bytes());

        // copy the actual data after the Account struct
        if !data.is_empty() {
            raw_data[account_size..account_size + data.len()].copy_from_slice(&data);
        }

        // Create the AccountInfo by pointing to our raw Account data
        let account_ptr = raw_data.as_mut_ptr() as *mut Account;
        let mut account_info_raw = vec![0u8; core::mem::size_of::<AccountInfo>()];
        account_info_raw[0..8].copy_from_slice(&(account_ptr as u64).to_le_bytes());

        // Need to leak the memory so it doesn't get dropped while the AccountInfo is still using it
        core::mem::forget(raw_data);
        core::mem::forget(account_info_raw);

        unsafe { core::mem::transmute::<*mut Account, AccountInfo>(account_ptr) }
    }

    #[test]
    fn test_get_account_info() {
        let mut rng = thread_rng();
        for _ in 0..1000 {
            let address = pubkey_unique();
            let owner = pubkey_unique();
            let is_signer = rng.gen();
            let is_writable = rng.gen();
            let is_executable = rng.gen();
            let data_len: u64 = rng.gen_range(0..3000);
            let data = (0..data_len).map(|_| rng.gen::<u8>()).collect::<Vec<u8>>();

            let account_info = get_account_info(
                address,
                owner,
                is_signer,
                is_writable,
                is_executable,
                data.clone(),
            );

            // Test the account matches the values we set
            assert_eq!(account_info.is_signer(), is_signer);
            assert_eq!(account_info.is_writable(), is_writable);
            assert_eq!(account_info.executable(), is_executable);
            assert_eq!(account_info.data_len(), data.len());

            // Test we can access the account data - this was the failing part originally
            unsafe {
                let account_data = account_info.borrow_data_unchecked();
                assert_eq!(account_data.len(), data.len());
                for (i, val) in data.iter().enumerate() {
                    assert_eq!(account_data[i], *val);
                }
            }
        }
    }
}

#[cfg(all(feature = "solana", feature = "std"))]
pub mod solana_program {
    extern crate std;
    use std::{cell::RefCell, rc::Rc, vec, vec::Vec};

    use solana_account_info::AccountInfo;
    use solana_pubkey::Pubkey;

    #[derive(Debug, PartialEq, Clone)]
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
}
