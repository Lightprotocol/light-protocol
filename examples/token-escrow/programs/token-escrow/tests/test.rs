#![cfg(feature = "test-sbf")]

// TODO: extend this example with a swap function
// TODO: implement a version with delegate and approve
// 1. escrow tokens with pda
// create test env
// create mint and mint tokens
// escrow compressed tokens - with normal pda
// - transfer tokens to compressed token account owned by pda
// - create escrow pda and just prove that utxo exists -> read utxo from compressed token account
// release compressed tokens

use light_hasher::Poseidon;
use light_system_program::sdk::{compressed_account::MerkleContext, event::PublicTransactionEvent};
use light_test_utils::airdrop_lamports;
use light_test_utils::indexer::{Indexer, TestIndexer};
use light_test_utils::spl::{create_mint_helper, mint_tokens_helper};
use light_test_utils::test_env::{setup_test_programs_with_accounts, EnvAccounts};

use light_test_utils::rpc::errors::{assert_rpc_error, RpcError};
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::transaction_params::{FeeConfig, TransactionParams};
use light_verifier::VerifierError;
use solana_sdk::instruction::Instruction;
use solana_sdk::signature::Keypair;
use solana_sdk::{pubkey::Pubkey, signer::Signer, transaction::Transaction};
use token_escrow::escrow_with_compressed_pda::sdk::get_token_owner_pda;
use token_escrow::escrow_with_pda::sdk::{
    create_escrow_instruction, create_withdrawal_escrow_instruction, get_timelock_pda,
    CreateEscrowInstructionInputs,
};
use token_escrow::{EscrowError, EscrowTimeLock};

/// Tests:
/// 1. create test env
/// 2. create mint and mint tokens
/// 3. escrow compressed tokens
/// 4. withdraw compressed tokens
/// 5. mint tokens to second payer
/// 6. escrow compressed tokens with lockup time
/// 7. try to withdraw before lockup time
/// 8. try to withdraw with invalid signer
/// 9. withdraw after lockup time
#[tokio::test]
async fn test_escrow_pda() {
    let (mut rpc, env) = setup_test_programs_with_accounts(Some(vec![(
        String::from("token_escrow"),
        token_escrow::ID,
    )]))
    .await;
    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let test_indexer = TestIndexer::init_from_env(&payer, &env, true, false);
    let mint = create_mint_helper(&mut rpc, &payer).await;
    let mut test_indexer = test_indexer.await;

    let amount = 10000u64;
    mint_tokens_helper(
        &mut rpc,
        &mut test_indexer,
        &merkle_tree_pubkey,
        &payer,
        &mint,
        vec![amount],
        vec![payer.pubkey()],
    )
    .await;
    let escrow_amount = 100u64;
    let lockup_time = 0u64;
    perform_escrow_with_event(
        &mut rpc,
        &mut test_indexer,
        &env,
        &payer,
        &escrow_amount,
        &lockup_time,
    )
    .await
    .unwrap();
    assert_escrow(
        &mut rpc,
        &test_indexer,
        &payer_pubkey,
        amount,
        escrow_amount,
        &lockup_time,
    )
    .await;

    println!("withdrawal _----------------------------------------------------------------");
    let withdrawal_amount = 50u64;
    perform_withdrawal_with_event(
        &mut rpc,
        &mut test_indexer,
        &env,
        &payer,
        &withdrawal_amount,
        None,
    )
    .await
    .unwrap();

    assert_withdrawal(
        &test_indexer,
        &payer_pubkey,
        withdrawal_amount,
        escrow_amount,
    );

    let second_payer = Keypair::new();
    let second_payer_pubkey = second_payer.pubkey();
    println!("second payer pub key {:?}", second_payer_pubkey);
    let second_payer_token_balance = 1_000_000_000;
    airdrop_lamports(&mut rpc, &second_payer_pubkey, 1_000_000_000)
        .await
        .unwrap();
    mint_tokens_helper(
        &mut rpc,
        &mut test_indexer,
        &merkle_tree_pubkey,
        &payer,
        &mint,
        vec![second_payer_token_balance],
        vec![second_payer_pubkey],
    )
    .await;

    let escrow_amount = 100u64;
    let lockup_time = 100u64;
    perform_escrow_with_event(
        &mut rpc,
        &mut test_indexer,
        &env,
        &second_payer,
        &escrow_amount,
        &lockup_time,
    )
    .await
    .unwrap();

    assert_escrow(
        &mut rpc,
        &test_indexer,
        &second_payer_pubkey,
        second_payer_token_balance,
        escrow_amount,
        &lockup_time,
    )
    .await;

    // try withdrawal before lockup time
    let withdrawal_amount = 50u64;
    let result = perform_withdrawal_failing(
        &mut rpc,
        &mut test_indexer,
        &env,
        &second_payer,
        &withdrawal_amount,
        None,
    )
    .await;

    assert_rpc_error(result, 0, EscrowError::EscrowLocked.into()).unwrap();

    rpc.warp_to_slot(1000).unwrap();
    // try withdrawal with invalid signer
    let result = perform_withdrawal_failing(
        &mut rpc,
        &mut test_indexer,
        &env,
        &second_payer,
        &withdrawal_amount,
        Some(payer_pubkey),
    )
    .await;

    assert_rpc_error(result, 0, VerifierError::ProofVerificationFailed.into()).unwrap();

    perform_withdrawal_with_event(
        &mut rpc,
        &mut test_indexer,
        &env,
        &second_payer,
        &withdrawal_amount,
        None,
    )
    .await
    .unwrap();
    assert_withdrawal(
        &test_indexer,
        &second_payer_pubkey,
        withdrawal_amount,
        escrow_amount,
    );
}

