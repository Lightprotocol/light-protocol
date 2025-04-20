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
// decompress compressed tokens into program owned token account - with compressed pda
// release compressed tokens

use anchor_lang::AnchorDeserialize;
use light_client::{indexer::Indexer, rpc::merkle_tree::MerkleTreeExt};
use light_compressed_account::{
    address::derive_address_legacy, compressed_account::MerkleContext,
    instruction_data::data::NewAddressParams, TreeType,
};
use light_hasher::{Hasher, Poseidon};
use light_program_test::{
    indexer::{TestIndexer, TestIndexerExtensions},
    test_env::{setup_test_programs_with_accounts, EnvAccounts},
};
use light_prover_client::gnark::helpers::{ProverConfig, ProverMode};
use light_test_utils::{
    conversions::sdk_to_program_token_data,
    spl::{create_mint_helper, mint_tokens_helper},
    FeeConfig, RpcConnection, RpcError, TransactionParams,
};
use solana_sdk::{
    instruction::{Instruction, InstructionError},
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};
use token_escrow::{
    escrow_with_compressed_pda::sdk::{
        create_escrow_instruction, create_withdrawal_instruction, get_token_owner_pda,
        CreateCompressedPdaEscrowInstructionInputs, CreateCompressedPdaWithdrawalInstructionInputs,
    },
    EscrowError, EscrowTimeLock,
};

#[tokio::test]
async fn test_escrow_with_compressed_pda() {
    let (mut rpc, env) = setup_test_programs_with_accounts(Some(vec![(
        String::from("token_escrow"),
        token_escrow::ID,
    )]))
    .await;
    let payer = rpc.get_payer().insecure_clone();

    let test_indexer = TestIndexer::init_from_env(
        &payer,
        &env,
        Some(ProverConfig {
            run_mode: Some(ProverMode::Rpc),
            circuits: vec![],
        }),
    );
    let mint = create_mint_helper(&mut rpc, &payer).await;
    let mut test_indexer = test_indexer.await;

    let amount = 10000u64;
    mint_tokens_helper(
        &mut rpc,
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
        &mut rpc,
        &env,
        &payer,
        lock_up_time,
        escrow_amount,
        seed,
    )
    .await
    .unwrap();

    let current_slot = rpc.get_slot().await.unwrap();
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
    let result = perform_withdrawal_failing(
        &mut rpc,
        &mut test_indexer,
        &env,
        &payer,
        lock_up_time,
        new_lock_up_time,
        withdrawal_amount,
    )
    .await;

    let instruction_error = InstructionError::Custom(EscrowError::EscrowLocked.into());
    let transaction_error =
        solana_sdk::transaction::TransactionError::InstructionError(0, instruction_error);
    let rpc_error = RpcError::TransactionError(transaction_error);
    assert!(matches!(result, Err(error) if error.to_string() == rpc_error.to_string()));

    rpc.warp_to_slot(lockup_end + 1).await.unwrap();
    perform_withdrawal_with_event(
        &mut rpc,
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
        &mut rpc,
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

pub async fn perform_escrow_failing<R: RpcConnection + MerkleTreeExt>(
    test_indexer: &mut TestIndexer<R>,
    rpc: &mut R,
    env: &EnvAccounts,
    payer: &Keypair,
    lock_up_time: u64,
    escrow_amount: u64,
    seed: [u8; 32],
) -> Result<solana_sdk::signature::Signature, RpcError> {
    let (payer_pubkey, instruction) = create_escrow_ix(
        payer,
        test_indexer,
        env,
        seed,
        rpc,
        lock_up_time,
        escrow_amount,
    )
    .await;
    let latest_blockhash = rpc.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_pubkey),
        &[&payer],
        latest_blockhash,
    );

    rpc.process_transaction(transaction).await
}

