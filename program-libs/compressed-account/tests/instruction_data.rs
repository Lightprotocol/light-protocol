#![cfg(all(test, feature = "new-unique"))]

use light_compressed_account::{
    compressed_account::{
        CompressedAccount, CompressedAccountData, PackedCompressedAccountWithMerkleContext,
        PackedMerkleContext, PackedReadOnlyCompressedAccount,
    },
    instruction_data::{
        compressed_proof::CompressedProof,
        cpi_context::CompressedCpiContext,
        data::{
            InstructionDataInvoke, NewAddressParamsAssignedPacked, NewAddressParamsPacked,
            OutputCompressedAccountWithPackedContext, PackedReadOnlyAddress,
        },
        invoke_cpi::InstructionDataInvokeCpi,
        with_account_info::{
            CompressedAccountInfo, InAccountInfo, InstructionDataInvokeCpiWithAccountInfo,
            OutAccountInfo,
        },
        with_readonly::{InAccount, InstructionDataInvokeCpiWithReadOnly},
    },
    pubkey::Pubkey,
};

#[test]
fn test_instruction_data_invoke_builder() {
    let test_proof = Some(CompressedProof {
        a: [1; 32],
        b: [2; 64],
        c: [3; 32],
    });

    // Test that new() only sets the proof field
    let after_new = InstructionDataInvoke::new(test_proof);
    assert_eq!(after_new.proof, test_proof);
    assert!(after_new
        .input_compressed_accounts_with_merkle_context
        .is_empty());
    assert!(after_new.output_compressed_accounts.is_empty());
    assert!(after_new.new_address_params.is_empty());
    assert_eq!(after_new.relay_fee, None);
    assert_eq!(after_new.compress_or_decompress_lamports, None);
    assert!(!after_new.is_compress);

    // Create reference struct with test data
    let reference = InstructionDataInvoke {
        proof: test_proof,
        input_compressed_accounts_with_merkle_context: vec![
            PackedCompressedAccountWithMerkleContext {
                read_only: false,
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 1,
                    queue_pubkey_index: 2,
                    leaf_index: 100,
                    prove_by_index: false,
                },
                root_index: 0,
                compressed_account: CompressedAccount {
                    owner: Pubkey::new_from_array([4; 32]),
                    lamports: 1000,
                    address: Some([5; 32]),
                    data: Some(CompressedAccountData {
                        discriminator: [6; 8],
                        data: vec![7, 8, 9],
                        data_hash: [10; 32],
                    }),
                },
            },
        ],
        output_compressed_accounts: vec![OutputCompressedAccountWithPackedContext {
            compressed_account: CompressedAccount {
                owner: Pubkey::new_from_array([11; 32]),
                lamports: 2000,
                address: Some([12; 32]),
                data: None,
            },
            merkle_tree_index: 1,
        }],
        relay_fee: None, // This field has no builder method, so keep as None
        new_address_params: vec![NewAddressParamsPacked {
            seed: [13; 32],
            address_queue_account_index: 2,
            address_merkle_tree_account_index: 3,
            address_merkle_tree_root_index: 4,
        }],
        compress_or_decompress_lamports: Some(5000),
        is_compress: true,
    };

    // Build second struct using builder methods with data from reference
    let built = InstructionDataInvoke::new(reference.proof)
        .with_input_compressed_accounts_with_merkle_context(
            &reference.input_compressed_accounts_with_merkle_context,
        )
        .with_output_compressed_accounts(&reference.output_compressed_accounts)
        .with_new_addresses(&reference.new_address_params)
        .compress_lamports(5000);

    // Assert equality
    assert_eq!(built, reference);
}

