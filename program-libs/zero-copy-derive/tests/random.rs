#![cfg(feature = "mut")]
use borsh::BorshSerialize;
use rand::{
    rngs::{StdRng, ThreadRng},
    Rng,
};
use light_zero_copy::{errors::ZeroCopyError, borsh::Deserialize};
use zerocopy::IntoBytes;

mod instruction_data;
use instruction_data::{
    CompressedAccount, CompressedAccountData, CompressedProof, InstructionDataInvoke,
    InstructionDataInvokeCpi, NewAddressParamsPacked, PackedCompressedAccountWithMerkleContext,
    PackedMerkleContext, CompressedCpiContext, OutputCompressedAccountWithPackedContext, Pubkey,
};

fn get_instruction_data_invoke_cpi() -> InstructionDataInvokeCpi {
    InstructionDataInvokeCpi {
        proof: Some(CompressedProof {
            a: [1; 32],
            b: [2; 64],
            c: [3; 32],
        }),
        new_address_params: vec![get_new_address_params(); 3],
        input_compressed_accounts_with_merkle_context: vec![get_test_input_account(); 3],
        output_compressed_accounts: vec![get_test_output_account(); 2],
        relay_fee: None,
        compress_or_decompress_lamports: Some(1),
        is_compress: true,
        cpi_context: Some(get_cpi_context()),
    }
}

fn get_rnd_instruction_data_invoke_cpi(rng: &mut StdRng) -> InstructionDataInvokeCpi {
    InstructionDataInvokeCpi {
        proof: Some(CompressedProof {
            a: rng.gen(),
            b: (0..64)
                .map(|_| rng.gen())
                .collect::<Vec<u8>>()
                .try_into()
                .unwrap(),
            c: rng.gen(),
        }),
        new_address_params: vec![get_rnd_new_address_params(rng); rng.gen_range(0..10)],
        input_compressed_accounts_with_merkle_context: vec![
            get_rnd_test_input_account(rng);
            rng.gen_range(0..10)
        ],
        output_compressed_accounts: vec![get_rnd_test_output_account(rng); rng.gen_range(0..10)],
        relay_fee: None,
        compress_or_decompress_lamports: rng.gen(),
        is_compress: rng.gen(),
        cpi_context: Some(get_rnd_cpi_context(rng)),
    }
}


fn get_cpi_context() -> CompressedCpiContext {
    CompressedCpiContext {
        first_set_context: true,
        set_context: true,
        cpi_context_account_index: 1,
    }
}

fn get_rnd_cpi_context(rng: &mut StdRng) -> CompressedCpiContext {
    CompressedCpiContext {
        first_set_context: rng.gen(),
        set_context: rng.gen(),
        cpi_context_account_index: rng.gen(),
    }
}

fn get_test_account_data() -> CompressedAccountData {
    CompressedAccountData {
        discriminator: 1u64.to_le_bytes(),
        data: vec![1, 2, 3, 4, 5, 6, 7, 8],
        data_hash: [1; 32],
    }
}

fn get_rnd_test_account_data(rng: &mut StdRng) -> CompressedAccountData {
    CompressedAccountData {
        discriminator: rng.gen(),
        data: (0..100).map(|_| rng.gen()).collect::<Vec<u8>>(),
        data_hash: rng.gen(),
    }
}

fn get_test_account() -> CompressedAccount {
    CompressedAccount {
        owner: Pubkey::new_unique().to_bytes(),
        lamports: 100,
        address: Some(Pubkey::new_unique().to_bytes()),
        data: Some(get_test_account_data()),
    }
}

fn get_rnd_test_account(rng: &mut StdRng) -> CompressedAccount {
    CompressedAccount {
        owner: Pubkey::new_unique().to_bytes(),
        lamports: rng.gen(),
        address: Some(Pubkey::new_unique().to_bytes()),
        data: Some(get_rnd_test_account_data(rng)),
    }
}

fn get_test_output_account() -> OutputCompressedAccountWithPackedContext {
    OutputCompressedAccountWithPackedContext {
        compressed_account: get_test_account(),
        merkle_tree_index: 1,
    }
}

fn get_rnd_test_output_account(rng: &mut StdRng) -> OutputCompressedAccountWithPackedContext {
    OutputCompressedAccountWithPackedContext {
        compressed_account: get_rnd_test_account(rng),
        merkle_tree_index: rng.gen(),
    }
}


fn get_test_input_account() -> PackedCompressedAccountWithMerkleContext {
    PackedCompressedAccountWithMerkleContext {
        compressed_account: CompressedAccount {
            owner: Pubkey::new_unique().to_bytes(),
            lamports: 100,
            address: Some(Pubkey::new_unique().to_bytes()),
            data: Some(CompressedAccountData {
                discriminator: 1u64.to_le_bytes(),
                data: vec![1, 2, 3, 4, 5, 6, 7, 8],
                data_hash: [1; 32],
            }),
        },
        merkle_context: PackedMerkleContext {
            merkle_tree_pubkey_index: 1,
            nullifier_queue_pubkey_index: 2,
            leaf_index: 3,
            prove_by_index: true,
        },
        root_index: 5,
        read_only: false,
    }
}