pub async fn perform_escrow_with_event<R: RpcConnection + MerkleTreeExt>(
    test_indexer: &mut TestIndexer<R>,
    rpc: &mut R,
    env: &EnvAccounts,
    payer: &Keypair,
    lock_up_time: u64,
    escrow_amount: u64,
    seed: [u8; 32],
) -> Result<(), RpcError> {
    let (_, instruction) = create_escrow_ix(
        payer,
        test_indexer,
        env,
        seed,
        rpc,
        lock_up_time,
        escrow_amount,
    )
    .await;
    let event = rpc
        .create_and_send_transaction_with_public_event(
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
    let slot = rpc.get_slot().await.unwrap();
    test_indexer.add_compressed_accounts_with_token_data(slot, &event.unwrap().0);
    Ok(())
}

async fn create_escrow_ix<R: RpcConnection + MerkleTreeExt>(
    payer: &Keypair,
    test_indexer: &mut TestIndexer<R>,
    env: &EnvAccounts,
    seed: [u8; 32],
    context: &mut R,
    lock_up_time: u64,
    escrow_amount: u64,
) -> (anchor_lang::prelude::Pubkey, Instruction) {
    let payer_pubkey = payer.pubkey();
    let input_compressed_token_account_data = test_indexer.token_compressed_accounts[0].clone();

    let compressed_input_account_with_context = input_compressed_token_account_data
        .compressed_account
        .clone();
    let input_compressed_account_hash = compressed_input_account_with_context.hash().unwrap();

    let address = derive_address_legacy(&env.address_merkle_tree_pubkey, &seed).unwrap();

    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(vec![input_compressed_account_hash]),
            Some(vec![
                compressed_input_account_with_context
                    .merkle_context
                    .merkle_tree_pubkey,
            ]),
            Some(&[address]),
            Some(vec![env.address_merkle_tree_pubkey]),
            context,
        )
        .await
        .unwrap();

    let new_address_params = NewAddressParams {
        seed,
        address_merkle_tree_pubkey: env.address_merkle_tree_pubkey,
        address_queue_pubkey: env.address_merkle_tree_queue_pubkey,
        address_merkle_tree_root_index: rpc_result.address_root_indices[0],
    };
    let create_ix_inputs = CreateCompressedPdaEscrowInstructionInputs {
        input_token_data: &[sdk_to_program_token_data(
            input_compressed_token_account_data.token_data.clone(),
        )],
        lock_up_time,
        signer: &payer_pubkey,
        input_merkle_context: &[MerkleContext {
            leaf_index: compressed_input_account_with_context
                .merkle_context
                .leaf_index,
            merkle_tree_pubkey: env.merkle_tree_pubkey,
            queue_pubkey: env.nullifier_queue_pubkey,
            prove_by_index: false,
            tree_type: TreeType::StateV1,
        }],
        output_compressed_account_merkle_tree_pubkeys: &[
            env.merkle_tree_pubkey,
            env.merkle_tree_pubkey,
        ],
        output_compressed_accounts: &Vec::new(),
        root_indices: &rpc_result.root_indices,
        proof: &Some(rpc_result.proof),
        mint: &input_compressed_token_account_data.token_data.mint,
        new_address_params,
        cpi_context_account: &env.cpi_context_account_pubkey,
        input_compressed_accounts: &[compressed_input_account_with_context.compressed_account],
    };
    let instruction = create_escrow_instruction(create_ix_inputs.clone(), escrow_amount);
    (payer_pubkey, instruction)
}

pub async fn assert_escrow<R: RpcConnection + MerkleTreeExt>(
    test_indexer: &mut TestIndexer<R>,
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
    println!(
        "test_indexer
        .compressed_accounts {:?}",
        test_indexer.compressed_accounts
    );

    let compressed_escrow_pda = test_indexer
        .compressed_accounts
        .iter()
        .find(|x| x.compressed_account.owner == token_escrow::ID)
        .unwrap()
        .clone();
    let address = derive_address_legacy(&env.address_merkle_tree_pubkey, seed).unwrap();

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
    let mut slot_bytes = [0u8; 32];
    slot_bytes[24..].copy_from_slice(&compressed_escrow_pda_data.slot.to_be_bytes());
    assert_eq!(
        compressed_escrow_pda_deserialized.data_hash,
        Poseidon::hash(&slot_bytes).unwrap(),
    );
}
pub async fn perform_withdrawal_with_event<R: RpcConnection + MerkleTreeExt>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    env: &EnvAccounts,
    payer: &Keypair,
    old_lock_up_time: u64,
    new_lock_up_time: u64,
    escrow_amount: u64,
) -> Result<(), RpcError> {
    let instruction = perform_withdrawal(
        rpc,
        test_indexer,
        env,
        payer,
        old_lock_up_time,
        new_lock_up_time,
        escrow_amount,
    )
    .await;
    let event = rpc
        .create_and_send_transaction_with_public_event(
            &[instruction],
            &payer.pubkey(),
            &[payer],
            None,
        )
        .await?;
    let slot = rpc.get_slot().await.unwrap();
    test_indexer.add_compressed_accounts_with_token_data(slot, &event.unwrap().0);
    Ok(())
}

