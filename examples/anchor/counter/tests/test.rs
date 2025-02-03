#![cfg(feature = "test-sbf")]

use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use counter::CounterAccount;
use light_client::{
    indexer::{AddressMerkleTreeAccounts, Indexer, StateMerkleTreeAccounts},
    rpc::merkle_tree::MerkleTreeExt,
};
use light_program_test::{
    indexer::{TestIndexer, TestIndexerExtensions},
    test_env::{setup_test_programs_with_accounts_v2, EnvAccounts},
    test_rpc::ProgramTestRpcConnection,
};
use light_prover_client::gnark::helpers::{spawn_prover, ProverConfig, ProverMode};
use light_sdk::{
    account_meta::LightAccountMeta,
    address::derive_address,
    instruction_data::LightInstructionData,
    merkle_context::{AddressMerkleContext, RemainingAccounts},
    utils::get_cpi_authority_pda,
    verify::find_cpi_signer,
    PROGRAM_ID_ACCOUNT_COMPRESSION, PROGRAM_ID_LIGHT_SYSTEM, PROGRAM_ID_NOOP,
};
use light_test_utils::{RpcConnection, RpcError};
use light_utils::instruction::compressed_account::CompressedAccountWithMerkleContext;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};

#[tokio::test]
async fn test_counter() {
    spawn_prover(
        true,
        ProverConfig {
            run_mode: Some(ProverMode::Rpc),
            circuits: vec![],
        },
    )
    .await;

    let (mut rpc, env) =
        setup_test_programs_with_accounts_v2(Some(vec![(String::from("counter"), counter::ID)]))
            .await;
    let payer = rpc.get_payer().insecure_clone();

    let mut test_indexer: TestIndexer<ProgramTestRpcConnection> = TestIndexer::new(
        Vec::from(&[StateMerkleTreeAccounts {
            merkle_tree: env.merkle_tree_pubkey,
            nullifier_queue: env.nullifier_queue_pubkey,
            cpi_context: env.cpi_context_account_pubkey,
        }]),
        Vec::from(&[AddressMerkleTreeAccounts {
            merkle_tree: env.address_merkle_tree_pubkey,
            queue: env.address_merkle_tree_queue_pubkey,
        }]),
        payer.insecure_clone(),
        env.group_pda,
        None,
    )
    .await;

    let mut remaining_accounts = RemainingAccounts::default();

    let address_merkle_context = AddressMerkleContext {
        address_merkle_tree_pubkey: env.address_merkle_tree_pubkey,
        address_queue_pubkey: env.address_merkle_tree_queue_pubkey,
    };

    let (address, _) = derive_address(
        &[b"counter", payer.pubkey().as_ref()],
        &address_merkle_context,
        &counter::ID,
    );

    let account_compression_authority = get_cpi_authority_pda(&PROGRAM_ID_LIGHT_SYSTEM);
    let registered_program_pda = Pubkey::find_program_address(
        &[PROGRAM_ID_LIGHT_SYSTEM.to_bytes().as_slice()],
        &PROGRAM_ID_ACCOUNT_COMPRESSION,
    )
    .0;

    // Create the counter.
    create_counter(
        &mut rpc,
        &mut test_indexer,
        &env,
        &mut remaining_accounts,
        &payer,
        &address,
        &account_compression_authority,
        &registered_program_pda,
        &PROGRAM_ID_LIGHT_SYSTEM,
    )
    .await
    .unwrap();

    // Check that it was created correctly.
    let compressed_accounts = test_indexer
        .get_compressed_accounts_by_owner(&counter::ID)
        .await
        .unwrap();
    assert_eq!(compressed_accounts.len(), 1);
    let compressed_account = &compressed_accounts[0];
    let counter = &compressed_account
        .compressed_account
        .data
        .as_ref()
        .unwrap()
        .data;
    let counter = CounterAccount::deserialize(&mut &counter[..]).unwrap();
    assert_eq!(counter.value, 0);

    // Increment the counter.
    increment_counter(
        &mut rpc,
        &mut test_indexer,
        &mut remaining_accounts,
        &payer,
        compressed_account,
        &account_compression_authority,
        &registered_program_pda,
        &PROGRAM_ID_LIGHT_SYSTEM,
    )
    .await
    .unwrap();

    // Check that it was incremented correctly.
    let compressed_accounts = test_indexer
        .get_compressed_accounts_by_owner(&counter::ID)
        .await
        .unwrap();
    assert_eq!(compressed_accounts.len(), 1);
    let compressed_account = &compressed_accounts[0];
    let counter = &compressed_account
        .compressed_account
        .data
        .as_ref()
        .unwrap()
        .data;
    let counter = CounterAccount::deserialize(&mut &counter[..]).unwrap();
    assert_eq!(counter.value, 1);

    // Decrement the counter.
    decrement_counter(
        &mut rpc,
        &mut test_indexer,
        &mut remaining_accounts,
        &payer,
        compressed_account,
        &account_compression_authority,
        &registered_program_pda,
        &PROGRAM_ID_LIGHT_SYSTEM,
    )
    .await
    .unwrap();

    // Check that it was decremented correctly.
    let compressed_accounts = test_indexer
        .get_compressed_accounts_by_owner(&counter::ID)
        .await
        .unwrap();
    assert_eq!(compressed_accounts.len(), 1);
    let compressed_account = &compressed_accounts[0];
    let counter = &compressed_account
        .compressed_account
        .data
        .as_ref()
        .unwrap()
        .data;
    let counter = CounterAccount::deserialize(&mut &counter[..]).unwrap();
    assert_eq!(counter.value, 0);

    // Reset the counter.
    reset_counter(
        &mut rpc,
        &mut test_indexer,
        &mut remaining_accounts,
        &payer,
        compressed_account,
        &account_compression_authority,
        &registered_program_pda,
        &PROGRAM_ID_LIGHT_SYSTEM,
    )
    .await
    .unwrap();

    // Check that it was reset correctly.
    let compressed_accounts = test_indexer
        .get_compressed_accounts_by_owner(&counter::ID)
        .await
        .unwrap();
    assert_eq!(compressed_accounts.len(), 1);
    let compressed_account = &compressed_accounts[0];
    let counter = &compressed_account
        .compressed_account
        .data
        .as_ref()
        .unwrap()
        .data;
    let counter = CounterAccount::deserialize(&mut &counter[..]).unwrap();
    assert_eq!(counter.value, 0);
}

