#![cfg(feature = "test-sbf")]

use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use light_client::{
    indexer::Indexer,
    rpc::{types::ProofRpcResult, RpcConnection},
};
use light_program_test::{
    indexer::{TestIndexer, TestIndexerExtensions},
    test_env::{setup_test_programs_with_accounts_v2, EnvAccounts},
    test_rpc::ProgramTestRpcConnection,
};
use light_sdk::{
    address::v1::derive_address,
    cpi::accounts::SystemAccountMetaConfig,
    instruction::{
        account_meta::CompressedAccountMeta,
        instruction_data::LightInstructionData,
        merkle_context::{pack_address_merkle_context, pack_merkle_context, AddressMerkleContext},
        pack_accounts::PackedAccounts,
    },
    light_compressed_account::compressed_account::CompressedAccountWithMerkleContext,
};
use serial_test::serial;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, signature::Signer};
use test_program::CounterCompressedAccount;

#[serial]
#[tokio::test]
async fn test() {
    // Issues:
    // 1. spawn prover only works in monorepo
    // 2. light prover client (dep of light client) doesn't work with old gnark server
    //     2.1. we could restore ligth-prover-client/src/gnark from version 1.2. on main hide it behind the devenv flag so that light-prover client works with both v1 and v2 prover servers.


    let (mut rpc, env) = setup_test_programs_with_accounts_v2(Some(vec![(
        String::from("test_program"),
        test_program::ID,
    )]))
    .await;
    let payer = rpc.get_payer().insecure_clone();

    let mut test_indexer: TestIndexer<ProgramTestRpcConnection> =
        TestIndexer::init_from_env(&payer, &env, None).await;
    let address_merkle_context = AddressMerkleContext {
        address_merkle_tree_pubkey: env.address_merkle_tree_pubkey,
        address_queue_pubkey: env.address_merkle_tree_queue_pubkey,
    };

    // Calculate address using the derive_address function
    let (address, _) = derive_address(
        &[b"counter", payer.pubkey().as_ref()],
        &address_merkle_context.address_merkle_tree_pubkey,
        &test_program::ID,
    );
    {
        let rpc_result = test_indexer
            .create_proof_for_compressed_accounts(
                None,
                None,
                Some(&[address]),
                Some(vec![env.address_merkle_tree_pubkey]),
                &mut rpc,
            )
            .await
            .unwrap();

        let instruction = create_account_instruction(&env, payer.pubkey(), rpc_result);
        let event = rpc
            .create_and_send_transaction_with_public_event(
                &[instruction],
                &payer.pubkey(),
                &[&payer],
                None,
            )
            .await
            .unwrap();
        let slot = rpc.get_slot().await.unwrap();
        test_indexer.add_compressed_accounts_with_token_data(slot, &event.unwrap().0);
    }
    // Check that it was created correctly.
    let compressed_accounts = test_indexer
        .get_compressed_accounts_by_owner_v2(&test_program::ID)
        .await
        .unwrap();
    assert_eq!(compressed_accounts.len(), 1);
    let compressed_account = &compressed_accounts[0];
    assert_eq!(compressed_account.compressed_account.address, Some(address));
    let counter_account = &compressed_account
        .compressed_account
        .data
        .as_ref()
        .unwrap()
        .data;
    let counter_account = CounterCompressedAccount::deserialize(&mut &counter_account[..]).unwrap();
    assert_eq!(counter_account.owner, payer.pubkey());
    assert_eq!(counter_account.counter, 0);

    // Increment counter.
    {
        let rpc_result = {
            let hash = compressed_account.hash().unwrap();
            let merkle_tree_pubkey = compressed_account.merkle_context.merkle_tree_pubkey;

            test_indexer
                .create_proof_for_compressed_accounts(
                    Some(Vec::from(&[hash])),
                    Some(Vec::from(&[merkle_tree_pubkey])),
                    None,
                    None,
                    &mut rpc,
                )
                .await
                .unwrap()
        };
        let instruction =
            create_increment_instruction(payer.pubkey(), compressed_account, rpc_result);

        let event = rpc
            .create_and_send_transaction_with_public_event(
                &[instruction],
                &payer.pubkey(),
                &[&payer],
                None,
            )
            .await
            .unwrap();
        let slot = rpc.get_slot().await.unwrap();
        test_indexer.add_compressed_accounts_with_token_data(slot, &event.unwrap().0);
    }
    // Check that it was updated correctly.
    let compressed_accounts = test_indexer
        .get_compressed_accounts_by_owner_v2(&test_program::ID)
        .await
        .unwrap();
    assert_eq!(compressed_accounts.len(), 1);
    let compressed_account = &compressed_accounts[0];
    let counter_account = &compressed_account
        .compressed_account
        .data
        .as_ref()
        .unwrap()
        .data;
    let counter_account = CounterCompressedAccount::deserialize(&mut &counter_account[..]).unwrap();
    assert_eq!(counter_account.owner, payer.pubkey());
    assert_eq!(counter_account.counter, 1);

    // Delete account.
    {
        let rpc_result = {
            let hash = compressed_account.hash().unwrap();
            let merkle_tree_pubkey = compressed_account.merkle_context.merkle_tree_pubkey;

            test_indexer
                .create_proof_for_compressed_accounts(
                    Some(Vec::from(&[hash])),
                    Some(Vec::from(&[merkle_tree_pubkey])),
                    None,
                    None,
                    &mut rpc,
                )
                .await
                .unwrap()
        };
        let instruction =
            create_delete_account_instruction(payer.pubkey(), compressed_account, rpc_result);
        let event = rpc
            .create_and_send_transaction_with_public_event(
                &[instruction],
                &payer.pubkey(),
                &[&payer],
                None,
            )
            .await
            .unwrap();
        let slot = rpc.get_slot().await.unwrap();
        test_indexer.add_compressed_accounts_with_token_data(slot, &event.unwrap().0);

        let compressed_accounts = test_indexer
            .get_compressed_accounts_by_owner_v2(&test_program::ID)
            .await
            .unwrap();
        assert_eq!(compressed_accounts.len(), 0);
    }
}

