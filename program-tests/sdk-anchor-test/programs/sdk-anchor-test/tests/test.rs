// #![cfg(feature = "test-sbf")]

use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use light_client::{
    indexer::{AddressMerkleTreeAccounts, Indexer, StateMerkleTreeAccounts},
    rpc::merkle_tree::MerkleTreeExt,
};
use light_compressed_account::compressed_account::CompressedAccountWithMerkleContext;
use light_program_test::{
    indexer::{TestIndexer, TestIndexerExtensions},
    test_env::{setup_test_programs_with_accounts_v2, EnvAccounts},
    test_rpc::ProgramTestRpcConnection,
};
use light_prover_client::gnark::helpers::{ProofType, ProverConfig};
use light_sdk::{
    account_meta::InputAccountMeta,
    address::derive_address,
    instruction_data::LightInstructionData,
    merkle_context::{
        pack_address_merkle_context, pack_merkle_context, AddressMerkleContext, CpiAccounts,
    },
    system_accounts::{get_light_system_account_metas, SystemAccountMetaConfig},
};
use light_test_utils::{RpcConnection, RpcError};
use sdk_anchor_test::{MyCompressedAccount, NestedData};
use solana_sdk::{
    instruction::Instruction,
    signature::{Keypair, Signer},
};

#[tokio::test]
async fn test_sdk_test() {
    let (mut rpc, env) = setup_test_programs_with_accounts_v2(Some(vec![(
        String::from("sdk_anchor_test"),
        sdk_anchor_test::ID,
    )]))
    .await;
    let payer = rpc.get_payer().insecure_clone();

    let mut test_indexer: TestIndexer<ProgramTestRpcConnection> = TestIndexer::new(
        vec![StateMerkleTreeAccounts {
            merkle_tree: env.merkle_tree_pubkey,
            nullifier_queue: env.nullifier_queue_pubkey,
            cpi_context: env.cpi_context_account_pubkey,
        }],
        vec![AddressMerkleTreeAccounts {
            merkle_tree: env.address_merkle_tree_pubkey,
            queue: env.address_merkle_tree_queue_pubkey,
        }],
        payer.insecure_clone(),
        env.group_pda,
        Some(ProverConfig {
            circuits: vec![ProofType::Inclusion, ProofType::NonInclusion],
            run_mode: None,
        }),
    )
    .await;

    let address_merkle_context = AddressMerkleContext {
        address_merkle_tree_pubkey: env.address_merkle_tree_pubkey,
        address_queue_pubkey: env.address_merkle_tree_queue_pubkey,
    };

    let (address, _) = derive_address(
        &[b"compressed", b"test"],
        &address_merkle_context,
        &sdk_anchor_test::ID,
    );

    with_nested_data(
        "test".to_string(),
        &mut rpc,
        &mut test_indexer,
        &env,
        &payer,
        &address,
    )
    .await
    .unwrap();

    // Check that it was created correctly.
    let compressed_accounts =
        test_indexer.get_compressed_accounts_with_merkle_context_by_owner(&sdk_anchor_test::ID);
    assert_eq!(compressed_accounts.len(), 1);
    let compressed_account = compressed_accounts[0].clone();
    let record = &compressed_account
        .compressed_account
        .data
        .as_ref()
        .unwrap()
        .data;
    let record = MyCompressedAccount::deserialize(&mut &record[..]).unwrap();
    assert_eq!(record.nested.one, 1);

    update_nested_data(
        &mut rpc,
        &mut test_indexer,
        NestedData {
            one: 2,
            two: 3,
            three: 3,
            four: 4,
            five: 5,
            six: 6,
            seven: 7,
            eight: 8,
            nine: 9,
            ten: 10,
            eleven: 11,
            twelve: 12,
        },
        &payer,
        compressed_account,
    )
    .await
    .unwrap();

    // Check that it was updated correctly.
    let compressed_accounts =
        test_indexer.get_compressed_accounts_with_merkle_context_by_owner(&sdk_anchor_test::ID);
    assert_eq!(compressed_accounts.len(), 1);
    let compressed_account = &compressed_accounts[0];
    let record = &compressed_account
        .compressed_account
        .data
        .as_ref()
        .unwrap()
        .data;
    let record = MyCompressedAccount::deserialize(&mut &record[..]).unwrap();
    assert_eq!(record.nested.one, 2);
}

