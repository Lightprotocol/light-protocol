#![cfg(feature = "test-sbf")]

use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use light_client::indexer::CompressedAccount;
use light_program_test::{
    indexer::TestIndexerExtensions, program_test::LightProgramTest, AddressWithTree, Indexer,
    ProgramTestConfig,
};
use light_sdk::{
    address::v1::derive_address,
    instruction::{
        account_meta::CompressedAccountMeta, accounts::SystemAccountMetaConfig,
        merkle_context::AddressMerkleContext, pack_accounts::PackedAccounts,
    },
};
use light_test_utils::{Rpc, RpcError};
use sdk_anchor_test::{MyCompressedAccount, NestedData};
use solana_sdk::{
    instruction::Instruction,
    signature::{Keypair, Signature, Signer},
};

#[tokio::test]
async fn test_sdk_test() {
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("sdk_anchor_test", sdk_anchor_test::ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let address_merkle_context = AddressMerkleContext {
        address_merkle_tree_pubkey: rpc.test_accounts.v1_address_trees[0].merkle_tree,
        address_queue_pubkey: rpc.test_accounts.v1_address_trees[0].queue,
    };
    rpc.get_state_merkle_tree_account();

    let (address, _) = derive_address(
        &[b"compressed", b"test".as_slice()],
        &address_merkle_context.address_merkle_tree_pubkey,
        &sdk_anchor_test::ID,
    );

    with_nested_data("test".to_string(), &mut rpc, &payer, &address)
        .await
        .unwrap();

    // Check that it was created correctly.
    let compressed_account = rpc
        .get_compressed_account(address, None)
        .await
        .unwrap()
        .value;

    let record = &compressed_account.data.as_ref().unwrap().data;
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

    let address_merkle_tree_info = rpc.get_address_tree_v1();

    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![AddressWithTree {
                address: *address,
                tree: address_merkle_tree_info.tree,
            }],
            None,
        )
        .await?
        .value;
    let packed_accounts = rpc_result.pack_tree_accounts(&mut remaining_accounts);

    let output_tree_index = rpc
        .get_random_state_tree_info()
        .get_output_tree_index(&mut remaining_accounts)
        .unwrap();

    let (remaining_accounts, _, _) = remaining_accounts.to_account_metas();

    let instruction_data = sdk_anchor_test::instruction::WithNestedData {
        proof: rpc_result.proof,
        address_merkle_context: packed_accounts.address_trees[0],
        name,
        output_tree_index,
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
    mut compressed_account: CompressedAccount,
) -> Result<Signature, RpcError> {
    let mut remaining_accounts = PackedAccounts::default();

    let config = SystemAccountMetaConfig::new(sdk_anchor_test::ID);
    remaining_accounts.add_system_accounts(config);
    let hash = compressed_account.hash;

    let rpc_result = rpc
        .get_validity_proof(vec![hash], vec![], None)
        .await?
        .value;

    let packed_tree_accounts = rpc_result
        .pack_tree_accounts(&mut remaining_accounts)
        .account_trees
        .unwrap();

    let (remaining_accounts, _, _) = remaining_accounts.to_account_metas();

    let my_compressed_account = MyCompressedAccount::deserialize(
        &mut compressed_account.data.as_mut().unwrap().data.as_slice(),
    )
    .unwrap();
    let instruction_data = sdk_anchor_test::instruction::UpdateNestedData {
        proof: rpc_result.proof,
        my_compressed_account,
        account_meta: CompressedAccountMeta {
            tree_info: packed_tree_accounts.packed_tree_infos[0],
            address: compressed_account.address.unwrap(),
            output_tree_index: packed_tree_accounts.output_tree_index,
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
