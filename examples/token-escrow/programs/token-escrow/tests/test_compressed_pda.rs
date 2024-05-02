#![cfg(feature = "test-sbf")]

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
use light_compressed_pda::sdk::address::derive_address;
use light_compressed_pda::sdk::compressed_account::MerkleContext;
use light_compressed_pda::sdk::event::PublicTransactionEvent;
use light_compressed_pda::NewAddressParams;
use light_hasher::{Hasher, Poseidon};
use light_test_utils::test_env::{setup_test_programs_with_accounts, EnvAccounts};
use light_test_utils::test_indexer::{create_mint_helper, mint_tokens_helper, TestIndexer};
use light_test_utils::{create_and_send_transaction_with_event, FeeConfig, TransactionParams};
use solana_program_test::{
    BanksClientError, BanksTransactionResultWithMetadata, ProgramTestContext,
};
use solana_sdk::instruction::{Instruction, InstructionError};
use solana_sdk::signature::Keypair;
use solana_sdk::{signer::Signer, transaction::Transaction};
use token_escrow::escrow_with_compressed_pda::sdk::{
    create_escrow_instruction, create_withdrawal_instruction, get_token_owner_pda,
    CreateCompressedPdaEscrowInstructionInputs, CreateCompressedPdaWithdrawalInstructionInputs,
};
use token_escrow::{EscrowError, EscrowTimeLock};

#[tokio::test]
async fn test_escrow_with_compressed_pda() {
    let (mut context, env) = setup_test_programs_with_accounts(Some(vec![(
        String::from("token_escrow"),
        token_escrow::ID,
    )]))
    .await;
    let payer = context.payer.insecure_clone();

    let address_merkle_tree_pubkey = env.address_merkle_tree_pubkey;
    let test_indexer = TestIndexer::new(
        env.merkle_tree_pubkey,
        env.nullifier_queue_pubkey,
        address_merkle_tree_pubkey,
        payer.insecure_clone(),
        true,
        true,
        "../../../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    );
    let mint = create_mint_helper(&mut context, &payer).await;
    let mut test_indexer = test_indexer.await;

    let amount = 10000u64;
    mint_tokens_helper(
        &mut context,
        &mut test_indexer,
        &env.merkle_tree_pubkey,
        &payer,
        &mint,
        vec![amount],
        vec![payer.pubkey()],
    )
    .await;

    let seed = [1u8; 32];
    let escrow_amount = 100u64;
    let lock_up_time = 1000u64;

    perform_escrow_with_event(
        &mut test_indexer,
        &mut context,
        &env,
        &payer,
        lock_up_time,
        escrow_amount,
        seed,
    )
    .await
    .unwrap();

    let current_slot = context.banks_client.get_root_slot().await.unwrap();
    let lockup_end = lock_up_time + current_slot;
    assert_escrow(
        &mut test_indexer,
        &env,
        &payer,
        &escrow_amount,
        &amount,
        &seed,
        &lockup_end,
    )
    .await;

    println!("withdrawal _----------------------------------------------------------------");
    let withdrawal_amount = escrow_amount;
    let new_lock_up_time = 2000u64;
    let res = perform_withdrawal_failing(
        &mut context,
        &mut test_indexer,
        &env,
        &payer,
        lock_up_time,
        new_lock_up_time,
        withdrawal_amount,
    )
    .await;

    assert_eq!(
        res.unwrap().result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            0,
            InstructionError::Custom(EscrowError::EscrowLocked.into())
        ))
    );
    context.warp_to_slot(lock_up_time + 1).unwrap();

    perform_withdrawal_with_event(
        &mut context,
        &mut test_indexer,
        &env,
        &payer,
        lockup_end,
        new_lock_up_time,
        withdrawal_amount,
    )
    .await
    .unwrap();

    assert_withdrawal(
        &mut context,
        &mut test_indexer,
        &env,
        &payer,
        &withdrawal_amount,
        &escrow_amount,
        &seed,
        new_lock_up_time,
    )
    .await;
}