fn create_account_instruction(
    env: &EnvAccounts,
    payer: Pubkey,
    rpc_result: ProofRpcResult,
) -> Instruction {
    let mut remaining_accounts = PackedAccounts::default();
    let config = SystemAccountMetaConfig::new(test_program::ID);
    remaining_accounts.add_system_accounts(config);
    let address_merkle_context = AddressMerkleContext {
        address_merkle_tree_pubkey: env.address_merkle_tree_pubkey,
        address_queue_pubkey: env.address_merkle_tree_queue_pubkey,
    };

    let output_merkle_tree_index = remaining_accounts.insert_or_get(env.merkle_tree_pubkey);
    let packed_address_merkle_context = pack_address_merkle_context(
        &address_merkle_context,
        &mut remaining_accounts,
        rpc_result.address_root_indices[0],
    );

    let light_ix_data = LightInstructionData {
        proof: Some(rpc_result.proof),
        new_addresses: Some(vec![packed_address_merkle_context]),
    };

    let instruction_data = test_program::instruction::Create {
        light_ix_data,
        output_merkle_tree_index,
    };

    let accounts = test_program::accounts::GenericAnchorAccounts { signer: payer };

    let (remaining_accounts_metas, _, _) = remaining_accounts.to_account_metas();

    Instruction {
        program_id: test_program::ID,
        accounts: [
            accounts.to_account_metas(Some(true)),
            remaining_accounts_metas,
        ]
        .concat(),
        data: instruction_data.data(),
    }
}

fn create_increment_instruction(
    payer: Pubkey,
    compressed_account: &CompressedAccountWithMerkleContext,
    rpc_result: ProofRpcResult,
) -> Instruction {
    let mut remaining_accounts = PackedAccounts::default();
    let config = SystemAccountMetaConfig::new(test_program::ID);
    remaining_accounts.add_system_accounts(config);

    let packed_merkle_context =
        pack_merkle_context(&compressed_account.merkle_context, &mut remaining_accounts);

    let counter_account = CounterCompressedAccount::deserialize(
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

    let instruction_data = test_program::instruction::Increment {
        light_ix_data,
        counter_value: counter_account.counter,
        account_meta,
    };

    let accounts = test_program::accounts::GenericAnchorAccounts { signer: payer };

    let (remaining_accounts_metas, _, _) = remaining_accounts.to_account_metas();

    Instruction {
        program_id: test_program::ID,
        accounts: [
            accounts.to_account_metas(Some(true)),
            remaining_accounts_metas,
        ]
        .concat(),
        data: instruction_data.data(),
    }
}

fn create_delete_account_instruction(
    payer: Pubkey,
    compressed_account: &CompressedAccountWithMerkleContext,
    rpc_result: ProofRpcResult,
) -> Instruction {
    let mut remaining_accounts = PackedAccounts::default();
    let config = SystemAccountMetaConfig::new(test_program::ID);
    remaining_accounts.add_system_accounts(config);

    let packed_merkle_context =
        pack_merkle_context(&compressed_account.merkle_context, &mut remaining_accounts);

    let counter_account = CounterCompressedAccount::deserialize(
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

    let instruction_data = test_program::instruction::Delete {
        light_ix_data,
        counter_value: counter_account.counter,
        account_meta,
    };

    let accounts = test_program::accounts::GenericAnchorAccounts { signer: payer };

    let (remaining_accounts_metas, _, _) = remaining_accounts.to_account_metas();

    Instruction {
        program_id: test_program::ID,
        accounts: [
            accounts.to_account_metas(Some(true)),
            remaining_accounts_metas,
        ]
        .concat(),
        data: instruction_data.data(),
    }
}
