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

use light_client::indexer::Indexer;
use light_compressed_account::{compressed_account::MerkleContext, TreeType};
use light_program_test::{
    indexer::TestIndexerExtensions, program_test::TestRpc, utils::assert::assert_rpc_error,
    LightProgramTest, ProgramTestConfig,
};
use light_system_program::errors::SystemProgramError;
use light_test_utils::{
    airdrop_lamports,
    conversions::sdk_to_program_token_data,
    spl::{create_mint_helper, mint_tokens_helper},
    FeeConfig, Rpc, RpcError, TransactionParams,
};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};
use token_escrow::{
    escrow_with_compressed_pda::sdk::get_token_owner_pda,
    escrow_with_pda::sdk::{
        create_escrow_instruction, create_withdrawal_escrow_instruction, get_timelock_pda,
        CreateEscrowInstructionInputs,
    },
    EscrowError, EscrowTimeLock,
};

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
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new(
        true,
        Some(vec![("token_escrow", token_escrow::ID)]),
    ))
    .await
    .unwrap();
    let env = rpc.test_accounts.clone();
    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();
    let merkle_tree_pubkey = env.v1_state_trees[0].merkle_tree;
    let mint = create_mint_helper(&mut rpc, &payer).await;

    let amount = 10000u64;
    let mut test_indexer = rpc.indexer.as_ref().unwrap().clone();
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
    *rpc.indexer.as_mut().unwrap() = test_indexer;
    let escrow_amount = 100u64;
    let lockup_time = 0u64;
    perform_escrow_with_event(&mut rpc, &payer, &escrow_amount, &lockup_time)
        .await
        .unwrap();
    assert_escrow(&mut rpc, &payer_pubkey, amount, escrow_amount, &lockup_time).await;

    println!("withdrawal _----------------------------------------------------------------");
    let withdrawal_amount = 50u64;
    perform_withdrawal_with_event(&mut rpc, &payer, &withdrawal_amount, None)
        .await
        .unwrap();

    assert_withdrawal(&rpc, &payer_pubkey, withdrawal_amount, escrow_amount);

    let second_payer = Keypair::new();
    let second_payer_pubkey = second_payer.pubkey();
    println!("second payer pub key {:?}", second_payer_pubkey);
    let second_payer_token_balance = 1_000_000_000;
    airdrop_lamports(&mut rpc, &second_payer_pubkey, 1_000_000_000)
        .await
        .unwrap();
    let mut test_indexer = rpc.indexer.as_ref().unwrap().clone();
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
    *rpc.indexer.as_mut().unwrap() = test_indexer;

    let escrow_amount = 100u64;
    let lockup_time = 100u64;
    perform_escrow_with_event(&mut rpc, &second_payer, &escrow_amount, &lockup_time)
        .await
        .unwrap();

    assert_escrow(
        &mut rpc,
        &second_payer_pubkey,
        second_payer_token_balance,
        escrow_amount,
        &lockup_time,
    )
    .await;

    // try withdrawal before lockup time
    let withdrawal_amount = 50u64;
    let result =
        perform_withdrawal_failing(&mut rpc, &second_payer, &withdrawal_amount, None).await;

    assert_rpc_error(result, 0, EscrowError::EscrowLocked.into()).unwrap();

    rpc.warp_to_slot(1000).unwrap();
    // try withdrawal with invalid signer
    let result = perform_withdrawal_failing(
        &mut rpc,
        &second_payer,
        &withdrawal_amount,
        Some(payer_pubkey),
    )
    .await;

    assert_rpc_error(
        result,
        0,
        SystemProgramError::ProofVerificationFailed.into(),
    )
    .unwrap();

    perform_withdrawal_with_event(&mut rpc, &second_payer, &withdrawal_amount, None)
        .await
        .unwrap();
    assert_withdrawal(&rpc, &second_payer_pubkey, withdrawal_amount, escrow_amount);
}

pub async fn perform_escrow(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    escrow_amount: &u64,
    lock_up_time: &u64,
) -> Instruction {
    let env = rpc.test_accounts().clone();
    let input_compressed_token_account_data = rpc
        .get_token_compressed_accounts()
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
    let input_compressed_account_hash = compressed_input_account_with_context.hash().unwrap();

    let rpc_result = rpc
        .get_validity_proof(vec![input_compressed_account_hash], vec![], None)
        .await
        .unwrap();

    let create_ix_inputs = CreateEscrowInstructionInputs {
        input_token_data: &[sdk_to_program_token_data(
            input_compressed_token_account_data.token_data.clone(),
        )],
        lock_up_time: *lock_up_time,
        signer: &payer_pubkey,
        input_merkle_context: &[MerkleContext {
            leaf_index: compressed_input_account_with_context
                .merkle_context
                .leaf_index,
            merkle_tree_pubkey: env.v1_state_trees[0].merkle_tree.into(),
            queue_pubkey: env.v1_state_trees[0].nullifier_queue.into(),
            prove_by_index: false,
            tree_type: TreeType::StateV1,
        }],
        output_compressed_account_merkle_tree_pubkeys: &[
            env.v1_state_trees[0].merkle_tree,
            env.v1_state_trees[0].merkle_tree,
        ],
        output_compressed_accounts: &Vec::new(),
        root_indices: &rpc_result.value.get_root_indices(),
        proof: &rpc_result.value.proof.0,
        mint: &input_compressed_token_account_data.token_data.mint,
        input_compressed_accounts: &[compressed_input_account_with_context.compressed_account],
    };
    create_escrow_instruction(create_ix_inputs, *escrow_amount)
}

