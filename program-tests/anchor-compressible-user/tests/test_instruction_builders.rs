#[cfg(test)]
mod test_instruction_builders {

    // use light_client::compressible::{CompressibleConfig, CompressibleInstruction};
    // use solana_sdk::{pubkey::Pubkey, system_program};

    // /// Test that our instruction builders follow Solana SDK patterns correctly
    // /// They should return Instruction directly, not Result<Instruction, _>
    // #[test]
    // fn test_initialize_compression_config_instruction_builder() {
    //     let program_id = Pubkey::new_unique();
    //     let payer = Pubkey::new_unique();
    //     let authority = Pubkey::new_unique();
    //     let compression_delay = 100u32;
    //     let rent_recipient = Pubkey::new_unique();
    //     let address_space = vec![Pubkey::new_unique()];

    //     // Following Solana SDK patterns like system_instruction::transfer()
    //     // Should return Instruction directly, not Result
    //     let instruction = CompressibleInstruction::initialize_compression_config(
    //         &program_id,
    //         &payer,
    //         &authority,
    //         compression_delay,
    //         rent_recipient,
    //         address_space,
    //     );

    //     // Verify instruction structure
    //     assert_eq!(instruction.program_id, program_id);
    //     assert_eq!(instruction.accounts.len(), 5); // payer, config, program_data, authority, system_program

    //     // Verify account order and permissions
    //     assert_eq!(instruction.accounts[0].pubkey, payer);
    //     assert!(instruction.accounts[0].is_signer); // payer signs
    //     assert!(instruction.accounts[0].is_writable); // payer pays

    //     let (expected_config_pda, _) = CompressibleConfig::derive_pda(&program_id);
    //     assert_eq!(instruction.accounts[1].pubkey, expected_config_pda);
    //     assert!(!instruction.accounts[1].is_signer); // config doesn't sign
    //     assert!(instruction.accounts[1].is_writable); // config is created/written

    //     assert_eq!(instruction.accounts[3].pubkey, authority);
    //     assert!(instruction.accounts[3].is_signer); // authority must sign
    //     assert!(!instruction.accounts[3].is_writable); // authority is read-only

    //     assert_eq!(instruction.accounts[4].pubkey, system_program::ID);
    //     assert!(!instruction.accounts[4].is_signer); // system program doesn't sign
    //     assert!(!instruction.accounts[4].is_writable); // system program is read-only

    //     // Verify instruction data is present
    //     assert!(!instruction.data.is_empty());

    //     println!("✅ Instruction builder follows Solana SDK patterns correctly!");
    // }

    // #[test]
    // fn test_update_config_instruction_builder() {
    //     let program_id = Pubkey::new_unique();
    //     let authority = Pubkey::new_unique();
    //     let new_compression_delay = Some(200u32);
    //     let new_rent_recipient = Some(Pubkey::new_unique());

    //     // Should return Instruction directly, following Solana SDK patterns
    //     let instruction = CompressibleInstruction::update_compression_config(
    //         &program_id,
    //         &authority,
    //         new_compression_delay,
    //         new_rent_recipient,
    //         None,
    //         None,
    //     );

    //     // Verify instruction structure
    //     assert_eq!(instruction.program_id, program_id);
    //     assert_eq!(instruction.accounts.len(), 2); // config, authority

    //     let (expected_config_pda, _) = CompressibleConfig::derive_pda(&program_id);
    //     assert_eq!(instruction.accounts[0].pubkey, expected_config_pda);
    //     assert!(!instruction.accounts[0].is_signer); // config doesn't sign
    //     assert!(instruction.accounts[0].is_writable); // config is updated

    //     assert_eq!(instruction.accounts[1].pubkey, authority);
    //     assert!(instruction.accounts[1].is_signer); // authority must sign
    //     assert!(!instruction.accounts[1].is_writable); // authority is read-only

    //     // Verify instruction data is present
    //     assert!(!instruction.data.is_empty());

    //     println!("✅ Update instruction builder follows Solana SDK patterns correctly!");
    // }

    // #[test]
    // fn test_decompress_multiple_pdas_instruction_builder() {
    //     use light_client::compressible::CompressedAccountData;
    //     use light_sdk::instruction::{account_meta::CompressedAccountMeta, ValidityProof};
    //     use solana_sdk::{instruction::AccountMeta, system_program};

    //     let program_id = Pubkey::new_unique();
    //     let fee_payer = Pubkey::new_unique();
    //     let rent_payer = Pubkey::new_unique();
    //     let pda1 = Pubkey::new_unique();
    //     let pda2 = Pubkey::new_unique();
    //     let pda_accounts = vec![pda1, pda2];

    //     // Create mock compressed account data
    //     let compressed_accounts = vec![
    //         CompressedAccountData {
    //             meta: CompressedAccountMeta {
    //                 tree_info: Default::default(),
    //                 address: [1u8; 32],
    //                 output_state_tree_index: 0,
    //             },
    //             data: vec![1, 2, 3, 4], // mock data
    //         },
    //         CompressedAccountData {
    //             meta: CompressedAccountMeta {
    //                 tree_info: Default::default(),
    //                 address: [2u8; 32],
    //                 output_state_tree_index: 1,
    //             },
    //             data: vec![5, 6, 7, 8], // mock data
    //         },
    //     ];

    //     let bumps = vec![250u8, 251u8]; // typical PDA bumps
    //     let system_accounts = vec![
    //         AccountMeta::new_readonly(Pubkey::new_unique(), false), // mock system account
    //     ];
    //     let discriminator = [1, 2, 3, 4, 5, 6, 7, 8]; // mock discriminator