pub async fn perform_withdrawal_failing<R: RpcConnection + MerkleTreeExt>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    env: &EnvAccounts,
    payer: &Keypair,
    old_lock_up_time: u64,
    new_lock_up_time: u64,
    escrow_amount: u64,
) -> Result<solana_sdk::signature::Signature, RpcError> {
    let instruction = perform_withdrawal(
        rpc,
        test_indexer,
        env,
        payer,
        old_lock_up_time,
        new_lock_up_time,
        escrow_amount,
    )
    .await;
    let latest_blockhash = rpc.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        latest_blockhash,
    );
    rpc.process_transaction(transaction).await
}
pub async fn perform_withdrawal<R: RpcConnection + MerkleTreeExt>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
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
    let token_escrow_account = token_escrow.compressed_account.clone();
    let token_escrow_account_hash = token_escrow_account.hash().unwrap();
    println!("token_data_escrow {:?}", token_escrow);
    println!("token escrow_account {:?}", token_escrow_account);
    let compressed_pda_hash = compressed_escrow_pda.hash().unwrap();
    println!("compressed_pda_hash {:?}", compressed_pda_hash);
    println!("token_escrow_account_hash {:?}", token_escrow_account_hash);
    // compressed pda will go first into the proof because in the program
    // the compressed pda program executes the transaction
    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(vec![compressed_pda_hash, token_escrow_account_hash]),
            Some(vec![
                compressed_escrow_pda.merkle_context.merkle_tree_pubkey,
                token_escrow_account.merkle_context.merkle_tree_pubkey,
            ]),
            None,
            None,
            rpc,
        )
        .await
        .unwrap();

    let create_withdrawal_ix_inputs = CreateCompressedPdaWithdrawalInstructionInputs {
        input_token_data: &[sdk_to_program_token_data(token_escrow.token_data.clone())],
        signer: &payer_pubkey,
        input_token_escrow_merkle_context: MerkleContext {
            leaf_index: token_escrow_account.merkle_context.leaf_index,
            merkle_tree_pubkey: env.merkle_tree_pubkey,
            queue_pubkey: env.nullifier_queue_pubkey,
            prove_by_index: false,
            tree_type: TreeType::StateV1,
        },

        input_cpda_merkle_context: MerkleContext {
            leaf_index: compressed_escrow_pda.merkle_context.leaf_index,
            merkle_tree_pubkey: env.merkle_tree_pubkey,
            queue_pubkey: env.nullifier_queue_pubkey,
            prove_by_index: false,
            tree_type: TreeType::StateV1,
        },
        output_compressed_account_merkle_tree_pubkeys: &[
            env.merkle_tree_pubkey,
            env.merkle_tree_pubkey,
        ],
        output_compressed_accounts: &Vec::new(),
        root_indices: &rpc_result.root_indices,
        proof: &Some(rpc_result.proof),
        mint: &token_escrow.token_data.mint,
        cpi_context_account: &env.cpi_context_account_pubkey,
        old_lock_up_time,
        new_lock_up_time,
        address: compressed_escrow_pda.compressed_account.address.unwrap(),
        input_compressed_accounts: &[compressed_escrow_pda.compressed_account],
    };
    create_withdrawal_instruction(create_withdrawal_ix_inputs.clone(), escrow_amount)
}

/// 1. Change escrow compressed account exists
/// 2. Withdrawal token account exists
/// 3. Compressed pda with update lock-up time exists
#[allow(clippy::too_many_arguments)]
pub async fn assert_withdrawal<R: RpcConnection + MerkleTreeExt>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
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

    let address = derive_address_legacy(&env.address_merkle_tree_pubkey, seed).unwrap();
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
    let current_slot = rpc.get_slot().await.unwrap();
    assert_eq!(compressed_escrow_pda_data.slot, lock_up_time + current_slot);
    assert_eq!(
        compressed_escrow_pda_deserialized.discriminator,
        1u64.to_le_bytes(),
    );
    let mut slot_bytes = [0u8; 32];
    slot_bytes[24..].copy_from_slice(&compressed_escrow_pda_data.slot.to_be_bytes());
    assert_eq!(
        compressed_escrow_pda_deserialized.data_hash,
        Poseidon::hash(&slot_bytes).unwrap(),
    );
}
