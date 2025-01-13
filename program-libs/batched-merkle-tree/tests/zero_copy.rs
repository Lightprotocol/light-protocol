use std::{cell::RefCell, rc::Rc};

use light_batched_merkle_tree::{
    constants::ACCOUNT_COMPRESSION_PROGRAM_ID,
    initialize_address_tree::{
        init_batched_address_merkle_tree_from_account_info, InitAddressTreeAccountsInstructionData,
    },
    initialize_state_tree::{
        init_batched_state_merkle_tree_from_account_info, InitStateTreeAccountsInstructionData,
    },
    merkle_tree::{get_merkle_tree_account_size_default, BatchedMerkleTreeAccount},
    queue::{get_output_queue_account_size_default, BatchedQueueAccount},
};
use solana_program::{account_info::AccountInfo, pubkey::Pubkey};

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

/// Test:
/// 1. functional init_batched_address_merkle_tree_from_account_info
/// 2. failing already initialized
/// 3. functional address_tree_from_account_info_mut
/// 4. failing invalid owner
/// 5. failing invalid discriminator
#[test]
fn address_from_account_info() {
    let key = Pubkey::new_unique();
    let owner = ACCOUNT_COMPRESSION_PROGRAM_ID;
    let mt_account_size = get_merkle_tree_account_size_default();
    let mut account = TestAccount::new(key, owner, mt_account_size);

    let params = InitAddressTreeAccountsInstructionData::test_default();
    let merkle_tree_rent = 1_000_000_000;
    account.lamports = merkle_tree_rent;
    // Test 1 functional init_batched_address_merkle_tree_from_account_info
    {
        let result = init_batched_address_merkle_tree_from_account_info(
            params,
            owner.into(),
            &account.get_account_info(),
        );
        assert!(result.is_ok());
    }
    // Test 2 already initialized
    {
        let result = init_batched_address_merkle_tree_from_account_info(
            params,
            owner.into(),
            &account.get_account_info(),
        );
        assert!(matches!(result,
            Err(error)  if   error.to_string().contains("Account is already initialized.")));
    }
    // Test 3 functional address_tree_from_account_info_mut
    let account_info = account.get_account_info();
    let result = BatchedMerkleTreeAccount::address_tree_from_account_info_mut(&account_info);
    assert!(result.is_ok());

    // Test 4 failing invalid owner
    {
        let mut account = account.clone();
        account.owner = Pubkey::new_unique();
        let account_info = account.get_account_info();
        let result = BatchedMerkleTreeAccount::address_tree_from_account_info_mut(&account_info);
        assert!(matches!(result,
           Err(error)  if   error.to_string().contains("Account owned by wrong program.")));
    }
    // Test 5 failing invalid discriminator
    {
        let mut account = account.clone();
        account.data[0] = 1;
        let account_info = account.get_account_info();
        let result = BatchedMerkleTreeAccount::address_tree_from_account_info_mut(&account_info);
        assert!(matches!(result,
           Err(error)  if   error.to_string().contains("Invalid Discriminator.")));
    }
}

/// Tests:
/// 1. functional init_batched_state_merkle_tree_from_account_info
/// 2. failing already initialized
/// 3. functional state_tree_from_account_info_mut
/// 4. failing invalid owner (state tree)
/// 5. failing invalid discriminator (state tree)
/// 6. functional output_queue_from_account_info_mut
/// 7. failing invalid owner (output queue)
/// 8. failing invalid discriminator (output queue)
#[test]
fn state_from_account_info() {
    let key = Pubkey::new_unique();
    let owner = ACCOUNT_COMPRESSION_PROGRAM_ID;
    let mt_account_size = get_merkle_tree_account_size_default();
    let output_queue_size = get_output_queue_account_size_default();
    let mut merkle_tree_account = TestAccount::new(key, owner, mt_account_size);
    let mut output_queue_account = TestAccount::new(Pubkey::new_unique(), owner, output_queue_size);

    let params = InitStateTreeAccountsInstructionData::test_default();
    let merkle_tree_rent = 1_000_000_000;
    merkle_tree_account.lamports = merkle_tree_rent;
    output_queue_account.lamports = merkle_tree_rent;
    let additional_rent = 1_000_000_000;
    // create first merkle tree

    // Test 1 functional init_batched_state_merkle_tree_from_account_info
    {
        let output_queue_account_info = output_queue_account.get_account_info();
        let merkle_tree_account_info = merkle_tree_account.get_account_info();

        let result = init_batched_state_merkle_tree_from_account_info(
            params,
            owner,
            &merkle_tree_account_info,
            &output_queue_account_info,
            additional_rent,
        );
        assert!(result.is_ok());
    }
    // Test 2 failing already initialized
    {
        let output_queue_account_info = output_queue_account.get_account_info();
        let merkle_tree_account_info = merkle_tree_account.get_account_info();

        let result = init_batched_state_merkle_tree_from_account_info(
            params,
            owner,
            &merkle_tree_account_info,
            &output_queue_account_info,
            additional_rent,
        );
        assert!(matches!(result,
        Err(error)  if   error.to_string().contains("Account is already initialized.")));
    }
    // Test 3 functional state_tree_from_account_info_mut
    {
        let account_info = merkle_tree_account.get_account_info();
        let result = BatchedMerkleTreeAccount::state_tree_from_account_info_mut(&account_info);
        assert!(result.is_ok());
    }
    // Test 4 failing invalid owner
    {
        let mut account = merkle_tree_account.clone();
        account.owner = Pubkey::new_unique();
        let account_info = account.get_account_info();

        let result = BatchedMerkleTreeAccount::state_tree_from_account_info_mut(&account_info);
        assert!(matches!(result,
           Err(error)  if   error.to_string().contains("Account owned by wrong program.")));
    }
    // Test 5 failing invalid discriminator
    {
        let mut account = merkle_tree_account.clone();
        account.data[0] = 1;
        let account_info = account.get_account_info();
        let result = BatchedMerkleTreeAccount::state_tree_from_account_info_mut(&account_info);
        assert!(matches!(result,
           Err(error)  if   error.to_string().contains("Invalid Discriminator.")));
    }
    // Test 6 functional output_queue_from_account_info_mut
    {
        let result = BatchedQueueAccount::output_queue_from_account_info_mut(
            &output_queue_account.get_account_info(),
        );
        assert!(result.is_ok());
    }
    // Test 7 failing invalid owner
    {
        let mut output_queue_account = output_queue_account.clone();
        output_queue_account.owner = Pubkey::new_unique();
        let account_info = output_queue_account.get_account_info();
        let result = BatchedQueueAccount::output_queue_from_account_info_mut(&account_info);
        assert!(matches!(result,
           Err(error)  if   error.to_string().contains("Account owned by wrong program.")));
    }
    // Test 8 failing invalid discriminator
    {
        let mut output_queue_account = output_queue_account.clone();
        output_queue_account.data[0] = 1;
        let account_info = output_queue_account.get_account_info();

        let result = BatchedQueueAccount::output_queue_from_account_info_mut(&account_info);
        assert!(matches!(result,
           Err(error)  if   error.to_string().contains("Invalid Discriminator.")));
    }
}