pub async fn perform_escrow<R: RpcConnection>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    env: &EnvAccounts,
    payer: &Keypair,
    escrow_amount: &u64,
    lock_up_time: &u64,
) -> Instruction {
    let input_compressed_token_account_data = test_indexer
        .token_compressed_accounts
        .iter()
        .find(|x| {
            println!("searching token account: {:?}", x.token_data);
            println!("escrow amount: {:?}", escrow_amount);
            println!("payer pub key: {:?}", payer.pubkey());
            x.token_data.owner == payer.pubkey() && x.token_data.amount >= *escrow_amount
        })
        .expect("no account with enough tokens")
        .clone();
    let payer_pubkey = payer.pubkey();
    let compressed_input_account_with_context = input_compressed_token_account_data
        .compressed_account
        .clone();
    let input_compressed_account_hash = compressed_input_account_with_context
        .compressed_account
        .hash::<Poseidon>(
            &env.merkle_tree_pubkey,
            &compressed_input_account_with_context
                .merkle_context
                .leaf_index,
        )
        .unwrap();

    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(&[input_compressed_account_hash]),
            Some(&[compressed_input_account_with_context
                .merkle_context
                .merkle_tree_pubkey]),
            None,
            None,
            rpc,
        )
        .await;

    let create_ix_inputs = CreateEscrowInstructionInputs {
        input_token_data: &[input_compressed_token_account_data.token_data.clone()],
        lock_up_time: *lock_up_time,
        signer: &payer_pubkey,
        input_merkle_context: &[MerkleContext {
            leaf_index: compressed_input_account_with_context
                .merkle_context
                .leaf_index,
            merkle_tree_pubkey: env.merkle_tree_pubkey,
            nullifier_queue_pubkey: env.nullifier_queue_pubkey,
            queue_index: None,
        }],
        output_compressed_account_merkle_tree_pubkeys: &[
            env.merkle_tree_pubkey,
            env.merkle_tree_pubkey,
        ],
        output_compressed_accounts: &Vec::new(),
        root_indices: &rpc_result.root_indices,
        proof: &Some(rpc_result.proof),
        mint: &input_compressed_token_account_data.token_data.mint,
        input_compressed_accounts: &[compressed_input_account_with_context.compressed_account],
    };
    create_escrow_instruction(create_ix_inputs, *escrow_amount)
}

