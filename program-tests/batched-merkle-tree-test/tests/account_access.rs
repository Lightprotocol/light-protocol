use light_account_checks::account_info::test_account_info::solana_program::TestAccount;
use light_batched_merkle_tree::{
    constants::{ACCOUNT_COMPRESSION_PROGRAM_ID, ADDRESS_TREE_INIT_ROOT_40},
    initialize_address_tree::{
        get_address_merkle_tree_account_size_from_params,
        init_batched_address_merkle_tree_from_account_info, InitAddressTreeAccountsInstructionData,
    },
    initialize_state_tree::{
        init_batched_state_merkle_tree_from_account_info, InitStateTreeAccountsInstructionData,
    },
    merkle_tree::{test_utils::get_merkle_tree_account_size_default, BatchedMerkleTreeAccount},
    queue::{test_utils::get_output_queue_account_size_default, BatchedQueueAccount},
};
use light_hasher::zero_bytes;
use solana_pubkey::Pubkey;

/// Test:
/// 1. functional init_batched_address_merkle_tree_from_account_info
/// 2. failing already initialized
/// 3. functional address_from_account_info
/// 4. failing invalid owner
/// 5. failing invalid discriminator
#[test]
fn address_from_account_info() {
    let key = Pubkey::new_unique();
    let owner = ACCOUNT_COMPRESSION_PROGRAM_ID;
    let mt_account_size = get_merkle_tree_account_size_default();
    let mut account = TestAccount::new(key, owner.into(), mt_account_size);

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
    // Test 3 functional address_from_account_info
    let account_info = account.get_account_info();
    let result = BatchedMerkleTreeAccount::address_from_account_info(&account_info);
    assert!(result.is_ok());

    // Test 4 failing invalid owner
    {
        let mut account = account.clone();
        account.owner = Pubkey::new_unique();
        let account_info = account.get_account_info();
        let result = BatchedMerkleTreeAccount::address_from_account_info(&account_info);
        assert!(matches!(result,
           Err(error)  if   error.to_string().contains("Account owned by wrong program.")));
    }
    // Test 5 failing invalid discriminator
    {
        let mut account = account.clone();
        account.data[0] = 1;
        let account_info = account.get_account_info();
        let result = BatchedMerkleTreeAccount::address_from_account_info(&account_info);
        assert!(matches!(result,
           Err(error)  if   error.to_string().contains("Invalid Discriminator.")));
    }
}

/// Tests:
/// 1. functional init_batched_state_merkle_tree_from_account_info
/// 2. failing already initialized
/// 3. functional state_from_account_info
/// 4. failing invalid owner (state tree)
/// 5. failing invalid discriminator (state tree)
/// 6. functional output_from_account_info
/// 7. failing invalid owner (output queue)
/// 8. failing invalid discriminator (output queue)
#[test]
fn state_from_account_info() {
    let key = Pubkey::new_unique();
    let owner = ACCOUNT_COMPRESSION_PROGRAM_ID;
    let mt_account_size = get_merkle_tree_account_size_default();
    let output_queue_size = get_output_queue_account_size_default();
    let mut merkle_tree_account = TestAccount::new(key, owner.into(), mt_account_size);
    let mut output_queue_account =
        TestAccount::new(Pubkey::new_unique(), owner.into(), output_queue_size);

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
            owner.into(),
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
            owner.into(),
            &merkle_tree_account_info,
            &output_queue_account_info,
            additional_rent,
        );
        assert!(matches!(result,
        Err(error)  if   error.to_string().contains("Account is already initialized.")));
    }
    // Test 3 functional state_from_account_info
    {
        let account_info = merkle_tree_account.get_account_info();
        let result = BatchedMerkleTreeAccount::state_from_account_info(&account_info);
        assert!(result.is_ok());
    }
    // Test 4 failing invalid owner
    {
        let mut account = merkle_tree_account.clone();
        account.owner = Pubkey::new_unique();
        let account_info = account.get_account_info();

        let result = BatchedMerkleTreeAccount::state_from_account_info(&account_info);
        assert!(matches!(result,
           Err(error)  if   error.to_string().contains("Account owned by wrong program.")));
    }
    // Test 5 failing invalid discriminator
    {
        let mut account = merkle_tree_account.clone();
        account.data[0] = 1;
        let account_info = account.get_account_info();
        let result = BatchedMerkleTreeAccount::state_from_account_info(&account_info);
        assert!(matches!(result,
           Err(error)  if   error.to_string().contains("Invalid Discriminator.")));
    }
    // Test 6 functional output_from_account_info
    {
        let result =
            BatchedQueueAccount::output_from_account_info(&output_queue_account.get_account_info());
        assert!(result.is_ok());
    }
    // Test 7 failing invalid owner
    {
        let mut output_queue_account = output_queue_account.clone();
        output_queue_account.owner = Pubkey::new_unique();
        let account_info = output_queue_account.get_account_info();
        let result = BatchedQueueAccount::output_from_account_info(&account_info);
        assert!(matches!(result,
           Err(error)  if   error.to_string().contains("Account owned by wrong program.")));
    }
    // Test 8 failing invalid discriminator
    {
        let mut output_queue_account = output_queue_account.clone();
        output_queue_account.data[0] = 1;
        let account_info = output_queue_account.get_account_info();

        let result = BatchedQueueAccount::output_from_account_info(&account_info);
        assert!(matches!(result,
           Err(error)  if   error.to_string().contains("Invalid Discriminator.")));
    }
}

