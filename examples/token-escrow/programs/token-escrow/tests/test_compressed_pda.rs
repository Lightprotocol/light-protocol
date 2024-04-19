//#![cfg(feature = "test-sbf")]

// 2. escrow tokens with compressed pda
// create test env
// create mint and mint tokens
// escrow compressed tokens - with compressed pda
// release compressed tokens

// TODO: 3. escrow tokens by decompression with compressed pda
// this design pattern can be used to use compressed accounts with an AMMM
// create test env
// create mint and mint tokens
// decomcompress compressed tokens into program owned token account - with compressed pda
// release compressed tokens

use anchor_lang::AnchorDeserialize;
use light_hasher::{Hasher, Poseidon};
use light_test_utils::test_env::setup_test_programs_with_accounts;
use light_test_utils::test_indexer::{create_mint_helper, mint_tokens_helper, TestIndexer};
use solana_sdk::{pubkey::Pubkey, signer::Signer, transaction::Transaction};
use token_escrow::compressed_pda_sdk::{create_escrow_instruction, create_withdrawal_instruction};
use token_escrow::{EscrowTimeLock, MerkleContext};

#[tokio::test]
async fn test_escrow_with_compressed_pda() {
    let env: light_test_utils::test_env::EnvWithAccounts = setup_test_programs_with_accounts(Some(
        vec![(String::from("token_escrow"), token_escrow::ID)],
    ))
    .await;
    let mut context = env.context;
    let cpi_signature_account_pubkey = env.cpi_signature_account_pubkey;
    let payer = context.payer.insecure_clone();
    let payer_pubkey = payer.pubkey();
    println!("payer_pubkey {:?}", payer_pubkey);
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let indexed_array_pubkey = env.indexed_array_pubkey;
    let address_merkle_tree_pubkey = env.address_merkle_tree_pubkey;
    let test_indexer = TestIndexer::new(
        merkle_tree_pubkey,
        indexed_array_pubkey,
        address_merkle_tree_pubkey,
        payer.insecure_clone(),
        true,
        true,
        false,
    );
    let mint = create_mint_helper(&mut context, &payer).await;
    let mut test_indexer = test_indexer.await;
    // big footgun signer check of token account is done with zkp onchain thus no conclusive error message
    // let recipient_keypair = Keypair::new();
    let amount = 10000u64;
    mint_tokens_helper(
        &mut context,
        &mut test_indexer,
        &merkle_tree_pubkey,
        &payer,
        &mint,
        vec![amount],
        vec![payer.pubkey()],
    )
    .await;
    let input_compressed_token_account_data = test_indexer.token_compressed_accounts[0].clone();

    let compressed_input_account_with_context =
        test_indexer.compressed_accounts[input_compressed_token_account_data.index].clone();
    let input_compressed_account_hash = test_indexer.compressed_accounts
        [input_compressed_token_account_data.index]
        .compressed_account
        .hash(
            &merkle_tree_pubkey,
            &compressed_input_account_with_context.leaf_index,
        )
        .unwrap();
    let seed = [1u8; 32];
    let address = psp_compressed_pda::compressed_account::derive_address(
        &env.address_merkle_tree_pubkey,
        &seed,
    )
    .unwrap();

    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(&[input_compressed_account_hash]),
            Some(&[address]),
            &mut context,
        )
        .await;

    let escrow_amount = 100u64;
    let new_address_params: psp_compressed_pda::NewAddressParams =
        psp_compressed_pda::NewAddressParams {
            seed: [1u8; 32],
            address_merkle_tree_pubkey: env.address_merkle_tree_pubkey,
            address_queue_pubkey: env.address_merkle_tree_queue_pubkey,
            address_merkle_tree_root_index: rpc_result.address_root_indices[0],
        };
    let create_ix_inputs =
        token_escrow::compressed_pda_sdk::CreateCompressedPdaEscrowInstructionInputs {
            input_token_data: &vec![input_compressed_token_account_data.token_data],
            lock_up_time: 0,
            signer: &payer_pubkey,
            input_compressed_account_merkle_tree_pubkeys: &[merkle_tree_pubkey],
            nullifier_array_pubkeys: &[indexed_array_pubkey],
            output_compressed_account_merkle_tree_pubkeys: &[
                merkle_tree_pubkey,
                merkle_tree_pubkey,
            ],
            output_compressed_accounts: &Vec::new(),
            root_indices: &rpc_result.root_indices,
            proof: &rpc_result.proof,
            leaf_indices: &[compressed_input_account_with_context.leaf_index],
            mint: &input_compressed_token_account_data.token_data.mint,
            new_address_params,
            cpi_signature_account: &cpi_signature_account_pubkey,
        };
    let instruction = create_escrow_instruction(create_ix_inputs.clone(), escrow_amount);
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        context.get_new_latest_blockhash().await.unwrap(),
    );
    let res = solana_program_test::BanksClient::process_transaction_with_metadata(
        &mut context.banks_client,
        transaction,
    )
    .await;
    test_indexer.add_compressed_accounts_with_token_data(
        res.unwrap()
            .metadata
            .unwrap()
            .return_data
            .unwrap()
            .data
            .to_vec(),
    );

    let token_escrow = test_indexer.token_compressed_accounts[1].token_data.clone();
    assert_eq!(token_escrow.amount, escrow_amount);
    let cpi_signer = Pubkey::find_program_address(
        &[b"escrow".as_slice(), payer_pubkey.to_bytes().as_slice()],
        &token_escrow::id(),
    )
    .0;
    assert_eq!(token_escrow.owner, cpi_signer);

    let token_data_change_compressed_token_account =
        test_indexer.token_compressed_accounts[2].token_data.clone();
    assert_eq!(
        token_data_change_compressed_token_account.amount,
        amount - escrow_amount
    );
    assert_eq!(
        token_data_change_compressed_token_account.owner,
        payer_pubkey
    );

    let compressed_escrow_pda = test_indexer.compressed_accounts[0].clone();
    assert_eq!(
        compressed_escrow_pda.compressed_account.address.unwrap(),
        address
    );
    assert_eq!(
        compressed_escrow_pda.compressed_account.owner,
        token_escrow::ID
    );
    let compressed_escrow_pda_deserialized = compressed_escrow_pda
        .compressed_account
        .data
        .as_ref()
        .unwrap();
    let compressed_escrow_pda_data =
        EscrowTimeLock::deserialize_reader(&mut &compressed_escrow_pda_deserialized.data[..])
            .unwrap();
    assert_ne!(compressed_escrow_pda_data.slot, 0);
    assert!(compressed_escrow_pda_data.slot < 100);
    assert_eq!(
        compressed_escrow_pda_deserialized.discriminator,
        1u64.to_le_bytes(),
    );
    assert_eq!(
        compressed_escrow_pda_deserialized.data_hash,
        Poseidon::hash(&compressed_escrow_pda_data.slot.to_le_bytes()).unwrap(),
    );
    println!("withdrawal _----------------------------------------------------------------");
    let token_escrow_account =
        test_indexer.compressed_accounts[test_indexer.token_compressed_accounts[1].index].clone();
    let token_escrow_account_hash = token_escrow_account
        .compressed_account
        .hash(&merkle_tree_pubkey, &token_escrow_account.leaf_index)
        .unwrap();
    println!("token_data_escrow {:?}", token_escrow);
    println!(
        "token escrow_account {:?}",
        test_indexer.compressed_accounts[test_indexer.token_compressed_accounts[1].index]
    );
    let compressed_pda_hash = compressed_escrow_pda
        .compressed_account
        .hash(&merkle_tree_pubkey, &compressed_escrow_pda.leaf_index)
        .unwrap();

    // compressed pda will go first into the proof because in the program
    // the compressed pda program executes the transaction
    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(&[compressed_pda_hash, token_escrow_account_hash]),
            None,
            &mut context,
        )
        .await;
    println!("rpc_result {:?}", rpc_result);
    let create_withdrawal_ix_inputs =
        token_escrow::compressed_pda_sdk::CreateCompressedPdaWithdrawalInstructionInputs {
            input_token_data: &vec![test_indexer.token_compressed_accounts[1].token_data],
            lock_up_time: 0,
            signer: &payer_pubkey,
            input_compressed_account_merkle_tree_pubkeys: &[merkle_tree_pubkey],
            nullifier_array_pubkeys: &[indexed_array_pubkey],
            output_compressed_account_merkle_tree_pubkeys: &[
                merkle_tree_pubkey,
                merkle_tree_pubkey,
            ],
            output_compressed_accounts: &Vec::new(),
            root_indices: &rpc_result.root_indices,
            proof: &rpc_result.proof,
            leaf_indices: &[
                compressed_escrow_pda.leaf_index,
                token_escrow_account.leaf_index,
            ],
            mint: &input_compressed_token_account_data.token_data.mint,
            cpi_signature_account: &cpi_signature_account_pubkey,
            old_lock_up_time: compressed_escrow_pda_data.slot,
            new_lock_up_time: 1000,
            address: compressed_escrow_pda.compressed_account.address.unwrap(),
            merkle_context: MerkleContext {
                leaf_index: compressed_escrow_pda.leaf_index,
                merkle_tree_pubkey,
                nullifier_queue_pubkey: indexed_array_pubkey,
            },
        };
    let instruction =
        create_withdrawal_instruction(create_withdrawal_ix_inputs.clone(), escrow_amount);
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        context.get_new_latest_blockhash().await.unwrap(),
    );
    let res = solana_program_test::BanksClient::process_transaction_with_metadata(
        &mut context.banks_client,
        transaction,
    )
    .await;
    test_indexer.add_compressed_accounts_with_token_data(
        res.unwrap()
            .metadata
            .unwrap()
            .return_data
            .unwrap()
            .data
            .to_vec(),
    );
}