    //     // Should return Instruction directly, following Solana SDK patterns
    //     let instruction = CompressibleInstruction::decompress_multiple_pdas(
    //         &program_id,
    //         &fee_payer,
    //         &rent_payer,
    //         &pda_accounts,
    //         ValidityProof::default(),
    //         compressed_accounts,
    //         bumps,
    //         system_accounts.clone(),
    //         &discriminator,
    //     );

    //     // Verify instruction structure
    //     assert_eq!(instruction.program_id, program_id);

    //     // Expected accounts: fee_payer, rent_payer, system_program, pda1, pda2, system_accounts
    //     let expected_account_count = 3 + pda_accounts.len() + system_accounts.len();
    //     assert_eq!(instruction.accounts.len(), expected_account_count);

    //     // Verify account order and permissions
    //     assert_eq!(instruction.accounts[0].pubkey, fee_payer);
    //     assert!(instruction.accounts[0].is_signer); // fee_payer signs
    //     assert!(instruction.accounts[0].is_writable); // fee_payer pays

    //     assert_eq!(instruction.accounts[1].pubkey, rent_payer);
    //     assert!(instruction.accounts[1].is_signer); // rent_payer signs
    //     assert!(instruction.accounts[1].is_writable); // rent_payer pays rent

    //     assert_eq!(instruction.accounts[2].pubkey, system_program::ID);
    //     assert!(!instruction.accounts[2].is_signer); // system program doesn't sign
    //     assert!(!instruction.accounts[2].is_writable); // system program is read-only

    //     // Verify PDA accounts
    //     assert_eq!(instruction.accounts[3].pubkey, pda1);
    //     assert!(!instruction.accounts[3].is_signer); // PDAs don't sign
    //     assert!(instruction.accounts[3].is_writable); // PDAs are written to

    //     assert_eq!(instruction.accounts[4].pubkey, pda2);
    //     assert!(!instruction.accounts[4].is_signer); // PDAs don't sign
    //     assert!(instruction.accounts[4].is_writable); // PDAs are written to

    //     // Verify instruction data is present and starts with discriminator
    //     assert!(!instruction.data.is_empty());
    //     assert_eq!(&instruction.data[0..8], &discriminator);

    //     println!("✅ Decompress multiple PDAs instruction builder follows Solana SDK patterns correctly!");
    // }

    // #[test]
    // #[should_panic(expected = "PDA accounts and compressed accounts must have same length")]
    // fn test_decompress_multiple_pdas_validation_accounts_mismatch() {
    //     use light_client::compressible::CompressedAccountData;
    //     use light_sdk::instruction::{account_meta::CompressedAccountMeta, ValidityProof};

    //     let program_id = Pubkey::new_unique();
    //     let fee_payer = Pubkey::new_unique();
    //     let rent_payer = Pubkey::new_unique();
    //     let pda_accounts = vec![Pubkey::new_unique()]; // 1 PDA

    //     // But 2 compressed accounts - should panic
    //     let compressed_accounts = vec![
    //         CompressedAccountData {
    //             meta: CompressedAccountMeta {
    //                 tree_info: Default::default(),
    //                 address: [1u8; 32],
    //                 output_state_tree_index: 0,
    //             },
    //             data: vec![1, 2, 3, 4],
    //         },
    //         CompressedAccountData {
    //             meta: CompressedAccountMeta {
    //                 tree_info: Default::default(),
    //                 address: [2u8; 32],
    //                 output_state_tree_index: 1,
    //             },
    //             data: vec![5, 6, 7, 8],
    //         },
    //     ];

    //     let bumps = vec![250u8];
    //     let discriminator = [1, 2, 3, 4, 5, 6, 7, 8];

    //     CompressibleInstruction::decompress_multiple_pdas(
    //         &program_id,
    //         &fee_payer,
    //         &rent_payer,
    //         &pda_accounts,
    //         ValidityProof::default(),
    //         compressed_accounts,
    //         bumps,
    //         vec![],
    //         &discriminator,
    //     );
    // }

    // #[test]
    // #[should_panic(expected = "PDA accounts and bumps must have same length")]
    // fn test_decompress_multiple_pdas_validation_bumps_mismatch() {
    //     use light_client::compressible::CompressedAccountData;
    //     use light_sdk::instruction::{account_meta::CompressedAccountMeta, ValidityProof};

    //     let program_id = Pubkey::new_unique();
    //     let fee_payer = Pubkey::new_unique();
    //     let rent_payer = Pubkey::new_unique();
    //     let pda_accounts = vec![Pubkey::new_unique()]; // 1 PDA

    //     let compressed_accounts = vec![CompressedAccountData {
    //         meta: CompressedAccountMeta {
    //             tree_info: Default::default(),
    //             address: [1u8; 32],
    //             output_state_tree_index: 0,
    //         },
    //         data: vec![1, 2, 3, 4],
    //     }];

    //     let bumps = vec![250u8, 251u8]; // 2 bumps but 1 PDA - should panic
    //     let discriminator = [1, 2, 3, 4, 5, 6, 7, 8];

    //     CompressibleInstruction::decompress_multiple_pdas(
    //         &program_id,
    //         &fee_payer,
    //         &rent_payer,
    //         &pda_accounts,
    //         ValidityProof::default(),
    //         compressed_accounts,
    //         bumps,
    //         vec![],
    //         &discriminator,
    //     );
    // }
}

// Add module declarations at the top level
mod common;