#[test]
fn test_get_state_root_by_index() {
    let key = Pubkey::new_unique();
    let owner = ACCOUNT_COMPRESSION_PROGRAM_ID;
    let mt_account_size = get_merkle_tree_account_size_default();
    let output_queue_size = get_output_queue_account_size_default();
    let mut merkle_tree_account = TestAccount::new(key, owner.into(), mt_account_size);
    let mut output_queue_account =
        TestAccount::new(Pubkey::new_unique(), owner.into(), output_queue_size);

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
            owner.into(),
            &merkle_tree_account_info,
            &output_queue_account_info,
            additional_rent,
        );
        assert!(result.is_ok());
    }

    // Test 2 functional get_state_root_by_index
    {
        let account_info = merkle_tree_account.get_account_info();
        let result = BatchedMerkleTreeAccount::get_state_root_by_index(&account_info, 0);
        assert_eq!(result.unwrap(), zero_bytes::poseidon::ZERO_BYTES[32]);
        for i in 1..params.root_history_capacity {
            let result =
                BatchedMerkleTreeAccount::get_state_root_by_index(&account_info, i as usize);
            assert_eq!(result.unwrap(), [0u8; 32]);
        }
    }
}

#[test]
fn test_get_address_root_by_index() {
    let key = Pubkey::new_unique();
    let owner = ACCOUNT_COMPRESSION_PROGRAM_ID;
    let params = InitAddressTreeAccountsInstructionData::test_default();

    let mt_account_size = get_address_merkle_tree_account_size_from_params(params);
    let mut merkle_tree_account = TestAccount::new(key, owner.into(), mt_account_size);

    let merkle_tree_rent = 1_000_000_000;
    merkle_tree_account.lamports = merkle_tree_rent;

    // Test 1 functional init_batched_address_merkle_tree_from_account_info
    {
        let merkle_tree_account_info = merkle_tree_account.get_account_info();

        let result = init_batched_address_merkle_tree_from_account_info(
            params,
            owner.into(),
            &merkle_tree_account_info,
        );
        assert!(result.is_ok());
    }

    // Test 2 functional get_address_root_by_index
    {
        let account_info = merkle_tree_account.get_account_info();
        let result = BatchedMerkleTreeAccount::get_address_root_by_index(&account_info, 0);
        assert_eq!(result.unwrap(), ADDRESS_TREE_INIT_ROOT_40);
        for i in 1..params.root_history_capacity {
            let result =
                BatchedMerkleTreeAccount::get_address_root_by_index(&account_info, i as usize);
            assert_eq!(result.unwrap(), [0u8; 32]);
        }
    }
}

#[test]
fn test_merkle_tree_getters() {
    let key = Pubkey::new_unique();
    let owner = ACCOUNT_COMPRESSION_PROGRAM_ID;
    let params = InitAddressTreeAccountsInstructionData::test_default();

    let mt_account_size = get_address_merkle_tree_account_size_from_params(params);
    let mut merkle_tree_account = TestAccount::new(key, owner.into(), mt_account_size);

    let merkle_tree_rent = 1_000_000_000;
    merkle_tree_account.lamports = merkle_tree_rent;

    // Test 1 functional init_batched_address_merkle_tree_from_account_info
    {
        let merkle_tree_account_info = merkle_tree_account.get_account_info();

        let result = init_batched_address_merkle_tree_from_account_info(
            params,
            owner.into(),
            &merkle_tree_account_info,
        );
        assert!(result.is_ok());
    }

    // Test 2 functional get_address_root_by_index
    {
        let account_info = merkle_tree_account.get_account_info();
        let tree = BatchedMerkleTreeAccount::address_from_account_info(&account_info).unwrap();
        assert_eq!(tree.get_root().unwrap(), ADDRESS_TREE_INIT_ROOT_40);
        assert_eq!(tree.get_root_index(), 0);
        assert_eq!(
            *tree.get_root_by_index(0).unwrap(),
            ADDRESS_TREE_INIT_ROOT_40
        );
        for i in 1..params.root_history_capacity {
            assert_eq!(*tree.get_root_by_index(i as usize).unwrap(), [0u8; 32]);
        }
    }
}
