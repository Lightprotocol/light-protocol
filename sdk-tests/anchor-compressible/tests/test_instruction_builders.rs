mod test_instruction_builders {

    use light_client::indexer::{CompressedAccount, TreeInfo, ValidityProofWithContext};
    use light_compressed_account::TreeType;
    use light_compressible_client::{CompressibleConfig, CompressibleInstruction};
    use light_sdk::instruction::ValidityProof;
    use solana_sdk::{pubkey::Pubkey, system_program};

    /// Test that our instruction builders follow Solana SDK patterns correctly
    /// They should return Instruction directly, not Result<Instruction, _>
    #[test]
    fn test_initialize_compression_config_instruction_builder() {
        let program_id = Pubkey::new_unique();
        let payer = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let compression_delay = 100u32;
        let rent_recipient = Pubkey::new_unique();
        let address_space = vec![Pubkey::new_unique()];

        // Following Solana SDK patterns like system_instruction::transfer()
        // Should return Instruction directly, not Result
        let instruction = CompressibleInstruction::initialize_compression_config(
            &program_id,
            &[5u8],
            &payer,
            &authority,
            compression_delay,
            rent_recipient,
            address_space,
            Some(0),
        );

        // Verify instruction structure
        assert_eq!(instruction.program_id, program_id);
        assert_eq!(instruction.accounts.len(), 5); // payer, config, program_data, authority, system_program

        // Verify account order and permissions
        assert_eq!(instruction.accounts[0].pubkey, payer);
        assert!(instruction.accounts[0].is_signer); // payer signs
        assert!(instruction.accounts[0].is_writable); // payer pays

        let (expected_config_pda, _) = CompressibleConfig::derive_pda(&program_id, 0);
        assert_eq!(instruction.accounts[1].pubkey, expected_config_pda);
        assert!(!instruction.accounts[1].is_signer); // config doesn't sign
        assert!(instruction.accounts[1].is_writable); // config is created/written

        assert_eq!(instruction.accounts[3].pubkey, authority);
        assert!(instruction.accounts[3].is_signer); // authority must sign
        assert!(!instruction.accounts[3].is_writable); // authority is read-only

        assert_eq!(instruction.accounts[4].pubkey, system_program::ID);
        assert!(!instruction.accounts[4].is_signer); // system program doesn't sign
        assert!(!instruction.accounts[4].is_writable); // system program is read-only

        // Verify instruction data is present
        assert!(!instruction.data.is_empty());

        println!("✅ Instruction builder follows Solana SDK patterns correctly!");
    }

    #[test]
    fn test_update_config_instruction_builder() {
        let program_id = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let new_compression_delay = Some(200u32);
        let new_rent_recipient = Some(Pubkey::new_unique());

        // Should return Instruction directly, following Solana SDK patterns
        let instruction = CompressibleInstruction::update_compression_config(
            &program_id,
            &[6u8],
            &authority,
            new_compression_delay,
            new_rent_recipient,
            None,
            None,
        );

        // Verify instruction structure
        assert_eq!(instruction.program_id, program_id);
        assert_eq!(instruction.accounts.len(), 2); // config, authority

        let (expected_config_pda, _) = CompressibleConfig::derive_pda(&program_id, 0);
        assert_eq!(instruction.accounts[0].pubkey, expected_config_pda);
        assert!(!instruction.accounts[0].is_signer); // config doesn't sign
        assert!(instruction.accounts[0].is_writable); // config is updated

        assert_eq!(instruction.accounts[1].pubkey, authority);
        assert!(instruction.accounts[1].is_signer); // authority must sign
        assert!(!instruction.accounts[1].is_writable); // authority is read-only

        // Verify instruction data is present
        assert!(!instruction.data.is_empty());

        println!("✅ Update instruction builder follows Solana SDK patterns correctly!");
    }

    #[test]
    fn test_decompress_accounts_idempotent_instruction_builder() {
        use light_client::indexer::{AccountProofInputs, RootIndex};

        let program_id = Pubkey::new_unique();
        let fee_payer = Pubkey::new_unique();
        let rent_payer = Pubkey::new_unique();
        let pda1 = Pubkey::new_unique();
        let pda2 = Pubkey::new_unique();
        let solana_accounts = vec![pda1, pda2];
        let config_pda = CompressibleConfig::derive_pda(&program_id, 0).0;

        // Create mock compressed accounts with tree info
        let tree_info = TreeInfo {
            queue: Pubkey::new_unique(),
            tree: Pubkey::new_unique(),
            tree_type: TreeType::StateV1,
            cpi_context: None,
            next_tree_info: None,
        };

        let compressed_account1 = CompressedAccount {
            address: Some([1u8; 32]),
            data: None,
            hash: [1u8; 32],
            lamports: 1000,
            leaf_index: 0,
            owner: program_id,
            prove_by_index: false,
            seq: Some(1),
            slot_created: 100,
            tree_info,
        };

        let compressed_account2 = CompressedAccount {
            address: Some([2u8; 32]),
            data: None,
            hash: [2u8; 32],
            lamports: 2000,
            leaf_index: 1,
            owner: program_id,
            prove_by_index: false,
            seq: Some(2),
            slot_created: 101,
            tree_info,
        };

        // Create account variant data (mock data for testing)
        let account_variant1 = vec![1u8, 2, 3, 4]; // Mock compressed account variant
        let account_variant2 = vec![5u8, 6, 7, 8]; // Mock compressed account variant

        let compressed_accounts = vec![
            (
                compressed_account1.clone(),
                account_variant1,
                vec![b"user_record".to_vec(), fee_payer.to_bytes().to_vec()],
            ),
            (
                compressed_account2.clone(),
                account_variant2,
                vec![b"game_session".to_vec(), 12345u64.to_le_bytes().to_vec()],
            ),
        ];

        let bumps = vec![250u8, 251u8]; // typical PDA bumps

        // Create proper AccountProofInputs for the ValidityProofWithContext
        let account_proof_inputs = vec![
            AccountProofInputs {
                hash: compressed_account1.hash,
                root: [0u8; 32], // Mock root
                root_index: RootIndex::new_some(0),
                leaf_index: compressed_account1.leaf_index as u64,
                tree_info: compressed_account1.tree_info,
            },
            AccountProofInputs {
                hash: compressed_account2.hash,
                root: [0u8; 32], // Mock root
                root_index: RootIndex::new_some(0),
                leaf_index: compressed_account2.leaf_index as u64,
                tree_info: compressed_account2.tree_info,
            },
        ];

        // Create mock validity proof with context
        let validity_proof_with_context = ValidityProofWithContext {
            proof: ValidityProof::default(),
            accounts: account_proof_inputs, // Provide proper account proof inputs
            addresses: vec![],              // Mock address proof inputs
        };

        let output_state_tree_info = tree_info;

        // Should return Result<Instruction, _> for the new API
        let result = CompressibleInstruction::decompress_accounts_idempotent(
            &program_id,
            &[7u8],
            &fee_payer,
            &rent_payer,
            &solana_accounts,
            &compressed_accounts,
            &bumps,
            validity_proof_with_context,
            output_state_tree_info,
        );

        // Verify instruction was created successfully
        assert!(result.is_ok(), "Instruction creation should succeed");
        let instruction = result.unwrap();

        // Verify instruction structure
        assert_eq!(instruction.program_id, program_id);

        // Expected accounts: fee_payer, rent_payer, system_program, plus system accounts
        assert!(instruction.accounts.len() >= 3); // At least the basic accounts

        // Verify account order and permissions
        assert_eq!(instruction.accounts[0].pubkey, fee_payer);
        assert!(instruction.accounts[0].is_signer); // fee_payer signs
        assert!(instruction.accounts[0].is_writable); // fee_payer pays

        assert_eq!(instruction.accounts[1].pubkey, rent_payer);
        assert!(instruction.accounts[1].is_signer); // rent_payer signs
        assert!(instruction.accounts[1].is_writable); // rent_payer pays rent

        assert_eq!(instruction.accounts[2].pubkey, config_pda);
        assert!(!instruction.accounts[2].is_signer); // system program doesn't sign
        assert!(!instruction.accounts[2].is_writable); // system program is read-only

        // Verify instruction data is present and starts with discriminator
        assert!(!instruction.data.is_empty());
        assert_eq!(&instruction.data[0..8], &[7, 0, 2, 0, 0, 0, 0, 0]);

        println!("✅ Decompress multiple accounts idempotent instruction builder follows Solana SDK patterns correctly!");
    }

    #[test]
    fn test_decompress_accounts_idempotent_validation_accounts_mismatch() {
        let program_id = Pubkey::new_unique();
        let fee_payer = Pubkey::new_unique();
        let rent_payer = Pubkey::new_unique();
        let solana_accounts = vec![Pubkey::new_unique()]; // 1 PDA

        // Create tree info
        let tree_info = TreeInfo {
            queue: Pubkey::new_unique(),
            tree: Pubkey::new_unique(),
            tree_type: TreeType::StateV1,
            cpi_context: None,
            next_tree_info: None,
        };

        // But 2 compressed accounts - should return error
        let compressed_account1 = CompressedAccount {
            address: Some([1u8; 32]),
            data: None,
            hash: [1u8; 32],
            lamports: 1000,
            leaf_index: 0,
            owner: program_id,
            prove_by_index: false,
            seq: Some(1),
            slot_created: 100,
            tree_info,
        };

        let compressed_account2 = CompressedAccount {
            address: Some([2u8; 32]),
            data: None,
            hash: [2u8; 32],
            lamports: 2000,
            leaf_index: 1,
            owner: program_id,
            prove_by_index: false,
            seq: Some(2),
            slot_created: 101,
            tree_info,
        };

        let compressed_accounts = vec![
            (
                compressed_account1,
                vec![1u8, 2, 3, 4],
                vec![b"user_record".to_vec(), fee_payer.to_bytes().to_vec()],
            ),
            (
                compressed_account2,
                vec![5u8, 6, 7, 8],
                vec![b"game_session".to_vec(), 12345u64.to_le_bytes().to_vec()],
            ),
        ];

        let bumps = vec![250u8];

        let validity_proof_with_context = ValidityProofWithContext {
            proof: ValidityProof::default(),
            accounts: vec![],
            addresses: vec![],
        };

        let result = CompressibleInstruction::decompress_accounts_idempotent(
            &program_id,
            &[7u8],
            &fee_payer,
            &rent_payer,
            &solana_accounts,
            &compressed_accounts,
            &bumps,
            validity_proof_with_context,
            tree_info,
        );

        assert!(
            result.is_err(),
            "Should return error for mismatched accounts"
        );
        assert!(result.unwrap_err().to_string().contains("same length"));
    }

    #[test]
    fn test_decompress_accounts_idempotent_validation_bumps_mismatch() {
        let program_id = Pubkey::new_unique();
        let fee_payer = Pubkey::new_unique();
        let rent_payer = Pubkey::new_unique();
        let solana_accounts = vec![Pubkey::new_unique()]; // 1 PDA

        let tree_info = TreeInfo {
            queue: Pubkey::new_unique(),
            tree: Pubkey::new_unique(),
            tree_type: TreeType::StateV1,
            cpi_context: None,
            next_tree_info: None,
        };

        let compressed_account = CompressedAccount {
            address: Some([1u8; 32]),
            data: None,
            hash: [1u8; 32],
            lamports: 1000,
            leaf_index: 0,
            owner: program_id,
            prove_by_index: false,
            seq: Some(1),
            slot_created: 100,
            tree_info,
        };

        let compressed_accounts = vec![(
            compressed_account,
            vec![1u8, 2, 3, 4],
            vec![b"user_record".to_vec(), fee_payer.to_bytes().to_vec()],
        )];

        let bumps = vec![250u8, 251u8]; // 2 bumps but 1 PDA - should return error

        let validity_proof_with_context = ValidityProofWithContext {
            proof: ValidityProof::default(),
            accounts: vec![],
            addresses: vec![],
        };

        let result = CompressibleInstruction::decompress_accounts_idempotent(
            &program_id,
            &[7u8],
            &fee_payer,
            &rent_payer,
            &solana_accounts,
            &compressed_accounts,
            &bumps,
            validity_proof_with_context,
            tree_info,
        );

        assert!(result.is_err(), "Should return error for mismatched bumps");
        assert!(result.unwrap_err().to_string().contains("same length"));
    }
}