pub async fn perform_escrow_failing(
    test_indexer: &mut TestIndexer,
    context: &mut ProgramTestContext,
    env: &EnvAccounts,
    payer: &Keypair,
    lock_up_time: u64,
    escrow_amount: u64,
    seed: [u8; 32],
) -> Result<BanksTransactionResultWithMetadata, BanksClientError> {
    let (payer_pubkey, instruction) = create_escrow_ix(
        payer,
        test_indexer,
        env,
        seed,
        context,
        lock_up_time,
        escrow_amount,
    )
    .await;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        context.get_new_latest_blockhash().await.unwrap(),
    );
    solana_program_test::BanksClient::process_transaction_with_metadata(
        &mut context.banks_client,
        transaction,
    )
    .await
}

pub async fn perform_escrow_with_event(
    test_indexer: &mut TestIndexer,
    context: &mut ProgramTestContext,
    env: &EnvAccounts,
    payer: &Keypair,
    lock_up_time: u64,
    escrow_amount: u64,
    seed: [u8; 32],
) -> Result<(), BanksClientError> {
    let (_, instruction) = create_escrow_ix(
        payer,
        test_indexer,
        env,
        seed,
        context,
        lock_up_time,
        escrow_amount,
    )
    .await;
    let event = create_and_send_transaction_with_event::<PublicTransactionEvent>(
        context,
        &[instruction],
        &payer.pubkey(),
        &[payer],
        Some(TransactionParams {
            num_input_compressed_accounts: 1,
            num_output_compressed_accounts: 3,
            num_new_addresses: 1,
            compress: 0,
            fee_config: FeeConfig::default(),
        }),
    )
    .await?;
    test_indexer.add_compressed_accounts_with_token_data(event.unwrap());
    Ok(())
}

async fn create_escrow_ix(
    payer: &Keypair,
    test_indexer: &mut TestIndexer,
    env: &EnvAccounts,
    seed: [u8; 32],
    context: &mut ProgramTestContext,
    lock_up_time: u64,
    escrow_amount: u64,
) -> (anchor_lang::prelude::Pubkey, Instruction) {
    let payer_pubkey = payer.pubkey();
    let input_compressed_token_account_data = test_indexer.token_compressed_accounts[0].clone();

    let compressed_input_account_with_context =
        test_indexer.compressed_accounts[input_compressed_token_account_data.index].clone();
    let input_compressed_account_hash = test_indexer.compressed_accounts
        [input_compressed_token_account_data.index]
        .compressed_account
        .hash(
            &env.merkle_tree_pubkey,
            &compressed_input_account_with_context
                .merkle_context
                .leaf_index,
        )
        .unwrap();

    let address = derive_address(&env.address_merkle_tree_pubkey, &seed).unwrap();

    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(&[input_compressed_account_hash]),
            Some(&[address]),
            context,
        )
        .await;

    let new_address_params = NewAddressParams {
        seed,
        address_merkle_tree_pubkey: env.address_merkle_tree_pubkey,
        address_queue_pubkey: env.address_merkle_tree_queue_pubkey,
        address_merkle_tree_root_index: rpc_result.address_root_indices[0],
    };
    let create_ix_inputs = CreateCompressedPdaEscrowInstructionInputs {
        input_token_data: &vec![input_compressed_token_account_data.token_data],
        lock_up_time,
        signer: &payer_pubkey,
        input_merkle_context: &[MerkleContext {
            leaf_index: compressed_input_account_with_context
                .merkle_context
                .leaf_index,
            merkle_tree_pubkey: env.merkle_tree_pubkey,
            nullifier_queue_pubkey: env.nullifier_queue_pubkey,
        }],
        output_compressed_account_merkle_tree_pubkeys: &[
            env.merkle_tree_pubkey,
            env.merkle_tree_pubkey,
        ],
        output_compressed_accounts: &Vec::new(),
        root_indices: &rpc_result.root_indices,
        proof: &rpc_result.proof,
        mint: &input_compressed_token_account_data.token_data.mint,
        new_address_params,
        cpi_signature_account: &env.cpi_signature_account_pubkey,
    };
    let instruction = create_escrow_instruction(create_ix_inputs.clone(), escrow_amount);
    (payer_pubkey, instruction)
}