#[allow(clippy::too_many_arguments)]
async fn create_counter<R>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    env: &EnvAccounts,
    remaining_accounts: &mut RemainingAccounts,
    payer: &Keypair,
    address: &[u8; 32],
    account_compression_authority: &Pubkey,
    registered_program_pda: &Pubkey,
    light_system_program: &Pubkey,
) -> Result<(), RpcError>
where
    R: RpcConnection + MerkleTreeExt,
{
    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            None,
            None,
            Some(&[*address]),
            Some(vec![env.address_merkle_tree_pubkey]),
            rpc,
        )
        .await;

    let address_merkle_context = AddressMerkleContext {
        address_merkle_tree_pubkey: env.address_merkle_tree_pubkey,
        address_queue_pubkey: env.address_merkle_tree_queue_pubkey,
    };
    let account = LightAccountMeta::new_init(
        &env.merkle_tree_pubkey,
        Some(&address_merkle_context),
        Some(rpc_result.address_root_indices[0]),
        remaining_accounts,
    )
    .unwrap();

    let inputs = LightInstructionData {
        proof: Some(rpc_result),
        accounts: Some(vec![account]),
    };
    let inputs = inputs.serialize().unwrap();
    let instruction_data = counter::instruction::CreateCounter { inputs };

    let cpi_signer = find_cpi_signer(&counter::ID);

    let accounts = counter::accounts::CreateCounter {
        signer: payer.pubkey(),
        light_system_program: *light_system_program,
        account_compression_program: PROGRAM_ID_ACCOUNT_COMPRESSION,
        account_compression_authority: *account_compression_authority,
        registered_program_pda: *registered_program_pda,
        noop_program: PROGRAM_ID_NOOP,
        self_program: counter::ID,
        cpi_signer,
        system_program: solana_sdk::system_program::id(),
    };

    let remaining_accounts = remaining_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: counter::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    };

    let event = rpc
        .create_and_send_transaction_with_event(&[instruction], &payer.pubkey(), &[payer], None)
        .await?;
    let slot = rpc.get_slot().await.unwrap();
    test_indexer.add_compressed_accounts_with_token_data(slot, &event.unwrap().0);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn increment_counter<R>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    remaining_accounts: &mut RemainingAccounts,
    payer: &Keypair,
    compressed_account: &CompressedAccountWithMerkleContext,
    account_compression_authority: &Pubkey,
    registered_program_pda: &Pubkey,
    light_system_program: &Pubkey,
) -> Result<(), RpcError>
where
    R: RpcConnection + MerkleTreeExt,
{
    let hash = compressed_account.hash().unwrap();
    let merkle_tree_pubkey = compressed_account.merkle_context.merkle_tree_pubkey;

    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(Vec::from(&[hash])),
            Some(Vec::from(&[merkle_tree_pubkey])),
            None,
            None,
            rpc,
        )
        .await;

    let compressed_account = LightAccountMeta::new_mut(
        compressed_account,
        rpc_result.root_indices[0].unwrap(),
        &merkle_tree_pubkey,
        remaining_accounts,
    );

    let inputs = LightInstructionData {
        proof: Some(rpc_result),
        accounts: Some(vec![compressed_account]),
    };
    let inputs = inputs.serialize().unwrap();
    let instruction_data = counter::instruction::IncrementCounter { inputs };

    let cpi_signer = find_cpi_signer(&counter::ID);

    let accounts = counter::accounts::UpdateCounter {
        signer: payer.pubkey(),
        light_system_program: *light_system_program,
        account_compression_program: PROGRAM_ID_ACCOUNT_COMPRESSION,
        account_compression_authority: *account_compression_authority,
        registered_program_pda: *registered_program_pda,
        noop_program: PROGRAM_ID_NOOP,
        self_program: counter::ID,
        cpi_signer,
        system_program: solana_sdk::system_program::id(),
    };

    let remaining_accounts = remaining_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: counter::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    };

    let event = rpc
        .create_and_send_transaction_with_event(&[instruction], &payer.pubkey(), &[payer], None)
        .await?;
    let slot = rpc.get_slot().await.unwrap();
    test_indexer.add_compressed_accounts_with_token_data(slot, &event.unwrap().0);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn decrement_counter<R>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    remaining_accounts: &mut RemainingAccounts,
    payer: &Keypair,
    compressed_account: &CompressedAccountWithMerkleContext,
    account_compression_authority: &Pubkey,
    registered_program_pda: &Pubkey,
    light_system_program: &Pubkey,
) -> Result<(), RpcError>
where
    R: RpcConnection + MerkleTreeExt,
{
    let hash = compressed_account.hash().unwrap();
    let merkle_tree_pubkey = compressed_account.merkle_context.merkle_tree_pubkey;

    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(Vec::from(&[hash])),
            Some(Vec::from(&[merkle_tree_pubkey])),
            None,
            None,
            rpc,
        )
        .await;

    let compressed_account = LightAccountMeta::new_mut(
        compressed_account,
        rpc_result.root_indices[0].unwrap(),
        &merkle_tree_pubkey,
        remaining_accounts,
    );

    let inputs = LightInstructionData {
        proof: Some(rpc_result),
        accounts: Some(vec![compressed_account]),
    };
    let inputs = inputs.serialize().unwrap();
    let instruction_data = counter::instruction::DecrementCounter { inputs };

    let cpi_signer = find_cpi_signer(&counter::ID);

    let accounts = counter::accounts::UpdateCounter {
        signer: payer.pubkey(),
        light_system_program: *light_system_program,
        account_compression_program: PROGRAM_ID_ACCOUNT_COMPRESSION,
        account_compression_authority: *account_compression_authority,
        registered_program_pda: *registered_program_pda,
        noop_program: PROGRAM_ID_NOOP,
        self_program: counter::ID,
        cpi_signer,
        system_program: solana_sdk::system_program::id(),
    };

    let remaining_accounts = remaining_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: counter::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    };

    let event = rpc
        .create_and_send_transaction_with_event(&[instruction], &payer.pubkey(), &[payer], None)
        .await?;
    let slot = rpc.get_slot().await.unwrap();
    test_indexer.add_compressed_accounts_with_token_data(slot, &event.unwrap().0);
    Ok(())
}

