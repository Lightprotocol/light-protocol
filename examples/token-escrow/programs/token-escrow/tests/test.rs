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

use account_compression::Pubkey;
use light_test_utils::test_env::{setup_test_programs_with_accounts, EnvAccounts};
use light_test_utils::test_indexer::{create_mint_helper, mint_tokens_helper, TestIndexer};
use light_test_utils::{airdrop_lamports, get_account};

use solana_program_test::{
    BanksClientError, BanksTransactionResultWithMetadata, ProgramTestContext,
};
use solana_sdk::instruction::InstructionError;
use solana_sdk::signature::Keypair;
use solana_sdk::{signer::Signer, transaction::Transaction};
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
    let (mut context, env) = setup_test_programs_with_accounts(Some(vec![(
        String::from("token_escrow"),
        token_escrow::ID,
    )]))
    .await;
    let payer = context.payer.insecure_clone();
    let payer_pubkey = payer.pubkey();
    let merkle_tree_pubkey = env.merkle_tree_pubkey;
    let indexed_array_pubkey = env.indexed_array_pubkey;
    let address_merkle_tree_pubkey = env.address_merkle_tree_pubkey;
    let test_indexer = TestIndexer::new(
        merkle_tree_pubkey,
        indexed_array_pubkey,
        address_merkle_tree_pubkey,
        payer.insecure_clone(),
        true,
        false,
        false,
    );
    let mint = create_mint_helper(&mut context, &payer).await;
    let mut test_indexer = test_indexer.await;

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

    let escrow_amount = 100u64;
    let lockup_time = 0u64;
    let res = perform_escrow(
        &mut context,
        &mut test_indexer,
        &env,
        &payer,
        &escrow_amount,
        &lockup_time,
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

    assert_escrow(
        &mut context,
        &test_indexer,
        &payer_pubkey,
        amount,
        escrow_amount,
        &lockup_time,
    )
    .await;

    println!("withdrawal _----------------------------------------------------------------");
    let withdrawal_amount = 50u64;
    let res = perform_withdrawal(
        &mut context,
        &mut test_indexer,
        &env,
        &payer,
        &withdrawal_amount,
        None,
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
    airdrop_lamports(&mut context, &second_payer_pubkey, 1_000_000_000)
        .await
        .unwrap();
    mint_tokens_helper(
        &mut context,
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
    let res = perform_escrow(
        &mut context,
        &mut test_indexer,
        &env,
        &second_payer,
        &escrow_amount,
        &lockup_time,
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
    assert_escrow(
        &mut context,
        &test_indexer,
        &second_payer_pubkey,
        second_payer_token_balance,
        escrow_amount,
        &lockup_time,
    )
    .await;

    // try withdrawal before lockup time
    let withdrawal_amount = 50u64;
    let res = perform_withdrawal(
        &mut context,
        &mut test_indexer,
        &env,
        &second_payer,
        &withdrawal_amount,
        None,
    )
    .await;
    assert_eq!(
        res.unwrap().result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            0,
            InstructionError::Custom(EscrowError::EscrowLocked.into())
        ))
    );
    context.warp_to_slot(1000).unwrap();
    // try withdrawal with invalid signer
    let res = perform_withdrawal(
        &mut context,
        &mut test_indexer,
        &env,
        &second_payer,
        &withdrawal_amount,
        Some(payer_pubkey),
    )
    .await;
    assert_eq!(
        res.unwrap().result,
        Err(solana_sdk::transaction::TransactionError::InstructionError(
            0,
            InstructionError::Custom(psp_compressed_pda::ErrorCode::ProofVerificationFailed.into())
        ))
    );
    let res = perform_withdrawal(
        &mut context,
        &mut test_indexer,
        &env,
        &second_payer,
        &withdrawal_amount,
        None,
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
    assert_withdrawal(
        &test_indexer,
        &second_payer_pubkey,
        withdrawal_amount,
        escrow_amount,
    );
}

pub async fn perform_escrow(
    context: &mut ProgramTestContext,
    test_indexer: &mut TestIndexer,
    env: &EnvAccounts,
    payer: &Keypair,
    escrow_amount: &u64,
    lock_up_time: &u64,
) -> Result<BanksTransactionResultWithMetadata, BanksClientError> {
    let input_compressed_token_account_data = test_indexer
        .token_compressed_accounts
        .iter()
        .find(|x| {
            println!("searching token account: {:?}", x.token_data);
            println!("escrow amount: {:?}", escrow_amount);
            println!("payer pub key: {:?}", payer.pubkey());
            return x.token_data.owner == payer.pubkey() && x.token_data.amount >= *escrow_amount;
        })
        .expect("no account with enough tokens")
        .clone();
    let payer_pubkey = payer.pubkey();
    let compressed_input_account_with_context =
        test_indexer.compressed_accounts[input_compressed_token_account_data.index].clone();
    let input_compressed_account_hash = test_indexer.compressed_accounts
        [input_compressed_token_account_data.index]
        .compressed_account
        .hash(
            &env.merkle_tree_pubkey,
            &compressed_input_account_with_context.leaf_index,
        )
        .unwrap();

    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(Some(&[input_compressed_account_hash]), None, context)
        .await;

    let create_ix_inputs = CreateEscrowInstructionInputs {
        input_token_data: &vec![input_compressed_token_account_data.token_data],
        lock_up_time: *lock_up_time,
        signer: &payer_pubkey,
        input_compressed_account_merkle_tree_pubkeys: &[env.merkle_tree_pubkey],
        nullifier_array_pubkeys: &[env.indexed_array_pubkey],
        output_compressed_account_merkle_tree_pubkeys: &[
            env.merkle_tree_pubkey,
            env.merkle_tree_pubkey,
        ],
        output_compressed_accounts: &Vec::new(),
        root_indices: &rpc_result.root_indices,
        proof: &rpc_result.proof,
        leaf_indices: &[compressed_input_account_with_context.leaf_index],
        mint: &input_compressed_token_account_data.token_data.mint,
    };
    let instruction = create_escrow_instruction(create_ix_inputs.clone(), *escrow_amount);
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

pub async fn assert_escrow(
    context: &mut ProgramTestContext,
    test_indexer: &TestIndexer,
    payer_pubkey: &Pubkey,
    amount: u64,
    escrow_amount: u64,
    lock_up_time: &u64,
) {
    let token_owner_pda = get_token_owner_pda(&payer_pubkey).0;
    let token_data_escrow = test_indexer
        .token_compressed_accounts
        .iter()
        .find(|x| x.token_data.owner == token_owner_pda)
        .unwrap()
        .token_data
        .clone();
    assert_eq!(token_data_escrow.amount, escrow_amount);
    assert_eq!(token_data_escrow.owner, token_owner_pda);

    let token_data_change_compressed_token_account = test_indexer.token_compressed_accounts
        [test_indexer.token_compressed_accounts.len() - 1]
        .token_data
        .clone();
    assert_eq!(
        token_data_change_compressed_token_account.amount,
        amount - escrow_amount
    );
    assert_eq!(
        token_data_change_compressed_token_account.owner,
        *payer_pubkey
    );
    let time_lock_pubkey = get_timelock_pda(&payer_pubkey);
    let timelock_account = get_account::<EscrowTimeLock>(context, time_lock_pubkey).await;
    let current_slot = context.banks_client.get_root_slot().await.unwrap();
    assert_eq!(timelock_account.slot, *lock_up_time + current_slot);
}

pub async fn perform_withdrawal(
    context: &mut ProgramTestContext,
    test_indexer: &mut TestIndexer,
    env: &EnvAccounts,
    payer: &Keypair,
    withdrawal_amount: &u64,
    invalid_signer: Option<Pubkey>,
) -> Result<BanksTransactionResultWithMetadata, BanksClientError> {
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
        test_indexer.compressed_accounts[escrow_token_data_with_context.index].clone();
    let input_compressed_account_hash = test_indexer.compressed_accounts
        [escrow_token_data_with_context.index]
        .compressed_account
        .hash(
            &env.merkle_tree_pubkey,
            &compressed_input_account_with_context.leaf_index,
        )
        .unwrap();

    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(Some(&[input_compressed_account_hash]), None, context)
        .await;

    let create_ix_inputs = CreateEscrowInstructionInputs {
        input_token_data: &vec![escrow_token_data_with_context.token_data],
        lock_up_time: 0,
        signer: &payer_pubkey,
        input_compressed_account_merkle_tree_pubkeys: &[env.merkle_tree_pubkey],
        nullifier_array_pubkeys: &[env.indexed_array_pubkey],
        output_compressed_account_merkle_tree_pubkeys: &[
            env.merkle_tree_pubkey,
            env.merkle_tree_pubkey,
        ],
        output_compressed_accounts: &Vec::new(),
        root_indices: &rpc_result.root_indices,
        proof: &rpc_result.proof,
        leaf_indices: &[compressed_input_account_with_context.leaf_index],
        mint: &escrow_token_data_with_context.token_data.mint,
    };

    let instruction = create_withdrawal_escrow_instruction(create_ix_inputs, *withdrawal_amount);
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
pub fn assert_withdrawal(
    test_indexer: &TestIndexer,
    payer_pubkey: &Pubkey,
    withdrawal_amount: u64,
    escrow_amount: u64,
) {
    let token_owner_pda = get_token_owner_pda(&payer_pubkey).0;
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
