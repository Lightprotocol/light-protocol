use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::{
    address::derive_address,
    compressed_account::{CompressedAccount, CompressedAccountData},
    instruction_data::{
        data::OutputCompressedAccountWithPackedContext,
        with_readonly::InstructionDataInvokeCpiWithReadOnly,
    },
    Pubkey,
};
use light_compressed_token::{
    constants::COMPRESSED_MINT_DISCRIMINATOR,
    mint::{
        output::create_output_compressed_mint_account,
        state::{CompressedMint, CompressedMintConfig},
    },
    shared::cpi_bytes_size::{
        allocate_invoke_with_read_only_cpi_bytes, cpi_bytes_config, CpiConfigInput,
    },
};
use light_zero_copy::ZeroCopyNew;
use rand::Rng;

#[test]
fn test_rnd_create_compressed_mint_account() {
    let mut rng = rand::thread_rng();
    let iter = 100;

    for _ in 0..iter {
        // Generate random mint parameters
        let mint_pda = Pubkey::new_from_array(rng.gen::<[u8; 32]>());
        let decimals = rng.gen_range(0..=18u8);
        let program_id = Pubkey::new_from_array(rng.gen::<[u8; 32]>());
        let address_merkle_tree = Pubkey::new_from_array(rng.gen::<[u8; 32]>());

        // Random freeze authority (50% chance)
        let freeze_authority = if rng.gen_bool(0.5) {
            Some(Pubkey::new_from_array(rng.gen::<[u8; 32]>()))
        } else {
            None
        };

        let mint_authority = Some(Pubkey::new_from_array(rng.gen::<[u8; 32]>()));

        // // Create mint config - match the real usage pattern (always reserve mint_authority space)
        let mint_config = CompressedMintConfig {
            mint_authority: (true, ()), // Always true like in cpi_bytes_config and mint_to_compressed
            freeze_authority: (freeze_authority.is_some(), ()),
        };
        // Derive compressed account address
        let compressed_account_address = derive_address(
            &mint_pda.to_bytes(),
            &address_merkle_tree.to_bytes(),
            &program_id.to_bytes(),
        );

        // Create a simple test structure for just the output account
        let config_input = CpiConfigInput {
            input_accounts: arrayvec::ArrayVec::new(),
            output_accounts: arrayvec::ArrayVec::new(),
            has_proof: false,
            compressed_mint: true,
            compressed_mint_with_freeze_authority: freeze_authority.is_some(),
        };

        let config = cpi_bytes_config(config_input);
        let mut cpi_bytes = allocate_invoke_with_read_only_cpi_bytes(&config);
        let (mut cpi_instruction_struct, _) =
            light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnly::new_zero_copy(
                &mut cpi_bytes[8..],
                config,
            )
            .unwrap();

        // Get the input and output compressed accounts
        let input_account = &mut cpi_instruction_struct.input_compressed_accounts[0];
        let output_account = &mut cpi_instruction_struct.output_compressed_accounts[0];

        // Create mock input data for the input compressed mint account test
        use light_compressed_account::compressed_account::PackedMerkleContext;
        use light_compressed_token::mint_to_compressed::instructions::CompressedMintInputs;
        use light_compressed_token::shared::context::TokenContext;
        use light_zero_copy::borsh::Deserialize;

        // Generate random values for more comprehensive testing
        let supply = rng.gen_range(0..=u64::MAX);
        let is_decompressed = rng.gen_bool(0.1); // 10% chance
        let num_extensions = rng.gen_range(0..=255u8);
        let merkle_tree_pubkey_index = rng.gen_range(0..=255u8);
        let queue_pubkey_index = rng.gen_range(0..=255u8);
        let leaf_index = rng.gen::<u32>();
        let prove_by_index = rng.gen_bool(0.5);
        let root_index = rng.gen::<u16>();
        let output_merkle_tree_index = rng.gen_range(0..=255u8);

        // Create mock input compressed mint data
        let input_compressed_mint = CompressedMintInputs {
            compressed_mint_input:
                light_compressed_token::mint_to_compressed::instructions::CompressedMintInput {
                    spl_mint: mint_pda,
                    supply,
                    decimals,
                    is_decompressed,
                    freeze_authority_is_set: freeze_authority.is_some(),
                    freeze_authority: freeze_authority.unwrap_or_default(),
                    num_extensions,
                },
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index,
                queue_pubkey_index,
                leaf_index,
                prove_by_index,
            },
            root_index,
            address: compressed_account_address,
            output_merkle_tree_index,
        };

        // Serialize and get zero-copy reference
        let input_data = input_compressed_mint.try_to_vec().unwrap();
        let (z_compressed_mint_inputs, _) =
            CompressedMintInputs::zero_copy_at(&input_data).unwrap();

        // Create token context and call input function
        let mut context = TokenContext::new();
        light_compressed_token::mint::input::create_input_compressed_mint_account(
            input_account,
            &mut context,
            &z_compressed_mint_inputs,
        )
        .unwrap();

        // Call the function under test
        create_output_compressed_mint_account(
            output_account,
            mint_pda,
            decimals,
            freeze_authority,
            mint_authority,
            &program_id,
            mint_config,
            compressed_account_address,
            output_merkle_tree_index,
        )
        .unwrap();

        // Final comparison with borsh deserialization - same pattern as token account tests
        let cpi_borsh =
            InstructionDataInvokeCpiWithReadOnly::deserialize(&mut &cpi_bytes[8..]).unwrap();

        // Build expected output
        let expected_compressed_mint = CompressedMint {
            spl_mint: mint_pda,
            supply: 0,
            decimals,
            is_decompressed: false,
            mint_authority,
            freeze_authority,
            num_extensions: 0,
        };

        let expected_data_hash = expected_compressed_mint.hash().unwrap();

        let expected_account = OutputCompressedAccountWithPackedContext {
            compressed_account: CompressedAccount {
                address: Some(compressed_account_address),
                owner: program_id,
                lamports: 0,
                data: Some(CompressedAccountData {
                    data: expected_compressed_mint.try_to_vec().unwrap(),
                    discriminator: COMPRESSED_MINT_DISCRIMINATOR,
                    data_hash: expected_data_hash,
                }),
            },
            merkle_tree_index: output_merkle_tree_index,
        };

        // Create expected input account data that matches what the input function should produce
        let expected_input_compressed_mint = CompressedMint {
            spl_mint: mint_pda,
            supply,
            decimals,
            is_decompressed,
            mint_authority: None, // Input validation typically doesn't set mint_authority
            freeze_authority,
            num_extensions,
        };
        let expected_input_data_hash = expected_input_compressed_mint.hash().unwrap();

        let expected_input_account =
            light_compressed_account::instruction_data::with_readonly::InAccount {
                discriminator: COMPRESSED_MINT_DISCRIMINATOR,
                data_hash: expected_input_data_hash,
                merkle_context: PackedMerkleContext {
                    merkle_tree_pubkey_index,
                    queue_pubkey_index,
                    leaf_index,
                    prove_by_index,
                },
                root_index,
                lamports: 0,
                address: Some(compressed_account_address),
            };

        let expected = InstructionDataInvokeCpiWithReadOnly {
            input_compressed_accounts: vec![expected_input_account],
            output_compressed_accounts: vec![expected_account],
            ..Default::default()
        };

        assert_eq!(cpi_borsh, expected);
    }
}
