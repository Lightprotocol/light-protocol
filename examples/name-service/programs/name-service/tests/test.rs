#![cfg(feature = "test-sbf")]

// 1. create test env
// 2. generate a unique pda
// 3. create compressed-account with pda
// 4. failing test that tries to create a compressed account with the same pda
use light_hasher::Poseidon;
use light_system_program::sdk::{compressed_account::MerkleContext, event::PublicTransactionEvent};
use light_test_utils::airdrop_lamports;
use light_test_utils::indexer::{create_mint_helper, TestIndexer};
use light_test_utils::spl::mint_tokens_helper;
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
async fn test_create_pda() {
    let (mut rpc, env) = setup_test_programs_with_accounts(Some(vec![(
        String::from("token_escrow"),
        token_escrow::ID,
    )]))
    .await;
    let payer = rpc.get_payer().insecure_clone();

    let test_indexer = TestIndexer::init_from_env(
        &payer,
        &env,
        true,
        true,
        "../../../../circuit-lib/circuitlib-rs/scripts/prover.sh",
    );
    let mint = create_mint_helper(&mut rpc, &payer).await;
    let mut test_indexer = test_indexer.await;

    // TODO: add example for hash_and_truncate for multiple seeds
    let seed = [1u8; 32];
    let escrow_amount = 100u64;
    let lock_up_time = 1000u64;


    let address = derive_address(&env.address_merkle_tree_pubkey, &seed).unwrap();

    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(&[input_compressed_account_hash]),
            Some(&[compressed_input_account_with_context
                .merkle_context
                .merkle_tree_pubkey]),
            Some(&[address]),
            Some(vec![env.address_merkle_tree_pubkey]),
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
        input_token_data: &[input_compressed_token_account_data.token_data],
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
        proof: &Some(rpc_result.proof),
        mint: &input_compressed_token_account_data.token_data.mint,
        new_address_params,
        cpi_context_account: &env.cpi_context_account_pubkey,
    };

    let instruction = create_escrow_instruction(create_ix_inputs.clone(), escrow_amount);


}

pub async fn perform_escrow_failing<R: RpcConnection>(
    test_indexer: &mut TestIndexer<200, R>,
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

pub async fn send_tx_and_index<R: RpcConnection>(
    test_indexer: &mut TestIndexer<200, R>,
    rpc: &mut R,
    env: &EnvAccounts,
    payer: &Keypair,
    instruction: Instruction,
) -> Result<(), RpcError> {
  
    let event = rpc
        .create_and_send_transaction_with_event::<PublicTransactionEvent>(
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

    test_indexer.add_compressed_accounts(&event.unwrap());
    Ok(())
}

async fn create_esc_ix<R: RpcConnection>(
    payer: &Keypair,
    test_indexer: &mut TestIndexer<200, R>,
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
    let input_compressed_account_hash = compressed_input_account_with_context
        .compressed_account
        .hash::<Poseidon>(
            &env.merkle_tree_pubkey,
            &compressed_input_account_with_context
                .merkle_context
                .leaf_index,
        )
        .unwrap();

   
}

pub async fn assert_pda_created<R: RpcConnection>(
    test_indexer: &mut TestIndexer<200, R>,
    env: &EnvAccounts,
    payer: &Keypair,
    seed: &[u8; 32],
) {
    let payer_pubkey = payer.pubkey();
    let token_owner_pda = get_token_owner_pda(&payer_pubkey).0;
    let token_data_escrow = test_indexer
        .token_compressed_accounts
        .iter()
        .find(|x| x.token_data.owner == token_owner_pda)
        .unwrap()
        .token_data;
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
    let address = derive_address(&env.address_merkle_tree_pubkey, seed).unwrap();
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