fn get_rnd_test_input_account(rng: &mut StdRng) -> PackedCompressedAccountWithMerkleContext {
    PackedCompressedAccountWithMerkleContext {
        compressed_account: CompressedAccount {
            owner: Pubkey::new_unique().to_bytes(),
            lamports: 100,
            address: Some(Pubkey::new_unique().to_bytes()),
            data: Some(get_rnd_test_account_data(rng)),
        },
        merkle_context: PackedMerkleContext {
            merkle_tree_pubkey_index: rng.gen(),
            nullifier_queue_pubkey_index: rng.gen(),
            leaf_index: rng.gen(),
            prove_by_index: rng.gen(),
        },
        root_index: rng.gen(),
        read_only: false,
    }
}
fn get_new_address_params() -> NewAddressParamsPacked {
    NewAddressParamsPacked {
        seed: [1; 32],
        address_queue_account_index: 1,
        address_merkle_tree_account_index: 2,
        address_merkle_tree_root_index: 3,
    }
}

// get_instruction_data_invoke_cpi
fn get_rnd_new_address_params(rng: &mut StdRng) -> NewAddressParamsPacked {
    NewAddressParamsPacked {
        seed: rng.gen(),
        address_queue_account_index: rng.gen(),
        address_merkle_tree_account_index: rng.gen(),
        address_merkle_tree_root_index: rng.gen(),
    }
}

#[test]
fn test_invoke_ix_data_deserialize_rnd() {
    use rand::{rngs::StdRng, Rng, SeedableRng};
    let mut thread_rng = ThreadRng::default();
    let seed = thread_rng.gen();
    // Keep this print so that in case the test fails
    // we can use the seed to reproduce the error.
    println!("\n\ne2e test seed for invoke_ix_data {}\n\n", seed);
    let mut rng = StdRng::seed_from_u64(seed);

    let num_iters = 1000;
    for i in 0..num_iters {
        // Create randomized instruction data
        let invoke_ref = InstructionDataInvoke {
            proof: if rng.gen() {
                Some(CompressedProof {
                    a: rng.gen(),
                    b: (0..64)
                        .map(|_| rng.gen())
                        .collect::<Vec<u8>>()
                        .try_into()
                        .unwrap(),
                    c: rng.gen(),
                })
            } else {
                None
            },
            input_compressed_accounts_with_merkle_context: if i % 5 == 0 {
                // Only add inputs occasionally to keep test manageable
                vec![get_rnd_test_input_account(&mut rng); rng.gen_range(1..3)]
            } else {
                vec![]
            },
            output_compressed_accounts: if i % 4 == 0 {
                vec![get_rnd_test_output_account(&mut rng); rng.gen_range(1..3)]
            } else {
                vec![]
            },
            relay_fee: None, // Relay fee is currently not supported
            new_address_params: if i % 3 == 0 {
                vec![get_rnd_new_address_params(&mut rng); rng.gen_range(1..3)]
            } else {
                vec![]
            },
            compress_or_decompress_lamports: if rng.gen() { Some(rng.gen()) } else { None },
            is_compress: rng.gen(),
        };

        let mut bytes = Vec::new();
        invoke_ref.serialize(&mut bytes).unwrap();

        let (z_copy, bytes) = InstructionDataInvoke::zero_copy_at(&bytes).unwrap();
        assert!(bytes.is_empty());

        // Test successful deserialization - the main goal is that it doesn't crash
        // and we can access some basic fields
        assert_eq!(z_copy.is_compress, if invoke_ref.is_compress { 1 } else { 0 });
    }
}


#[test]
fn test_instruction_data_invoke_cpi_rnd() {
    use rand::{rngs::StdRng, Rng, SeedableRng};
    let mut thread_rng = ThreadRng::default();
    let seed = thread_rng.gen();
    // Keep this print so that in case the test fails
    // we can use the seed to reproduce the error.
    println!("\n\ne2e test seed {}\n\n", seed);
    let mut rng = StdRng::seed_from_u64(seed);

    let num_iters = 10000;
    for _ in 0..num_iters {
        let value = get_rnd_instruction_data_invoke_cpi(&mut rng);
        let mut vec = Vec::new();
        value.serialize(&mut vec).unwrap();
        let (zero_copy, _) = InstructionDataInvokeCpi::zero_copy_at(&vec).unwrap();
        // Test successful deserialization and basic field access
        assert_eq!(zero_copy.is_compress, if value.is_compress { 1 } else { 0 });
    }
}
