use anchor_compressed_token::TokenData as AnchorTokenData;
use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use light_account_checks::account_info::test_account_info::pinocchio::get_account_info;
use light_compressed_account::instruction_data::with_readonly::{
    InAccount, InstructionDataInvokeCpiWithReadOnly,
};
use light_compressed_token::{
    constants::{
        TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR, TOKEN_COMPRESSED_ACCOUNT_V2_DISCRIMINATOR,
        TOKEN_COMPRESSED_ACCOUNT_V3_DISCRIMINATOR,
    },
    shared::{
        cpi_bytes_size::{
            allocate_invoke_with_read_only_cpi_bytes, cpi_bytes_config, CpiConfigInput,
        },
        token_input::{set_input_compressed_account, set_input_compressed_account_frozen},
    },
};
use light_ctoken_types::{
    hash_cache::HashCache, instructions::transfer2::MultiInputTokenDataWithContext,
    state::CompressedTokenAccountState,
};
use light_sdk::instruction::PackedMerkleContext;
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyNew};
use pinocchio::account_info::AccountInfo;
use rand::Rng;

#[test]
fn test_rnd_create_input_compressed_account() {
    let mut rng = rand::thread_rng();
    let iter = 1000;

    for _ in 0..iter {
        // Generate random parameters
        let mint_pubkey = Pubkey::new_from_array(rng.gen::<[u8; 32]>());
        let owner_pubkey = Pubkey::new_from_array(rng.gen::<[u8; 32]>());
        let delegate_pubkey = Pubkey::new_from_array(rng.gen::<[u8; 32]>());

        // Random amount from 0 to u64::MAX
        let amount = rng.gen::<u64>();
        let lamports = rng.gen_range(0..=1000000u64);

        // Random delegate flag (30% chance)
        let has_delegate = rng.gen_bool(0.3);

        // Random merkle hash_cache fields
        let merkle_tree_pubkey_index = rng.gen_range(0..=255u8);
        let queue_pubkey_index = rng.gen_range(0..=255u8);
        let leaf_index = rng.gen::<u32>();
        let prove_by_index = rng.gen_bool(0.5);
        let root_index = rng.gen::<u16>();
        let version = rng.gen_range(1..=3u8);

        // Create input token data
        let input_token_data = MultiInputTokenDataWithContext {
            amount,
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index,
                queue_pubkey_index,
                leaf_index,
                prove_by_index,
            },
            root_index,
            mint: 0,  // mint is at index 0 in remaining_accounts
            owner: 1, // owner is at index 1 in remaining_accounts
            has_delegate,
            delegate: if has_delegate { 2 } else { 0 }, // delegate at index 2 if present
            version,
        };

        // Serialize and get zero-copy reference
        let input_data = input_token_data.try_to_vec().unwrap();
        let (z_input_data, _) = MultiInputTokenDataWithContext::zero_copy_at(&input_data).unwrap();

        // Create mock remaining accounts
        let mut mock_accounts = vec![
            create_mock_account(mint_pubkey, false), // mint at index 0
            create_mock_account(owner_pubkey, !has_delegate), // owner at index 1, signer if no delegate
        ];

        if has_delegate {
            mock_accounts.push(create_mock_account(delegate_pubkey, true)); // delegate at index 2, signer
        }

        let remaining_accounts: Vec<AccountInfo> = mock_accounts;

        // Test both frozen and unfrozen states
        for is_frozen in [false, true] {
            // Allocate CPI bytes structure like in other tests
            let config_input = CpiConfigInput {
                input_accounts: {
                    let mut arr = tinyvec::ArrayVec::<[bool; 8]>::new();
                    arr.push(false); // Basic input account
                    arr
                },
                output_accounts: tinyvec::ArrayVec::<[(bool, u32); 35]>::new(),
                has_proof: false,
                new_address_params: 0,
            };

            let config = cpi_bytes_config(config_input);
            let mut cpi_bytes = allocate_invoke_with_read_only_cpi_bytes(&config).unwrap();
            let (mut cpi_instruction_struct, _) =
                InstructionDataInvokeCpiWithReadOnly::new_zero_copy(&mut cpi_bytes[8..], config)
                    .unwrap();

            // Get the input account reference
            let input_account = &mut cpi_instruction_struct.input_compressed_accounts[0];

            let mut hash_cache = HashCache::new();

            // Call the function under test
            let result = if is_frozen {
                set_input_compressed_account_frozen(
                    input_account,
                    &mut hash_cache,
                    &z_input_data,
                    remaining_accounts.as_slice(),
                    lamports,
                )
            } else {
                set_input_compressed_account(
                    input_account,
                    &mut hash_cache,
                    &z_input_data,
                    remaining_accounts.as_slice(),
                    lamports,
                )
            };

            assert!(result.is_ok(), "Function failed: {:?}", result.err());

            // Deserialize for validation using borsh pattern like other tests
            let cpi_borsh =
                InstructionDataInvokeCpiWithReadOnly::deserialize(&mut &cpi_bytes[8..]).unwrap();

            // Create expected token data for validation
            let expected_owner = owner_pubkey;
            let expected_delegate = if has_delegate {
                Some(delegate_pubkey)
            } else {
                None
            };

            let expected_token_data = AnchorTokenData {
                mint: mint_pubkey.into(),
                owner: expected_owner.into(),
                amount,
                delegate: expected_delegate.map(|d| d.into()),
                state: if is_frozen {
                    CompressedTokenAccountState::Frozen as u8
                } else {
                    CompressedTokenAccountState::Initialized as u8
                },
                tlv: None,
            };

            // Calculate expected data hash
            let (expected_hash, discriminator) = if version == 3 {
                (
                    expected_token_data.hash_sha_flat().unwrap(),
                    TOKEN_COMPRESSED_ACCOUNT_V3_DISCRIMINATOR,
                )
            } else if version == 2 {
                (
                    expected_token_data.hash_v2().unwrap(),
                    TOKEN_COMPRESSED_ACCOUNT_V2_DISCRIMINATOR,
                )
            } else {
                (
                    expected_token_data.hash_v1().unwrap(),
                    TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR,
                )
            };

            // Build expected input account
            let expected_input_account = InAccount {
                discriminator,
                data_hash: expected_hash,
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index,
                    queue_pubkey_index,
                    leaf_index,
                    prove_by_index,
                },
                root_index,
                lamports,
                address: None,
            };

            let expected = InstructionDataInvokeCpiWithReadOnly {
                input_compressed_accounts: vec![expected_input_account],
                ..Default::default()
            };

            assert_eq!(cpi_borsh, expected);
        }
    }
}

// Helper function to create mock AccountInfo
fn create_mock_account(pubkey: Pubkey, is_signer: bool) -> AccountInfo {
    get_account_info(
        pubkey.to_bytes(),
        Pubkey::default().to_bytes(), // owner is not checked,
        is_signer,
        false,
        false,
        vec![],
    )
}
