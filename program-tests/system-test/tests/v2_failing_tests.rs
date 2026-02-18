//! Test for CPI context address owner derivation.
//!
//! When creating new addresses via CPI context, the owner should always be the
//! invoking program ID. This test verifies that `store_data` correctly uses the
//! `invoking_program` parameter (passed through the call chain) as the owner for
//! new addresses, regardless of whether input accounts exist.
//!
//! The fix: `programs/system/src/cpi_context/state.rs` now receives `invoking_program`
//! as a parameter and uses it directly for the new address owner:
//! ```
//! pub fn store_data<...>(
//!     &mut self,
//!     instruction_data: &WrappedInstructionData<'b, T>,
//!     invoking_program: Pubkey,  // New parameter
//! ) -> Result<(), SystemProgramError> {
//!     let owner_bytes = invoking_program;  // Use invoking_program directly
//!     // ...
//! }
//! ```

use std::collections::HashMap;

use anchor_lang::{AnchorSerialize, Discriminator};
use create_address_test_program::create_invoke_cpi_instruction;
use light_account_checks::account_info::test_account_info::pinocchio::get_account_info;
use light_compressed_account::{
    compressed_account::{
        CompressedAccount, CompressedAccountData, PackedCompressedAccountWithMerkleContext,
        PackedMerkleContext,
    },
    instruction_data::{
        cpi_context::CompressedCpiContext,
        data::{NewAddressParamsPacked, OutputCompressedAccountWithPackedContext},
        invoke_cpi::InstructionDataInvokeCpi,
        with_readonly::{InAccount, InstructionDataInvokeCpiWithReadOnly},
        zero_copy::ZInstructionDataInvokeCpi,
    },
};
use light_program_test::{
    indexer::TestIndexerExtensions, utils::assert::assert_rpc_error, AddressWithTree, Indexer,
    LightProgramTest, ProgramTestConfig, Rpc,
};
use light_sdk::address::{v2::derive_address, NewAddressParamsAssigned};
use light_system_program_pinocchio::{
    context::WrappedInstructionData,
    cpi_context::{
        process_cpi_context::set_cpi_context,
        state::{
            cpi_context_account_new, deserialize_cpi_context_account, CpiContextAccountInitParams,
        },
    },
    ID,
};
use light_test_utils::{
    e2e_test_env::to_account_metas_light,
    pack::{
        pack_compressed_accounts, pack_new_address_params_assigned, pack_output_compressed_accounts,
    },
};
use light_zero_copy::traits::ZeroCopyAt;
use pinocchio::pubkey::Pubkey as PinocchioPubkey;
use solana_sdk::{pubkey::Pubkey, signature::Signer};

/// Creates a test CPI context account with the given associated merkle tree.
fn create_test_cpi_context_account(
    associated_merkle_tree: Option<PinocchioPubkey>,
) -> pinocchio::account_info::AccountInfo {
    let associated_merkle_tree =
        associated_merkle_tree.unwrap_or_else(|| Pubkey::new_unique().to_bytes());
    let params = CpiContextAccountInitParams::new(associated_merkle_tree);
    let account_info = get_account_info(
        Pubkey::new_unique().to_bytes(),
        ID,
        false,
        true,
        false,
        vec![0u8; 20000],
    );
    cpi_context_account_new::<false>(&account_info, params).unwrap();
    account_info
}

/// Creates instruction data with new addresses but NO input accounts.
fn create_instruction_data_with_new_address_no_inputs(
    output_account_owner: [u8; 32],
) -> InstructionDataInvokeCpi {
    let seed = [1u8; 32];

    InstructionDataInvokeCpi {
        proof: None,
        // New address with a seed - this is what we're testing
        new_address_params: vec![NewAddressParamsPacked {
            seed,
            address_queue_account_index: 0,
            address_merkle_tree_account_index: 0,
            address_merkle_tree_root_index: 0,
        }],
        // NO INPUT ACCOUNTS
        input_compressed_accounts_with_merkle_context: vec![],
        // One output account
        output_compressed_accounts: vec![OutputCompressedAccountWithPackedContext {
            compressed_account: CompressedAccount {
                owner: output_account_owner.into(),
                lamports: 0,
                address: None,
                data: None,
            },
            merkle_tree_index: 0,
        }],
        relay_fee: None,
        compress_or_decompress_lamports: None,
        is_compress: false,
        cpi_context: Some(CompressedCpiContext {
            first_set_context: true,
            set_context: true,
            cpi_context_account_index: 0,
        }),
    }
}