#[test]
fn test_instruction_data_invoke_cpi_builder() {
    let test_proof = Some(CompressedProof {
        a: [14; 32],
        b: [15; 64],
        c: [16; 32],
    });

    // Test that new() only sets the proof field
    let after_new = InstructionDataInvokeCpi::new(test_proof);
    assert_eq!(after_new.proof, test_proof);
    assert!(after_new.new_address_params.is_empty());
    assert!(after_new
        .input_compressed_accounts_with_merkle_context
        .is_empty());
    assert!(after_new.output_compressed_accounts.is_empty());
    assert_eq!(after_new.relay_fee, None);
    assert_eq!(after_new.compress_or_decompress_lamports, None);
    assert!(!after_new.is_compress);
    assert_eq!(after_new.cpi_context, None);

    // Create reference struct with test data including CPI context
    let reference = InstructionDataInvokeCpi {
        proof: test_proof,
        new_address_params: vec![NewAddressParamsPacked {
            seed: [17; 32],
            address_queue_account_index: 5,
            address_merkle_tree_account_index: 6,
            address_merkle_tree_root_index: 7,
        }],
        input_compressed_accounts_with_merkle_context: vec![
            PackedCompressedAccountWithMerkleContext {
                read_only: false,
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 8,
                    queue_pubkey_index: 9,
                    leaf_index: 200,
                    prove_by_index: true,
                },
                root_index: 1,
                compressed_account: CompressedAccount {
                    owner: Pubkey::new_from_array([18; 32]),
                    lamports: 3000,
                    address: None,
                    data: Some(CompressedAccountData {
                        discriminator: [19; 8],
                        data: vec![20, 21],
                        data_hash: [22; 32],
                    }),
                },
            },
        ],
        output_compressed_accounts: vec![OutputCompressedAccountWithPackedContext {
            compressed_account: CompressedAccount {
                owner: Pubkey::new_from_array([23; 32]),
                lamports: 4000,
                address: Some([24; 32]),
                data: None,
            },
            merkle_tree_index: 2,
        }],
        relay_fee: None,
        compress_or_decompress_lamports: Some(3000),
        is_compress: false,
        cpi_context: Some(CompressedCpiContext::set()),
    };

    // Build using builder pattern
    let built = InstructionDataInvokeCpi::new(reference.proof)
        .with_new_addresses(&reference.new_address_params)
        .with_input_compressed_accounts_with_merkle_context(
            &reference.input_compressed_accounts_with_merkle_context,
        )
        .with_output_compressed_accounts(&reference.output_compressed_accounts)
        .decompress_lamports(3000)
        .write_to_cpi_context_set();

    // Assert equality
    assert_eq!(built, reference);
}

#[test]
fn test_instruction_data_invoke_cpi_with_readonly_builder() {
    let test_pubkey = Pubkey::new_from_array([25; 32]);
    let test_bump = 42;
    let test_proof = Some(CompressedProof {
        a: [26; 32],
        b: [27; 64],
        c: [28; 32],
    });

    // Test that new() only sets the specified fields
    let after_new = InstructionDataInvokeCpiWithReadOnly::new(test_pubkey, test_bump, test_proof);
    assert_eq!(after_new.invoking_program_id, test_pubkey);
    assert_eq!(after_new.bump, test_bump);
    assert_eq!(after_new.proof, test_proof);
    assert_eq!(after_new.mode, 1); // default
    assert_eq!(after_new.compress_or_decompress_lamports, 0);
    assert!(!after_new.is_compress);
    assert!(!after_new.with_cpi_context);
    assert!(!after_new.with_transaction_hash);
    assert_eq!(after_new.cpi_context, CompressedCpiContext::default());
    assert!(after_new.new_address_params.is_empty());
    assert!(after_new.input_compressed_accounts.is_empty());
    assert!(after_new.output_compressed_accounts.is_empty());
    assert!(after_new.read_only_addresses.is_empty());
    assert!(after_new.read_only_accounts.is_empty());

    // Create reference struct with all fields populated
    let reference = InstructionDataInvokeCpiWithReadOnly {
        mode: 0,
        bump: test_bump,
        invoking_program_id: test_pubkey,
        compress_or_decompress_lamports: 0, // No builder method for this
        is_compress: false,                 // No builder method for this
        with_cpi_context: true,
        with_transaction_hash: true,
        cpi_context: CompressedCpiContext::first(),
        proof: test_proof,
        new_address_params: vec![NewAddressParamsAssignedPacked {
            seed: [29; 32],
            address_queue_account_index: 10,
            address_merkle_tree_account_index: 11,
            address_merkle_tree_root_index: 12,
            assigned_to_account: true,
            assigned_account_index: 13,
        }],
        input_compressed_accounts: vec![InAccount {
            discriminator: [30; 8],
            data_hash: [31; 32],
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: 14,
                queue_pubkey_index: 15,
                leaf_index: 300,
                prove_by_index: false,
            },
            root_index: 2,
            lamports: 5000,
            address: Some([32; 32]),
        }],
        output_compressed_accounts: vec![OutputCompressedAccountWithPackedContext {
            compressed_account: CompressedAccount {
                owner: Pubkey::new_from_array([33; 32]),
                lamports: 6000,
                address: Some([34; 32]),
                data: None,
            },
            merkle_tree_index: 3,
        }],
        read_only_addresses: vec![PackedReadOnlyAddress {
            address: [35; 32],
            address_merkle_tree_account_index: 16,
            address_merkle_tree_root_index: 17,
        }],
        read_only_accounts: vec![PackedReadOnlyCompressedAccount {
            account_hash: [36; 32],
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: 18,
                queue_pubkey_index: 19,
                leaf_index: 400,
                prove_by_index: true,
            },
            root_index: 3,
        }],
    };

    // Build using builder pattern
    let built = InstructionDataInvokeCpiWithReadOnly::new(
        reference.invoking_program_id,
        reference.bump,
        reference.proof,
    )
    .mode_v1()
    .write_to_cpi_context_first()
    .with_with_transaction_hash(true)
    .with_new_addresses(&reference.new_address_params)
    .with_input_compressed_accounts(&reference.input_compressed_accounts)
    .with_output_compressed_accounts(&reference.output_compressed_accounts)
    .with_read_only_addresses(&reference.read_only_addresses)
    .with_read_only_accounts(&reference.read_only_accounts);

    // Assert equality
    assert_eq!(built, reference);
}

