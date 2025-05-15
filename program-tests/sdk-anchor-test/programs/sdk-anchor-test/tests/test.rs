#![cfg(feature = "test-sbf")]

use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use light_compressed_account::compressed_account::CompressedAccountWithMerkleContext;
use light_program_test::{
    indexer::TestIndexerExtensions, program_test::LightProgramTest, AddressWithTree, Indexer,
    ProgramTestConfig,
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
use light_test_utils::{RpcConnection, RpcError};
use sdk_anchor_test::{MyCompressedAccount, NestedData};
use solana_sdk::{
    instruction::Instruction,
    signature::{Keypair, Signature, Signer},
};

#[tokio::test]
async fn test_sdk_test() {
    let config = ProgramTestConfig::new(true, Some(vec![("sdk_anchor_test", sdk_anchor_test::ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let address_merkle_context = AddressMerkleContext {
        address_merkle_tree_pubkey: rpc.test_accounts.v1_address_trees[0].merkle_tree,
        address_queue_pubkey: rpc.test_accounts.v1_address_trees[0].queue,
    };
    rpc.get_state_merkle_tree();

    let (address, _) = derive_address(
        &[b"compressed", b"test".as_slice()],
        &address_merkle_context.address_merkle_tree_pubkey,
        &sdk_anchor_test::ID,
    );

    with_nested_data("test".to_string(), &mut rpc, &payer, &address)
        .await
        .unwrap();

    // Check that it was created correctly.
    let compressed_accounts =
        rpc.get_compressed_accounts_with_merkle_context_by_owner(&sdk_anchor_test::ID);
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
        rpc.get_compressed_accounts_with_merkle_context_by_owner(&sdk_anchor_test::ID);
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

async fn with_nested_data(
    name: String,
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    address: &[u8; 32],
) -> Result<Signature, RpcError> {
    let config = SystemAccountMetaConfig::new(sdk_anchor_test::ID);
    let mut remaining_accounts = PackedAccounts::default();
    remaining_accounts.add_system_accounts(config);

    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![AddressWithTree {
                address: *address,
                tree: rpc.test_accounts.v1_address_trees[0].merkle_tree,
            }],
        )
        .await?;

    let address_merkle_context = AddressMerkleContext {
        address_merkle_tree_pubkey: rpc.test_accounts.v1_address_trees[0].merkle_tree,
        address_queue_pubkey: rpc.test_accounts.v1_address_trees[0].queue,
    };
    let output_merkle_tree_index =
        remaining_accounts.insert_or_get(rpc.test_accounts.v1_state_trees[0].merkle_tree);
    let packed_address_merkle_context = pack_address_merkle_context(
        &address_merkle_context,
        &mut remaining_accounts,
        rpc_result.address_root_indices[0],
    );

    let (remaining_accounts, _, _) = remaining_accounts.to_account_metas();

    let instruction_data = sdk_anchor_test::instruction::WithNestedData {
        proof: rpc_result.proof.into(),
        address_merkle_context: packed_address_merkle_context,
        name,
        output_merkle_tree_index,
    };

    let accounts = sdk_anchor_test::accounts::WithNestedData {
        signer: payer.pubkey(),
    };

    let instruction = Instruction {
        program_id: sdk_anchor_test::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}

async fn update_nested_data(
    rpc: &mut LightProgramTest,
    nested_data: NestedData,
    payer: &Keypair,
    mut compressed_account: CompressedAccountWithMerkleContext,
) -> Result<Signature, RpcError> {
    let mut remaining_accounts = PackedAccounts::default();

    let config = SystemAccountMetaConfig::new(sdk_anchor_test::ID);
    remaining_accounts.add_system_accounts(config);
    let hash = compressed_account.hash().unwrap();

    let rpc_result = rpc.get_validity_proof_v2(vec![hash], vec![]).await?;

    let packed_merkle_context =
        pack_merkle_context(&compressed_account.merkle_context, &mut remaining_accounts);
    let (remaining_accounts, _, _) = remaining_accounts.to_account_metas();

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
        proof: rpc_result.proof.into(),
        my_compressed_account,
        account_meta: CompressedAccountMeta {
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

    let instruction = Instruction {
        program_id: sdk_anchor_test::ID,
        accounts: [accounts.to_account_metas(Some(true)), remaining_accounts].concat(),
        data: instruction_data.data(),
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}
