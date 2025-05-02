#![cfg(feature = "test-sbf")]

use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use counter::CounterAccount;
use light_client::{
    indexer::{AddressMerkleTreeAccounts, Indexer, StateMerkleTreeAccounts},
    rpc::merkle_tree::MerkleTreeExt,
};
use light_compressed_account::compressed_account::CompressedAccountWithMerkleContext;
use light_program_test::{
    indexer::{TestIndexer, TestIndexerExtensions},
    test_env::{setup_test_programs_with_accounts_v2, TestAccounts},
    program_test::LightProgramTest,
};
use light_prover_client::gnark::helpers::{spawn_prover, ProverConfig, ProverMode};
use light_sdk::{
    address::v1::derive_address,
    cpi::accounts::SystemAccountMetaConfig,
    instruction::{
        account_meta::CompressedAccountMeta,
        instruction_data::LightInstructionData,
        merkle_context::{pack_address_merkle_context, pack_merkle_context, AddressMerkleContext},
        pack_accounts::PackedAccounts,
    },
};
use light_test_utils::{RpcConnection, RpcError};
use solana_sdk::{
    instruction::Instruction,
    signature::{Keypair, Signer},
};

#[tokio::test]
async fn test_counter() {
    spawn_prover(ProverConfig::default()).await;

    let (mut rpc, env) =
        setup_test_programs_with_accounts_v2(Some(vec![("counter", counter::ID)]))
            .await;
    let payer = rpc.get_payer().insecure_clone();

    let mut test_indexer: TestIndexer = TestIndexer::new(
        Vec::from(&[StateMerkleTreeAccounts {
            merkle_tree: env.v1_state_trees[0].merkle_tree,
            nullifier_queue: env.v1_state_trees[0].nullifier_queue,
            cpi_context: env.v1_state_trees[0].cpi_context,
        }]),
        Vec::from(&[AddressMerkleTreeAccounts {
            merkle_tree: env.v1_address_trees[0].merkle_tree,
            queue: env.v1_address_trees[0].queue,
        }]),
        payer.insecure_clone(),
        env.protocol.group_pda,
        None,
    )
    .await;

    let address_merkle_context = AddressMerkleContext {
        address_merkle_tree_pubkey: env.v1_address_trees[0].merkle_tree,
        address_queue_pubkey: env.v1_address_trees[0].queue,
    };

    let (address, _) = derive_address(
        &[b"counter", payer.pubkey().as_ref()],
        &address_merkle_context.address_merkle_tree_pubkey,
        &counter::ID,
    );

    // Create the counter.
    create_counter(&mut rpc, &mut test_indexer, &env, &payer, &address)
        .await
        .unwrap();

    // Check that it was created correctly.
    let compressed_accounts = test_indexer
        .get_compressed_accounts_by_owner_v2(&counter::ID)
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
    increment_counter(&mut rpc, &mut test_indexer, &payer, compressed_account)
        .await
        .unwrap();

    // Check that it was incremented correctly.
    let compressed_accounts = test_indexer
        .get_compressed_accounts_by_owner_v2(&counter::ID)
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
    decrement_counter(&mut rpc, &mut test_indexer, &payer, compressed_account)
        .await
        .unwrap();

    // Check that it was decremented correctly.
    let compressed_accounts = test_indexer
        .get_compressed_accounts_by_owner_v2(&counter::ID)
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
    reset_counter(&mut rpc, &mut test_indexer, &payer, compressed_account)
        .await
        .unwrap();

    // Check that it was reset correctly.
    let compressed_accounts = test_indexer
        .get_compressed_accounts_by_owner_v2(&counter::ID)
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

    // Close the counter.
    close_counter(&mut rpc, &mut test_indexer, &payer, compressed_account)
        .await
        .unwrap();

    // Check that it was closed correctly (no compressed accounts after closing).
    let compressed_accounts = test_indexer
        .get_compressed_accounts_by_owner_v2(&counter::ID)
        .await
        .unwrap();
    assert_eq!(compressed_accounts.len(), 0);
}