async fn reset_counter<R>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer<R>,
    remaining_accounts: &mut RemainingAccounts,
    payer: &Keypair,
    compressed_account: &CompressedAccountWithMerkleContext,
    account_compression_authority: &Pubkey,
    registered_program_pda: &Pubkey,
    light_system_program: &Pubkey,
) -> Result<(), RpcError>
where
    R: RpcConnection + MerkleTreeExt,
{
    let hash = compressed_account.hash().unwrap();
    let merkle_tree_pubkey = compressed_account.merkle_context.merkle_tree_pubkey;

    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(Vec::from(&[hash])),
            Some(Vec::from(&[merkle_tree_pubkey])),
            None,
            None,
            rpc,
        )
        .await;

    let compressed_account = LightAccountMeta::new_mut(
        compressed_account,
        rpc_result.root_indices[0].unwrap(),
        &merkle_tree_pubkey,
        remaining_accounts,
    );

    let inputs = LightInstructionData {
        proof: Some(rpc_result),
        accounts: Some(vec![compressed_account]),
    };
    let inputs = inputs.serialize().unwrap();
    let instruction_data = counter::instruction::ResetCounter { inputs };

    let cpi_signer = find_cpi_signer(&counter::ID);

    let accounts = counter::accounts::UpdateCounter {
        signer: payer.pubkey(),
        light_system_program: *light_system_program,
        account_compression_program: PROGRAM_ID_ACCOUNT_COMPRESSION,
        account_compression_authority: *account_compression_authority,
        registered_program_pda: *registered_program_pda,
        noop_program: PROGRAM_ID_NOOP,
        self_program: counter::ID,
        cpi_signer,
        system_program: solana_sdk::system_program::id(),
    };

    let remaining_accounts = remaining_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: counter::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    };

    let event = rpc
        .create_and_send_transaction_with_event(&[instruction], &payer.pubkey(), &[payer], None)
        .await?;
    let slot = rpc.get_slot().await.unwrap();
    test_indexer.add_compressed_accounts_with_token_data(slot, &event.unwrap().0);
    Ok(())
}