#[test]
fn test_instruction_data_invoke_cpi_with_account_info_builder() {
    let test_pubkey = Pubkey::new_from_array([37; 32]);
    let test_bump = 24;
    let test_proof = Some(CompressedProof {
        a: [38; 32],
        b: [39; 64],
        c: [40; 32],
    });

    // Test that new() only sets the specified fields
    let after_new =
        InstructionDataInvokeCpiWithAccountInfo::new(test_pubkey, test_bump, test_proof);
    assert_eq!(after_new.invoking_program_id, test_pubkey);
    assert_eq!(after_new.bump, test_bump);
    assert_eq!(after_new.proof, test_proof);
    assert_eq!(after_new.mode, 1); // default
    assert_eq!(after_new.compress_or_decompress_lamports, 0);
    assert!(!after_new.is_compress);
    assert!(!after_new.with_cpi_context);
    assert!(!after_new.with_transaction_hash);
    assert_eq!(after_new.cpi_context, CompressedCpiContext::default());
    assert!(after_new.new_address_params.is_empty());
    assert!(after_new.account_infos.is_empty());
    assert!(after_new.read_only_addresses.is_empty());
    assert!(after_new.read_only_accounts.is_empty());

    // Create reference struct
    let reference = InstructionDataInvokeCpiWithAccountInfo {
        mode: 0,
        bump: test_bump,
        invoking_program_id: test_pubkey,
        compress_or_decompress_lamports: 1500,
        is_compress: false,
        with_cpi_context: true,
        with_transaction_hash: false,
        cpi_context: CompressedCpiContext::set(),
        proof: test_proof,
        new_address_params: vec![NewAddressParamsAssignedPacked {
            seed: [41; 32],
            address_queue_account_index: 20,
            address_merkle_tree_account_index: 21,
            address_merkle_tree_root_index: 22,
            assigned_to_account: false,
            assigned_account_index: 0,
        }],
        account_infos: vec![CompressedAccountInfo {
            address: Some([42; 32]),
            input: Some(InAccountInfo {
                discriminator: [43; 8],
                data_hash: [44; 32],
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index: 23,
                    queue_pubkey_index: 24,
                    leaf_index: 500,
                    prove_by_index: false,
                },
                root_index: 4,
                lamports: 7000,
            }),
            output: Some(OutAccountInfo {
                discriminator: [45; 8],
                data_hash: [46; 32],
                output_merkle_tree_index: 5,
                lamports: 8000,
                data: vec![47, 48, 49],
            }),
        }],
        read_only_addresses: vec![PackedReadOnlyAddress {
            address: [50; 32],
            address_merkle_tree_account_index: 25,
            address_merkle_tree_root_index: 26,
        }],
        read_only_accounts: vec![PackedReadOnlyCompressedAccount {
            account_hash: [51; 32],
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: 27,
                queue_pubkey_index: 28,
                leaf_index: 600,
                prove_by_index: true,
            },
            root_index: 5,
        }],
    };

    // Build using builder pattern
    let built = InstructionDataInvokeCpiWithAccountInfo::new(
        reference.invoking_program_id,
        reference.bump,
        reference.proof,
    )
    .mode_v1()
    .write_to_cpi_context_set()
    .decompress_lamports(1500)
    .with_new_addresses(&reference.new_address_params)
    .with_account_infos(&reference.account_infos)
    .with_read_only_addresses(&reference.read_only_addresses)
    .with_read_only_accounts(&reference.read_only_accounts);

    // Assert equality
    assert_eq!(built, reference);
}