#[allow(clippy::too_many_arguments)]
async fn create_counter<R>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer,
    env: &TestAccounts,
    payer: &Keypair,
    address: &[u8; 32],
) -> Result<(), RpcError>
where
    R: RpcConnection + MerkleTreeExt,
{
    let mut remaining_accounts = PackedAccounts::default();
    let config = SystemAccountMetaConfig::new(counter::ID);
    remaining_accounts.add_system_accounts(config);

    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            None,
            None,
            Some(&[*address]),
            Some(vec![env.v1_address_trees[0].merkle_tree]),
            rpc,
        )
        .await
        .unwrap();

    let address_merkle_context = AddressMerkleContext {
        address_merkle_tree_pubkey: env.v1_address_trees[0].merkle_tree,
        address_queue_pubkey: env.v1_address_trees[0].queue,
    };

    let output_merkle_tree_index = remaining_accounts.insert_or_get(env.v1_state_trees[0].merkle_tree);
    let packed_address_merkle_context = pack_address_merkle_context(
        &address_merkle_context,
        &mut remaining_accounts,
        rpc_result.address_root_indices[0],
    );

    let light_ix_data = LightInstructionData {
        proof: Some(rpc_result.proof),
        new_addresses: Some(vec![packed_address_merkle_context]),
    };

    let instruction_data = counter::instruction::CreateCounter {
        light_ix_data,
        output_merkle_tree_index,
    };

    let accounts = counter::accounts::GenericAnchorAccounts {
        signer: payer.pubkey(),
    };

    let (remaining_accounts_metas, _, _) = remaining_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: counter::ID,
        accounts: [
            accounts.to_account_metas(Some(true)),
            remaining_accounts_metas,
        ]
        .concat(),
        data: instruction_data.data(),
    };

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

#[allow(clippy::too_many_arguments)]
async fn increment_counter<R>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer,
    payer: &Keypair,
    compressed_account: &CompressedAccountWithMerkleContext,
) -> Result<(), RpcError>
where
    R: RpcConnection + MerkleTreeExt,
{
    let mut remaining_accounts = PackedAccounts::default();
    let config = SystemAccountMetaConfig::new(counter::ID);
    remaining_accounts.add_system_accounts(config);

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
        .await
        .unwrap();

    let packed_merkle_context =
        pack_merkle_context(&compressed_account.merkle_context, &mut remaining_accounts);

    let counter_account = CounterAccount::deserialize(
        &mut compressed_account
            .compressed_account
            .data
            .as_ref()
            .unwrap()
            .data
            .as_slice(),
    )
    .unwrap();

    let light_ix_data = LightInstructionData {
        proof: Some(rpc_result.proof),
        new_addresses: None,
    };

    let account_meta = CompressedAccountMeta {
        merkle_context: packed_merkle_context,
        address: compressed_account.compressed_account.address.unwrap(),
        root_index: Some(rpc_result.root_indices[0].unwrap()),
        output_merkle_tree_index: packed_merkle_context.merkle_tree_pubkey_index,
    };

    let instruction_data = counter::instruction::IncrementCounter {
        light_ix_data,
        counter_value: counter_account.value,
        account_meta,
    };

    let accounts = counter::accounts::GenericAnchorAccounts {
        signer: payer.pubkey(),
    };

    let (remaining_accounts_metas, _, _) = remaining_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: counter::ID,
        accounts: [
            accounts.to_account_metas(Some(true)),
            remaining_accounts_metas,
        ]
        .concat(),
        data: instruction_data.data(),
    };

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

#[allow(clippy::too_many_arguments)]
async fn decrement_counter<R>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer,
    payer: &Keypair,
    compressed_account: &CompressedAccountWithMerkleContext,
) -> Result<(), RpcError>
where
    R: RpcConnection + MerkleTreeExt,
{
    let mut remaining_accounts = PackedAccounts::default();
    let config = SystemAccountMetaConfig::new(counter::ID);
    remaining_accounts.add_system_accounts(config);

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
        .await
        .unwrap();

    let packed_merkle_context =
        pack_merkle_context(&compressed_account.merkle_context, &mut remaining_accounts);

    let counter_account = CounterAccount::deserialize(
        &mut compressed_account
            .compressed_account
            .data
            .as_ref()
            .unwrap()
            .data
            .as_slice(),
    )
    .unwrap();

    let light_ix_data = LightInstructionData {
        proof: Some(rpc_result.proof),
        new_addresses: None,
    };

    let account_meta = CompressedAccountMeta {
        merkle_context: packed_merkle_context,
        address: compressed_account.compressed_account.address.unwrap(),
        root_index: Some(rpc_result.root_indices[0].unwrap()),
        output_merkle_tree_index: packed_merkle_context.merkle_tree_pubkey_index,
    };

    let instruction_data = counter::instruction::DecrementCounter {
        light_ix_data,
        counter_value: counter_account.value,
        account_meta,
    };

    let accounts = counter::accounts::GenericAnchorAccounts {
        signer: payer.pubkey(),
    };

    let (remaining_accounts_metas, _, _) = remaining_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: counter::ID,
        accounts: [
            accounts.to_account_metas(Some(true)),
            remaining_accounts_metas,
        ]
        .concat(),
        data: instruction_data.data(),
    };

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

