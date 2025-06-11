#[cfg(test)]
mod tests {
    use light_compressed_token_client::{
        batch_compress, compress, create_decompress_instruction, AccountState, CompressedAccount,
        DecompressParams, MerkleContext, TokenData, TreeType,
    };
    use solana_sdk::pubkey::Pubkey;

    #[test]
    fn test_simple_compress() {
        let payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let source_token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let recipient = Pubkey::new_unique();
        let output_state_tree = Pubkey::new_unique();

        let instruction = compress(
            payer,
            owner,
            source_token_account,
            mint,
            1000, // amount
            recipient,
            output_state_tree,
        )
        .expect("Failed to create compress instruction");

        assert_eq!(
            instruction.program_id,
            light_compressed_token_client::PROGRAM_ID
        );
        assert!(!instruction.accounts.is_empty());

        let account_keys: Vec<_> = instruction.accounts.iter().map(|a| a.pubkey).collect();
        assert!(account_keys.contains(&payer));
        assert!(account_keys.contains(&owner));
        assert!(account_keys.contains(&source_token_account));
        assert!(account_keys.contains(&output_state_tree));
    }

    #[test]
    fn test_batch_compress() {
        let payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let source_token_account = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let output_state_tree = Pubkey::new_unique();

        let recipients = vec![
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
        ];
        let amounts = vec![500, 300, 200];

        let total_amount: u64 = amounts.iter().sum();
        assert_eq!(total_amount, 1000);

        let instruction = batch_compress(
            payer,
            owner,
            source_token_account,
            mint,
            recipients,
            amounts,
            output_state_tree,
        )
        .expect("Failed to create batch compress instruction");

        assert_eq!(
            instruction.program_id,
            light_compressed_token_client::PROGRAM_ID
        );
        assert!(!instruction.accounts.is_empty());

        let account_keys: Vec<_> = instruction.accounts.iter().map(|a| a.pubkey).collect();
        assert!(account_keys.contains(&payer));
        assert!(account_keys.contains(&owner));
        assert!(account_keys.contains(&source_token_account));

        assert!(account_keys.contains(&output_state_tree));
    }

    #[test]
    fn test_decompress() {
        let payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let destination_token_account = Pubkey::new_unique();
        let merkle_tree = Pubkey::new_unique();
        let queue = Pubkey::new_unique();

        let compressed_account = CompressedAccount {
            owner: light_compressed_token_client::PROGRAM_ID,
            lamports: 0,
            data: None,
            address: None,
        };

        let token_data = TokenData {
            mint,
            owner,
            amount: 1000,
            delegate: None,
            state: AccountState::Initialized,
            tlv: None,
        };

        let merkle_context = MerkleContext {
            merkle_tree_pubkey: merkle_tree,
            queue_pubkey: queue,
            leaf_index: 0,
            prove_by_index: false,
            tree_type: TreeType::StateV2,
        };

        let params = DecompressParams {
            payer,
            input_compressed_token_accounts: vec![(
                compressed_account.clone(),
                token_data.clone(),
                merkle_context.clone(),
            )],
            to_address: destination_token_account,
            amount: 500,
            recent_input_state_root_indices: vec![Some(0)],
            recent_validity_proof: None,
            output_state_tree: Some(merkle_tree),
            token_program_id: None,
        };

        let instruction =
            create_decompress_instruction(params).expect("Failed to create decompress instruction");

        assert_eq!(
            instruction.program_id,
            light_compressed_token_client::PROGRAM_ID
        );
        assert!(!instruction.accounts.is_empty());

        let account_keys: Vec<_> = instruction.accounts.iter().map(|a| a.pubkey).collect();
        assert!(account_keys.contains(&payer));
        assert!(account_keys.contains(&destination_token_account));
        assert!(account_keys.contains(&merkle_tree));
        assert!(account_keys.contains(&queue));

        assert_eq!(token_data.amount, 1000);
        assert_eq!(token_data.owner, owner);
        assert_eq!(token_data.mint, mint);
        assert_eq!(token_data.state, AccountState::Initialized);

        assert_eq!(
            compressed_account.owner,
            light_compressed_token_client::PROGRAM_ID
        );
        assert_eq!(compressed_account.lamports, 0);

        assert_eq!(merkle_context.merkle_tree_pubkey, merkle_tree);
        assert_eq!(merkle_context.queue_pubkey, queue);
        assert_eq!(merkle_context.leaf_index, 0);
        assert!(!merkle_context.prove_by_index);
        assert_eq!(merkle_context.tree_type, TreeType::StateV2);
    }

    #[test]
    fn test_decompress_partial_amount() {
        let payer = Pubkey::new_unique();
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let destination_token_account = Pubkey::new_unique();
        let merkle_tree = Pubkey::new_unique();
        let queue = Pubkey::new_unique();

        let total_amount = 1000u64;
        let decompress_amount = 500u64;

        let compressed_account = CompressedAccount {
            owner: light_compressed_token_client::PROGRAM_ID,
            lamports: 0,
            data: None,
            address: None,
        };

        let token_data = TokenData {
            mint,
            owner,
            amount: total_amount,
            delegate: None,
            state: AccountState::Initialized,
            tlv: None,
        };

        let merkle_context = MerkleContext {
            merkle_tree_pubkey: merkle_tree,
            queue_pubkey: queue,
            leaf_index: 0,
            prove_by_index: false,
            tree_type: TreeType::StateV2,
        };

        let params = DecompressParams {
            payer,
            input_compressed_token_accounts: vec![(
                compressed_account,
                token_data.clone(),
                merkle_context,
            )],
            to_address: destination_token_account,
            amount: decompress_amount,
            recent_input_state_root_indices: vec![Some(0)],
            recent_validity_proof: None,
            output_state_tree: Some(merkle_tree),
            token_program_id: None,
        };

        let instruction =
            create_decompress_instruction(params).expect("Failed to create decompress instruction");

        assert!(instruction.accounts.len() > 0);
        assert_eq!(token_data.amount, total_amount);
        assert!(decompress_amount < total_amount);
        assert_eq!(total_amount - decompress_amount, 500);
    }
}