pub async fn perform_escrow_with_event(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    escrow_amount: &u64,
    lock_up_time: &u64,
) -> Result<(), RpcError> {
    let instruction = perform_escrow(rpc, payer, escrow_amount, lock_up_time).await;
    let rent = rpc
        .get_minimum_balance_for_rent_exemption(16)
        .await
        .unwrap();
    TestRpc::create_and_send_transaction_with_public_event(
        rpc,
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
    .await?;
    Ok(())
}

pub async fn perform_escrow_failing(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    escrow_amount: &u64,
    lock_up_time: &u64,
) -> Result<solana_sdk::signature::Signature, RpcError> {
    let instruction = perform_escrow(rpc, payer, escrow_amount, lock_up_time).await;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        rpc.get_latest_blockhash().await.unwrap().0,
    );
    rpc.process_transaction(transaction).await
}

pub async fn assert_escrow(
    rpc: &mut LightProgramTest,
    payer_pubkey: &Pubkey,
    amount: u64,
    escrow_amount: u64,
    lock_up_time: &u64,
) {
    let token_owner_pda = get_token_owner_pda(payer_pubkey).0;
    let token_data_escrow = rpc
        .get_token_compressed_accounts()
        .iter()
        .find(|x| x.token_data.owner == token_owner_pda)
        .unwrap()
        .token_data
        .clone();
    assert_eq!(token_data_escrow.amount, escrow_amount);
    assert_eq!(token_data_escrow.owner, token_owner_pda);

    let token_data_change_compressed_token_account =
        rpc.get_token_compressed_accounts()[0].token_data.clone();
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

pub async fn perform_withdrawal(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    withdrawal_amount: &u64,
    invalid_signer: Option<Pubkey>,
) -> Instruction {
    let env = rpc.test_accounts.clone();
    let payer_pubkey = payer.pubkey();
    let token_owner_pda = get_token_owner_pda(&invalid_signer.unwrap_or(payer_pubkey)).0;
    let escrow_token_data_with_context = rpc
        .get_token_compressed_accounts()
        .iter()
        .find(|x| {
            x.token_data.owner == token_owner_pda && x.token_data.amount >= *withdrawal_amount
        })
        .expect("no account with enough tokens")
        .clone();
    let compressed_input_account_with_context =
        escrow_token_data_with_context.compressed_account.clone();
    let input_compressed_account_hash = compressed_input_account_with_context.hash().unwrap();

    let rpc_result = rpc
        .get_validity_proof(vec![input_compressed_account_hash], vec![], None)
        .await
        .unwrap();

    let create_ix_inputs = CreateEscrowInstructionInputs {
        input_token_data: &[sdk_to_program_token_data(
            escrow_token_data_with_context.token_data.clone(),
        )],
        lock_up_time: 0,
        signer: &payer_pubkey,
        input_merkle_context: &[MerkleContext {
            leaf_index: compressed_input_account_with_context
                .merkle_context
                .leaf_index,
            merkle_tree_pubkey: env.v1_state_trees[0].merkle_tree.into(),
            queue_pubkey: env.v1_state_trees[0].nullifier_queue.into(),
            prove_by_index: false,
            tree_type: TreeType::StateV1,
        }],
        output_compressed_account_merkle_tree_pubkeys: &[
            env.v1_state_trees[0].merkle_tree,
            env.v1_state_trees[0].merkle_tree,
        ],
        output_compressed_accounts: &Vec::new(),
        root_indices: &rpc_result.value.get_root_indices(),
        proof: &rpc_result.value.proof.0,
        mint: &escrow_token_data_with_context.token_data.mint,
        input_compressed_accounts: &[compressed_input_account_with_context.compressed_account],
    };

    create_withdrawal_escrow_instruction(create_ix_inputs, *withdrawal_amount)
}

pub async fn perform_withdrawal_with_event(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    withdrawal_amount: &u64,
    invalid_signer: Option<Pubkey>,
) -> Result<Signature, RpcError> {
    let instruction = perform_withdrawal(rpc, payer, withdrawal_amount, invalid_signer).await;
    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}

pub async fn perform_withdrawal_failing(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    withdrawal_amount: &u64,
    invalid_signer: Option<Pubkey>,
) -> Result<solana_sdk::signature::Signature, RpcError> {
    let instruction = perform_withdrawal(rpc, payer, withdrawal_amount, invalid_signer).await;

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[&payer])
        .await
}
pub fn assert_withdrawal<I: Indexer + TestIndexerExtensions>(
    test_indexer: &I,
    payer_pubkey: &Pubkey,
    withdrawal_amount: u64,
    escrow_amount: u64,
) {
    let token_owner_pda = get_token_owner_pda(payer_pubkey).0;
    let token_data_withdrawal = test_indexer
        .get_token_compressed_accounts()
        .iter()
        .any(|x| x.token_data.owner == *payer_pubkey && x.token_data.amount == withdrawal_amount);

    assert!(
        token_data_withdrawal,
        "Withdrawal compressed account doesn't exist or has incorrect amount {} expected amount",
        withdrawal_amount
    );
    let token_data_escrow_change = test_indexer
        .get_token_compressed_accounts()
        .iter()
        .any(|x| {
            x.token_data.owner == token_owner_pda
                && x.token_data.amount == escrow_amount - withdrawal_amount
        });
    assert!(
        token_data_escrow_change,
        "Escrow change compressed account doesn't exist or has incorrect amount {} expected amount",
        escrow_amount - withdrawal_amount
    );
}