async fn reset_counter<R>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer,
    payer: &Keypair,
    compressed_account: &CompressedAccountWithMerkleContext,
) -> Result<(), RpcError>
where
    R: RpcConnection + MerkleTreeExt,
{
    let mut remaining_accounts = PackedAccounts::default();
    let config = SystemAccountMetaConfig::new(counter::ID);
    remaining_accounts.add_system_accounts(config);

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
        .await
        .unwrap();

    let packed_merkle_context =
        pack_merkle_context(&compressed_account.merkle_context, &mut remaining_accounts);

    let counter_account = CounterAccount::deserialize(
        &mut compressed_account
            .compressed_account
            .data
            .as_ref()
            .unwrap()
            .data
            .as_slice(),
    )
    .unwrap();

    let light_ix_data = LightInstructionData {
        proof: Some(rpc_result.proof),
        new_addresses: None,
    };

    let account_meta = CompressedAccountMeta {
        merkle_context: packed_merkle_context,
        address: compressed_account.compressed_account.address.unwrap(),
        root_index: Some(rpc_result.root_indices[0].unwrap()),
        output_merkle_tree_index: packed_merkle_context.merkle_tree_pubkey_index,
    };

    let instruction_data = counter::instruction::ResetCounter {
        light_ix_data,
        counter_value: counter_account.value,
        account_meta,
    };

    let accounts = counter::accounts::GenericAnchorAccounts {
        signer: payer.pubkey(),
    };

    let (remaining_accounts_metas, _, _) = remaining_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: counter::ID,
        accounts: [
            accounts.to_account_metas(Some(true)),
            remaining_accounts_metas,
        ]
        .concat(),
        data: instruction_data.data(),
    };

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

async fn close_counter<R>(
    rpc: &mut R,
    test_indexer: &mut TestIndexer,
    payer: &Keypair,
    compressed_account: &CompressedAccountWithMerkleContext,
) -> Result<(), RpcError>
where
    R: RpcConnection + MerkleTreeExt,
{
    let mut remaining_accounts = PackedAccounts::default();
    let config = SystemAccountMetaConfig::new(counter::ID);
    remaining_accounts.add_system_accounts(config);

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
        .await
        .unwrap();

    let packed_merkle_context =
        pack_merkle_context(&compressed_account.merkle_context, &mut remaining_accounts);

    let counter_account = CounterAccount::deserialize(
        &mut compressed_account
            .compressed_account
            .data
            .as_ref()
            .unwrap()
            .data
            .as_slice(),
    )
    .unwrap();

    let light_ix_data = LightInstructionData {
        proof: Some(rpc_result.proof),
        new_addresses: None,
    };

    let account_meta = CompressedAccountMeta {
        merkle_context: packed_merkle_context,
        address: compressed_account.compressed_account.address.unwrap(),
        root_index: Some(rpc_result.root_indices[0].unwrap()),
        output_merkle_tree_index: packed_merkle_context.merkle_tree_pubkey_index,
    };

    let instruction_data = counter::instruction::CloseCounter {
        light_ix_data,
        counter_value: counter_account.value,
        account_meta,
    };

    let accounts = counter::accounts::GenericAnchorAccounts {
        signer: payer.pubkey(),
    };

    let (remaining_accounts_metas, _, _) = remaining_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: counter::ID,
        accounts: [
            accounts.to_account_metas(Some(true)),
            remaining_accounts_metas,
        ]
        .concat(),
        data: instruction_data.data(),
    };

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