/// Test that creating new addresses via CPI context without input accounts
/// correctly uses the invoking_program as the owner.
///
/// The owner for new addresses should always be the invoking program, regardless
/// of whether input accounts exist. This test verifies the fix works correctly
/// when there are NO input accounts.
#[test]
fn test_cpi_context_new_address_uses_invoking_program_owner_without_inputs() {
    // The invoking program - this should be used for new addresses
    let invoking_program: PinocchioPubkey = Pubkey::new_unique().to_bytes();
    let output_account_owner: [u8; 32] = Pubkey::new_unique().to_bytes();
    let fee_payer: PinocchioPubkey = Pubkey::new_unique().to_bytes();

    // Create CPI context account
    let cpi_context_account = create_test_cpi_context_account(None);

    // Create instruction data with:
    // - 1 new address
    // - 0 input accounts (NO INPUTS)
    // - 1 output account with a different owner
    let instruction_data = create_instruction_data_with_new_address_no_inputs(output_account_owner);

    // Serialize and deserialize to zero-copy format
    let input_bytes = instruction_data.try_to_vec().unwrap();
    let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
    let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();

    // Store the instruction data in the CPI context with invoking_program
    let result = set_cpi_context(
        fee_payer,
        invoking_program,
        &cpi_context_account,
        w_instruction_data,
    );
    assert!(result.is_ok(), "set_cpi_context should succeed");

    // Deserialize the CPI context account to inspect stored data
    let cpi_context = deserialize_cpi_context_account(&cpi_context_account).unwrap();

    // Verify we have one new address stored
    assert_eq!(
        cpi_context.new_addresses.len(),
        1,
        "Should have exactly 1 new address stored"
    );

    // Get the stored new address
    let stored_address = cpi_context.new_addresses.get(0).unwrap();

    // Verify the seed was stored correctly
    assert_eq!(
        stored_address.seed, [1u8; 32],
        "Seed should match the input"
    );

    // The owner should be the invoking_program, NOT the output account's owner
    assert_eq!(
        stored_address.owner, invoking_program,
        "New address owner should be the invoking_program"
    );

    // Verify it's NOT zero
    assert_ne!(stored_address.owner, [0u8; 32], "Owner should NOT be zero");

    // Verify it's NOT the output account's owner (they're different in this test)
    assert_ne!(
        stored_address.owner, output_account_owner,
        "Owner should NOT be output account's owner"
    );
}