pub async fn assert_escrow(
    test_indexer: &mut TestIndexer,
    env: &EnvAccounts,
    payer: &Keypair,
    escrow_amount: &u64,
    amount: &u64,
    seed: &[u8; 32],
    lock_up_time: &u64,
) {
    let payer_pubkey = payer.pubkey();
    let token_owner_pda = get_token_owner_pda(&payer_pubkey).0;
    let token_data_escrow = test_indexer
        .token_compressed_accounts
        .iter()
        .find(|x| x.token_data.owner == token_owner_pda)
        .unwrap()
        .token_data
        .clone();
    assert_eq!(token_data_escrow.amount, *escrow_amount);
    assert_eq!(token_data_escrow.owner, token_owner_pda);

    let token_data_change_compressed_token_account_exist =
        test_indexer.token_compressed_accounts.iter().any(|x| {
            x.token_data.owner == payer.pubkey() && x.token_data.amount == amount - escrow_amount
        });
    assert!(token_data_change_compressed_token_account_exist);

    let compressed_escrow_pda = test_indexer
        .compressed_accounts
        .iter()
        .find(|x| x.compressed_account.owner == token_escrow::ID)
        .unwrap()
        .clone();
    let address = derive_address(&env.address_merkle_tree_pubkey, &seed).unwrap();
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
    println!(
        "compressed_escrow_pda_data {:?}",
        compressed_escrow_pda_data
    );
    assert_eq!(compressed_escrow_pda_data.slot, *lock_up_time);
    assert_eq!(
        compressed_escrow_pda_deserialized.discriminator,
        1u64.to_le_bytes(),
    );
    assert_eq!(
        compressed_escrow_pda_deserialized.data_hash,
        Poseidon::hash(&compressed_escrow_pda_data.slot.to_le_bytes()).unwrap(),
    );
}
pub async fn perform_withdrawal_with_event(
    context: &mut ProgramTestContext,
    test_indexer: &mut TestIndexer,
    env: &EnvAccounts,
    payer: &Keypair,
    old_lock_up_time: u64,
    new_lock_up_time: u64,
    escrow_amount: u64,
) -> Result<(), BanksClientError> {
    let instruction = perform_withdrawal(
        context,
        test_indexer,
        env,
        payer,
        old_lock_up_time,
        new_lock_up_time,
        escrow_amount,
    )
    .await;
    let event = create_and_send_transaction_with_event::<PublicTransactionEvent>(
        context,
        &[instruction],
        &payer.pubkey(),
        &[payer],
        None,
    )
    .await?;
    test_indexer.add_compressed_accounts_with_token_data(event.unwrap());
    Ok(())
}