async fn with_nested_data<R, I>(
    name: String,
    rpc: &mut R,
    test_indexer: &mut I,
    env: &EnvAccounts,
    payer: &Keypair,
    address: &[u8; 32],
) -> Result<(), RpcError>
where
    R: RpcConnection + MerkleTreeExt,
    I: Indexer<R> + TestIndexerExtensions<R>,
{
    let mut remaining_accounts = CpiAccounts::default();

    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            None,
            None,
            Some(&[*address]),
            Some(vec![env.address_merkle_tree_pubkey]),
            rpc,
        )
        .await
        .unwrap();

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

    let instruction_data = sdk_anchor_test::instruction::WithNestedData {
        light_ix_data,
        name,
        output_merkle_tree_index,
    };

    let accounts = sdk_anchor_test::accounts::WithNestedData {
        signer: payer.pubkey(),
    };

    let remaining_accounts = remaining_accounts.to_account_metas();

    let config = SystemAccountMetaConfig::new(sdk_anchor_test::ID);
    let instruction = Instruction {
        program_id: sdk_anchor_test::ID,
        accounts: [
            accounts.to_account_metas(Some(true)),
            get_light_system_account_metas(config),
            remaining_accounts,
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

async fn update_nested_data<R, I>(
    rpc: &mut R,
    test_indexer: &mut I,
    nested_data: NestedData,
    payer: &Keypair,
    mut compressed_account: CompressedAccountWithMerkleContext,
) -> Result<(), RpcError>
where
    R: RpcConnection + MerkleTreeExt,
    I: Indexer<R> + TestIndexerExtensions<R>,
{
    let mut remaining_accounts = CpiAccounts::default();

    let hash = compressed_account.hash().unwrap();
    let merkle_tree_pubkey = compressed_account.merkle_context.merkle_tree_pubkey;

    let rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            Some(vec![hash]),
            Some(vec![merkle_tree_pubkey]),
            None,
            None,
            rpc,
        )
        .await
        .unwrap();

    // let compressed_account = LightAccountMeta::new_mut(
    //     compressed_account,
    //     rpc_result.root_indices[0].unwrap(),
    //     &merkle_tree_pubkey,
    //     remaining_accounts,
    // );
    let packed_merkle_context =
        pack_merkle_context(&compressed_account.merkle_context, &mut remaining_accounts);
    let light_ix_data = LightInstructionData {
        proof: Some(rpc_result.proof),
        new_addresses: None,
    };
    let my_compressed_account = MyCompressedAccount::deserialize(
        &mut compressed_account
            .compressed_account
            .data
            .as_mut()
            .unwrap()
            .data
            .as_slice(),
    )
    .unwrap();
    let instruction_data = sdk_anchor_test::instruction::UpdateNestedData {
        light_ix_data,
        my_compressed_account,
        account_meta: InputAccountMeta {
            merkle_context: packed_merkle_context,
            address: compressed_account.compressed_account.address.unwrap(),
            root_index: rpc_result.root_indices[0],
            output_merkle_tree_index: packed_merkle_context.merkle_tree_pubkey_index,
        },
        nested_data,
    };

    let accounts = sdk_anchor_test::accounts::UpdateNestedData {
        signer: payer.pubkey(),
    };

    let remaining_accounts = remaining_accounts.to_account_metas();
    let config = SystemAccountMetaConfig::new(sdk_anchor_test::ID);

    let instruction = Instruction {
        program_id: sdk_anchor_test::ID,
        accounts: [
            accounts.to_account_metas(Some(true)),
            get_light_system_account_metas(config),
            remaining_accounts,
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