/// Test that creating new addresses via CPI context WITH input accounts
/// also correctly uses the invoking_program as the owner.
///
/// This demonstrates that invoking_program is used consistently, regardless
/// of whether input accounts exist.
#[test]
fn test_cpi_context_new_address_uses_invoking_program_owner_with_inputs() {
    // The invoking program - this should be used for new addresses
    let invoking_program: PinocchioPubkey = Pubkey::new_unique().to_bytes();
    // Input account owner is different from invoking_program
    let input_account_owner: [u8; 32] = Pubkey::new_unique().to_bytes();
    let fee_payer: PinocchioPubkey = Pubkey::new_unique().to_bytes();

    // Create CPI context account
    let cpi_context_account = create_test_cpi_context_account(None);

    // Create instruction data WITH input accounts
    let instruction_data = InstructionDataInvokeCpi {
        proof: None,
        new_address_params: vec![NewAddressParamsPacked {
            seed: [2u8; 32],
            address_queue_account_index: 0,
            address_merkle_tree_account_index: 0,
            address_merkle_tree_root_index: 0,
        }],
        // HAS INPUT ACCOUNTS
        input_compressed_accounts_with_merkle_context: vec![
            PackedCompressedAccountWithMerkleContext {
                compressed_account: CompressedAccount {
                    owner: input_account_owner.into(),
                    lamports: 100,
                    address: None,
                    data: None,
                },
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 0,
                    queue_pubkey_index: 0,
                    leaf_index: 0,
                    prove_by_index: false,
                },
                root_index: 0,
                read_only: false,
            },
        ],
        output_compressed_accounts: vec![OutputCompressedAccountWithPackedContext {
            compressed_account: CompressedAccount {
                owner: input_account_owner.into(),
                lamports: 100,
                address: None,
                data: None,
            },
            merkle_tree_index: 0,
        }],
        relay_fee: None,
        compress_or_decompress_lamports: None,
        is_compress: false,
        cpi_context: Some(CompressedCpiContext {
            first_set_context: true,
            set_context: true,
            cpi_context_account_index: 0,
        }),
    };

    // Serialize and deserialize to zero-copy format
    let input_bytes = instruction_data.try_to_vec().unwrap();
    let (z_inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(&input_bytes).unwrap();
    let w_instruction_data = WrappedInstructionData::new(z_inputs).unwrap();

    // Store the instruction data with invoking_program
    let result = set_cpi_context(
        fee_payer,
        invoking_program,
        &cpi_context_account,
        w_instruction_data,
    );
    assert!(result.is_ok(), "set_cpi_context should succeed");

    // Deserialize and check
    let cpi_context = deserialize_cpi_context_account(&cpi_context_account).unwrap();

    assert_eq!(
        cpi_context.new_addresses.len(),
        1,
        "Should have exactly 1 new address"
    );

    let stored_address = cpi_context.new_addresses.get(0).unwrap();

    // The owner should be the invoking_program, NOT the first input account's owner
    assert_eq!(
        stored_address.owner, invoking_program,
        "New address owner should be the invoking_program, not the input account's owner"
    );

    // Verify it's NOT zero
    assert_ne!(stored_address.owner, [0u8; 32], "Owner should NOT be zero");

    // Verify it's NOT the input account's owner (they're different in this test)
    assert_ne!(
        stored_address.owner, input_account_owner,
        "Owner should NOT be input account's owner"
    );
}

/// Test that duplicate addresses are rejected in V2 batched address trees.
///
/// This test verifies that the bloom filter catches duplicate address
/// insertions within the same transaction.
#[tokio::test]
async fn test_duplicate_address_in_v2_batched_tree() {
    let mut rpc = LightProgramTest::new({
        let mut config = ProgramTestConfig::default_with_batched_trees(true);
        config.additional_programs = Some(vec![(
            "create_address_test_program",
            create_address_test_program::ID,
        )]);
        config
    })
    .await
    .expect("Failed to setup test programs");

    let env = rpc.test_accounts.clone();
    let payer = rpc.get_payer().insecure_clone();

    let v2_address_tree = env.v2_address_trees[0];
    let address_seed = [42u8; 32];

    let (derived_address, address_seed) = derive_address(
        &[address_seed.as_slice()],
        &v2_address_tree,
        &create_address_test_program::ID,
    );

    // Request proof for the same address twice
    let addresses_with_tree = vec![
        AddressWithTree {
            address: derived_address,
            tree: v2_address_tree,
        },
        AddressWithTree {
            address: derived_address,
            tree: v2_address_tree,
        },
    ];

    let proof_result = rpc
        .get_validity_proof(Vec::new(), addresses_with_tree, None)
        .await
        .expect("Failed to get validity proof")
        .value;

    // Two new_address_params with the SAME seed
    let new_address_params = vec![
        NewAddressParamsAssigned {
            seed: address_seed.into(),
            address_queue_pubkey: v2_address_tree.into(),
            address_merkle_tree_pubkey: v2_address_tree.into(),
            address_merkle_tree_root_index: proof_result.get_address_root_indices()[0],
            assigned_account_index: None,
        },
        NewAddressParamsAssigned {
            seed: address_seed.into(),
            address_queue_pubkey: v2_address_tree.into(),
            address_merkle_tree_pubkey: v2_address_tree.into(),
            address_merkle_tree_root_index: proof_result.get_address_root_indices()[0],
            assigned_account_index: None,
        },
    ];

    let mut remaining_accounts = HashMap::<Pubkey, usize>::new();
    let packed_new_address_params =
        pack_new_address_params_assigned(new_address_params.as_slice(), &mut remaining_accounts);

    let ix_data = InstructionDataInvokeCpiWithReadOnly {
        mode: 0,
        bump: 255,
        with_cpi_context: false,
        invoking_program_id: create_address_test_program::ID.into(),
        proof: proof_result.proof.0,
        new_address_params: packed_new_address_params,
        is_compress: false,
        compress_or_decompress_lamports: 0,
        output_compressed_accounts: vec![],
        input_compressed_accounts: vec![],
        with_transaction_hash: true,
        ..Default::default()
    };

    let remaining_accounts_light: HashMap<light_compressed_account::Pubkey, usize> =
        remaining_accounts
            .into_iter()
            .map(|(k, v)| (k.into(), v))
            .collect();
    let remaining_accounts = to_account_metas_light(remaining_accounts_light);

    let instruction = create_invoke_cpi_instruction(
        payer.pubkey(),
        [
            light_system_program::instruction::InvokeCpiWithReadOnly::DISCRIMINATOR.to_vec(),
            ix_data.try_to_vec().unwrap(),
        ]
        .concat(),
        remaining_accounts,
        None,
    );

    let result = rpc
        .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await;

    // Expected: BloomFilterError::Full (14201)
    assert_rpc_error(result, 0, 14201).unwrap();
}

