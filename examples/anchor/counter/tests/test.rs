// #![cfg(feature = "test-sbf")]

use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use counter::CounterAccount;
use light_client::indexer::{CompressedAccount, TreeInfo};
use light_program_test::{
    program_test::LightProgramTest, AddressWithTree, Indexer, ProgramTestConfig, Rpc, RpcError,
};
use light_sdk::{
    address::v1::derive_address,
    instruction::{
        account_meta::CompressedAccountMeta, accounts::SystemAccountMetaConfig,
        pack_accounts::PackedAccounts,
    },
};
use solana_sdk::{
    instruction::Instruction,
    signature::{Keypair, Signature, Signer},
};

#[tokio::test]
async fn test_counter() {
    let config = ProgramTestConfig::new_v2(true, Some(vec![("counter", counter::ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let address_tree_info = rpc.get_address_tree_v1();

    let (address, _) = derive_address(
        &[b"counter", payer.pubkey().as_ref()],
        &address_tree_info.tree,
        &counter::ID,
    );

    // Create the counter.
    create_counter(&mut rpc, &payer, &address, address_tree_info)
        .await
        .unwrap();

    // Check that it was created correctly.
    let compressed_account = rpc
        .get_compressed_account(address, None)
        .await
        .unwrap()
        .value;
    assert_eq!(compressed_account.leaf_index, 0);
    let counter = &compressed_account.data.as_ref().unwrap().data;
    let counter = CounterAccount::deserialize(&mut &counter[..]).unwrap();
    assert_eq!(counter.value, 0);

    // Increment the counter.
    increment_counter(&mut rpc, &payer, &compressed_account)
        .await
        .unwrap();

    // Check that it was incremented correctly.
    let compressed_account = rpc
        .get_compressed_account(address, None)
        .await
        .unwrap()
        .value;

    assert_eq!(compressed_account.leaf_index, 1);
    let counter = &compressed_account.data.as_ref().unwrap().data;
    let counter = CounterAccount::deserialize(&mut &counter[..]).unwrap();
    assert_eq!(counter.value, 1);

    // Decrement the counter.
    decrement_counter(&mut rpc, &payer, &compressed_account)
        .await
        .unwrap();

    // Check that it was decremented correctly.
    let compressed_account = rpc
        .get_compressed_account(address, None)
        .await
        .unwrap()
        .value;

    assert_eq!(compressed_account.leaf_index, 2);

    let counter = &compressed_account.data.as_ref().unwrap().data;
    let counter = CounterAccount::deserialize(&mut &counter[..]).unwrap();
    assert_eq!(counter.value, 0);

    // Reset the counter.
    reset_counter(&mut rpc, &payer, &compressed_account)
        .await
        .unwrap();

    // Check that it was reset correctly.
    let compressed_account = rpc
        .get_compressed_account(address, None)
        .await
        .unwrap()
        .value;
    let counter = &compressed_account.data.as_ref().unwrap().data;
    let counter = CounterAccount::deserialize(&mut &counter[..]).unwrap();
    assert_eq!(counter.value, 0);

    // Close the counter.
    close_counter(&mut rpc, &payer, &compressed_account)
        .await
        .unwrap();

    // Check that it was closed correctly (no compressed accounts after closing).
    let compressed_accounts = rpc
        .get_compressed_accounts_by_owner(&counter::ID, None, None)
        .await
        .unwrap();
    assert_eq!(compressed_accounts.value.items.len(), 0);
}

async fn create_counter<R>(
    rpc: &mut R,
    payer: &Keypair,
    address: &[u8; 32],
    address_tree_info: TreeInfo,
) -> Result<Signature, RpcError>
where
    R: Rpc + Indexer,
{
    let mut remaining_accounts = PackedAccounts::default();
    let config = SystemAccountMetaConfig::new(counter::ID);
    remaining_accounts.add_system_accounts(config);

    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![AddressWithTree {
                tree: address_tree_info.tree,
                address: *address,
            }],
            None,
        )
        .await?
        .value;

    let output_tree_index = rpc
        .get_random_state_tree_info()
        .get_output_tree_index(&mut remaining_accounts)?;
    let packed_address_tree_info = rpc_result
        .pack_tree_accounts(&mut remaining_accounts)
        .packed_new_address_tree_infos[0];

    let instruction_data = counter::instruction::CreateCounter {
        proof: rpc_result.proof,
        address_merkle_context: packed_address_tree_info,
        output_tree_index,
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
    compressed_account: &CompressedAccount,
) -> Result<Signature, RpcError>
where
    R: Rpc + Indexer,
{
    let mut remaining_accounts = PackedAccounts::default();
    let config = SystemAccountMetaConfig::new(counter::ID);
    remaining_accounts.add_system_accounts(config);

    let hash = compressed_account.hash;

    let rpc_result = rpc
        .get_validity_proof(vec![hash], vec![], None)
        .await?
        .value;

    let merkle_context = rpc_result.pack_tree_accounts(&mut remaining_accounts);

    let counter_account =
        CounterAccount::deserialize(&mut compressed_account.data.as_ref().unwrap().data.as_slice())
            .unwrap();

    let account_meta = CompressedAccountMeta {
        tree_info: merkle_context.packed_tree_infos[0],
        address: compressed_account.address.unwrap(),
        output_tree_index: merkle_context.output_tree_index.unwrap(),
    };

    let instruction_data = counter::instruction::IncrementCounter {
        proof: rpc_result.proof,
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
    compressed_account: &CompressedAccount,
) -> Result<Signature, RpcError>
where
    R: Rpc + Indexer,
{
    let mut remaining_accounts = PackedAccounts::default();
    let config = SystemAccountMetaConfig::new(counter::ID);
    remaining_accounts.add_system_accounts(config);

    let hash = compressed_account.hash;

    let rpc_result = rpc
        .get_validity_proof(Vec::from(&[hash]), vec![], None)
        .await?
        .value;

    let packed_merkle_contexts = rpc_result.pack_tree_accounts(&mut remaining_accounts);

    let counter_account =
        CounterAccount::deserialize(&mut compressed_account.data.as_ref().unwrap().data.as_slice())
            .unwrap();

    let account_meta = CompressedAccountMeta {
        tree_info: packed_merkle_contexts.packed_tree_infos[0],
        address: compressed_account.address.unwrap(),
        output_tree_index: packed_merkle_contexts.output_tree_index.unwrap(),
    };

    let instruction_data = counter::instruction::DecrementCounter {
        proof: rpc_result.proof,
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
    compressed_account: &CompressedAccount,
) -> Result<Signature, RpcError>
where
    R: Rpc + Indexer,
{
    let mut remaining_accounts = PackedAccounts::default();
    let config = SystemAccountMetaConfig::new(counter::ID);
    remaining_accounts.add_system_accounts(config);

    let hash = compressed_account.hash;

    let rpc_result = rpc
        .get_validity_proof(Vec::from(&[hash]), vec![], None)
        .await?
        .value;

    let packed_merkle_context = rpc_result.pack_tree_accounts(&mut remaining_accounts);

    let counter_account =
        CounterAccount::deserialize(&mut compressed_account.data.as_ref().unwrap().data.as_slice())
            .unwrap();

    let account_meta = CompressedAccountMeta {
        tree_info: packed_merkle_context.packed_tree_infos[0],
        address: compressed_account.address.unwrap(),
        output_tree_index: packed_merkle_context.output_tree_index.unwrap(),
    };

    let instruction_data = counter::instruction::ResetCounter {
        proof: rpc_result.proof,
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
    compressed_account: &CompressedAccount,
) -> Result<Signature, RpcError>
where
    R: Rpc + Indexer,
{
    let mut remaining_accounts = PackedAccounts::default();
    let config = SystemAccountMetaConfig::new(counter::ID);
    remaining_accounts.add_system_accounts(config);

    let hash = compressed_account.hash;

    let rpc_result = rpc
        .get_validity_proof(Vec::from(&[hash]), vec![], None)
        .await
        .unwrap()
        .value;

    let packed_merkle_contexts = rpc_result.pack_tree_accounts(&mut remaining_accounts);

    let counter_account =
        CounterAccount::deserialize(&mut compressed_account.data.as_ref().unwrap().data.as_slice())
            .unwrap();

    let account_meta = CompressedAccountMeta {
        tree_info: packed_merkle_contexts.packed_tree_infos[0],
        address: compressed_account.address.unwrap(),
        output_tree_index: packed_merkle_contexts.output_tree_index.unwrap(),
    };

    let instruction_data = counter::instruction::CloseCounter {
        proof: rpc_result.proof,
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