pub async fn perform_escrow_with_event<R: RpcConnection>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    env: &EnvAccounts,
    payer: &Keypair,
    escrow_amount: &u64,
    lock_up_time: &u64,
) -> Result<(), RpcError> {
    let instruction =
        perform_escrow(rpc, test_indexer, env, payer, escrow_amount, lock_up_time).await;
    let rent = rpc
        .get_minimum_balance_for_rent_exemption(16)
        .await
        .unwrap();
    let event = rpc
        .create_and_send_transaction_with_event::<PublicTransactionEvent>(
            &[instruction],
            &payer.pubkey(),
            &[payer],
            Some(TransactionParams {
                num_input_compressed_accounts: 1,
                num_output_compressed_accounts: 2,
                num_new_addresses: 0,
                compress: rent as i64,
                fee_config: FeeConfig::default(),
            }),
        )
        .await?
        .unwrap();
    test_indexer.add_compressed_accounts_with_token_data(&event.0);
    Ok(())
}

pub async fn perform_escrow_failing<R: RpcConnection>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    env: &EnvAccounts,
    payer: &Keypair,
    escrow_amount: &u64,
    lock_up_time: &u64,
) -> Result<solana_sdk::signature::Signature, RpcError> {
    let instruction =
        perform_escrow(rpc, test_indexer, env, payer, escrow_amount, lock_up_time).await;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        rpc.get_latest_blockhash().await.unwrap(),
    );
    rpc.process_transaction(transaction).await
}

pub async fn assert_escrow<R: RpcConnection>(
    rpc: &mut R,
    test_indexer: &TestIndexer<R>,
    payer_pubkey: &Pubkey,
    amount: u64,
    escrow_amount: u64,
    lock_up_time: &u64,
) {
    let token_owner_pda = get_token_owner_pda(payer_pubkey).0;
    let token_data_escrow = test_indexer
        .token_compressed_accounts
        .iter()
        .find(|x| x.token_data.owner == token_owner_pda)
        .unwrap()
        .token_data
        .clone();
    assert_eq!(token_data_escrow.amount, escrow_amount);
    assert_eq!(token_data_escrow.owner, token_owner_pda);

    let token_data_change_compressed_token_account =
        test_indexer.token_compressed_accounts[0].token_data.clone();
    assert_eq!(
        token_data_change_compressed_token_account.amount,
        amount - escrow_amount
    );
    assert_eq!(
        token_data_change_compressed_token_account.owner,
        *payer_pubkey
    );
    let time_lock_pubkey = get_timelock_pda(payer_pubkey);
    let timelock_account = rpc
        .get_anchor_account::<EscrowTimeLock>(&time_lock_pubkey)
        .await
        .unwrap()
        .unwrap();
    let current_slot = rpc.get_slot().await.unwrap();
    assert_eq!(timelock_account.slot, *lock_up_time + current_slot);
}

