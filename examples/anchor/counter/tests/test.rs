#![cfg(feature = "test-sbf")]

use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use counter::CounterAccount;
use light_compressed_account::compressed_account::CompressedAccountWithMerkleContext;
use light_program_test::{
    program_test::LightProgramTest, AddressWithTree, Indexer, ProgramTestConfig, RpcConnection,
    RpcError,
};
use light_sdk::{
    address::v1::derive_address,
    instruction::{
        account_meta::CompressedAccountMeta,
        accounts::SystemAccountMetaConfig,
        merkle_context::{pack_address_merkle_context, pack_merkle_context, AddressMerkleContext},
        pack_accounts::PackedAccounts,
    },
};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
};

#[tokio::test]
async fn test_counter() {
    let config = ProgramTestConfig::new(true, Some(vec![("counter", counter::ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let address_merkle_context = AddressMerkleContext {
        address_merkle_tree_pubkey: rpc.test_accounts.v1_address_trees[0].merkle_tree,
        address_queue_pubkey: rpc.test_accounts.v1_address_trees[0].queue,
    };

    let (address, _) = derive_address(
        &[b"counter", payer.pubkey().as_ref()],
        &address_merkle_context.address_merkle_tree_pubkey,
        &counter::ID,
    );

    let output_merkle_tree = rpc.test_accounts.v1_state_trees[0].merkle_tree;
    // Create the counter.
    create_counter(
        &mut rpc,
        &payer,
        &address,
        address_merkle_context,
        output_merkle_tree,
    )
    .await
    .unwrap();

    // Check that it was created correctly.
    let compressed_accounts = rpc
        .get_compressed_accounts_by_owner(&counter::ID, None)
        .await
        .unwrap()
        .value;
    assert_eq!(compressed_accounts.len(), 1);
    let compressed_account: CompressedAccountWithMerkleContext =
        compressed_accounts[0].clone().into();
    let counter = &compressed_account
        .compressed_account
        .data
        .as_ref()
        .unwrap()
        .data;
    let counter = CounterAccount::deserialize(&mut &counter[..]).unwrap();
    assert_eq!(counter.value, 0);

    // Increment the counter.
    increment_counter(&mut rpc, &payer, &compressed_account)
        .await
        .unwrap();

    // Check that it was incremented correctly.
    let compressed_accounts = rpc
        .get_compressed_accounts_by_owner(&counter::ID, None)
        .await
        .unwrap()
        .value;
    assert_eq!(compressed_accounts.len(), 1);
    let compressed_account: CompressedAccountWithMerkleContext =
        compressed_accounts[0].clone().into();
    let counter = &compressed_account
        .compressed_account
        .data
        .as_ref()
        .unwrap()
        .data;
    let counter = CounterAccount::deserialize(&mut &counter[..]).unwrap();
    assert_eq!(counter.value, 1);

    // Decrement the counter.
    decrement_counter(&mut rpc, &payer, &compressed_account)
        .await
        .unwrap();

    // Check that it was decremented correctly.
    let compressed_accounts = rpc
        .get_compressed_accounts_by_owner(&counter::ID, None)
        .await
        .unwrap()
        .value;
    assert_eq!(compressed_accounts.len(), 1);
    let compressed_account: CompressedAccountWithMerkleContext =
        compressed_accounts[0].clone().into();
    let counter = &compressed_account
        .compressed_account
        .data
        .as_ref()
        .unwrap()
        .data;
    let counter = CounterAccount::deserialize(&mut &counter[..]).unwrap();
    assert_eq!(counter.value, 0);

    // Reset the counter.
    reset_counter(&mut rpc, &payer, &compressed_account)
        .await
        .unwrap();

    // Check that it was reset correctly.
    let compressed_accounts = rpc
        .get_compressed_accounts_by_owner(&counter::ID, None)
        .await
        .unwrap()
        .value;
    assert_eq!(compressed_accounts.len(), 1);
    let compressed_account: CompressedAccountWithMerkleContext =
        compressed_accounts[0].clone().into();
    let counter = &compressed_account
        .compressed_account
        .data
        .as_ref()
        .unwrap()
        .data;
    let counter = CounterAccount::deserialize(&mut &counter[..]).unwrap();
    assert_eq!(counter.value, 0);

    // Close the counter.
    close_counter(&mut rpc, &payer, &compressed_account)
        .await
        .unwrap();

    // Check that it was closed correctly (no compressed accounts after closing).
    let compressed_accounts = rpc
        .get_compressed_accounts_by_owner(&counter::ID, None)
        .await
        .unwrap();
    assert_eq!(compressed_accounts.value.len(), 0);
}

async fn create_counter<R>(
    rpc: &mut R,
    payer: &Keypair,
    address: &[u8; 32],
    address_merkle_context: AddressMerkleContext,
    output_merkle_tree: Pubkey,
) -> Result<Signature, RpcError>
where
    R: RpcConnection + Indexer,
{
    let mut remaining_accounts = PackedAccounts::default();
    let config = SystemAccountMetaConfig::new(counter::ID);
    remaining_accounts.add_system_accounts(config);

    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![AddressWithTree {
                tree: address_merkle_context.address_merkle_tree_pubkey,
                address: *address,
            }],
            None,
        )
        .await
        .unwrap();

    let output_merkle_tree_index = remaining_accounts.insert_or_get(output_merkle_tree);
    let packed_address_merkle_context = pack_address_merkle_context(
        &address_merkle_context,
        &mut remaining_accounts,
        rpc_result.value.get_address_root_indices()[0],
    );

    let instruction_data = counter::instruction::CreateCounter {
        proof: rpc_result.value.compressed_proof,
        address_merkle_context: packed_address_merkle_context,
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

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}

#[allow(clippy::too_many_arguments)]
async fn increment_counter<R>(
    rpc: &mut R,
    payer: &Keypair,
    compressed_account: &CompressedAccountWithMerkleContext,
) -> Result<Signature, RpcError>
where
    R: RpcConnection + Indexer,
{
    let mut remaining_accounts = PackedAccounts::default();
    let config = SystemAccountMetaConfig::new(counter::ID);
    remaining_accounts.add_system_accounts(config);

    let hash = compressed_account.hash().unwrap();

    let rpc_result = rpc
        .get_validity_proof(Vec::from(&[hash]), vec![], None)
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

    let account_meta = CompressedAccountMeta {
        merkle_context: packed_merkle_context,
        address: compressed_account.compressed_account.address.unwrap(),
        root_index: Some(rpc_result.value.get_root_indices()[0].unwrap()),
        output_merkle_tree_index: packed_merkle_context.merkle_tree_pubkey_index,
    };

    let instruction_data = counter::instruction::IncrementCounter {
        proof: rpc_result.value.compressed_proof,
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

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}

#[allow(clippy::too_many_arguments)]
async fn decrement_counter<R>(
    rpc: &mut R,
    payer: &Keypair,
    compressed_account: &CompressedAccountWithMerkleContext,
) -> Result<Signature, RpcError>
where
    R: RpcConnection + Indexer,
{
    let mut remaining_accounts = PackedAccounts::default();
    let config = SystemAccountMetaConfig::new(counter::ID);
    remaining_accounts.add_system_accounts(config);

    let hash = compressed_account.hash().unwrap();

    let rpc_result = rpc
        .get_validity_proof(Vec::from(&[hash]), vec![], None)
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

    let account_meta = CompressedAccountMeta {
        merkle_context: packed_merkle_context,
        address: compressed_account.compressed_account.address.unwrap(),
        root_index: rpc_result.value.get_root_indices()[0],
        output_merkle_tree_index: packed_merkle_context.merkle_tree_pubkey_index,
    };

    let instruction_data = counter::instruction::DecrementCounter {
        proof: rpc_result.value.compressed_proof,
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

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}

async fn reset_counter<R>(
    rpc: &mut R,

    payer: &Keypair,
    compressed_account: &CompressedAccountWithMerkleContext,
) -> Result<Signature, RpcError>
where
    R: RpcConnection + Indexer,
{
    let mut remaining_accounts = PackedAccounts::default();
    let config = SystemAccountMetaConfig::new(counter::ID);
    remaining_accounts.add_system_accounts(config);

    let hash = compressed_account.hash().unwrap();

    let rpc_result = rpc
        .get_validity_proof(Vec::from(&[hash]), vec![], None)
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

    let account_meta = CompressedAccountMeta {
        merkle_context: packed_merkle_context,
        address: compressed_account.compressed_account.address.unwrap(),
        root_index: Some(rpc_result.value.get_root_indices()[0].unwrap()),
        output_merkle_tree_index: packed_merkle_context.merkle_tree_pubkey_index,
    };

    let instruction_data = counter::instruction::ResetCounter {
        proof: rpc_result.value.compressed_proof,
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

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}

async fn close_counter<R>(
    rpc: &mut R,
    payer: &Keypair,
    compressed_account: &CompressedAccountWithMerkleContext,
) -> Result<Signature, RpcError>
where
    R: RpcConnection + Indexer,
{
    let mut remaining_accounts = PackedAccounts::default();
    let config = SystemAccountMetaConfig::new(counter::ID);
    remaining_accounts.add_system_accounts(config);

    let hash = compressed_account.hash().unwrap();

    let rpc_result = rpc
        .get_validity_proof(Vec::from(&[hash]), vec![], None)
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

    let account_meta = CompressedAccountMeta {
        merkle_context: packed_merkle_context,
        address: compressed_account.compressed_account.address.unwrap(),
        root_index: Some(rpc_result.value.get_root_indices()[0].unwrap()),
        output_merkle_tree_index: packed_merkle_context.merkle_tree_pubkey_index,
    };

    let instruction_data = counter::instruction::CloseCounter {
        proof: rpc_result.value.compressed_proof,
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

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}