pub async fn perform_withdrawal_failing(
    context: &mut ProgramTestContext,
    test_indexer: &mut TestIndexer,
    env: &EnvAccounts,
    payer: &Keypair,
    old_lock_up_time: u64,
    new_lock_up_time: u64,
    escrow_amount: u64,
) -> Result<BanksTransactionResultWithMetadata, BanksClientError> {
    let instruction = perform_withdrawal(
        context,
        test_indexer,
        env,
        payer,
        old_lock_up_time,
        new_lock_up_time,
        escrow_amount,
    )
    .await;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        context.get_new_latest_blockhash().await.unwrap(),
    );
    context
        .banks_client
        .process_transaction_with_metadata(transaction)
        .await
}
pub async fn perform_withdrawal(
    context: &mut ProgramTestContext,
    test_indexer: &mut TestIndexer,
    env: &EnvAccounts,
    payer: &Keypair,
    old_lock_up_time: u64,
    new_lock_up_time: u64,
    escrow_amount: u64,
) -> Instruction {
    let payer_pubkey = payer.pubkey();
    let compressed_escrow_pda = test_indexer
        .compressed_accounts
        .iter()
        .find(|x| x.compressed_account.owner == token_escrow::ID)
        .unwrap()
        .clone();
    println!("compressed_escrow_pda {:?}", compressed_escrow_pda);
    let token_owner_pda = get_token_owner_pda(&payer_pubkey).0;
    let token_escrow = test_indexer
        .token_compressed_accounts
        .iter()
        .find(|x| x.token_data.owner == token_owner_pda)
        .unwrap()
        .clone();
    let token_escrow_account = test_indexer.compressed_accounts[token_escrow.index].clone();
    let token_escrow_account_hash = token_escrow_account
        .compressed_account
        .hash(
            &env.merkle_tree_pubkey,
            &token_escrow_account.merkle_context.leaf_index,
        )
        .unwrap();
    println!("token_data_escrow {:?}", token_escrow);
    println!(
        "token escrow_account {:?}",
        test_indexer.compressed_accounts[token_escrow.index]
    );
    let compressed_pda_hash = compressed_escrow_pda
        .compressed_account
        .hash(
            &env.merkle_tree_pubkey,
            &compressed_escrow_pda.merkle_context.leaf_index,
        )
        .unwrap();
    println!("compressed_pda_hash {:?}", compressed_pda_hash);
    println!("token_escrow_account_hash {:?}", token_escrow_account_hash);
    // compressed pda will go first into the proof because in the program
    // the compressed pda program executes the transaction
    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(&[compressed_pda_hash, token_escrow_account_hash]),
            None,
            context,
        )
        .await;

    let create_withdrawal_ix_inputs = CreateCompressedPdaWithdrawalInstructionInputs {
        input_token_data: &vec![token_escrow.token_data],
        signer: &payer_pubkey,
        input_token_escrow_merkle_context: MerkleContext {
            leaf_index: token_escrow_account.merkle_context.leaf_index,
            merkle_tree_pubkey: env.merkle_tree_pubkey,
            nullifier_queue_pubkey: env.nullifier_queue_pubkey,
        },
        input_cpda_merkle_context: MerkleContext {
            leaf_index: compressed_escrow_pda.merkle_context.leaf_index,
            merkle_tree_pubkey: env.merkle_tree_pubkey,
            nullifier_queue_pubkey: env.nullifier_queue_pubkey,
        },
        output_compressed_account_merkle_tree_pubkeys: &[
            env.merkle_tree_pubkey,
            env.merkle_tree_pubkey,
        ],
        output_compressed_accounts: &Vec::new(),
        root_indices: &rpc_result.root_indices,
        proof: &rpc_result.proof,
        mint: &token_escrow.token_data.mint,
        cpi_signature_account: &env.cpi_signature_account_pubkey,
        old_lock_up_time,
        new_lock_up_time,
        address: compressed_escrow_pda.compressed_account.address.unwrap(),
    };
    create_withdrawal_instruction(create_withdrawal_ix_inputs.clone(), escrow_amount)
}

/// 1. Change escrow compressed account exists
/// 2. Withdrawal token account exists
/// 3. Compressed pda with update lock up time exists
pub async fn assert_withdrawal(
    context: &mut ProgramTestContext,
    test_indexer: &mut TestIndexer,
    env: &EnvAccounts,
    payer: &Keypair,
    withdrawal_amount: &u64,
    escrow_amount: &u64,
    seed: &[u8; 32],
    lock_up_time: u64,
) {
    let escrow_change_amount = escrow_amount - withdrawal_amount;

    let payer_pubkey = payer.pubkey();
    let token_owner_pda = get_token_owner_pda(&payer_pubkey).0;
    let token_data_escrow = test_indexer.token_compressed_accounts.iter().any(|x| {
        x.token_data.owner == token_owner_pda && x.token_data.amount == escrow_change_amount
    });

    assert!(
        token_data_escrow,
        "change escrow token account does not exist or has incorrect amount",
    );
    let withdrawal_account_exits = test_indexer
        .token_compressed_accounts
        .iter()
        .any(|x| x.token_data.owner == payer.pubkey() && x.token_data.amount == *withdrawal_amount);
    assert!(withdrawal_account_exits);

    let compressed_escrow_pda = test_indexer
        .compressed_accounts
        .iter()
        .find(|x| x.compressed_account.owner == token_escrow::ID)
        .unwrap()
        .clone();

    let address = derive_address(&env.address_merkle_tree_pubkey, &seed).unwrap();
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
    let current_slot = context.banks_client.get_root_slot().await.unwrap();
    assert_eq!(compressed_escrow_pda_data.slot, lock_up_time + current_slot);
    assert_eq!(
        compressed_escrow_pda_deserialized.discriminator,
        1u64.to_le_bytes(),
    );
    assert_eq!(
        compressed_escrow_pda_deserialized.data_hash,
        Poseidon::hash(&compressed_escrow_pda_data.slot.to_le_bytes()).unwrap(),
    );
}