#[tokio::test]
async fn test_address_position_with_none_in_context_addresses() {
    let mut rpc = LightProgramTest::new({
        let mut config = ProgramTestConfig::default_with_batched_trees(true);
        config.additional_programs = Some(vec![(
            "create_address_test_program",
            create_address_test_program::ID,
        )]);
        config
    })
    .await
    .expect("Failed to setup test programs");

    let env = rpc.test_accounts.clone();
    let payer = rpc.get_payer().insecure_clone();
    let v2_state_tree = env.v2_state_trees[0];
    let v2_address_tree = env.v2_address_trees[0];
    let output_queue = v2_state_tree.output_queue;

    // -------------------------------------------------------------------
    // Transaction 1: Create a compressed account WITHOUT an address.
    // Owner = create_address_test_program so it can be consumed via CPI.
    // Data is set to zero discriminator/hash so InAccount can reproduce the
    // same account hash when consuming it.
    // -------------------------------------------------------------------
    {
        let mut remaining_accounts = HashMap::<Pubkey, usize>::new();
        let packed_outputs = pack_output_compressed_accounts(
            &[CompressedAccount {
                owner: create_address_test_program::ID.into(),
                lamports: 0,
                address: None,
                data: Some(CompressedAccountData {
                    discriminator: [0u8; 8],
                    data: vec![],
                    data_hash: [0u8; 32],
                }),
            }],
            &[output_queue],
            &mut remaining_accounts,
        );

        let ix_data = InstructionDataInvokeCpiWithReadOnly {
            mode: 0,
            bump: 255,
            with_cpi_context: false,
            invoking_program_id: create_address_test_program::ID.into(),
            output_compressed_accounts: packed_outputs,
            with_transaction_hash: true,
            ..Default::default()
        };

        let remaining_accounts_light: HashMap<light_compressed_account::Pubkey, usize> =
            remaining_accounts
                .into_iter()
                .map(|(k, v)| (k.into(), v))
                .collect();
        let remaining_accounts_vec = to_account_metas_light(remaining_accounts_light);

        let instruction = create_invoke_cpi_instruction(
            payer.pubkey(),
            [
                light_system_program::instruction::InvokeCpiWithReadOnly::DISCRIMINATOR.to_vec(),
                ix_data.try_to_vec().unwrap(),
            ]
            .concat(),
            remaining_accounts_vec,
            None,
        );

        rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
            .await
            .expect("Transaction 1 (create account without address) should succeed");
    }

    // -------------------------------------------------------------------
    // Transaction 2: Consume the addressless account as input, create ONE
    // -------------------------------------------------------------------
    {
        let accounts = rpc
            .get_compressed_accounts_with_merkle_context_by_owner(&create_address_test_program::ID);
        assert_eq!(
            accounts.len(),
            1,
            "Should have exactly 1 compressed account"
        );
        let input_account = accounts[0].clone();
        let input_hash = input_account.hash().unwrap();

        // Derive a single address.
        let (addr, seed) = derive_address(
            &[&[1u8; 32]],
            &v2_address_tree,
            &create_address_test_program::ID,
        );

        // Proof covers the input account inclusion and addr non-inclusion.
        let proof_result = rpc
            .get_validity_proof(
                vec![input_hash],
                vec![AddressWithTree {
                    address: addr,
                    tree: v2_address_tree,
                }],
                None,
            )
            .await
            .expect("Failed to get validity proof")
            .value;

        let mut remaining_accounts = HashMap::<Pubkey, usize>::new();

        // Pack the input account.
        let root_indices = proof_result.get_root_indices();
        let packed_inputs =
            pack_compressed_accounts(&[input_account], &root_indices, &mut remaining_accounts);
        let in_accounts: Vec<InAccount> = packed_inputs.into_iter().map(InAccount::from).collect();

        // Single new address param, assigned to output[0].
        // context.addresses will be [None, Some(addr)] when outputs are processed.
        let address_root_indices = proof_result.get_address_root_indices();
        let new_address_params = vec![NewAddressParamsAssigned {
            seed: seed.into(),
            address_queue_pubkey: v2_address_tree.into(),
            address_merkle_tree_pubkey: v2_address_tree.into(),
            address_merkle_tree_root_index: address_root_indices[0],
            assigned_account_index: Some(0),
        }];
        let packed_new_address_params =
            pack_new_address_params_assigned(&new_address_params, &mut remaining_accounts);

        // Both outputs claim the same address - the exploit.
        let zero_data = Some(CompressedAccountData {
            discriminator: [0u8; 8],
            data: vec![],
            data_hash: [0u8; 32],
        });
        let packed_outputs = pack_output_compressed_accounts(
            &[
                CompressedAccount {
                    owner: create_address_test_program::ID.into(),
                    lamports: 0,
                    address: Some(addr),
                    data: zero_data.clone(),
                },
                CompressedAccount {
                    owner: create_address_test_program::ID.into(),
                    lamports: 0,
                    address: Some(addr),
                    data: zero_data,
                },
            ],
            &[output_queue, output_queue],
            &mut remaining_accounts,
        );

        let ix_data = InstructionDataInvokeCpiWithReadOnly {
            mode: 0,
            bump: 255,
            with_cpi_context: false,
            invoking_program_id: create_address_test_program::ID.into(),
            proof: proof_result.proof.0,
            new_address_params: packed_new_address_params,
            input_compressed_accounts: in_accounts,
            output_compressed_accounts: packed_outputs,
            with_transaction_hash: true,
            ..Default::default()
        };

        let remaining_accounts_light: HashMap<light_compressed_account::Pubkey, usize> =
            remaining_accounts
                .into_iter()
                .map(|(k, v)| (k.into(), v))
                .collect();
        let remaining_accounts_vec = to_account_metas_light(remaining_accounts_light);

        let instruction = create_invoke_cpi_instruction(
            payer.pubkey(),
            [
                light_system_program::instruction::InvokeCpiWithReadOnly::DISCRIMINATOR.to_vec(),
                ix_data.try_to_vec().unwrap(),
            ]
            .concat(),
            remaining_accounts_vec,
            None,
        );

        // Correctly rejects duplicate address usage with InvalidAddress (6006).
        let result = rpc
            .create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
            .await;
        assert_rpc_error(result, 0, 6006).unwrap();
    }
}
