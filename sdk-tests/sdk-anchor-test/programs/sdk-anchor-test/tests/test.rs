#![cfg(feature = "test-sbf")]

use anchor_lang::AnchorDeserialize;
use light_client::indexer::CompressedAccount;
use light_compressed_account::compressed_account::CompressedAccountData;
use light_program_test::{
    indexer::TestIndexerExtensions, program_test::LightProgramTest, AddressWithTree, Indexer,
    ProgramTestConfig,
};
use light_sdk::{
    address::v1::derive_address,
    instruction::{account_meta::CompressedAccountMeta, PackedAccounts, SystemAccountMetaConfig},
};
use light_test_utils::{Rpc, RpcError};
use sdk_anchor_test::{MyCompressedAccount, NestedData};
use serial_test::serial;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    signature::{Keypair, Signature, Signer},
};

#[serial]
#[tokio::test]
async fn test_anchor_sdk_test() {
    let config =
        ProgramTestConfig::new_v2(true, Some(vec![("sdk_anchor_test", sdk_anchor_test::ID)]));
    let mut rpc = LightProgramTest::new(config).await.unwrap();
    let payer = rpc.get_payer().insecure_clone();

    let address_tree_info = rpc.get_address_tree_v1();

    let (address, _) = derive_address(
        &[b"compressed", b"test".as_slice()],
        &address_tree_info.tree,
        &sdk_anchor_test::ID,
    );

    create_compressed_account("test".to_string(), &mut rpc, &payer, &address)
        .await
        .unwrap();

    // Check that it was created correctly.
    let compressed_account = rpc
        .get_compressed_account(address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    let record = &compressed_account.data.as_ref().unwrap().data;
    let record = MyCompressedAccount::deserialize(&mut &record[..]).unwrap();
    assert_eq!(record.nested.one, 1);

    update_compressed_account(
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

    // Test close_compressed_account (non-permanent close - data should be None)
    // Get the account fresh from RPC for the correct type
    let account_to_close = rpc
        .get_compressed_account(address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    close_compressed_account(&mut rpc, &payer, account_to_close)
        .await
        .unwrap();

    // Check that account still exists but data is None
    let closed_account = rpc
        .get_compressed_account(address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    // Account should still exist at the address
    assert_eq!(closed_account.address.unwrap(), address);
    assert_eq!(closed_account.owner, sdk_anchor_test::ID_CONST);

    // Data should be None after close
    assert_eq!(
        closed_account.data,
        Some(CompressedAccountData::default()),
        "Data should be zero after close"
    );

    // Now reinit the closed account to test permanent close
    reinit_closed_account(&mut rpc, &payer, address)
        .await
        .unwrap();

    // Get the reinited account
    let reinited_account = rpc
        .get_compressed_account(address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    // Test close_compressed_account_permanent (account should not exist after)
    close_compressed_account_permanent(&mut rpc, &payer, reinited_account)
        .await
        .unwrap();

    // Check that account no longer exists at address
    // After permanent close, the account should not exist
    let result = rpc.get_compressed_account(address, None).await.unwrap();

    // The query should succeed but return None/null for the account
    assert!(
        result.value.is_none(),
        "Account should not exist after permanent close"
    );
}

async fn create_compressed_account(
    name: String,
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    address: &[u8; 32],
) -> Result<Signature, RpcError> {
    let config = SystemAccountMetaConfig::new(sdk_anchor_test::ID);
    let mut remaining_accounts = PackedAccounts::default();
    remaining_accounts.add_system_accounts(config).unwrap();

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
    let packed_accounts = rpc_result.pack_tree_infos(&mut remaining_accounts);

    let output_tree_index = rpc
        .get_random_state_tree_info()
        .unwrap()
        .pack_output_tree_index(&mut remaining_accounts)
        .unwrap();

    let (remaining_accounts, _, _) = remaining_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: sdk_anchor_test::ID,
        accounts: [
            vec![AccountMeta::new(payer.pubkey(), true)],
            remaining_accounts,
        ]
        .concat(),
        data: {
            use anchor_lang::InstructionData;
            sdk_anchor_test::instruction::CreateCompressedAccount {
                proof: rpc_result.proof,
                address_tree_info: packed_accounts.address_trees[0],
                output_tree_index,
                name,
            }
            .data()
        },
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}

async fn update_compressed_account(
    rpc: &mut LightProgramTest,
    nested_data: NestedData,
    payer: &Keypair,
    mut compressed_account: CompressedAccount,
) -> Result<Signature, RpcError> {
    let mut remaining_accounts = PackedAccounts::default();

    let config = SystemAccountMetaConfig::new(sdk_anchor_test::ID);
    remaining_accounts.add_system_accounts(config).unwrap();
    let hash = compressed_account.hash;

    let rpc_result = rpc
        .get_validity_proof(vec![hash], vec![], None)
        .await?
        .value;

    let packed_tree_accounts = rpc_result
        .pack_tree_infos(&mut remaining_accounts)
        .state_trees
        .unwrap();

    let (remaining_accounts, _, _) = remaining_accounts.to_account_metas();

    let my_compressed_account = MyCompressedAccount::deserialize(
        &mut compressed_account.data.as_mut().unwrap().data.as_slice(),
    )
    .unwrap();
    let instruction = Instruction {
        program_id: sdk_anchor_test::ID,
        accounts: [
            vec![AccountMeta::new(payer.pubkey(), true)],
            remaining_accounts,
        ]
        .concat(),
        data: {
            use anchor_lang::InstructionData;
            sdk_anchor_test::instruction::UpdateCompressedAccount {
                proof: rpc_result.proof,
                my_compressed_account,
                account_meta: CompressedAccountMeta {
                    tree_info: packed_tree_accounts.packed_tree_infos[0],
                    address: compressed_account.address.unwrap(),
                    output_state_tree_index: packed_tree_accounts.output_tree_index,
                },
                nested_data,
            }
            .data()
        },
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}

async fn close_compressed_account(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    mut compressed_account: CompressedAccount,
) -> Result<Signature, RpcError> {
    let mut remaining_accounts = PackedAccounts::default();

    let config = SystemAccountMetaConfig::new(sdk_anchor_test::ID);
    remaining_accounts.add_system_accounts(config)?;
    let hash = compressed_account.hash;

    let rpc_result = rpc
        .get_validity_proof(vec![hash], vec![], None)
        .await?
        .value;

    let packed_tree_accounts = rpc_result
        .pack_tree_infos(&mut remaining_accounts)
        .state_trees
        .unwrap();

    let (remaining_accounts, _, _) = remaining_accounts.to_account_metas();

    let my_compressed_account = MyCompressedAccount::deserialize(
        &mut compressed_account.data.as_mut().unwrap().data.as_slice(),
    )
    .unwrap();

    let instruction = Instruction {
        program_id: sdk_anchor_test::ID,
        accounts: [
            vec![AccountMeta::new(payer.pubkey(), true)],
            remaining_accounts,
        ]
        .concat(),
        data: {
            use anchor_lang::InstructionData;
            sdk_anchor_test::instruction::CloseCompressedAccount {
                proof: rpc_result.proof,
                my_compressed_account,
                account_meta: CompressedAccountMeta {
                    tree_info: packed_tree_accounts.packed_tree_infos[0],
                    address: compressed_account.address.unwrap(),
                    output_state_tree_index: packed_tree_accounts.output_tree_index,
                },
            }
            .data()
        },
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}

async fn reinit_closed_account(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    address: [u8; 32],
) -> Result<Signature, RpcError> {
    let mut remaining_accounts = PackedAccounts::default();

    let config = SystemAccountMetaConfig::new(sdk_anchor_test::ID);
    remaining_accounts.add_system_accounts(config)?;

    // Get closed account
    let closed_account = rpc
        .get_compressed_account(address, None)
        .await
        .unwrap()
        .value
        .unwrap();

    let hash = closed_account.hash;

    let rpc_result = rpc
        .get_validity_proof(vec![hash], vec![], None)
        .await?
        .value;

    let packed_tree_accounts = rpc_result
        .pack_tree_infos(&mut remaining_accounts)
        .state_trees
        .unwrap();

    let (remaining_accounts, _, _) = remaining_accounts.to_account_metas();

    let instruction = Instruction {
        program_id: sdk_anchor_test::ID,
        accounts: [
            vec![AccountMeta::new(payer.pubkey(), true)],
            remaining_accounts,
        ]
        .concat(),
        data: {
            use anchor_lang::InstructionData;
            sdk_anchor_test::instruction::ReinitClosedAccount {
                proof: rpc_result.proof,
                account_meta: CompressedAccountMeta {
                    tree_info: packed_tree_accounts.packed_tree_infos[0],
                    address: closed_account.address.unwrap(),
                    output_state_tree_index: packed_tree_accounts.output_tree_index,
                },
            }
            .data()
        },
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}

async fn close_compressed_account_permanent(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    compressed_account: CompressedAccount,
) -> Result<Signature, RpcError> {
    let mut remaining_accounts = PackedAccounts::default();

    let config = SystemAccountMetaConfig::new(sdk_anchor_test::ID);
    remaining_accounts.add_system_accounts(config)?;
    let hash = compressed_account.hash;

    let rpc_result = rpc
        .get_validity_proof(vec![hash], vec![], None)
        .await?
        .value;

    let packed_tree_accounts = rpc_result
        .pack_tree_infos(&mut remaining_accounts)
        .state_trees
        .unwrap();

    let (remaining_accounts, _, _) = remaining_accounts.to_account_metas();

    // Import CompressedAccountMetaBurn
    use light_sdk::instruction::account_meta::CompressedAccountMetaBurn;

    let instruction = Instruction {
        program_id: sdk_anchor_test::ID,
        accounts: [
            vec![AccountMeta::new(payer.pubkey(), true)],
            remaining_accounts,
        ]
        .concat(),
        data: {
            use anchor_lang::InstructionData;
            sdk_anchor_test::instruction::CloseCompressedAccountPermanent {
                proof: rpc_result.proof,
                account_meta: CompressedAccountMetaBurn {
                    tree_info: packed_tree_accounts.packed_tree_infos[0],
                    address: compressed_account.address.unwrap(),
                },
            }
            .data()
        },
    };

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await
}