pub async fn perform_withdrawal<R: RpcConnection>(
    context: &mut R,
    test_indexer: &mut TestIndexer<R>,
    env: &EnvAccounts,
    payer: &Keypair,
    withdrawal_amount: &u64,
    invalid_signer: Option<Pubkey>,
) -> Instruction {
    let payer_pubkey = payer.pubkey();
    let token_owner_pda = get_token_owner_pda(&invalid_signer.unwrap_or(payer_pubkey)).0;
    let escrow_token_data_with_context = test_indexer
        .token_compressed_accounts
        .iter()
        .find(|x| {
            x.token_data.owner == token_owner_pda && x.token_data.amount >= *withdrawal_amount
        })
        .expect("no account with enough tokens")
        .clone();
    let compressed_input_account_with_context =
        escrow_token_data_with_context.compressed_account.clone();
    let input_compressed_account_hash = compressed_input_account_with_context
        .compressed_account
        .hash::<Poseidon>(
            &env.merkle_tree_pubkey,
            &compressed_input_account_with_context
                .merkle_context
                .leaf_index,
        )
        .unwrap();

    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(&[input_compressed_account_hash]),
            Some(&[compressed_input_account_with_context
                .merkle_context
                .merkle_tree_pubkey]),
            None,
            None,
            context,
        )
        .await;

    let create_ix_inputs = CreateEscrowInstructionInputs {
        input_token_data: &[escrow_token_data_with_context.token_data.clone()],
        lock_up_time: 0,
        signer: &payer_pubkey,
        input_merkle_context: &[MerkleContext {
            leaf_index: compressed_input_account_with_context
                .merkle_context
                .leaf_index,
            merkle_tree_pubkey: env.merkle_tree_pubkey,
            nullifier_queue_pubkey: env.nullifier_queue_pubkey,
            queue_index: None,
        }],
        output_compressed_account_merkle_tree_pubkeys: &[
            env.merkle_tree_pubkey,
            env.merkle_tree_pubkey,
        ],
        output_compressed_accounts: &Vec::new(),
        root_indices: &rpc_result.root_indices,
        proof: &Some(rpc_result.proof),
        mint: &escrow_token_data_with_context.token_data.mint,
        input_compressed_accounts: &[compressed_input_account_with_context.compressed_account],
    };

    create_withdrawal_escrow_instruction(create_ix_inputs, *withdrawal_amount)
}

pub async fn perform_withdrawal_with_event<R: RpcConnection>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    env: &EnvAccounts,
    payer: &Keypair,
    withdrawal_amount: &u64,
    invalid_signer: Option<Pubkey>,
) -> Result<(), RpcError> {
    let instruction = perform_withdrawal(
        rpc,
        test_indexer,
        env,
        payer,
        withdrawal_amount,
        invalid_signer,
    )
    .await;
    let event = rpc
        .create_and_send_transaction_with_event::<PublicTransactionEvent>(
            &[instruction],
            &payer.pubkey(),
            &[payer],
            None,
        )
        .await?
        .unwrap();
    test_indexer.add_compressed_accounts_with_token_data(&event.0);
    Ok(())
}

pub async fn perform_withdrawal_failing<R: RpcConnection>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    env: &EnvAccounts,
    payer: &Keypair,
    withdrawal_amount: &u64,
    invalid_signer: Option<Pubkey>,
) -> Result<solana_sdk::signature::Signature, RpcError> {
    let instruction = perform_withdrawal(
        rpc,
        test_indexer,
        env,
        payer,
        withdrawal_amount,
        invalid_signer,
    )
    .await;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        rpc.get_latest_blockhash().await.unwrap(),
    );
    rpc.process_transaction(transaction).await
}
pub fn assert_withdrawal<R: RpcConnection>(
    test_indexer: &TestIndexer<R>,
    payer_pubkey: &Pubkey,
    withdrawal_amount: u64,
    escrow_amount: u64,
) {
    let token_owner_pda = get_token_owner_pda(payer_pubkey).0;
    let token_data_withdrawal = test_indexer
        .token_compressed_accounts
        .iter()
        .any(|x| x.token_data.owner == *payer_pubkey && x.token_data.amount == withdrawal_amount);

    assert!(
        token_data_withdrawal,
        "Withdrawal compressed account doesn't exist or has incorrect amount {} expected amount",
        withdrawal_amount
    );
    let token_data_escrow_change = test_indexer.token_compressed_accounts.iter().any(|x| {
        x.token_data.owner == token_owner_pda
            && x.token_data.amount == escrow_amount - withdrawal_amount
    });
    assert!(
        token_data_escrow_change,
        "Escrow change compressed account doesn't exist or has incorrect amount {} expected amount",
        escrow_amount - withdrawal_amount
    );
}
