#[cfg(test)]
mod test_instruction_builders {

    use light_sdk::compressible::{CompressibleConfig, CompressibleInstruction};
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
            &payer,
            &authority,
            compression_delay,
            rent_recipient,
            address_space,
        );

        // Verify instruction structure
        assert_eq!(instruction.program_id, program_id);
        assert_eq!(instruction.accounts.len(), 5); // payer, config, program_data, authority, system_program

        // Verify account order and permissions
        assert_eq!(instruction.accounts[0].pubkey, payer);
        assert!(instruction.accounts[0].is_signer); // payer signs
        assert!(instruction.accounts[0].is_writable); // payer pays

        let (expected_config_pda, _) = CompressibleConfig::derive_pda(&program_id);
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
            &authority,
            new_compression_delay,
            new_rent_recipient,
            None,
            None,
        );

        // Verify instruction structure
        assert_eq!(instruction.program_id, program_id);
        assert_eq!(instruction.accounts.len(), 2); // config, authority

        let (expected_config_pda, _) = CompressibleConfig::derive_pda(&program_id);
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
}

// Add module declarations at the top level
mod common;
