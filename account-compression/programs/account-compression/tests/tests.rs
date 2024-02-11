#[cfg(feature = "solana-tests")]
mod test {
    use account_compression::{
        instruction::{InitializeAddressMerkleTree, InitializeAddressQueue, InsertAddresses, UpdateAddressMerkleTree},
        state::{AddressMerkleTreeAccount, AddressQueueAccount},
        ID,
    };
    use account_compression_state::{address_merkle_tree_from_bytes, address_queue_from_bytes, address_queue_from_bytes_mut, MERKLE_TREE_HEIGHT, MERKLE_TREE_CHANGELOG, MERKLE_TREE_ROOTS, QUEUE_ELEMENTS};
    use anchor_lang::{InstructionData, ZeroCopy};
    use ark_ff::{BigInteger, BigInteger256};
    use light_hasher::Poseidon;
    use light_indexed_merkle_tree::{array::IndexingArray, reference, IndexedMerkleTree};
    use solana_program_test::{ProgramTest, ProgramTestContext};
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        system_instruction, system_program,
        transaction::Transaction,
    };

    async fn get_account_zero_copy<T>(context: &mut ProgramTestContext, pubkey: Pubkey) -> &T
    where
        T: ZeroCopy,
    {
        let account = context
            .banks_client
            .get_account(pubkey)
            .await
            .unwrap()
            .unwrap();

        // TODO: Check discriminator.

        unsafe {
            let ptr = account.data[8..].as_ptr() as *const T;
            &*ptr
        }
    }

    async fn get_account_zero_copy_mut<T>(context: &mut ProgramTestContext, pubkey: Pubkey) -> &mut T
    where
        T: ZeroCopy,
    {
        let account = context
            .banks_client
            .get_account(pubkey)
            .await
            .unwrap()
            .unwrap();

        // TODO: Check discriminator.

        unsafe {
            let ptr = account.data[8..].as_ptr() as *mut T;
            &mut *ptr
        }
    }

    async fn create_account_ix(
        context: &mut ProgramTestContext,
        size: usize,
    ) -> (Keypair, Instruction) {
        let keypair = Keypair::new();
        let instruction = system_instruction::create_account(
            &context.payer.pubkey(),
            &keypair.pubkey(),
            context
                .banks_client
                .get_rent()
                .await
                .unwrap()
                .minimum_balance(size),
            size as u64,
            &ID,
        );
        (keypair, instruction)
    }

    async fn initialize_address_queue_ix(
        context: &ProgramTestContext,
        pubkey: Pubkey,
    ) -> Instruction {
        let instruction_data = InitializeAddressQueue {};
        let initialize_ix = Instruction {
            program_id: ID,
            accounts: vec![
                AccountMeta::new(context.payer.pubkey(), true),
                AccountMeta::new(pubkey, true),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data: instruction_data.data(),
        };
        initialize_ix
    }

    async fn create_and_initialize_address_queue(context: &mut ProgramTestContext) -> Keypair {
        let (address_queue_keypair, account_create_ix) =
            create_account_ix(context, AddressQueueAccount::LEN).await;

        // Instruction: initialize address queue.
        let initialize_ix =
            initialize_address_queue_ix(context, address_queue_keypair.pubkey()).await;

        // Transaction: initialize address queue.
        let transaction = Transaction::new_signed_with_payer(
            &[account_create_ix, initialize_ix],
            Some(&context.payer.pubkey()),
            &[&context.payer, &address_queue_keypair],
            context.last_blockhash,
        );
        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        address_queue_keypair
    }

    async fn initialize_address_merkle_tree_ix(
        context: &ProgramTestContext,
        pubkey: Pubkey,
    ) -> Instruction {
        let instruction_data = InitializeAddressMerkleTree {};
        let initialize_ix = Instruction {
            program_id: ID,
            accounts: vec![
                AccountMeta::new(context.payer.pubkey(), true),
                AccountMeta::new(pubkey, true),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data: instruction_data.data(),
        };
        initialize_ix
    }

    async fn create_and_initialize_address_merkle_tree(
        context: &mut ProgramTestContext,
    ) -> Keypair {
        let (address_merkle_tree_keypair, account_create_ix) =
            create_account_ix(context, AddressMerkleTreeAccount::LEN).await;

        // Instruction: initialize address Merkle tree.
        let initialize_ix =
            initialize_address_merkle_tree_ix(context, address_merkle_tree_keypair.pubkey()).await;

        // Transaction: initialize address Merkle tree.
        let transaction = Transaction::new_signed_with_payer(
            &[account_create_ix, initialize_ix],
            Some(&context.payer.pubkey()),
            &[&context.payer, &address_merkle_tree_keypair],
            context.last_blockhash,
        );
        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        address_merkle_tree_keypair
    }

    async fn insert_addresses(
        context: &mut ProgramTestContext,
        address_queue_pubkey: Pubkey,
        addresses: Vec<[u8; 32]>,
    ) {
        let instruction_data = InsertAddresses { addresses };
        let insert_ix = Instruction {
            program_id: ID,
            accounts: vec![
                AccountMeta::new(context.payer.pubkey(), true),
                AccountMeta::new(address_queue_pubkey, false),
            ],
            data: instruction_data.data(),
        };
        let transaction = Transaction::new_signed_with_payer(
            &[insert_ix],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );
        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();
    }

    async fn update_merkle_tree(context: &mut ProgramTestContext, address_queue_pubkey: Pubkey, address_merkle_tree_pubkey: Pubkey, queue_index: u16) {
        let address_queue: &mut AddressQueueAccount = get_account_zero_copy_mut(&mut context, address_queue_pubkey).await;
        let address_queue = address_queue_from_bytes_mut(&mut address_queue.queue);
        let address_merkle_tree: &AddressMerkleTreeAccount = get_account_zero_copy(&mut context, address_merkle_tree_pubkey).await;
        let address_merkle_tree = address_merkle_tree_from_bytes(&address_merkle_tree.merkle_tree);

        // Remove the address from the queue.
        let mut address = address_queue.dequeue_at(queue_index).unwrap().unwrap();

        let instruction_data = UpdateAddressMerkleTree {
            changelog_index: address_merkle_tree.changelog_index() as u16,
            queue_index,
        };
        let update_ix = Instruction {
            program_id: ID,
            accounts: vec![
                AccountMeta::new(context.payer.pubkey(), true),
                AccountMeta::new(address_queue_pubkey, false),
                AccountMeta::new(address_merkle_tree_pubkey, false),
            ],
            data: instruction_data.data(),
        };
        let transaction = Transaction::new_signed_with_payer(
            &[update_ix],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );
        context
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();
    }

    async fn relayer_update(context: &mut ProgramTestContext, address_queue_pubkey: Pubkey, address_merkle_tree_pubkey: Pubkey) {
        let address_queue: &AddressQueueAccount = get_account_zero_copy(context, address_queue_pubkey).await;
        let address_queue = address_queue_from_bytes(&address_queue.queue);
        let address_merkle_tree: &AddressMerkleTreeAccount = get_account_zero_copy(context, address_merkle_tree_pubkey).await;
        let address_merkle_tree = address_merkle_tree_from_bytes(&address_merkle_tree.merkle_tree);

        let mut relayer_indexing_array = IndexingArray::<Poseidon, BigInteger256, QUEUE_ELEMENTS>::default();
        let mut relayer_merkle_tree = reference::IndexedMerkleTree::<
            Poseidon,
            BigInteger256,
            MERKLE_TREE_HEIGHT,
            MERKLE_TREE_ROOTS,
        >::new()
        .unwrap();

        while !address_queue.is_empty() {
            let changelog_index = address_merkle_tree.changelog_index();

            let lowest_from_queue = match address_queue.lowest() {
                Some(lowest) => lowest,
                None => break,
            };

            // Create new element from the dequeued value.
            let (old_low_address, old_low_address_next_value) = relayer_indexing_array
                .find_low_element(&lowest_from_queue.value)
                .unwrap();
            let address_bundle = relayer_indexing_array
                .new_element_with_low_element_index(old_low_address.index, lowest_from_queue.value);

            // Get the Merkle proof for updaring low element.
            let low_address_proof =
                relayer_merkle_tree.get_proof_of_leaf(usize::from(old_low_address.index));


        }
    }

    #[tokio::test]
    async fn test_address_queue() {
        let mut program_test = ProgramTest::default();
        program_test.add_program("account_compression", ID, None);
        // program_test.set_compute_max_units(1_400_000u64);
        let mut context = program_test.start_with_context().await;

        let address_queue_keypair = create_and_initialize_address_queue(&mut context).await;
        let address_merkle_tree_keypair =
            create_and_initialize_address_merkle_tree(&mut context).await;

        // Insert a pair of addresses.
        let address1 = BigInteger256::from(30_u32);
        let address2 = BigInteger256::from(10_u32);
        let addresses: Vec<[u8; 32]> = vec![
            address1.to_bytes_be().try_into().unwrap(),
            address2.to_bytes_be().try_into().unwrap(),
        ];
        insert_addresses(&mut context, address_queue_keypair.pubkey(), addresses).await;

        // Check if addresses were inserted properly.
        let address_queue: &AddressQueueAccount =
            get_account_zero_copy(&mut context, address_queue_keypair.pubkey()).await;
        let address_queue = address_queue_from_bytes(&address_queue.queue);
        let element0 = address_queue.get(0).unwrap();
        assert_eq!(element0.index, 0);
        assert_eq!(element0.value, BigInteger256::from(0_u32));
        assert_eq!(element0.next_index, 2);
        let element1 = address_queue.get(1).unwrap();
        assert_eq!(element1.index, 1);
        assert_eq!(element1.value, BigInteger256::from(30_u32));
        assert_eq!(element1.next_index, 0);
        let element2 = address_queue.get(2).unwrap();
        assert_eq!(element2.index, 2);
        assert_eq!(element2.value, BigInteger256::from(10_u32));
        assert_eq!(element2.next_index, 1);
    }
}
